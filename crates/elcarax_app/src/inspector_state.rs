#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_commands::CommandHistory;
#[cfg(test)]
use elcarax_scene_model::PropertyValue;
use elcarax_scene_model::{
    InspectorDiagnostic, PropertyEditKind, PropertyPath, build_inspector_for_selection,
    build_inspector_object, parse_property_text,
};

use crate::adapter_state::AdapterState;
use crate::edit_service::SessionEditService;
use crate::inspector_display::{
    InspectorUiSnapshot, inspector_summary_for_object, inspector_ui_snapshot_with_scroll,
};
use crate::scene_state::SceneState;

pub(crate) const INSPECTOR_SHOW_SELECTED_COMMAND: &str = "inspector.show_selected";
pub(crate) const INSPECTOR_CLEAR_COMMAND: &str = "inspector.clear";
pub(crate) const INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND: &str = "inspector.show_property_count";
pub(crate) const EDIT_UNDO_COMMAND: &str = "edit.undo";
pub(crate) const EDIT_REDO_COMMAND: &str = "edit.redo";
pub(crate) const EDIT_SET_PROPERTY_COMMAND: &str = "edit.set_property";

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
            InspectorCommand::ShowPropertyCount => self.show_property_count(scene),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    pub(crate) fn execute_edit_command_id(
        &mut self,
        id: &str,
        scene: &mut SceneState,
        history: &mut CommandHistory,
        adapter: Option<&mut AdapterState>,
    ) -> Option<InspectorCommandResult> {
        let command = InspectorEditCommand::from_id(id)?;
        let result = match command {
            InspectorEditCommand::Undo => SessionEditService::undo(scene, history, adapter),
            InspectorEditCommand::Redo => SessionEditService::redo(scene, history, adapter),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn commit_inspector_property(
        &mut self,
        scene: &mut SceneState,
        history: &mut CommandHistory,
        adapter: Option<&mut AdapterState>,
        path: &str,
        edit_kind: PropertyEditKind,
        text: &str,
        label: &str,
    ) -> InspectorCommandResult {
        self.suppressed = false;
        let path = match PropertyPath::parse(path) {
            Ok(path) => path,
            Err(error) => {
                return self.edit_error(
                    scene,
                    EDIT_SET_PROPERTY_COMMAND,
                    format!("Invalid property path: {error}"),
                );
            }
        };
        let value = match parse_property_text(&path, edit_kind, text) {
            Ok(value) => value,
            Err(error) => {
                return self.edit_error(scene, EDIT_SET_PROPERTY_COMMAND, error.message());
            }
        };
        let result = SessionEditService::commit_property_result(
            scene, history, adapter, &path, value, label,
        );
        self.last_command_result = Some(result.clone());
        result
    }

    pub(crate) fn on_scene_selection_changed(&mut self) {
        self.suppressed = false;
        self.last_command_result = None;
    }

    pub(crate) fn set_last_command_result(&mut self, result: InspectorCommandResult) {
        self.suppressed = false;
        self.last_command_result = Some(result);
    }

    pub(crate) fn ui_snapshot(&self, scene: &SceneState) -> InspectorUiSnapshot {
        self.ui_snapshot_at(scene, 0)
    }

    pub(crate) fn ui_snapshot_at(
        &self,
        scene: &SceneState,
        scroll_offset: usize,
    ) -> InspectorUiSnapshot {
        inspector_ui_snapshot_with_scroll(
            scene,
            self.suppressed,
            self.last_command_result
                .as_ref()
                .map(InspectorCommandResult::message),
            scroll_offset,
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

    pub(crate) fn on_project_closed(&mut self) {
        self.suppressed = true;
        self.diagnostics.clear();
        self.last_command_result = None;
    }

    fn clear(&mut self) -> InspectorCommandResult {
        self.on_project_closed();
        InspectorCommandResult::new(INSPECTOR_CLEAR_COMMAND, "Cleared inspector view")
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

    #[cfg(test)]
    fn set_fixture_property(
        &mut self,
        scene: &mut SceneState,
        history: &mut CommandHistory,
        command_id: &str,
        path: &str,
        new_value: PropertyValue,
        label: &str,
    ) -> InspectorCommandResult {
        self.suppressed = false;
        let path = match PropertyPath::parse(path) {
            Ok(path) => path,
            Err(error) => {
                return self.edit_error(
                    scene,
                    command_id,
                    format!("Invalid property path: {error}"),
                );
            }
        };
        match SessionEditService::commit_property(scene, history, None, &path, new_value, label) {
            Ok(message) => self.edit_success(scene, command_id, message),
            Err(error) => self.edit_error(scene, command_id, error),
        }
    }

    #[cfg(test)]
    fn edit_success(
        &mut self,
        scene: &mut SceneState,
        command_id: &str,
        message: impl Into<String>,
    ) -> InspectorCommandResult {
        let message = message.into();
        scene.mark_document_modified();
        scene.record_status(command_id, message.clone());
        let result = InspectorCommandResult::new(command_id, message);
        self.last_command_result = Some(result.clone());
        result
    }

    fn edit_error(
        &mut self,
        scene: &mut SceneState,
        command_id: &str,
        message: impl Into<String>,
    ) -> InspectorCommandResult {
        let message = message.into();
        let diagnostic = if message.starts_with("Diagnostic:") {
            message
        } else {
            format!("Diagnostic: {message}")
        };
        scene.record_status(command_id, diagnostic.clone());
        let result = InspectorCommandResult::new(command_id, diagnostic);
        self.last_command_result = Some(result.clone());
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectorCommandResult {
    command_id: String,
    message: String,
}

impl InspectorCommandResult {
    pub(crate) fn new(command_id: &str, message: impl Into<String>) -> Self {
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
    ShowPropertyCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectorEditCommand {
    Undo,
    Redo,
}

impl InspectorCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            INSPECTOR_SHOW_SELECTED_COMMAND => Some(Self::ShowSelected),
            INSPECTOR_CLEAR_COMMAND => Some(Self::Clear),
            INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND => Some(Self::ShowPropertyCount),
            _ => None,
        }
    }
}

impl InspectorEditCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            EDIT_UNDO_COMMAND => Some(Self::Undo),
            EDIT_REDO_COMMAND => Some(Self::Redo),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_commands::CommandHistory;
    use elcarax_scene_model::{
        ObjectSchema, PropertyGroup, PropertyKind, PropertySchema, PropertyValue, SceneObject,
        SceneObjectKind, SceneSnapshot,
    };

    #[test]
    fn fixture_property_edit_updates_inspector_and_undo_stack() {
        let (mut scene, mut history, mut inspector) = fixture();
        let result = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_health",
            "gameplay.health",
            PropertyValue::I64(75),
            "Set Health",
        );
        assert!(result.message().contains("75") || result.message().contains("Command:"));
        assert_eq!(history.undo_count(), 1);
    }

    #[test]
    fn undo_restores_old_property_value() {
        let (mut scene, mut history, mut inspector) = fixture();
        let _ = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_health",
            "gameplay.health",
            PropertyValue::I64(75),
            "Set Health",
        );
        let result =
            inspector.execute_edit_command_id(EDIT_UNDO_COMMAND, &mut scene, &mut history, None);
        assert_eq!(
            result.map(|result| result.message().to_string()),
            Some("Command: edit.undo".to_string())
        );
        assert_eq!(health(&scene), PropertyValue::I64(100));
    }

    #[test]
    fn redo_restores_new_property_value() {
        let (mut scene, mut history, mut inspector) = fixture();
        let _ = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_health",
            "gameplay.health",
            PropertyValue::I64(75),
            "Set Health",
        );
        let _ =
            inspector.execute_edit_command_id(EDIT_UNDO_COMMAND, &mut scene, &mut history, None);
        let result =
            inspector.execute_edit_command_id(EDIT_REDO_COMMAND, &mut scene, &mut history, None);
        assert_eq!(
            result.map(|result| result.message().to_string()),
            Some("Command: edit.redo".to_string())
        );
        assert_eq!(health(&scene), PropertyValue::I64(75));
    }

    #[test]
    fn failed_edit_without_selection_does_not_push_undo_entry() {
        let (mut scene, mut history, mut inspector) = fixture();
        let _ = scene.execute_command_id(crate::scene_state::SCENE_CLEAR_SELECTION_COMMAND);
        let result = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_health",
            "gameplay.health",
            PropertyValue::I64(75),
            "Set Health",
        );
        assert!(result.message().contains("Diagnostic:"));
        assert_eq!(history.undo_count(), 0);
    }

    fn fixture() -> (SceneState, CommandHistory, InspectorState) {
        let path = match PropertyPath::parse("gameplay.health") {
            Ok(path) => path,
            Err(error) => panic!("path should parse: {error}"),
        };
        let schema = ObjectSchema::new("Actor").with_property(PropertySchema::editable(
            path.clone(),
            "Health",
            PropertyKind::I64,
            PropertyGroup::new("Gameplay"),
        ));
        let mut object = SceneObject::new("Actor", SceneObjectKind::Character, schema.type_id);
        object.set_property(path, PropertyValue::I64(100));
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::empty();
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
        let mut scene = SceneState::default();
        scene.load_fixture_snapshot(snapshot);
        assert!(scene.select_object(object_id));
        (scene, CommandHistory::new(), InspectorState::default())
    }

    fn health(scene: &SceneState) -> PropertyValue {
        let snapshot = match scene.snapshot() {
            Some(snapshot) => snapshot,
            None => panic!("scene should be loaded"),
        };
        let actor = match snapshot.object_by_name("Actor") {
            Some(actor) => actor,
            None => panic!("actor should exist"),
        };
        let path = match PropertyPath::parse("gameplay.health") {
            Ok(path) => path,
            Err(error) => panic!("path should parse: {error}"),
        };
        match actor.property(&path) {
            Some(value) => value.clone(),
            None => panic!("health should exist"),
        }
    }
}
