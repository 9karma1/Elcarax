use elcarax_scene_model::{
    SceneDiagnostic, SceneExpansion, SceneObjectId, SceneSelection, SceneSnapshot,
    demo_scene_snapshot,
};

use crate::scene_display::{SceneUiSnapshot, scene_ui_snapshot};

pub(crate) const SCENE_LOAD_DEMO_COMMAND: &str = "scene.load_demo";
pub(crate) const SCENE_SELECT_ROOT_COMMAND: &str = "scene.select_root";
pub(crate) const SCENE_SELECT_PLAYER_COMMAND: &str = "scene.select_player";
pub(crate) const SCENE_CLEAR_SELECTION_COMMAND: &str = "scene.clear_selection";
pub(crate) const SCENE_EXPAND_ALL_COMMAND: &str = "scene.expand_all";
pub(crate) const SCENE_COLLAPSE_ALL_COMMAND: &str = "scene.collapse_all";
pub(crate) const SCENE_SHOW_SELECTED_COMMAND: &str = "scene.show_selected";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SceneState {
    snapshot: Option<SceneSnapshot>,
    selection: SceneSelection,
    expansion: SceneExpansion,
    diagnostics: Vec<SceneDiagnostic>,
    last_command_result: Option<SceneCommandResult>,
}

impl SceneState {
    pub(crate) fn execute_command_id(&mut self, id: &str) -> Option<SceneCommandResult> {
        let command = SceneCommand::from_id(id)?;
        let result = match command {
            SceneCommand::LoadDemo => self.load_demo(),
            SceneCommand::SelectRoot => self.select_root(),
            SceneCommand::SelectPlayer => self.select_player(),
            SceneCommand::ClearSelection => self.clear_selection(),
            SceneCommand::ExpandAll => self.expand_all(),
            SceneCommand::CollapseAll => self.collapse_all(),
            SceneCommand::ShowSelected => self.show_selected(),
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

    fn load_demo(&mut self) -> SceneCommandResult {
        let snapshot = demo_scene_snapshot();
        let count = snapshot.object_count();
        self.snapshot = Some(snapshot);
        self.selection.clear();
        self.expansion.collapse_all();
        self.diagnostics.clear();
        SceneCommandResult::new(
            SCENE_LOAD_DEMO_COMMAND,
            format!("Loaded demo scene with {count} objects"),
        )
    }

    fn select_root(&mut self) -> SceneCommandResult {
        let Some(snapshot) = &self.snapshot else {
            return SceneCommandResult::new(SCENE_SELECT_ROOT_COMMAND, "No scene loaded");
        };
        let Some(root_id) = snapshot.root_object_id() else {
            return SceneCommandResult::new(SCENE_SELECT_ROOT_COMMAND, "No root object");
        };
        if self.selection.select_existing(snapshot, root_id).is_err() {
            return SceneCommandResult::new(SCENE_SELECT_ROOT_COMMAND, "Root object not found");
        }
        let Ok(object) = snapshot.object(root_id) else {
            return SceneCommandResult::new(SCENE_SELECT_ROOT_COMMAND, "Root object not found");
        };
        SceneCommandResult::new(
            SCENE_SELECT_ROOT_COMMAND,
            format!("Selected {} ({})", object.display_name, object.kind.label()),
        )
    }

    fn select_player(&mut self) -> SceneCommandResult {
        let Some(snapshot) = &self.snapshot else {
            return SceneCommandResult::new(SCENE_SELECT_PLAYER_COMMAND, "No scene loaded");
        };
        let Some(player) = snapshot.object_by_name("Player") else {
            return SceneCommandResult::new(SCENE_SELECT_PLAYER_COMMAND, "Player object not found");
        };
        if self.selection.select_existing(snapshot, player.id).is_err() {
            return SceneCommandResult::new(SCENE_SELECT_PLAYER_COMMAND, "Player object not found");
        }
        SceneCommandResult::new(
            SCENE_SELECT_PLAYER_COMMAND,
            format!("Selected {} ({})", player.display_name, player.kind.label()),
        )
    }

    fn clear_selection(&mut self) -> SceneCommandResult {
        self.selection.clear();
        SceneCommandResult::new(SCENE_CLEAR_SELECTION_COMMAND, "Cleared scene selection")
    }

    fn expand_all(&mut self) -> SceneCommandResult {
        let Some(snapshot) = &self.snapshot else {
            return SceneCommandResult::new(SCENE_EXPAND_ALL_COMMAND, "No scene loaded");
        };
        self.expansion.expand_all(snapshot);
        SceneCommandResult::new(
            SCENE_EXPAND_ALL_COMMAND,
            format!("Expanded {} nodes", self.expansion.len()),
        )
    }

    fn collapse_all(&mut self) -> SceneCommandResult {
        self.expansion.collapse_all();
        SceneCommandResult::new(SCENE_COLLAPSE_ALL_COMMAND, "Collapsed scene tree")
    }

    fn show_selected(&self) -> SceneCommandResult {
        let Some(snapshot) = &self.snapshot else {
            return SceneCommandResult::new(SCENE_SHOW_SELECTED_COMMAND, "No scene loaded");
        };
        let Some(id) = self.selection.selected() else {
            return SceneCommandResult::new(SCENE_SHOW_SELECTED_COMMAND, "No object selected");
        };
        let Ok(object) = snapshot.object(id) else {
            return SceneCommandResult::new(SCENE_SHOW_SELECTED_COMMAND, "No object selected");
        };
        SceneCommandResult::new(
            SCENE_SHOW_SELECTED_COMMAND,
            format!("{} ({})", object.display_name, object.kind.label()),
        )
    }
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            snapshot: None,
            selection: SceneSelection::none(),
            expansion: SceneExpansion::new(),
            diagnostics: Vec::new(),
            last_command_result: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneCommand {
    LoadDemo,
    SelectRoot,
    SelectPlayer,
    ClearSelection,
    ExpandAll,
    CollapseAll,
    ShowSelected,
}

impl SceneCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            SCENE_LOAD_DEMO_COMMAND => Some(Self::LoadDemo),
            SCENE_SELECT_ROOT_COMMAND => Some(Self::SelectRoot),
            SCENE_SELECT_PLAYER_COMMAND => Some(Self::SelectPlayer),
            SCENE_CLEAR_SELECTION_COMMAND => Some(Self::ClearSelection),
            SCENE_EXPAND_ALL_COMMAND => Some(Self::ExpandAll),
            SCENE_COLLAPSE_ALL_COMMAND => Some(Self::CollapseAll),
            SCENE_SHOW_SELECTED_COMMAND => Some(Self::ShowSelected),
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
    use elcarax_ui::{CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey};

    #[test]
    fn scene_load_demo_populates_scene_snapshot() {
        let mut state = SceneState::default();
        let result = state.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        assert_eq!(
            result.as_ref().map(SceneCommandResult::message),
            Some("Loaded demo scene with 10 objects")
        );
        assert_eq!(state.snapshot().map(|scene| scene.object_count()), Some(10));
    }

    #[test]
    fn scene_select_player_updates_scene_selection() {
        let mut state = SceneState::default();
        let _ = state.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let _ = state.execute_command_id(SCENE_SELECT_PLAYER_COMMAND);
        assert!(state.selection.selected().is_some());
    }

    #[test]
    fn scene_clear_selection_clears_scene_selection() {
        let mut state = SceneState::default();
        let _ = state.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let _ = state.execute_command_id(SCENE_SELECT_PLAYER_COMMAND);
        let _ = state.execute_command_id(SCENE_CLEAR_SELECTION_COMMAND);
        assert_eq!(state.selection.selected(), None);
    }

    #[test]
    fn scene_expand_all_updates_expanded_set() {
        let mut state = SceneState::default();
        let _ = state.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let _ = state.execute_command_id(SCENE_EXPAND_ALL_COMMAND);
        assert_eq!(state.expansion.len(), 3);
    }

    #[test]
    fn scene_collapse_all_clears_expanded_set() {
        let mut state = SceneState::default();
        let _ = state.execute_command_id(SCENE_LOAD_DEMO_COMMAND);
        let _ = state.execute_command_id(SCENE_EXPAND_ALL_COMMAND);
        let _ = state.execute_command_id(SCENE_COLLAPSE_ALL_COMMAND);
        assert!(state.expansion.is_empty());
    }

    #[test]
    fn scene_commands_are_discoverable_through_registry() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let id = match CommandId::new(SCENE_LOAD_DEMO_COMMAND) {
            Ok(id) => id,
            Err(error) => panic!("scene command ID should be valid: {error}"),
        };
        assert!(matches!(registry.invoke(&id), CommandResult::Invoked(_)));
    }

    #[test]
    fn command_palette_can_execute_scene_load_demo() {
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
        for character in SCENE_LOAD_DEMO_COMMAND.chars() {
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
        assert_eq!(selected_id.as_str(), SCENE_LOAD_DEMO_COMMAND);
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
