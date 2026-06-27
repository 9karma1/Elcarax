use elcarax_assets::{
    AssetId, AssetIndex, AssetScan, AssetSelection, apply_selection_after_scan, scan_demo_assets,
};

use crate::asset_display::{AssetUiSnapshot, asset_ui_snapshot};

pub(crate) const ASSET_SCAN_DEMO_COMMAND: &str = "asset.scan_demo";
pub(crate) const ASSET_SELECT_FIRST_COMMAND: &str = "asset.select_first";
pub(crate) const ASSET_CLEAR_SELECTION_COMMAND: &str = "asset.clear_selection";
pub(crate) const ASSET_SHOW_SELECTED_COMMAND: &str = "asset.show_selected";

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
            AssetCommand::ScanDemo => self.scan_demo(project_loaded),
            AssetCommand::SelectFirst => self.select_first(),
            AssetCommand::ClearSelection => self.clear_selection(),
            AssetCommand::ShowSelected => self.show_selected(),
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

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn selection(&self) -> &AssetSelection {
        &self.selection
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn kind_summary(&self) -> String {
        self.index.kind_summary()
    }

    fn scan_demo(&mut self, project_loaded: bool) -> AssetCommandResult {
        if !project_loaded {
            return AssetCommandResult::new(ASSET_SCAN_DEMO_COMMAND, "No project loaded");
        }
        let scan = scan_demo_assets();
        let count = scan.asset_count();
        self.index = scan.index.clone();
        self.last_scan = Some(scan);
        if let Some(scan) = self.last_scan.as_ref() {
            apply_selection_after_scan(scan, &mut self.selection);
        }
        AssetCommandResult::new(
            ASSET_SCAN_DEMO_COMMAND,
            format!("Scanned {count} demo assets"),
        )
    }

    fn select_first(&mut self) -> AssetCommandResult {
        if self.index.is_empty() {
            return AssetCommandResult::new(ASSET_SELECT_FIRST_COMMAND, "No assets to select");
        }
        let selected = self.selection.select_first(&self.index);
        if !selected {
            return AssetCommandResult::new(ASSET_SELECT_FIRST_COMMAND, "No assets to select");
        }
        let record = match self.index.first() {
            Some(record) => record,
            None => {
                return AssetCommandResult::new(ASSET_SELECT_FIRST_COMMAND, "No assets to select");
            }
        };
        AssetCommandResult::new(
            ASSET_SELECT_FIRST_COMMAND,
            format!(
                "Selected {} ({})",
                record.name.as_str(),
                record.kind.label()
            ),
        )
    }

    fn clear_selection(&mut self) -> AssetCommandResult {
        self.selection.clear();
        AssetCommandResult::new(ASSET_CLEAR_SELECTION_COMMAND, "Cleared asset selection")
    }

    fn show_selected(&self) -> AssetCommandResult {
        let Some(id) = self.selection.selected() else {
            return AssetCommandResult::new(ASSET_SHOW_SELECTED_COMMAND, "No asset selected");
        };
        let Some(record) = self.index.find(id) else {
            return AssetCommandResult::new(ASSET_SHOW_SELECTED_COMMAND, "No asset selected");
        };
        AssetCommandResult::new(
            ASSET_SHOW_SELECTED_COMMAND,
            format!(
                "{} | {} | {}",
                record.name.as_str(),
                record.kind.label(),
                record.path.display()
            ),
        )
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
    ScanDemo,
    SelectFirst,
    ClearSelection,
    ShowSelected,
}

impl AssetCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            ASSET_SCAN_DEMO_COMMAND => Some(Self::ScanDemo),
            ASSET_SELECT_FIRST_COMMAND => Some(Self::SelectFirst),
            ASSET_CLEAR_SELECTION_COMMAND => Some(Self::ClearSelection),
            ASSET_SHOW_SELECTED_COMMAND => Some(Self::ShowSelected),
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

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
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
    use elcarax_commands::{CommandId, CommandResult, RegisteredCommand, built_in_commands};
    use elcarax_ui::{CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey};

    fn project_loaded() -> bool {
        true
    }

    #[test]
    fn demo_scan_kind_summary_is_stable() {
        let mut state = AssetState::default();
        let _ = state.execute_command_id(ASSET_SCAN_DEMO_COMMAND, project_loaded());
        assert_eq!(
            state.kind_summary(),
            "Audio=1, Image=1, Material=1, Model=1, Scene=1, Script=1, Text=1"
        );
    }

    #[test]
    fn asset_scan_demo_populates_index() {
        let mut state = AssetState::default();
        let result = state.execute_command_id(ASSET_SCAN_DEMO_COMMAND, project_loaded());
        assert_eq!(
            result.as_ref().map(AssetCommandResult::command_id),
            Some(ASSET_SCAN_DEMO_COMMAND)
        );
        assert_eq!(state.index().len(), 7);
    }

    #[test]
    fn asset_scan_demo_without_project_returns_clear_result() {
        let mut state = AssetState::default();
        let result = state.execute_command_id(ASSET_SCAN_DEMO_COMMAND, false);
        assert_eq!(
            result.as_ref().map(AssetCommandResult::message),
            Some("No project loaded")
        );
        assert!(state.index().is_empty());
    }

    #[test]
    fn asset_select_first_updates_selection() {
        let mut state = AssetState::default();
        let _ = state.execute_command_id(ASSET_SCAN_DEMO_COMMAND, project_loaded());
        let _ = state.execute_command_id(ASSET_SELECT_FIRST_COMMAND, project_loaded());
        assert!(state.selection().selected().is_some());
    }

    #[test]
    fn asset_clear_selection_clears_selection() {
        let mut state = AssetState::default();
        let _ = state.execute_command_id(ASSET_SCAN_DEMO_COMMAND, project_loaded());
        let _ = state.execute_command_id(ASSET_SELECT_FIRST_COMMAND, project_loaded());
        let _ = state.execute_command_id(ASSET_CLEAR_SELECTION_COMMAND, project_loaded());
        assert_eq!(state.selection().selected(), None);
    }

    #[test]
    fn asset_commands_are_discoverable_through_registry() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let id = match CommandId::new(ASSET_SCAN_DEMO_COMMAND) {
            Ok(id) => id,
            Err(error) => panic!("asset command ID should be valid: {error}"),
        };
        assert!(matches!(registry.invoke(&id), CommandResult::Invoked(_)));
    }

    #[test]
    fn command_palette_can_execute_asset_scan_demo() {
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
        for character in ASSET_SCAN_DEMO_COMMAND.chars() {
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
        assert_eq!(selected_id.as_str(), ASSET_SCAN_DEMO_COMMAND);
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
