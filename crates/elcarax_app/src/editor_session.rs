#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use std::path::PathBuf;

use elcarax_commands::CommandHistory;
use elcarax_project::save_project_editor_settings;
use elcarax_scene_model::PropertyEditKind;

use crate::adapter_state::{ADAPTER_DISCONNECT_COMMAND, AdapterState};
use crate::asset_state::{ASSET_REFRESH_COMMAND, ASSET_SCAN_COMMAND, AssetState};
use crate::inspector_state::{InspectorCommandResult, InspectorState};
use crate::project_config::AppProjectConfig;
use crate::project_state::{
    PROJECT_CLOSE_COMMAND, PROJECT_CREATE_COMMAND, PROJECT_OPEN_COMMAND,
    PROJECT_REOPEN_LAST_COMMAND, ProjectCommandResult, ProjectState,
};
use crate::scene_state::{
    SCENE_LOAD_COMMAND, SCENE_SAVE_COMMAND, SceneCommandResult, SceneState, UNSAVED_SCENE_MESSAGE,
};
use crate::viewport_state::{AppViewportState, VIEWPORT_CLEAR_COMMAND};

pub(crate) const SWITCH_PROJECT_COMMANDS: [&str; 3] = [
    PROJECT_OPEN_COMMAND,
    PROJECT_CREATE_COMMAND,
    PROJECT_REOPEN_LAST_COMMAND,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EditorSessionPolicy {
    pub scan_assets_on_open: bool,
}

pub(crate) struct EditorShellContext<'a> {
    pub adapter: &'a mut AdapterState,
    pub viewport: &'a mut AppViewportState,
}

impl EditorShellContext<'_> {
    pub(crate) fn release_adapter_session(&mut self, scene: &mut SceneState) {
        if self.adapter.is_connected() {
            let _ = self
                .adapter
                .execute_command_id(ADAPTER_DISCONNECT_COMMAND, scene);
        }
        self.viewport.on_adapter_disconnected();
        let _ = self
            .viewport
            .execute_command_id(VIEWPORT_CLEAR_COMMAND, self.adapter);
    }
}

pub(crate) struct EditorSessionState {
    pub project: ProjectState,
    pub assets: AssetState,
    pub scene: SceneState,
    pub inspector: InspectorState,
    pub edit_history: CommandHistory,
    policy: EditorSessionPolicy,
}

impl EditorSessionState {
    pub(crate) fn new(project_config: AppProjectConfig) -> Self {
        Self {
            project: ProjectState::new(project_config),
            assets: AssetState::default(),
            scene: SceneState::default(),
            inspector: InspectorState::default(),
            edit_history: CommandHistory::new(),
            policy: EditorSessionPolicy::default(),
        }
    }

    pub(crate) fn session_mut(&mut self) -> EditorSession<'_> {
        EditorSession::new(self, self.policy)
    }
}

pub(crate) struct EditorSession<'a> {
    state: &'a mut EditorSessionState,
    policy: EditorSessionPolicy,
}

impl<'a> EditorSession<'a> {
    fn new(state: &'a mut EditorSessionState, policy: EditorSessionPolicy) -> Self {
        Self { state, policy }
    }

    pub(crate) fn can_leave_project(&self) -> bool {
        !self.state.scene.has_unsaved_changes()
    }

    pub(crate) fn open_project_at(
        &mut self,
        path: PathBuf,
        shell: Option<&mut EditorShellContext<'_>>,
    ) -> ProjectCommandResult {
        if !self.can_leave_project() {
            return self.blocked_project_switch(PROJECT_OPEN_COMMAND);
        }
        let result = self.state.project.open_project_at_path(path);
        self.bind_opened_project(shell);
        result
    }

    pub(crate) fn create_project_at(
        &mut self,
        root: PathBuf,
        shell: Option<&mut EditorShellContext<'_>>,
    ) -> ProjectCommandResult {
        if !self.can_leave_project() {
            return self.blocked_project_switch(PROJECT_CREATE_COMMAND);
        }
        let result = self.state.project.create_project_at_root(root);
        self.bind_opened_project(shell);
        result
    }

    pub(crate) fn close_project(
        &mut self,
        shell: Option<&mut EditorShellContext<'_>>,
    ) -> ProjectCommandResult {
        if !self.can_leave_project() {
            return self.blocked_project_close();
        }
        let result = match self.state.project.execute_command_id(PROJECT_CLOSE_COMMAND) {
            Some(result) => result,
            None => blocked_project_command(PROJECT_CLOSE_COMMAND, "Failed to close project"),
        };
        self.clear_project_dependents(shell);
        result
    }

    pub(crate) fn execute_project_command(
        &mut self,
        command_id: &str,
        shell: Option<&mut EditorShellContext<'_>>,
    ) -> Option<ProjectCommandResult> {
        match command_id {
            PROJECT_CLOSE_COMMAND => Some(self.close_project(shell)),
            PROJECT_OPEN_COMMAND => self
                .state
                .project
                .config()
                .open_path
                .clone()
                .map(|path| self.open_project_at(path, shell)),
            PROJECT_CREATE_COMMAND => self
                .state
                .project
                .config()
                .create_root_or_open_parent()
                .map(|root| self.create_project_at(root, shell)),
            PROJECT_REOPEN_LAST_COMMAND => Some(self.reopen_last_project(shell)),
            _ => {
                let result = self.state.project.execute_command_id(command_id)?;
                self.after_non_switch_project_command(command_id, shell);
                Some(result)
            }
        }
    }

    pub(crate) fn save_scene(&mut self) -> Option<SceneSaveOutcome> {
        let result = self.state.scene.execute_command_id(SCENE_SAVE_COMMAND)?;
        let manifest_warning = if scene_save_succeeded(&result) {
            self.persist_active_scene_to_manifest()
        } else {
            None
        };
        Some(SceneSaveOutcome {
            result,
            manifest_warning,
        })
    }

    pub(crate) fn execute_scene_command(
        &mut self,
        command_id: &str,
    ) -> Option<SceneCommandOutcome> {
        let result = self.state.scene.execute_command_id(command_id)?;
        let manifest_warning = if scene_save_succeeded(&result) {
            self.persist_active_scene_to_manifest()
        } else {
            None
        };
        if command_id == SCENE_LOAD_COMMAND {
            self.state.inspector.on_scene_selection_changed();
        }
        Some(SceneCommandOutcome {
            result,
            manifest_warning,
        })
    }

    pub(crate) fn after_asset_command(&mut self, command_id: &str) {
        if matches!(command_id, ASSET_SCAN_COMMAND | ASSET_REFRESH_COMMAND) {
            self.state
                .project
                .set_scanned_asset_count(self.state.assets.scanned_asset_count());
        }
    }

    pub(crate) fn commit_inspector_property(
        &mut self,
        adapter: &mut AdapterState,
        component_id: elcarax_scene_model::ComponentInstanceId,
        path: &str,
        edit_kind: PropertyEditKind,
        text: &str,
        label: &str,
    ) -> InspectorCommandResult {
        self.state.inspector.commit_inspector_property(
            &mut self.state.scene,
            &mut self.state.edit_history,
            Some(adapter),
            component_id,
            path,
            edit_kind,
            text,
            label,
        )
    }

    pub(crate) fn execute_edit_command(
        &mut self,
        adapter: &mut AdapterState,
        command_id: &str,
    ) -> Option<InspectorCommandResult> {
        self.state.inspector.execute_edit_command_id(
            command_id,
            &mut self.state.scene,
            &mut self.state.edit_history,
            Some(adapter),
        )
    }

    fn reopen_last_project(
        &mut self,
        shell: Option<&mut EditorShellContext<'_>>,
    ) -> ProjectCommandResult {
        if !self.can_leave_project() {
            return self.blocked_project_switch(PROJECT_REOPEN_LAST_COMMAND);
        }
        let result = match self
            .state
            .project
            .execute_command_id(PROJECT_REOPEN_LAST_COMMAND)
        {
            Some(result) => result,
            None => blocked_project_command(
                PROJECT_REOPEN_LAST_COMMAND,
                "Failed to reopen last project",
            ),
        };
        self.bind_opened_project(shell);
        result
    }

    fn bind_opened_project(&mut self, shell: Option<&mut EditorShellContext<'_>>) {
        if let Some(shell) = shell {
            shell.release_adapter_session(&mut self.state.scene);
        }
        if !self.state.project.is_project_loaded() {
            return;
        }
        let project_root = self.state.project.project_root().map(PathBuf::from);
        let asset_root = self.state.project.asset_root().map(PathBuf::from);
        if let (Some(project_root), Some(asset_root)) = (project_root, asset_root) {
            self.state
                .assets
                .on_project_opened(project_root.as_path(), asset_root.as_path());
        }
        if let Some(scene_root) = self.state.project.scene_root() {
            self.state
                .scene
                .on_project_opened(scene_root, self.state.project.active_scene_relative());
            let _ = self.state.scene.execute_command_id(SCENE_LOAD_COMMAND);
        } else {
            self.state.scene.on_project_closed();
        }
        self.state.inspector.on_project_closed();
        self.state.project.set_scanned_asset_count(None);
        self.state.edit_history.clear();
        if self.policy.scan_assets_on_open {
            let _ = self
                .state
                .assets
                .execute_command_id(ASSET_SCAN_COMMAND, true);
            self.after_asset_command(ASSET_SCAN_COMMAND);
        }
    }

    fn clear_project_dependents(&mut self, shell: Option<&mut EditorShellContext<'_>>) {
        if let Some(shell) = shell {
            shell.release_adapter_session(&mut self.state.scene);
        }
        self.state.assets.on_project_closed();
        self.state.scene.on_project_closed();
        self.state.inspector.on_project_closed();
        self.state.edit_history.clear();
    }

    fn after_non_switch_project_command(
        &mut self,
        command_id: &str,
        shell: Option<&mut EditorShellContext<'_>>,
    ) {
        if command_id == PROJECT_CLOSE_COMMAND {
            self.clear_project_dependents(shell);
            return;
        }
        if SWITCH_PROJECT_COMMANDS.contains(&command_id) && self.state.project.is_project_loaded() {
            self.bind_opened_project(shell);
        }
    }

    fn persist_active_scene_to_manifest(&mut self) -> Option<String> {
        let relative_scene = self.state.scene.active_scene_relative_path()?;
        let project = self.state.project.current_project_mut()?;
        project.set_active_scene(Some(relative_scene));
        let project_root = project.root().as_path().to_path_buf();
        let editor = project.editor_settings().clone();
        match save_project_editor_settings(project_root.as_path(), &editor) {
            Ok(()) => None,
            Err(error) => Some(format!(
                "Scene saved, but project manifest update failed: {error}"
            )),
        }
    }

    fn blocked_project_close(&mut self) -> ProjectCommandResult {
        let result = blocked_project_command(PROJECT_CLOSE_COMMAND, UNSAVED_SCENE_MESSAGE);
        self.state.project.record_command_result(result.clone());
        result
    }

    fn blocked_project_switch(&mut self, command_id: &'static str) -> ProjectCommandResult {
        blocked_project_command(command_id, UNSAVED_SCENE_MESSAGE)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneSaveOutcome {
    pub result: SceneCommandResult,
    pub manifest_warning: Option<String>,
}

impl SceneSaveOutcome {
    pub(crate) fn status_message(&self) -> String {
        match &self.manifest_warning {
            Some(warning) => format!("{} | {warning}", self.result.message()),
            None => self.result.message().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneCommandOutcome {
    pub result: SceneCommandResult,
    pub manifest_warning: Option<String>,
}

impl SceneCommandOutcome {
    pub(crate) fn status_message(&self) -> String {
        match &self.manifest_warning {
            Some(warning) => format!("{} | {warning}", self.result.message()),
            None => self.result.message().to_string(),
        }
    }
}

fn blocked_project_command(
    command_id: &'static str,
    message: &'static str,
) -> ProjectCommandResult {
    ProjectCommandResult::blocked(command_id, message)
}

pub(crate) fn scene_save_succeeded(result: &SceneCommandResult) -> bool {
    result.command_id() == SCENE_SAVE_COMMAND && result.message().starts_with("Saved scene to")
}

impl Default for EditorSessionState {
    fn default() -> Self {
        Self::new(AppProjectConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_scene_model::{ObjectSchema, SceneObject, SceneObjectKind, kinds};

    #[test]
    fn project_close_is_blocked_while_scene_is_dirty() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-session-close-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let mut session = EditorSessionState::new(AppProjectConfig {
            create_root: Some(temp.clone()),
            ..AppProjectConfig::default()
        });
        let _ = session
            .session_mut()
            .execute_project_command(PROJECT_CREATE_COMMAND, None);
        session.scene.mark_document_modified();
        let close = session.session_mut().close_project(None);
        assert_eq!(close.message(), UNSAVED_SCENE_MESSAGE);
        assert!(session.project.is_project_loaded());
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn bind_opened_project_clears_edit_history() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-session-history-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let config = AppProjectConfig {
            create_root: Some(temp.clone()),
            ..AppProjectConfig::default()
        };
        let mut session = EditorSessionState::new(config);
        let _ = session
            .session_mut()
            .execute_project_command(PROJECT_CREATE_COMMAND, None);
        assert!(session.project.is_project_loaded());
        assert_eq!(session.edit_history.undo_count(), 0);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn scene_save_persists_active_scene_in_manifest() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-session-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let mut session = EditorSessionState::new(AppProjectConfig {
            create_root: Some(temp.clone()),
            ..AppProjectConfig::default()
        });
        let _ = session
            .session_mut()
            .execute_project_command(PROJECT_CREATE_COMMAND, None);
        if let Some(snapshot) = session.scene.snapshot_mut() {
            let schema = ObjectSchema::new("Marker");
            let object = SceneObject::new(
                "Saved Root",
                SceneObjectKind::new(kinds::WORLD),
                schema.type_id,
            );
            snapshot.add_schema(schema);
            snapshot.add_root_object(object);
        }
        let outcome = session.session_mut().save_scene();
        assert!(outcome.is_some_and(|value| value.manifest_warning.is_none()));
        let reopened =
            elcarax_project::open_project(&elcarax_project::ProjectOpenRequest::new(&temp));
        let reopened = match reopened {
            Ok(value) => value,
            Err(error) => panic!("reopen should succeed: {error}"),
        };
        assert_eq!(
            reopened
                .project
                .editor_settings()
                .active_scene_relative()
                .map(|path| path.display().to_string()),
            Some(elcarax_scene_model::DEFAULT_SCENE_FILENAME.to_string())
        );
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn adapter_backed_inspector_commit_routes_through_adapter_state() {
        use elcarax_adapter_api::{
            AdapterId, AdapterRequestId, AdapterResponseMessage, SetPropertyResponse,
            SetPropertyStatus,
        };
        use elcarax_adapter_host::{AdapterSession, FakeAdapterTransport, response_line};
        use elcarax_scene_model::{
            ComponentInstance, ComponentSchema, components, kinds, PropertyEditKind, PropertyKind,
            PropertyPath, PropertySchema, PropertyValue, SceneName, SceneObject, SceneObjectKind,
            ScenePatch, SceneSnapshot,
        };

        use crate::adapter_state::AdapterState;

        let health_path = match PropertyPath::parse("health") {
            Ok(path) => path,
            Err(error) => panic!("fixture path should parse: {error}"),
        };
        let schema = ObjectSchema::new("Actor").with_component(
            ComponentSchema::new(components::GAMEPLAY, "Gameplay").with_property(
                PropertySchema::editable(health_path.clone(), "Health", PropertyKind::I64),
            ),
        );
        let component = ComponentInstance::new(components::GAMEPLAY, "Gameplay")
            .with_property(health_path.clone(), PropertyValue::I64(100));
        let component_id = component.id;
        let object = SceneObject::new(
            "Fixture Actor",
            SceneObjectKind::new(kinds::CHARACTER),
            schema.type_id,
        )
        .with_component(component);
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Fixture Scene"));
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
        let mut session = EditorSessionState::default();
        session.scene.load_external_snapshot(
            snapshot,
            AdapterId::new("fixture-adapter"),
            "test",
            "Loaded adapter scene",
        );
        assert!(session.scene.select_object(object_id));
        let scene_id = session
            .scene
            .snapshot()
            .map(|value| value.scene_id())
            .unwrap_or_else(|| panic!("adapter fixture scene should be loaded"));
        let response = match response_line(
            AdapterRequestId(1),
            AdapterResponseMessage::SetProperty(SetPropertyResponse {
                status: SetPropertyStatus::Accepted,
                scene_id,
                object_id,
                component_id,
                path: health_path.clone(),
                old_value: Some(PropertyValue::I64(100)),
                confirmed_new_value: Some(PropertyValue::I64(65)),
                patch: Some(ScenePatch::property_updated(
                    object_id,
                    component_id,
                    health_path,
                    PropertyValue::I64(65),
                )),
                diagnostics: Vec::new(),
            }),
        ) {
            Ok(line) => line,
            Err(error) => panic!("response should serialize: {error}"),
        };
        let mut adapter = AdapterState::default();
        adapter.attach_fake_session_for_tests(AdapterSession::new(FakeAdapterTransport::new(
            vec![response],
        )));
        let result = session.session_mut().commit_inspector_property(
            &mut adapter,
            component_id,
            "health",
            PropertyEditKind::Integer,
            "65",
            "Set Fixture Health",
        );
        assert!(result.message().contains("Command:"));
        assert_eq!(session.edit_history.undo_count(), 1);
    }
}
