#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use std::path::{Path, PathBuf};

use elcarax_assets::{
    AssetDiagnostic, AssetId, AssetIndex, AssetScan, AssetScanRequest, AssetSelection,
    AssetWatchError, AssetWatchEvent, AssetWatchEventKind, AssetWatchService, AssetWatchStatus,
    apply_selection_after_scan,
};

use crate::asset_display::{AssetUiSnapshot, asset_ui_snapshot};

pub(crate) const ASSET_SCAN_COMMAND: &str = "asset.scan";
pub(crate) const ASSET_REFRESH_COMMAND: &str = "asset.refresh";
pub(crate) const ASSET_START_WATCHING_COMMAND: &str = "asset.start_watching";
pub(crate) const ASSET_STOP_WATCHING_COMMAND: &str = "asset.stop_watching";
pub(crate) const ASSET_CLEAR_SELECTION_COMMAND: &str = "asset.clear_selection";
pub(crate) const ASSET_SHOW_SELECTED_COMMAND: &str = "asset.show_selected";
pub(crate) const ASSET_REVEAL_ROOT_COMMAND: &str = "asset.reveal_root";

pub(crate) struct AssetState {
    project_root: Option<PathBuf>,
    asset_root: Option<PathBuf>,
    index: AssetIndex,
    selection: AssetSelection,
    scan_status: AssetScanStatus,
    watch_status: AssetWatchStatus,
    diagnostics: Vec<AssetDiagnostic>,
    last_scan: Option<AssetScan>,
    last_command_result: Option<AssetCommandResult>,
    index_dirty: bool,
    watch_service: Option<AssetWatchService>,
}

impl AssetState {
    pub(crate) fn execute_command_id(
        &mut self,
        id: &str,
        project_loaded: bool,
    ) -> Option<AssetCommandResult> {
        self.poll_watch_events();
        let command = AssetCommand::from_id(id)?;
        let result = match command {
            AssetCommand::Scan => self.scan(project_loaded),
            AssetCommand::Refresh => self.refresh(project_loaded),
            AssetCommand::StartWatching => self.start_watching(project_loaded),
            AssetCommand::StopWatching => self.stop_watching(),
            AssetCommand::ClearSelection => self.clear_selection(),
            AssetCommand::ShowSelected => self.show_selected(project_loaded),
            AssetCommand::RevealRoot => self.reveal_root(project_loaded),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    pub(crate) fn on_project_opened(&mut self, project_root: &Path, asset_root: &Path) {
        self.stop_watch_service();
        self.project_root = Some(project_root.to_path_buf());
        self.asset_root = Some(asset_root.to_path_buf());
        self.index = AssetIndex::new();
        self.selection = AssetSelection::none();
        self.scan_status = AssetScanStatus::Ready;
        self.watch_status = AssetWatchStatus::Stopped;
        self.diagnostics.clear();
        self.last_scan = None;
        self.last_command_result = None;
        self.index_dirty = false;
    }

    pub(crate) fn on_project_closed(&mut self) {
        self.stop_watch_service();
        self.project_root = None;
        self.asset_root = None;
        self.index = AssetIndex::new();
        self.selection = AssetSelection::none();
        self.scan_status = AssetScanStatus::UnavailableNoProject;
        self.watch_status = AssetWatchStatus::Stopped;
        self.diagnostics.clear();
        self.last_scan = None;
        self.last_command_result = None;
        self.index_dirty = false;
    }

    pub(crate) fn poll_watch_events(&mut self) -> bool {
        let Some(service) = &mut self.watch_service else {
            return false;
        };
        let events = service.drain_events();
        if events.is_empty() {
            return false;
        }
        self.apply_watch_events(&events);
        true
    }

    pub(crate) fn apply_watch_events(&mut self, events: &[AssetWatchEvent]) {
        for event in events {
            if let AssetWatchEventKind::Error(message) = &event.kind {
                self.diagnostics
                    .push(AssetDiagnostic::warning("watch", message.clone()));
                self.watch_status = AssetWatchStatus::Error(message.clone());
                self.scan_status = AssetScanStatus::Error;
            }
        }
        if events
            .iter()
            .any(|event| !matches!(event.kind, AssetWatchEventKind::Error(_)))
        {
            self.index_dirty = true;
            self.scan_status = AssetScanStatus::Dirty;
        }
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
            AssetUiState {
                project_loaded: self.asset_root.is_some(),
                scan_status: self.scan_status,
                watch_status: &self.watch_status,
                diagnostics: &self.diagnostics,
                last_command_message: self
                    .last_command_result
                    .as_ref()
                    .map(AssetCommandResult::message),
                dirty: self.index_dirty,
                scanned: self.last_scan.is_some(),
            },
        )
    }

    pub(crate) fn scanned_asset_count(&self) -> Option<usize> {
        self.last_scan.as_ref().map(|scan| scan.asset_count())
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn scan_status(&self) -> AssetScanStatus {
        self.scan_status
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn watch_status(&self) -> &AssetWatchStatus {
        &self.watch_status
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_dirty(&self) -> bool {
        self.index_dirty
    }

    fn scan(&mut self, project_loaded: bool) -> AssetCommandResult {
        self.scan_with_command(project_loaded, ASSET_SCAN_COMMAND, false)
    }

    fn refresh(&mut self, project_loaded: bool) -> AssetCommandResult {
        self.scan_with_command(project_loaded, ASSET_REFRESH_COMMAND, true)
    }

    fn scan_with_command(
        &mut self,
        project_loaded: bool,
        command_id: &'static str,
        clear_dirty: bool,
    ) -> AssetCommandResult {
        if !project_loaded {
            return AssetCommandResult::new(command_id, "No project open");
        }
        let Some(request) = self.scan_request() else {
            return AssetCommandResult::new(command_id, "No asset root loaded");
        };
        self.scan_status = AssetScanStatus::Scanning;
        let scan = AssetScan::scan(request);
        apply_selection_after_scan(&scan, &mut self.selection);
        self.diagnostics = scan.diagnostics.clone();
        self.index = scan.index.clone();
        self.last_scan = Some(scan);
        self.index_dirty = if clear_dirty { false } else { self.index_dirty };
        self.scan_status = if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == elcarax_core::Severity::Error)
        {
            AssetScanStatus::Error
        } else if self.index_dirty {
            AssetScanStatus::Dirty
        } else {
            AssetScanStatus::Ready
        };
        AssetCommandResult::new(command_id, self.scan_message(command_id))
    }

    fn scan_request(&self) -> Option<AssetScanRequest> {
        Some(AssetScanRequest::new(
            self.project_root.clone()?,
            self.asset_root.clone()?,
        ))
    }

    fn scan_message(&self, command_id: &str) -> String {
        let verb = if command_id == ASSET_REFRESH_COMMAND {
            "Refreshed"
        } else {
            "Scanned"
        };
        if self.diagnostics.is_empty() {
            return format!("{verb} {} asset(s)", self.index.len());
        }
        format!(
            "{verb} {} asset(s); {}",
            self.index.len(),
            self.diagnostics[0].summary()
        )
    }

    fn start_watching(&mut self, project_loaded: bool) -> AssetCommandResult {
        if !project_loaded {
            return AssetCommandResult::new(ASSET_START_WATCHING_COMMAND, "No project open");
        }
        let Some(root) = self.asset_root.clone() else {
            return AssetCommandResult::new(ASSET_START_WATCHING_COMMAND, "No asset root loaded");
        };
        self.stop_watch_service();
        match AssetWatchService::start(&root) {
            Ok(service) => {
                self.watch_status = service.status().clone();
                self.watch_service = Some(service);
                AssetCommandResult::new(ASSET_START_WATCHING_COMMAND, "Started asset watcher")
            }
            Err(error) => self.record_watch_error(error),
        }
    }

    fn record_watch_error(&mut self, error: AssetWatchError) -> AssetCommandResult {
        let message = error.to_string();
        self.watch_status = AssetWatchStatus::Error(message.clone());
        self.scan_status = AssetScanStatus::Error;
        self.diagnostics
            .push(AssetDiagnostic::warning("watch", message.clone()));
        AssetCommandResult::new(ASSET_START_WATCHING_COMMAND, message)
    }

    fn stop_watching(&mut self) -> AssetCommandResult {
        self.stop_watch_service();
        AssetCommandResult::new(ASSET_STOP_WATCHING_COMMAND, "Stopped asset watcher")
    }

    fn stop_watch_service(&mut self) {
        if let Some(service) = &mut self.watch_service {
            service.stop();
        }
        self.watch_service = None;
        self.watch_status = AssetWatchStatus::Stopped;
    }

    #[cfg(test)]
    fn load_fixture_scan(&mut self, scan: AssetScan) {
        self.asset_root = scan.root.clone();
        self.project_root = scan
            .root
            .as_ref()
            .and_then(|root| root.parent().map(Path::to_path_buf));
        self.index = scan.index.clone();
        self.diagnostics = scan.diagnostics.clone();
        self.last_scan = Some(scan);
        self.scan_status = AssetScanStatus::Ready;
    }

    fn clear_selection(&mut self) -> AssetCommandResult {
        self.selection.clear();
        AssetCommandResult::new(ASSET_CLEAR_SELECTION_COMMAND, "Cleared asset selection")
    }

    fn show_selected(&self, project_loaded: bool) -> AssetCommandResult {
        if !project_loaded {
            return AssetCommandResult::new(ASSET_SHOW_SELECTED_COMMAND, "No project open");
        }
        let Some(id) = self.selection.selected() else {
            return AssetCommandResult::new(ASSET_SHOW_SELECTED_COMMAND, "No asset selected");
        };
        let Some(record) = self.index.find(id) else {
            return AssetCommandResult::new(ASSET_SHOW_SELECTED_COMMAND, "No asset selected");
        };
        AssetCommandResult::new(
            ASSET_SHOW_SELECTED_COMMAND,
            format!("Selected asset: {}", record.path.display()),
        )
    }

    fn reveal_root(&self, project_loaded: bool) -> AssetCommandResult {
        if !project_loaded {
            return AssetCommandResult::new(ASSET_REVEAL_ROOT_COMMAND, "No project open");
        }
        let Some(root) = &self.asset_root else {
            return AssetCommandResult::new(ASSET_REVEAL_ROOT_COMMAND, "No asset root loaded");
        };
        AssetCommandResult::new(
            ASSET_REVEAL_ROOT_COMMAND,
            format!("Asset root: {}", root.display()),
        )
    }
}

impl Default for AssetState {
    fn default() -> Self {
        Self {
            project_root: None,
            asset_root: None,
            index: AssetIndex::new(),
            selection: AssetSelection::none(),
            scan_status: AssetScanStatus::UnavailableNoProject,
            watch_status: AssetWatchStatus::Stopped,
            diagnostics: Vec::new(),
            last_scan: None,
            last_command_result: None,
            index_dirty: false,
            watch_service: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssetScanStatus {
    UnavailableNoProject,
    Ready,
    Scanning,
    Dirty,
    Error,
}

pub(crate) struct AssetUiState<'a> {
    pub(crate) project_loaded: bool,
    pub(crate) scan_status: AssetScanStatus,
    pub(crate) watch_status: &'a AssetWatchStatus,
    pub(crate) diagnostics: &'a [AssetDiagnostic],
    pub(crate) last_command_message: Option<&'a str>,
    pub(crate) dirty: bool,
    pub(crate) scanned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetCommand {
    Scan,
    Refresh,
    StartWatching,
    StopWatching,
    ClearSelection,
    ShowSelected,
    RevealRoot,
}

impl AssetCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            ASSET_SCAN_COMMAND => Some(Self::Scan),
            ASSET_REFRESH_COMMAND => Some(Self::Refresh),
            ASSET_START_WATCHING_COMMAND => Some(Self::StartWatching),
            ASSET_STOP_WATCHING_COMMAND => Some(Self::StopWatching),
            ASSET_CLEAR_SELECTION_COMMAND => Some(Self::ClearSelection),
            ASSET_SHOW_SELECTED_COMMAND => Some(Self::ShowSelected),
            ASSET_REVEAL_ROOT_COMMAND => Some(Self::RevealRoot),
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

    #[cfg_attr(any(not(test), feature = "native-shell"), allow(dead_code))]
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
    use elcarax_assets::{AssetKind, AssetRecord, stable_asset_id_from_path};
    use elcarax_commands::{CommandId, CommandResult, RegisteredCommand, built_in_commands};
    use elcarax_project::{ProjectCreateRequest, create_project};
    use elcarax_ui::{CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey};
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn fixture_scan_kind_summary_is_stable() {
        let mut state = AssetState::default();
        state.load_fixture_scan(fixture_scan());
        assert_eq!(state.kind_summary(), "Model=1, Scene=1, Text=1");
    }

    #[test]
    fn asset_scan_uses_project_asset_root() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-app-asset-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = create_project(&ProjectCreateRequest::new(&temp, "Asset Scan"));
        let mut state = AssetState::default();
        state.on_project_opened(&temp, temp.join("assets").as_path());
        let result = state.execute_command_id(ASSET_SCAN_COMMAND, true);
        assert_eq!(
            result.as_ref().map(AssetCommandResult::message),
            Some("Scanned 0 asset(s)")
        );
        assert_eq!(state.index().len(), 0);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn asset_refresh_rescans_and_clears_dirty_state() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-app-asset-refresh-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let assets = temp.join("assets");
        assert!(fs::create_dir_all(&assets).is_ok());
        assert!(fs::write(assets.join("one.txt"), "one").is_ok());
        let mut state = AssetState::default();
        state.on_project_opened(&temp, assets.as_path());
        let _ = state.execute_command_id(ASSET_SCAN_COMMAND, true);
        state.apply_watch_events(&[AssetWatchEvent::synthetic_change(assets.join("two.txt"))]);
        assert!(state.is_dirty());
        assert!(fs::write(assets.join("two.txt"), "two").is_ok());
        let result = state.execute_command_id(ASSET_REFRESH_COMMAND, true);
        assert_eq!(
            result.as_ref().map(AssetCommandResult::message),
            Some("Refreshed 2 asset(s)")
        );
        assert!(!state.is_dirty());
        assert_eq!(state.index().len(), 2);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn asset_scan_without_root_reports_empty_state() {
        let mut state = AssetState::default();
        let result = state.execute_command_id(ASSET_SCAN_COMMAND, true);
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
    fn watch_events_mark_index_dirty() {
        let mut state = AssetState::default();
        state.on_project_opened(Path::new("project"), Path::new("project/assets"));
        state.apply_watch_events(&[AssetWatchEvent::synthetic_change("project/assets/new.txt")]);
        assert!(state.is_dirty());
        assert_eq!(state.scan_status(), AssetScanStatus::Dirty);
    }

    #[test]
    fn asset_start_and_stop_watching_update_state() {
        let temp = std::env::temp_dir().join(format!("elcarax-watch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let assets = temp.join("assets");
        assert!(fs::create_dir_all(&assets).is_ok());
        let mut state = AssetState::default();
        state.on_project_opened(&temp, assets.as_path());
        let start = state.execute_command_id(ASSET_START_WATCHING_COMMAND, true);
        assert_eq!(
            start.as_ref().map(AssetCommandResult::message),
            Some("Started asset watcher")
        );
        assert!(matches!(
            state.watch_status(),
            AssetWatchStatus::Watching(_)
        ));
        let stop = state.execute_command_id(ASSET_STOP_WATCHING_COMMAND, true);
        assert_eq!(
            stop.as_ref().map(AssetCommandResult::message),
            Some("Stopped asset watcher")
        );
        assert_eq!(state.watch_status(), &AssetWatchStatus::Stopped);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn project_close_stops_watcher_and_clears_asset_state() {
        let temp = std::env::temp_dir().join(format!("elcarax-watch-close-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let assets = temp.join("assets");
        assert!(fs::create_dir_all(&assets).is_ok());
        let mut state = AssetState::default();
        state.on_project_opened(&temp, assets.as_path());
        let _ = state.execute_command_id(ASSET_START_WATCHING_COMMAND, true);
        state.on_project_closed();
        assert!(state.index().is_empty());
        assert!(state.scanned_asset_count().is_none());
        assert_eq!(state.watch_status(), &AssetWatchStatus::Stopped);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn selected_asset_survives_refresh_when_id_still_exists() {
        let temp = std::env::temp_dir().join(format!("elcarax-select-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let assets = temp.join("assets");
        assert!(fs::create_dir_all(&assets).is_ok());
        let file = assets.join("same.txt");
        assert!(fs::write(&file, "one").is_ok());
        let mut state = AssetState::default();
        state.on_project_opened(&temp, assets.as_path());
        let _ = state.execute_command_id(ASSET_SCAN_COMMAND, true);
        assert!(state.select_row(0));
        let selected = state.selection().selected();
        assert!(fs::write(&file, "two").is_ok());
        let _ = state.execute_command_id(ASSET_REFRESH_COMMAND, true);
        assert_eq!(state.selection().selected(), selected);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn selected_asset_clears_if_file_disappears() {
        let temp = std::env::temp_dir().join(format!("elcarax-select-gone-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let assets = temp.join("assets");
        assert!(fs::create_dir_all(&assets).is_ok());
        let file = assets.join("same.txt");
        assert!(fs::write(&file, "one").is_ok());
        let mut state = AssetState::default();
        state.on_project_opened(&temp, assets.as_path());
        let _ = state.execute_command_id(ASSET_SCAN_COMMAND, true);
        assert!(state.select_row(0));
        assert!(fs::remove_file(&file).is_ok());
        let _ = state.execute_command_id(ASSET_REFRESH_COMMAND, true);
        assert_eq!(state.selection().selected(), None);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn asset_show_selected_reports_selected_asset() {
        let mut state = AssetState::default();
        state.load_fixture_scan(fixture_scan());
        assert!(state.selection.select_first(&state.index));
        let result = state.execute_command_id(ASSET_SHOW_SELECTED_COMMAND, true);
        assert_eq!(
            result.as_ref().map(AssetCommandResult::message),
            Some("Selected asset: README.md")
        );
    }

    #[test]
    fn asset_reveal_root_reports_root() {
        let mut state = AssetState::default();
        state.on_project_opened(Path::new("project"), Path::new("project/assets"));
        let result = state.execute_command_id(ASSET_REVEAL_ROOT_COMMAND, true);
        assert_eq!(
            result.as_ref().map(AssetCommandResult::message),
            Some("Asset root: project/assets")
        );
    }

    #[test]
    fn asset_clear_selection_clears_selection() {
        let mut state = AssetState::default();
        state.load_fixture_scan(fixture_scan());
        assert!(state.selection.select_first(&state.index));
        let _ = state.execute_command_id(ASSET_CLEAR_SELECTION_COMMAND, true);
        assert_eq!(state.selection().selected(), None);
    }

    #[test]
    fn asset_commands_are_discoverable_through_registry() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        for command_id in [
            ASSET_SCAN_COMMAND,
            ASSET_REFRESH_COMMAND,
            ASSET_START_WATCHING_COMMAND,
            ASSET_STOP_WATCHING_COMMAND,
            ASSET_CLEAR_SELECTION_COMMAND,
            ASSET_SHOW_SELECTED_COMMAND,
            ASSET_REVEAL_ROOT_COMMAND,
        ] {
            let id = match CommandId::new(command_id) {
                Ok(id) => id,
                Err(error) => panic!("asset command ID should be valid: {error}"),
            };
            assert!(matches!(registry.invoke(&id), CommandResult::Invoked(_)));
        }
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
            request: None,
            index: AssetIndex::from_records(vec![
                AssetRecord::from_parts(
                    stable_asset_id_from_path(Path::new("README.md")),
                    "README.md",
                    PathBuf::from("README.md"),
                    AssetKind::Text,
                ),
                AssetRecord::from_parts(
                    stable_asset_id_from_path(Path::new("models/hero.glb")),
                    "hero.glb",
                    PathBuf::from("models/hero.glb"),
                    AssetKind::Model,
                ),
                AssetRecord::from_parts(
                    stable_asset_id_from_path(Path::new("scenes/level.scene")),
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
