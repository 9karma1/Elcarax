use elcarax_scene_model::{
    InspectorDiagnostic, build_inspector_for_selection, build_inspector_object,
};

use crate::inspector_display::{
    InspectorUiSnapshot, inspector_summary_for_object, inspector_ui_snapshot,
};
use crate::scene_state::{SCENE_SELECT_PLAYER_COMMAND, SCENE_SELECT_ROOT_COMMAND, SceneState};

pub(crate) const INSPECTOR_SHOW_SELECTED_COMMAND: &str = "inspector.show_selected";
pub(crate) const INSPECTOR_CLEAR_COMMAND: &str = "inspector.clear";
pub(crate) const INSPECTOR_INSPECT_PLAYER_COMMAND: &str = "inspector.inspect_player";
pub(crate) const INSPECTOR_INSPECT_ROOT_COMMAND: &str = "inspector.inspect_root";
pub(crate) const INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND: &str = "inspector.show_property_count";

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct InspectorState {
    suppressed: bool,
    diagnostics: Vec<InspectorDiagnostic>,
    last_command_result: Option<InspectorCommandResult>,
}

impl InspectorState {
    pub(crate) fn execute_command_id(
        &mut self,
        id: &str,
        scene: &mut SceneState,
    ) -> Option<InspectorCommandResult> {
        let command = InspectorCommand::from_id(id)?;
        let result = match command {
            InspectorCommand::ShowSelected => self.show_selected(scene),
            InspectorCommand::Clear => self.clear(),
            InspectorCommand::InspectPlayer => self.inspect_player(scene),
            InspectorCommand::InspectRoot => self.inspect_root(scene),
            InspectorCommand::ShowPropertyCount => self.show_property_count(scene),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    pub(crate) fn on_scene_selection_changed(&mut self) {
        self.suppressed = false;
        self.last_command_result = None;
    }

    pub(crate) fn ui_snapshot(&self, scene: &SceneState) -> InspectorUiSnapshot {
        inspector_ui_snapshot(
            scene,
            self.suppressed,
            self.last_command_result
                .as_ref()
                .map(InspectorCommandResult::message),
        )
    }

    fn show_selected(&mut self, scene: &SceneState) -> InspectorCommandResult {
        self.suppressed = false;
        let Some(snapshot) = scene.snapshot() else {
            return InspectorCommandResult::new(
                INSPECTOR_SHOW_SELECTED_COMMAND,
                InspectorDiagnostic::NoSceneLoaded.message(),
            );
        };
        let Some(selected) = scene.selection().selected() else {
            return InspectorCommandResult::new(
                INSPECTOR_SHOW_SELECTED_COMMAND,
                InspectorDiagnostic::NoObjectSelected.message(),
            );
        };
        match build_inspector_object(snapshot, selected) {
            Ok(inspector) => InspectorCommandResult::new(
                INSPECTOR_SHOW_SELECTED_COMMAND,
                inspector_summary_for_object(&inspector),
            ),
            Err(diagnostic) => {
                InspectorCommandResult::new(INSPECTOR_SHOW_SELECTED_COMMAND, diagnostic.message())
            }
        }
    }

    fn clear(&mut self) -> InspectorCommandResult {
        self.suppressed = true;
        InspectorCommandResult::new(INSPECTOR_CLEAR_COMMAND, "Cleared inspector view")
    }

    fn inspect_player(&mut self, scene: &mut SceneState) -> InspectorCommandResult {
        self.suppressed = false;
        if scene
            .execute_command_id(SCENE_SELECT_PLAYER_COMMAND)
            .is_none()
        {
            return InspectorCommandResult::new(
                INSPECTOR_INSPECT_PLAYER_COMMAND,
                "Player object not found",
            );
        }
        self.show_selected(scene)
    }

    fn inspect_root(&mut self, scene: &mut SceneState) -> InspectorCommandResult {
        self.suppressed = false;
        if scene
            .execute_command_id(SCENE_SELECT_ROOT_COMMAND)
            .is_none()
        {
            return InspectorCommandResult::new(
                INSPECTOR_INSPECT_ROOT_COMMAND,
                "Root object not found",
            );
        }
        self.show_selected(scene)
    }

    fn show_property_count(&mut self, scene: &SceneState) -> InspectorCommandResult {
        let Some(snapshot) = scene.snapshot() else {
            return InspectorCommandResult::new(
                INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND,
                InspectorDiagnostic::NoSceneLoaded.message(),
            );
        };
        if self.suppressed {
            return InspectorCommandResult::new(
                INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND,
                "0 properties",
            );
        }
        let count = match build_inspector_for_selection(snapshot, scene.selection().selected()) {
            Ok(inspector) => inspector.property_count(),
            Err(_) => 0,
        };
        InspectorCommandResult::new(
            INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND,
            format!("{count} properties"),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectorCommandResult {
    command_id: String,
    message: String,
}

impl InspectorCommandResult {
    fn new(command_id: &str, message: impl Into<String>) -> Self {
        Self {
            command_id: command_id.to_string(),
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        self.message.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectorCommand {
    ShowSelected,
    Clear,
    InspectPlayer,
    InspectRoot,
    ShowPropertyCount,
}

impl InspectorCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            INSPECTOR_SHOW_SELECTED_COMMAND => Some(Self::ShowSelected),
            INSPECTOR_CLEAR_COMMAND => Some(Self::Clear),
            INSPECTOR_INSPECT_PLAYER_COMMAND => Some(Self::InspectPlayer),
            INSPECTOR_INSPECT_ROOT_COMMAND => Some(Self::InspectRoot),
            INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND => Some(Self::ShowPropertyCount),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use elcarax_commands::{CommandId, CommandResult, built_in_commands};

    use super::*;
    use crate::scene_state::{SCENE_LOAD_DEMO_COMMAND, SCENE_SELECT_PLAYER_COMMAND, SceneState};

    #[test]
    fn inspector_show_selected_without_selection_returns_no_object_state() {
        let mut scene = SceneState::default();
        let mut inspector = InspectorState::default();
        let _ = scene.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let result = inspector.execute_command_id(INSPECTOR_SHOW_SELECTED_COMMAND, &mut scene);
        assert!(result.is_some());
        let snapshot = inspector.ui_snapshot(&scene);
        assert!(!snapshot.has_selection);
    }

    #[test]
    fn scene_select_player_followed_by_show_selected_shows_player_rows() {
        let mut scene = SceneState::default();
        let mut inspector = InspectorState::default();
        let _ = scene.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let _ = scene.execute_command_id(SCENE_SELECT_PLAYER_COMMAND);
        inspector.on_scene_selection_changed();
        let snapshot = inspector.ui_snapshot(&scene);
        assert_eq!(snapshot.object_name, "Player");
        assert_eq!(snapshot.property_count, 7);
    }

    #[test]
    fn inspector_inspect_player_selects_and_inspects_player() {
        let mut scene = SceneState::default();
        let mut inspector = InspectorState::default();
        let _ = scene.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let result = inspector.execute_command_id(INSPECTOR_INSPECT_PLAYER_COMMAND, &mut scene);
        assert!(result.is_some());
        let snapshot = inspector.ui_snapshot(&scene);
        assert_eq!(snapshot.object_name, "Player");
    }

    #[test]
    fn inspector_clear_clears_inspector_view() {
        let mut scene = SceneState::default();
        let mut inspector = InspectorState::default();
        let _ = scene.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let _ = scene.execute_command_id(SCENE_SELECT_PLAYER_COMMAND);
        let _ = inspector.execute_command_id(INSPECTOR_CLEAR_COMMAND, &mut scene);
        let snapshot = inspector.ui_snapshot(&scene);
        assert!(!snapshot.has_selection);
    }

    #[test]
    fn property_count_command_reports_expected_count() {
        let mut scene = SceneState::default();
        let mut inspector = InspectorState::default();
        let _ = scene.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let _ = scene.execute_command_id(SCENE_SELECT_PLAYER_COMMAND);
        let result =
            inspector.execute_command_id(INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND, &mut scene);
        assert_eq!(
            result.map(|value| value.message().to_string()),
            Some("7 properties".to_string())
        );
    }

    #[test]
    fn inspector_commands_are_discoverable_through_registry() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let id = match CommandId::new(INSPECTOR_SHOW_SELECTED_COMMAND) {
            Ok(id) => id,
            Err(error) => panic!("inspector command ID should be valid: {error}"),
        };
        assert!(matches!(registry.invoke(&id), CommandResult::Invoked(_)));
    }
}
