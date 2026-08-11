#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_commands::CommandHistory;
#[cfg(test)]
use elcarax_scene_model::PropertyValue;
use elcarax_scene_model::{
    InspectorDiagnostic, PropertyEditKind, PropertyPath, PropertyTypeRegistry,
    build_inspector_for_selection, build_inspector_object, parse_property_text,
};

use crate::adapter_state::AdapterState;
use crate::edit_service::{ScenePropertyEdit, SessionEditService};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectorPropertyCommit {
    pub(crate) component_id: elcarax_scene_model::ComponentInstanceId,
    pub(crate) path: String,
    pub(crate) edit_kind: PropertyEditKind,
    pub(crate) extension_type_id: Option<String>,
    pub(crate) text: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct InspectorState {
    suppressed: bool,
    diagnostics: Vec<InspectorDiagnostic>,
    last_command_result: Option<InspectorCommandResult>,
}

impl InspectorState {
    pub(crate) fn execute(
        &mut self,
        command: InspectorCommand,
        scene: &mut SceneState,
        property_types: &PropertyTypeRegistry,
    ) -> InspectorCommandResult {
        let result = match command {
            InspectorCommand::ShowSelected => self.show_selected(scene, property_types),
            InspectorCommand::Clear => self.clear(),
            InspectorCommand::ShowPropertyCount => self.show_property_count(scene, property_types),
        };
        self.last_command_result = Some(result.clone());
        result
    }

    pub(crate) fn execute_edit(
        &mut self,
        command: InspectorEditCommand,
        scene: &mut SceneState,
        history: &mut CommandHistory,
        adapter: Option<&mut AdapterState>,
        property_types: &PropertyTypeRegistry,
    ) -> InspectorCommandResult {
        let result = match command {
            InspectorEditCommand::Undo => {
                SessionEditService::undo(scene, history, adapter, property_types)
            }
            InspectorEditCommand::Redo => {
                SessionEditService::redo(scene, history, adapter, property_types)
            }
        };
        self.last_command_result = Some(result.clone());
        result
    }

    pub(crate) fn commit_inspector_property(
        &mut self,
        scene: &mut SceneState,
        history: &mut CommandHistory,
        adapter: Option<&mut AdapterState>,
        request: InspectorPropertyCommit,
        property_types: &PropertyTypeRegistry,
    ) -> InspectorCommandResult {
        self.suppressed = false;
        let InspectorPropertyCommit {
            component_id,
            path: raw_path,
            edit_kind,
            extension_type_id,
            text,
            label,
        } = request;
        let path = match PropertyPath::parse(&raw_path) {
            Ok(path) => path,
            Err(error) => {
                return self.edit_error(
                    scene,
                    EDIT_SET_PROPERTY_COMMAND,
                    format!("Invalid property path: {error}"),
                );
            }
        };
        let value = match parse_property_text(
            &path,
            edit_kind,
            extension_type_id.as_deref(),
            text.as_str(),
            property_types,
        ) {
            Ok(value) => value,
            Err(error) => {
                return self.edit_error(scene, EDIT_SET_PROPERTY_COMMAND, error.message());
            }
        };
        let result = SessionEditService::commit_property_result(
            scene,
            history,
            adapter,
            ScenePropertyEdit::new(component_id, path, value, label),
            property_types,
        );
        self.last_command_result = Some(result.clone());
        result
    }

    pub(crate) fn on_scene_selection_changed(&mut self) {
        self.suppressed = false;
        self.last_command_result = None;
    }

    pub(crate) fn ui_snapshot(
        &self,
        scene: &SceneState,
        property_types: &PropertyTypeRegistry,
    ) -> InspectorUiSnapshot {
        self.ui_snapshot_at(scene, 0, property_types)
    }

    pub(crate) fn ui_snapshot_at(
        &self,
        scene: &SceneState,
        scroll_offset: usize,
        property_types: &PropertyTypeRegistry,
    ) -> InspectorUiSnapshot {
        inspector_ui_snapshot_with_scroll(
            scene,
            self.suppressed,
            self.last_command_result
                .as_ref()
                .map(InspectorCommandResult::message),
            scroll_offset,
            property_types,
        )
    }

    fn show_selected(
        &mut self,
        scene: &SceneState,
        property_types: &PropertyTypeRegistry,
    ) -> InspectorCommandResult {
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
        match build_inspector_object(snapshot, selected, property_types) {
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

    fn show_property_count(
        &mut self,
        scene: &SceneState,
        property_types: &PropertyTypeRegistry,
    ) -> InspectorCommandResult {
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
        let count = match build_inspector_for_selection(
            snapshot,
            scene.selection().selected(),
            property_types,
        ) {
            Ok(inspector) => inspector.property_count(),
            Err(_) => 0,
        };
        InspectorCommandResult::new(
            INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND,
            format!("{count} properties"),
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn set_fixture_property(
        &mut self,
        scene: &mut SceneState,
        history: &mut CommandHistory,
        command_id: &str,
        component_id: elcarax_scene_model::ComponentInstanceId,
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
        match SessionEditService::commit_property(
            scene,
            history,
            None,
            ScenePropertyEdit::new(component_id, path, new_value, label),
            &PropertyTypeRegistry::default(),
        ) {
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
pub(crate) enum InspectorCommand {
    ShowSelected,
    Clear,
    ShowPropertyCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectorEditCommand {
    Undo,
    Redo,
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_commands::CommandHistory;
    use elcarax_scene_model::{
        ComponentInstance, ComponentSchema, ComponentTypeName, ObjectSchema, PropertyKind,
        PropertySchema, PropertyValue, SceneObject, SceneObjectKind, SceneSnapshot, components,
        kinds,
    };

    #[test]
    fn fixture_property_edit_updates_inspector_and_undo_stack() {
        let (mut scene, mut history, mut inspector, component_id) = fixture();
        let result = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_health",
            component_id,
            "health",
            PropertyValue::I64(75),
            "Set Health",
        );
        assert!(result.message().contains("75") || result.message().contains("Command:"));
        assert_eq!(history.undo_count(), 1);
    }

    #[test]
    fn undo_restores_old_property_value() {
        let (mut scene, mut history, mut inspector, component_id) = fixture();
        let _ = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_health",
            component_id,
            "health",
            PropertyValue::I64(75),
            "Set Health",
        );
        let result = inspector.execute_edit(
            InspectorEditCommand::Undo,
            &mut scene,
            &mut history,
            None,
            &PropertyTypeRegistry::default(),
        );
        assert_eq!(result.message(), "Command: edit.undo");
        assert_eq!(health(&scene), PropertyValue::I64(100));
    }

    #[test]
    fn redo_restores_new_property_value() {
        let (mut scene, mut history, mut inspector, component_id) = fixture();
        let _ = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_health",
            component_id,
            "health",
            PropertyValue::I64(75),
            "Set Health",
        );
        let _ = inspector.execute_edit(
            InspectorEditCommand::Undo,
            &mut scene,
            &mut history,
            None,
            &PropertyTypeRegistry::default(),
        );
        let result = inspector.execute_edit(
            InspectorEditCommand::Redo,
            &mut scene,
            &mut history,
            None,
            &PropertyTypeRegistry::default(),
        );
        assert_eq!(result.message(), "Command: edit.redo");
        assert_eq!(health(&scene), PropertyValue::I64(75));
    }

    #[test]
    fn failed_edit_without_selection_does_not_push_undo_entry() {
        let (mut scene, mut history, mut inspector, component_id) = fixture();
        let _ = scene.execute(crate::scene_state::SceneCommand::ClearSelection);
        let result = inspector.set_fixture_property(
            &mut scene,
            &mut history,
            "test.set_health",
            component_id,
            "health",
            PropertyValue::I64(75),
            "Set Health",
        );
        assert!(result.message().contains("Diagnostic:"));
        assert_eq!(history.undo_count(), 0);
    }

    fn fixture() -> (
        SceneState,
        CommandHistory,
        InspectorState,
        elcarax_scene_model::ComponentInstanceId,
    ) {
        let path = match PropertyPath::parse("health") {
            Ok(path) => path,
            Err(error) => panic!("path should parse: {error}"),
        };
        let schema = ObjectSchema::new("Actor").with_component(
            ComponentSchema::new(components::GAMEPLAY, "Gameplay").with_property(
                PropertySchema::editable(path.clone(), "Health", PropertyKind::I64),
            ),
        );
        let component = ComponentInstance::new(components::GAMEPLAY, "Gameplay")
            .with_property(path, PropertyValue::I64(100));
        let component_id = component.id;
        let object = SceneObject::new(
            "Actor",
            SceneObjectKind::new(kinds::CHARACTER),
            schema.type_id,
        )
        .with_component(component);
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::empty();
        snapshot.add_schema(schema);
        let _ = snapshot.add_object(
            None,
            0,
            object,
            &elcarax_scene_model::PropertyTypeRegistry::default(),
        );
        let mut scene = SceneState::default();
        scene.load_fixture_snapshot(snapshot);
        assert!(scene.select_object(object_id));
        (
            scene,
            CommandHistory::new(),
            InspectorState::default(),
            component_id,
        )
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
        let component = match actor.component_by_type(&ComponentTypeName::new(components::GAMEPLAY))
        {
            Some(component) => component,
            None => panic!("gameplay component should exist"),
        };
        let path = match PropertyPath::parse("health") {
            Ok(path) => path,
            Err(error) => panic!("path should parse: {error}"),
        };
        match component.property(&path) {
            Some(value) => value.clone(),
            None => panic!("health should exist"),
        }
    }
}
