#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

#[cfg(test)]
use elcarax_commands::SetScenePropertyCommand;
use elcarax_commands::{CommandContext, CommandHistory, RedoCommand, UndoCommand};
use elcarax_scene_model::{
    InspectorDiagnostic, build_inspector_for_selection, build_inspector_object,
};
#[cfg(test)]
use elcarax_scene_model::{PropertyPath, PropertyValue, prepare_property_change};

use crate::inspector_display::{
    InspectorUiSnapshot, inspector_summary_for_object, inspector_ui_snapshot,
};
use crate::scene_state::SceneState;

pub(crate) const INSPECTOR_SHOW_SELECTED_COMMAND: &str = "inspector.show_selected";
pub(crate) const INSPECTOR_CLEAR_COMMAND: &str = "inspector.clear";
pub(crate) const INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND: &str = "inspector.show_property_count";
pub(crate) const EDIT_UNDO_COMMAND: &str = "edit.undo";
pub(crate) const EDIT_REDO_COMMAND: &str = "edit.redo";

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
    ) -> Option<InspectorCommandResult> {
        let command = InspectorEditCommand::from_id(id)?;
        let result = match command {
            InspectorEditCommand::Undo => self.undo(scene, history),
            InspectorEditCommand::Redo => self.redo(scene, history),
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
        let result = execute_set_property(scene, history, &path, new_value, label);
        match result {
            Ok(message) => self.edit_success(scene, command_id, message),
            Err(error) => self.edit_error(scene, command_id, error),
        }
    }

    fn undo(
        &mut self,
        scene: &mut SceneState,
        history: &mut CommandHistory,
    ) -> InspectorCommandResult {
        let Some(snapshot) = scene.snapshot_mut() else {
            return self.edit_error(scene, EDIT_UNDO_COMMAND, "No scene loaded");
        };
        let mut context = CommandContext { scene: snapshot };
        match UndoCommand::apply(history, &mut context) {
            Ok(Some(_)) => self.edit_success(scene, EDIT_UNDO_COMMAND, "Command: edit.undo"),
            Ok(None) => self.edit_error(scene, EDIT_UNDO_COMMAND, "Nothing to undo"),
            Err(error) => self.edit_error(scene, EDIT_UNDO_COMMAND, error.to_string()),
        }
    }

    fn redo(
        &mut self,
        scene: &mut SceneState,
        history: &mut CommandHistory,
    ) -> InspectorCommandResult {
        let Some(snapshot) = scene.snapshot_mut() else {
            return self.edit_error(scene, EDIT_REDO_COMMAND, "No scene loaded");
        };
        let mut context = CommandContext { scene: snapshot };
        match RedoCommand::apply(history, &mut context) {
            Ok(Some(_)) => self.edit_success(scene, EDIT_REDO_COMMAND, "Command: edit.redo"),
            Ok(None) => self.edit_error(scene, EDIT_REDO_COMMAND, "Nothing to redo"),
            Err(error) => self.edit_error(scene, EDIT_REDO_COMMAND, error.to_string()),
        }
    }

    fn edit_success(
        &mut self,
        scene: &mut SceneState,
        command_id: &str,
        message: impl Into<String>,
    ) -> InspectorCommandResult {
        let message = message.into();
        scene.record_status(command_id, message.clone());
        InspectorCommandResult::new(command_id, message)
    }

    fn edit_error(
        &mut self,
        scene: &mut SceneState,
        command_id: &str,
        message: impl Into<String>,
    ) -> InspectorCommandResult {
        let message = message.into();
        let diagnostic = format!("Diagnostic: {message}");
        scene.record_status(command_id, diagnostic.clone());
        InspectorCommandResult::new(command_id, diagnostic)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectorCommandResult {
    command_id: String,
    message: String,
}

#[cfg(test)]
fn execute_set_property(
    scene: &mut SceneState,
    history: &mut CommandHistory,
    path: &PropertyPath,
    new_value: PropertyValue,
    label: &str,
) -> Result<String, String> {
    let Some(snapshot) = scene.snapshot() else {
        return Err(InspectorDiagnostic::NoSceneLoaded.message().to_string());
    };
    let Some(object_id) = scene.selection().selected() else {
        return Err(InspectorDiagnostic::NoObjectSelected.message().to_string());
    };
    let change = prepare_property_change(snapshot, object_id, path, &new_value)
        .map_err(|error| error.message())?;
    let old_label = change.old_value.display_label();
    let new_label = change.new_value.display_label();
    let Some(snapshot) = scene.snapshot_mut() else {
        return Err(InspectorDiagnostic::NoSceneLoaded.message().to_string());
    };
    let mut context = CommandContext { scene: snapshot };
    history
        .execute(
            Box::new(SetScenePropertyCommand::new(change, label.to_string())),
            &mut context,
        )
        .map_err(|error| error.to_string())?;
    Ok(format!("Command: {label} | {old_label} -> {new_label}"))
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
    ShowPropertyCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectorEditCommand {
    Undo,
    Redo,
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

#[cfg(test)]
mod tests {
    use elcarax_commands::{CommandHistory, CommandId, CommandResult, built_in_commands};
    use elcarax_scene_model::{
        ObjectSchema, PropertyGroup, PropertyKind, PropertySchema, SceneName, SceneObject,
        SceneObjectId, SceneObjectKind, SceneSnapshot,
    };

    use super::*;

    #[test]
    fn inspector_show_selected_without_selection_returns_no_object_state() {
        let (mut scene, _) = selected_fixture_scene(false);
        let mut inspector = InspectorState::default();
        let result = inspector.execute_command_id(INSPECTOR_SHOW_SELECTED_COMMAND, &mut scene);
        assert!(result.is_some());
        let snapshot = inspector.ui_snapshot(&scene);
        assert!(!snapshot.has_selection);
    }

    #[test]
    fn scene_selection_followed_by_show_selected_shows_rows() {
        let (scene, _) = selected_fixture_scene(true);
        let mut inspector = InspectorState::default();
        inspector.on_scene_selection_changed();
        let snapshot = inspector.ui_snapshot(&scene);
        assert_eq!(snapshot.object_name, "Fixture Actor");
        assert_eq!(snapshot.property_count, 2);
    }

    #[test]
    fn inspector_clear_clears_inspector_view() {
        let (mut scene, _) = selected_fixture_scene(true);
        let mut inspector = InspectorState::default();
        let _ = inspector.execute_command_id(INSPECTOR_CLEAR_COMMAND, &mut scene);
        let snapshot = inspector.ui_snapshot(&scene);
        assert!(!snapshot.has_selection);
    }

    #[test]
    fn property_count_command_reports_expected_count() {
        let (mut scene, _) = selected_fixture_scene(true);
        let mut inspector = InspectorState::default();
        let result =
            inspector.execute_command_id(INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND, &mut scene);
        assert_eq!(
            result.map(|value| value.message().to_string()),
            Some("2 properties".to_string())
        );
    }

    #[test]
    fn inspector_commands_are_discoverable_through_registry() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let id = match CommandId::new(INSPECTOR_CLEAR_COMMAND) {
            Ok(id) => id,
            Err(error) => panic!("inspector command ID should be valid: {error}"),
        };
        assert!(matches!(registry.invoke(&id), CommandResult::Invoked(_)));
    }

    #[test]
    fn fixture_property_edit_updates_inspector_and_undo_stack() {
        let (mut scene, _) = selected_fixture_scene(true);
        let mut inspector = InspectorState::default();
        let mut history = CommandHistory::new();
        let result = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_fixture_health",
            "gameplay.health",
            PropertyValue::I64(75),
            "Set Fixture Health",
        );
        assert!(result.message().contains("Set Fixture Health"));
        assert_eq!(history.undo_count(), 1);
        let snapshot = inspector.ui_snapshot(&scene);
        assert_eq!(row_value(&snapshot, "Health"), "75  [Set]");
    }

    #[test]
    fn undo_restores_old_property_value() {
        let (mut scene, _) = selected_fixture_scene(true);
        let mut inspector = InspectorState::default();
        let mut history = CommandHistory::new();
        let _ = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_fixture_health",
            "gameplay.health",
            PropertyValue::I64(75),
            "Set Fixture Health",
        );
        let result = inspector.execute_edit_command_id(EDIT_UNDO_COMMAND, &mut scene, &mut history);
        assert_eq!(
            result.map(|value| value.message().to_string()),
            Some("Command: edit.undo".to_string())
        );
        let snapshot = inspector.ui_snapshot(&scene);
        assert_eq!(row_value(&snapshot, "Health"), "100  [Set]");
    }

    #[test]
    fn redo_restores_new_property_value() {
        let (mut scene, _) = selected_fixture_scene(true);
        let mut inspector = InspectorState::default();
        let mut history = CommandHistory::new();
        let _ = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_fixture_health",
            "gameplay.health",
            PropertyValue::I64(75),
            "Set Fixture Health",
        );
        let _ = inspector.execute_edit_command_id(EDIT_UNDO_COMMAND, &mut scene, &mut history);
        let result = inspector.execute_edit_command_id(EDIT_REDO_COMMAND, &mut scene, &mut history);
        assert_eq!(
            result.map(|value| value.message().to_string()),
            Some("Command: edit.redo".to_string())
        );
        let snapshot = inspector.ui_snapshot(&scene);
        assert_eq!(row_value(&snapshot, "Health"), "75  [Set]");
    }

    #[test]
    fn failed_edit_without_selection_does_not_push_undo_entry() {
        let mut scene = SceneState::default();
        let mut inspector = InspectorState::default();
        let mut history = CommandHistory::new();
        let result = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_fixture_health",
            "gameplay.health",
            PropertyValue::I64(75),
            "Set Fixture Health",
        );
        assert_eq!(history.undo_count(), 0);
        assert!(result.message().contains("No scene loaded"));
    }

    fn selected_fixture_scene(select_object: bool) -> (SceneState, SceneObjectId) {
        let health_path = fixture_path("gameplay.health");
        let name_path = fixture_path("general.name");
        let schema = ObjectSchema::new("Actor")
            .with_property(PropertySchema::editable(
                name_path.clone(),
                "Name",
                PropertyKind::String,
                PropertyGroup::new("General"),
            ))
            .with_property(PropertySchema::editable(
                health_path.clone(),
                "Health",
                PropertyKind::I64,
                PropertyGroup::new("Gameplay"),
            ));
        let mut object =
            SceneObject::new("Fixture Actor", SceneObjectKind::Character, schema.type_id);
        object.set_property(
            name_path,
            PropertyValue::String("Fixture Actor".to_string()),
        );
        object.set_property(health_path, PropertyValue::I64(100));
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Fixture Scene"));
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
        let mut scene = SceneState::default();
        scene.load_fixture_snapshot(snapshot);
        if select_object {
            assert!(scene.select_object(object_id));
        }
        (scene, object_id)
    }

    fn fixture_path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("fixture path should parse: {error}"),
        }
    }

    fn row_value(snapshot: &InspectorUiSnapshot, label: &str) -> String {
        snapshot
            .row_labels
            .iter()
            .position(|row_label| row_label == label)
            .and_then(|index| snapshot.row_values.get(index))
            .cloned()
            .unwrap_or_else(|| "<missing>".to_string())
    }
}
