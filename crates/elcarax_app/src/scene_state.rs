#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_adapter_api::AdapterId;
use elcarax_scene_model::{
    SceneDiagnostic, SceneExpansion, SceneObjectId, SceneSelection, SceneSnapshot,
};

use crate::scene_display::{SceneUiSnapshot, scene_ui_snapshot};

pub(crate) const SCENE_LOAD_COMMAND: &str = "scene.load";
pub(crate) const SCENE_CLEAR_COMMAND: &str = "scene.clear";
pub(crate) const SCENE_CLEAR_SELECTION_COMMAND: &str = "scene.clear_selection";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SceneState {
    snapshot: Option<SceneSnapshot>,
    source: SceneSource,
    selection: SceneSelection,
    expansion: SceneExpansion,
    diagnostics: Vec<SceneDiagnostic>,
    last_command_result: Option<SceneCommandResult>,
}

impl SceneState {
    pub(crate) fn execute_command_id(&mut self, id: &str) -> Option<SceneCommandResult> {
        let command = SceneCommand::from_id(id)?;
        let result = match command {
            SceneCommand::Load => self.load(),
            SceneCommand::Clear => self.clear(),
            SceneCommand::ClearSelection => self.clear_selection(),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    #[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
    pub(crate) fn select_object(&mut self, id: SceneObjectId) -> bool {
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        if self.selection.select_existing(snapshot, id).is_err() {
            return false;
        }
        self.last_command_result = None;
        true
    }

    #[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
    pub(crate) fn toggle_expand_row(&mut self, row_index: usize) -> bool {
        let id = match self.ui_snapshot().visible_object_ids[row_index] {
            Some(id) => id,
            None => return false,
        };
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        let Ok(object) = snapshot.object(id) else {
            return false;
        };
        if object.children.is_empty() {
            return false;
        }
        self.expansion.toggle(id);
        self.last_command_result = None;
        true
    }

    pub(crate) fn ui_snapshot(&self) -> SceneUiSnapshot {
        scene_ui_snapshot(
            self.snapshot.as_ref(),
            &self.selection,
            &self.expansion,
            &self.diagnostics,
            self.last_command_result
                .as_ref()
                .map(SceneCommandResult::message),
        )
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn snapshot(&self) -> Option<&SceneSnapshot> {
        self.snapshot.as_ref()
    }

    pub(crate) fn snapshot_mut(&mut self) -> Option<&mut SceneSnapshot> {
        self.snapshot.as_mut()
    }

    #[cfg(test)]
    pub(crate) const fn source(&self) -> &SceneSource {
        &self.source
    }

    #[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
    pub(crate) fn adapter_id(&self) -> Option<&AdapterId> {
        match &self.source {
            SceneSource::Adapter(id) => Some(id),
            #[cfg(test)]
            SceneSource::Local => None,
            SceneSource::None => None,
        }
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn selection(&self) -> &SceneSelection {
        &self.selection
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn expansion(&self) -> &SceneExpansion {
        &self.expansion
    }

    pub(crate) fn record_status(&mut self, command_id: &str, message: impl Into<String>) {
        self.last_command_result = Some(SceneCommandResult::new(command_id, message));
    }

    #[allow(dead_code)]
    pub(crate) fn load_external_snapshot(
        &mut self,
        snapshot: SceneSnapshot,
        adapter_id: AdapterId,
        command_id: &str,
        message: impl Into<String>,
    ) {
        self.snapshot = Some(snapshot);
        self.source = SceneSource::Adapter(adapter_id);
        self.selection.clear();
        self.expansion.collapse_all();
        self.diagnostics.clear();
        self.last_command_result = Some(SceneCommandResult::new(command_id, message));
    }

    fn load(&mut self) -> SceneCommandResult {
        SceneCommandResult::new(SCENE_LOAD_COMMAND, "No scene source configured")
    }

    fn clear(&mut self) -> SceneCommandResult {
        self.snapshot = None;
        self.source = SceneSource::None;
        self.selection.clear();
        self.expansion.collapse_all();
        self.diagnostics.clear();
        SceneCommandResult::new(SCENE_CLEAR_COMMAND, "Cleared loaded scene")
    }

    fn clear_selection(&mut self) -> SceneCommandResult {
        self.selection.clear();
        SceneCommandResult::new(SCENE_CLEAR_SELECTION_COMMAND, "Cleared scene selection")
    }

    #[cfg(test)]
    pub(crate) fn load_fixture_snapshot(&mut self, snapshot: SceneSnapshot) {
        self.snapshot = Some(snapshot);
        self.source = SceneSource::Local;
        self.selection.clear();
        self.expansion.collapse_all();
        self.diagnostics.clear();
        self.last_command_result = None;
    }

    #[cfg(test)]
    fn expand_all(&mut self) -> SceneCommandResult {
        let Some(snapshot) = &self.snapshot else {
            return SceneCommandResult::new("scene.expand_all", "No scene loaded");
        };
        self.expansion.expand_all(snapshot);
        SceneCommandResult::new(
            "scene.expand_all",
            format!("Expanded {} nodes", self.expansion.len()),
        )
    }

    #[cfg(test)]
    fn collapse_all(&mut self) -> SceneCommandResult {
        self.expansion.collapse_all();
        SceneCommandResult::new("scene.collapse_all", "Collapsed scene tree")
    }
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            snapshot: None,
            source: SceneSource::None,
            selection: SceneSelection::none(),
            expansion: SceneExpansion::new(),
            diagnostics: Vec::new(),
            last_command_result: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SceneSource {
    None,
    #[cfg(test)]
    Local,
    #[allow(dead_code)]
    Adapter(AdapterId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneCommand {
    Load,
    Clear,
    ClearSelection,
}

impl SceneCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            SCENE_LOAD_COMMAND => Some(Self::Load),
            SCENE_CLEAR_COMMAND => Some(Self::Clear),
            SCENE_CLEAR_SELECTION_COMMAND => Some(Self::ClearSelection),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneCommandResult {
    command_id: String,
    message: String,
}

impl SceneCommandResult {
    fn new(command_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            command_id: command_id.into(),
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        self.message.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_commands::{CommandId, CommandResult, RegisteredCommand, built_in_commands};
    use elcarax_scene_model::{
        ObjectSchema, PropertyGroup, PropertyKind, PropertyPath, PropertySchema, PropertyValue,
        SceneName, SceneObject, SceneObjectKind,
    };
    use elcarax_ui::{CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey};

    #[test]
    fn scene_load_reports_missing_source_without_fixture_data() {
        let mut state = SceneState::default();
        let result = state.execute_command_id(SCENE_LOAD_COMMAND);
        assert_eq!(
            result.as_ref().map(SceneCommandResult::message),
            Some("No scene source configured")
        );
        assert!(state.snapshot().is_none());
    }

    #[test]
    fn scene_object_selection_updates_scene_selection() {
        let (mut state, object_id) = loaded_fixture_scene();
        assert!(state.select_object(object_id));
        assert!(state.selection.selected().is_some());
    }

    #[test]
    fn scene_clear_selection_clears_scene_selection() {
        let (mut state, object_id) = loaded_fixture_scene();
        assert!(state.select_object(object_id));
        let _ = state.execute_command_id(SCENE_CLEAR_SELECTION_COMMAND);
        assert_eq!(state.selection.selected(), None);
    }

    #[test]
    fn scene_expand_all_updates_expanded_set() {
        let (mut state, _) = loaded_fixture_scene();
        let _ = state.expand_all();
        assert_eq!(state.expansion.len(), 0);
    }

    #[test]
    fn scene_collapse_all_clears_expanded_set() {
        let (mut state, _) = loaded_fixture_scene();
        let _ = state.expand_all();
        let _ = state.collapse_all();
        assert!(state.expansion.is_empty());
    }

    #[test]
    fn scene_commands_are_discoverable_through_registry() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let id = match CommandId::new(SCENE_LOAD_COMMAND) {
            Ok(id) => id,
            Err(error) => panic!("scene command ID should be valid: {error}"),
        };
        assert!(matches!(registry.invoke(&id), CommandResult::Invoked(_)));
    }

    #[test]
    fn command_palette_can_execute_scene_load() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let mut palette = CommandPaletteState::new(
            registry
                .all()
                .into_iter()
                .map(palette_entry_from_command)
                .collect(),
        );
        palette.open();
        for character in SCENE_LOAD_COMMAND.chars() {
            assert_eq!(
                palette.handle_key(KeyboardKey::Character(character.to_string())),
                CommandPaletteAction::None
            );
        }
        assert_eq!(
            palette.handle_key(KeyboardKey::Enter),
            CommandPaletteAction::Execute
        );
        let selected_id = match palette.selected_entry() {
            Some(entry) => match CommandId::new(entry.id.as_str()) {
                Ok(id) => id,
                Err(error) => panic!("selected scene command ID should be valid: {error}"),
            },
            None => panic!("scene command should be selected"),
        };
        assert_eq!(selected_id.as_str(), SCENE_LOAD_COMMAND);
    }

    fn loaded_fixture_scene() -> (SceneState, SceneObjectId) {
        let path = fixture_path("general.name");
        let schema = ObjectSchema::new("Entity").with_property(PropertySchema::editable(
            path.clone(),
            "Name",
            PropertyKind::String,
            PropertyGroup::new("General"),
        ));
        let mut object =
            SceneObject::new("Fixture Object", SceneObjectKind::Character, schema.type_id);
        object.set_property(path, PropertyValue::String("Fixture Object".to_string()));
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Fixture Scene"));
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
        let mut state = SceneState::default();
        state.load_fixture_snapshot(snapshot);
        (state, object_id)
    }

    fn fixture_path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("fixture path should parse: {error}"),
        }
    }

    fn palette_entry_from_command(command: &RegisteredCommand) -> CommandPaletteEntry {
        CommandPaletteEntry::new(
            command.id().as_str(),
            command.name().as_str(),
            command.category().label(),
            command
                .description()
                .map(|description| description.as_str().to_string()),
            command.enabled(),
        )
    }
}
