#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_assets::{AssetId, AssetIndex, AssetScan, AssetSelection};

use crate::asset_display::{AssetUiSnapshot, asset_ui_snapshot};

pub(crate) const ASSET_SCAN_COMMAND: &str = "asset.scan";
pub(crate) const ASSET_CLEAR_SELECTION_COMMAND: &str = "asset.clear_selection";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetState {
    index: AssetIndex,
    selection: AssetSelection,
    last_scan: Option<AssetScan>,
    last_command_result: Option<AssetCommandResult>,
}

impl AssetState {
    pub(crate) fn execute_command_id(
        &mut self,
        id: &str,
        project_loaded: bool,
    ) -> Option<AssetCommandResult> {
        let command = AssetCommand::from_id(id)?;
        let result = match command {
            AssetCommand::Scan => self.scan(project_loaded),
            AssetCommand::ClearSelection => self.clear_selection(),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    #[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
    pub(crate) fn select_asset(&mut self, id: AssetId) -> bool {
        if self.index.find(id).is_none() {
            return false;
        }
        self.selection.select(id);
        self.last_command_result = None;
        true
    }

    #[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
    pub(crate) fn select_row(&mut self, row_index: usize) -> bool {
        let Some(record) = self.index.records().get(row_index) else {
            return false;
        };
        self.select_asset(record.id)
    }

    pub(crate) fn ui_snapshot(&self) -> AssetUiSnapshot {
        asset_ui_snapshot(
            &self.index,
            &self.selection,
            self.last_scan.as_ref(),
            self.last_command_result
                .as_ref()
                .map(AssetCommandResult::message),
        )
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn index(&self) -> &AssetIndex {
        &self.index
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn selection(&self) -> &AssetSelection {
        &self.selection
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn kind_summary(&self) -> String {
        self.index.kind_summary()
    }

    fn scan(&mut self, project_loaded: bool) -> AssetCommandResult {
        if !project_loaded {
            return AssetCommandResult::new(ASSET_SCAN_COMMAND, "No project open");
        }
        AssetCommandResult::new(ASSET_SCAN_COMMAND, "No asset root loaded")
    }

    #[cfg(test)]
    fn load_fixture_scan(&mut self, scan: AssetScan) {
        self.index = scan.index.clone();
        self.last_scan = Some(scan);
    }

    fn clear_selection(&mut self) -> AssetCommandResult {
        self.selection.clear();
        AssetCommandResult::new(ASSET_CLEAR_SELECTION_COMMAND, "Cleared asset selection")
    }
}

impl Default for AssetState {
    fn default() -> Self {
        Self {
            index: AssetIndex::new(),
            selection: AssetSelection::none(),
            last_scan: None,
            last_command_result: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetCommand {
    Scan,
    ClearSelection,
}

impl AssetCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            ASSET_SCAN_COMMAND => Some(Self::Scan),
            ASSET_CLEAR_SELECTION_COMMAND => Some(Self::ClearSelection),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetCommandResult {
    command_id: String,
    message: String,
}

impl AssetCommandResult {
    fn new(command_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            command_id: command_id.into(),
            message: message.into(),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn command_id(&self) -> &str {
        self.command_id.as_str()
    }

    pub(crate) fn message(&self) -> &str {
        self.message.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_assets::{AssetKind, AssetRecord, stable_asset_id};
    use elcarax_commands::{CommandId, CommandResult, RegisteredCommand, built_in_commands};
    use elcarax_ui::{CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey};
    use std::path::PathBuf;

    fn project_loaded() -> bool {
        true
    }

    #[test]
    fn fixture_scan_kind_summary_is_stable() {
        let mut state = AssetState::default();
        state.load_fixture_scan(fixture_scan());
        assert_eq!(state.kind_summary(), "Model=1, Scene=1, Text=1");
    }

    #[test]
    fn asset_scan_without_root_reports_empty_state() {
        let mut state = AssetState::default();
        let result = state.execute_command_id(ASSET_SCAN_COMMAND, project_loaded());
        assert_eq!(
            result.as_ref().map(AssetCommandResult::command_id),
            Some(ASSET_SCAN_COMMAND)
        );
        assert_eq!(
            result.as_ref().map(AssetCommandResult::message),
            Some("No asset root loaded")
        );
        assert_eq!(state.index().len(), 0);
    }

    #[test]
    fn asset_scan_without_project_returns_clear_result() {
        let mut state = AssetState::default();
        let result = state.execute_command_id(ASSET_SCAN_COMMAND, false);
        assert_eq!(
            result.as_ref().map(AssetCommandResult::message),
            Some("No project open")
        );
        assert!(state.index().is_empty());
    }

    #[test]
    fn asset_select_first_updates_selection() {
        let mut state = AssetState::default();
        state.load_fixture_scan(fixture_scan());
        assert!(state.selection.select_first(&state.index));
        assert!(state.selection().selected().is_some());
    }

    #[test]
    fn asset_clear_selection_clears_selection() {
        let mut state = AssetState::default();
        state.load_fixture_scan(fixture_scan());
        assert!(state.selection.select_first(&state.index));
        let _ = state.execute_command_id(ASSET_CLEAR_SELECTION_COMMAND, project_loaded());
        assert_eq!(state.selection().selected(), None);
    }

    #[test]
    fn asset_commands_are_discoverable_through_registry() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let id = match CommandId::new(ASSET_SCAN_COMMAND) {
            Ok(id) => id,
            Err(error) => panic!("asset command ID should be valid: {error}"),
        };
        assert!(matches!(registry.invoke(&id), CommandResult::Invoked(_)));
    }

    #[test]
    fn command_palette_can_execute_asset_scan() {
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
        for character in ASSET_SCAN_COMMAND.chars() {
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
                Err(error) => panic!("selected asset command ID should be valid: {error}"),
            },
            None => panic!("asset command should be selected"),
        };
        assert_eq!(selected_id.as_str(), ASSET_SCAN_COMMAND);
    }

    fn fixture_scan() -> AssetScan {
        AssetScan {
            root: Some(PathBuf::from("fixtures/assets")),
            index: AssetIndex::from_records(vec![
                AssetRecord::from_parts(
                    stable_asset_id(1),
                    "README.md",
                    PathBuf::from("README.md"),
                    AssetKind::Text,
                ),
                AssetRecord::from_parts(
                    stable_asset_id(2),
                    "hero.glb",
                    PathBuf::from("models/hero.glb"),
                    AssetKind::Model,
                ),
                AssetRecord::from_parts(
                    stable_asset_id(3),
                    "level.scene",
                    PathBuf::from("scenes/level.scene"),
                    AssetKind::Scene,
                ),
            ]),
            diagnostics: Vec::new(),
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
