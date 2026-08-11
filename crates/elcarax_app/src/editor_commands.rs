#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_commands::{
    CommandAvailability, CommandBindingRegistry, CommandCategory, CommandId, CommandRegistry,
    RegisteredCommand,
};
use elcarax_ui::CommandPaletteEntry;

use crate::adapter_state::{
    ADAPTER_CONNECT_COMMAND, ADAPTER_DISCONNECT_COMMAND, ADAPTER_HANDSHAKE_COMMAND,
    ADAPTER_LOAD_PROJECT_COMMAND, ADAPTER_LOAD_SCENE_COMMAND, ADAPTER_SHOW_DIAGNOSTICS_COMMAND,
    ADAPTER_SHOW_STATUS_COMMAND, AdapterCommand, AdapterState,
};
use crate::asset_state::{
    ASSET_CLEAR_SELECTION_COMMAND, ASSET_REFRESH_COMMAND, ASSET_REVEAL_ROOT_COMMAND,
    ASSET_SCAN_COMMAND, ASSET_SHOW_SELECTED_COMMAND, ASSET_START_WATCHING_COMMAND,
    ASSET_STOP_WATCHING_COMMAND, AssetCommand,
};
use crate::editor_session::{EditorSessionState, EditorShellContext};
use crate::inspector_state::{
    EDIT_REDO_COMMAND, EDIT_UNDO_COMMAND, INSPECTOR_CLEAR_COMMAND,
    INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND, INSPECTOR_SHOW_SELECTED_COMMAND, InspectorCommand,
    InspectorEditCommand,
};
use crate::project_state::{
    PROJECT_CLOSE_COMMAND, PROJECT_CREATE_COMMAND, PROJECT_OPEN_COMMAND,
    PROJECT_REOPEN_LAST_COMMAND, PROJECT_SHOW_RECENT_COMMAND, PROJECT_VALIDATE_COMMAND,
    ProjectCommand,
};
use crate::scene_state::{
    SCENE_CLEAR_COMMAND, SCENE_CLEAR_SELECTION_COMMAND, SCENE_LOAD_COMMAND, SCENE_SAVE_COMMAND,
    SceneCommand, UNSAVED_SCENE_MESSAGE,
};
use crate::viewport_state::{
    AppViewportState, VIEWPORT_CLEAR_COMMAND, VIEWPORT_REQUEST_FRAME_COMMAND,
    VIEWPORT_SHOW_STATUS_COMMAND, ViewportCommand, ViewportFrameRequestSize,
};

pub(crate) const HELP_SHORTCUTS_COMMAND: &str = "help.shortcuts";
pub(crate) const HELP_COMMANDS_COMMAND: &str = "help.commands";
pub(crate) const PALETTE_OPEN_COMMAND: &str = "elcarax.palette.open";
pub(crate) const PALETTE_CLOSE_COMMAND: &str = "elcarax.palette.close";
pub(crate) const SHOW_RENDERER_STATS_COMMAND: &str = "elcarax.status.show_renderer_stats";
pub(crate) const SHOW_READY_STATUS_COMMAND: &str = "elcarax.status.show_ready";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorCommand {
    Project(ProjectCommand),
    Asset(AssetCommand),
    Scene(SceneCommand),
    Inspector(InspectorCommand),
    Edit(InspectorEditCommand),
    Adapter(AdapterCommand),
    Viewport(ViewportCommand),
    Ui(EditorUiCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorUiCommand {
    HelpShortcuts,
    HelpCommands,
    OpenPalette,
    ClosePalette,
    ShowRendererStats,
    ShowReady,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorCommandOutcome {
    pub(crate) message: Option<String>,
    pub(crate) request_viewport_frame: bool,
    pub(crate) ui_command: Option<EditorUiCommand>,
}

impl EditorCommandOutcome {
    fn message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            request_viewport_frame: false,
            ui_command: None,
        }
    }

    fn ui_command(command: EditorUiCommand) -> Self {
        Self {
            message: None,
            request_viewport_frame: false,
            ui_command: Some(command),
        }
    }
}

pub(crate) struct EditorCommandContext<'a> {
    pub(crate) editor: &'a mut EditorSessionState,
    pub(crate) adapter: &'a mut AdapterState,
    pub(crate) viewport: &'a mut AppViewportState,
    pub(crate) viewport_request_size: ViewportFrameRequestSize,
}

pub(crate) struct EditorCommandRouter;

impl EditorCommandRouter {
    pub(crate) fn parse(id: &str) -> Option<EditorCommand> {
        Some(match id {
            PROJECT_CREATE_COMMAND => EditorCommand::Project(ProjectCommand::Create),
            PROJECT_OPEN_COMMAND => EditorCommand::Project(ProjectCommand::Open),
            PROJECT_CLOSE_COMMAND => EditorCommand::Project(ProjectCommand::Close),
            PROJECT_VALIDATE_COMMAND => EditorCommand::Project(ProjectCommand::Validate),
            PROJECT_SHOW_RECENT_COMMAND => EditorCommand::Project(ProjectCommand::ShowRecent),
            PROJECT_REOPEN_LAST_COMMAND => EditorCommand::Project(ProjectCommand::ReopenLast),
            ASSET_SCAN_COMMAND => EditorCommand::Asset(AssetCommand::Scan),
            ASSET_REFRESH_COMMAND => EditorCommand::Asset(AssetCommand::Refresh),
            ASSET_START_WATCHING_COMMAND => EditorCommand::Asset(AssetCommand::StartWatching),
            ASSET_STOP_WATCHING_COMMAND => EditorCommand::Asset(AssetCommand::StopWatching),
            ASSET_CLEAR_SELECTION_COMMAND => EditorCommand::Asset(AssetCommand::ClearSelection),
            ASSET_SHOW_SELECTED_COMMAND => EditorCommand::Asset(AssetCommand::ShowSelected),
            ASSET_REVEAL_ROOT_COMMAND => EditorCommand::Asset(AssetCommand::RevealRoot),
            SCENE_LOAD_COMMAND => EditorCommand::Scene(SceneCommand::Load),
            SCENE_SAVE_COMMAND => EditorCommand::Scene(SceneCommand::Save),
            SCENE_CLEAR_COMMAND => EditorCommand::Scene(SceneCommand::Clear),
            SCENE_CLEAR_SELECTION_COMMAND => EditorCommand::Scene(SceneCommand::ClearSelection),
            INSPECTOR_CLEAR_COMMAND => EditorCommand::Inspector(InspectorCommand::Clear),
            INSPECTOR_SHOW_SELECTED_COMMAND => {
                EditorCommand::Inspector(InspectorCommand::ShowSelected)
            }
            INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND => {
                EditorCommand::Inspector(InspectorCommand::ShowPropertyCount)
            }
            EDIT_UNDO_COMMAND => EditorCommand::Edit(InspectorEditCommand::Undo),
            EDIT_REDO_COMMAND => EditorCommand::Edit(InspectorEditCommand::Redo),
            ADAPTER_CONNECT_COMMAND => EditorCommand::Adapter(AdapterCommand::Connect),
            ADAPTER_HANDSHAKE_COMMAND => EditorCommand::Adapter(AdapterCommand::Handshake),
            ADAPTER_LOAD_PROJECT_COMMAND => EditorCommand::Adapter(AdapterCommand::LoadProject),
            ADAPTER_LOAD_SCENE_COMMAND => EditorCommand::Adapter(AdapterCommand::LoadScene),
            ADAPTER_SHOW_STATUS_COMMAND => EditorCommand::Adapter(AdapterCommand::ShowStatus),
            ADAPTER_SHOW_DIAGNOSTICS_COMMAND => {
                EditorCommand::Adapter(AdapterCommand::ShowDiagnostics)
            }
            ADAPTER_DISCONNECT_COMMAND => EditorCommand::Adapter(AdapterCommand::Disconnect),
            VIEWPORT_REQUEST_FRAME_COMMAND => {
                EditorCommand::Viewport(ViewportCommand::RequestFrame)
            }
            VIEWPORT_CLEAR_COMMAND => EditorCommand::Viewport(ViewportCommand::Clear),
            VIEWPORT_SHOW_STATUS_COMMAND => EditorCommand::Viewport(ViewportCommand::ShowStatus),
            HELP_SHORTCUTS_COMMAND => EditorCommand::Ui(EditorUiCommand::HelpShortcuts),
            HELP_COMMANDS_COMMAND => EditorCommand::Ui(EditorUiCommand::HelpCommands),
            PALETTE_OPEN_COMMAND => EditorCommand::Ui(EditorUiCommand::OpenPalette),
            PALETTE_CLOSE_COMMAND => EditorCommand::Ui(EditorUiCommand::ClosePalette),
            SHOW_RENDERER_STATS_COMMAND => EditorCommand::Ui(EditorUiCommand::ShowRendererStats),
            SHOW_READY_STATUS_COMMAND => EditorCommand::Ui(EditorUiCommand::ShowReady),
            _ => return None,
        })
    }

    pub(crate) fn execute(
        command: EditorCommand,
        context: &mut EditorCommandContext<'_>,
    ) -> EditorCommandOutcome {
        match command {
            EditorCommand::Project(command) => {
                let result = {
                    let mut shell = EditorShellContext {
                        adapter: context.adapter,
                        viewport: context.viewport,
                    };
                    context
                        .editor
                        .session_mut()
                        .execute_project_command(command, Some(&mut shell))
                };
                EditorCommandOutcome::message(result.message())
            }
            EditorCommand::Asset(command) => {
                let result = context
                    .editor
                    .assets
                    .execute(command, context.editor.project.is_project_loaded());
                context.editor.session_mut().after_asset_command(command);
                EditorCommandOutcome::message(result.message())
            }
            EditorCommand::Scene(SceneCommand::Save) => context
                .editor
                .session_mut()
                .save_scene()
                .map(|outcome| EditorCommandOutcome::message(outcome.status_message()))
                .unwrap_or_else(|| EditorCommandOutcome::message("No scene loaded")),
            EditorCommand::Scene(command) => {
                let outcome = context.editor.session_mut().execute_scene_command(command);
                EditorCommandOutcome::message(outcome.status_message())
            }
            EditorCommand::Inspector(command) => {
                let result = context.editor.inspector.execute(
                    command,
                    &mut context.editor.scene,
                    &context.editor.property_types,
                );
                EditorCommandOutcome::message(result.message())
            }
            EditorCommand::Edit(command) => {
                let result = context
                    .editor
                    .session_mut()
                    .execute_edit_command(context.adapter, command);
                EditorCommandOutcome::message(result.message())
            }
            EditorCommand::Adapter(command) => {
                let result = context.adapter.execute(command, &mut context.editor.scene);
                #[cfg(feature = "native-shell")]
                if command == AdapterCommand::Handshake
                    && let Some((adapter_id, supports_preview)) =
                        context.adapter.connected_viewport_info()
                {
                    context
                        .viewport
                        .on_adapter_connected(&adapter_id, supports_preview);
                }
                if command == AdapterCommand::Disconnect {
                    context.viewport.on_adapter_disconnected();
                }
                context.editor.inspector.on_scene_selection_changed();
                let request_viewport_frame = command == AdapterCommand::LoadScene;
                EditorCommandOutcome {
                    message: Some(result.message().to_string()),
                    request_viewport_frame,
                    ui_command: None,
                }
            }
            EditorCommand::Viewport(command) => {
                let result = context.viewport.execute(
                    command,
                    context.adapter,
                    context.viewport_request_size,
                );
                EditorCommandOutcome::message(result.message())
            }
            EditorCommand::Ui(command) => EditorCommandOutcome::ui_command(command),
        }
    }

    pub(crate) fn availability_for_id(
        id: &str,
        editor: &EditorSessionState,
        adapter: &AdapterState,
        viewport: &AppViewportState,
    ) -> CommandAvailability {
        let Some(command) = Self::parse(id) else {
            return CommandAvailability::disabled("Command is not routable");
        };
        Self::availability_for(command, editor, adapter, viewport)
    }

    pub(crate) fn availability_for(
        command: EditorCommand,
        editor: &EditorSessionState,
        adapter: &AdapterState,
        _viewport: &AppViewportState,
    ) -> CommandAvailability {
        match command {
            EditorCommand::Project(ProjectCommand::Create)
            | EditorCommand::Project(ProjectCommand::Open)
            | EditorCommand::Project(ProjectCommand::ReopenLast) => {
                if editor.scene.has_unsaved_changes() {
                    CommandAvailability::disabled(UNSAVED_SCENE_MESSAGE)
                } else {
                    CommandAvailability::enabled()
                }
            }
            EditorCommand::Project(ProjectCommand::Close) => {
                if !editor.project.is_project_loaded() {
                    CommandAvailability::disabled("No project open")
                } else if editor.scene.has_unsaved_changes() {
                    CommandAvailability::disabled(UNSAVED_SCENE_MESSAGE)
                } else {
                    CommandAvailability::enabled()
                }
            }
            EditorCommand::Project(ProjectCommand::Validate) => {
                require_project(editor, "No project open")
            }
            EditorCommand::Project(ProjectCommand::ShowRecent) => CommandAvailability::enabled(),
            EditorCommand::Asset(AssetCommand::Scan)
            | EditorCommand::Asset(AssetCommand::StartWatching)
            | EditorCommand::Asset(AssetCommand::StopWatching)
            | EditorCommand::Asset(AssetCommand::ClearSelection)
            | EditorCommand::Asset(AssetCommand::ShowSelected)
            | EditorCommand::Asset(AssetCommand::RevealRoot) => {
                require_project(editor, "No project open")
            }
            EditorCommand::Asset(AssetCommand::Refresh) => {
                if !editor.project.is_project_loaded() {
                    CommandAvailability::disabled("No project open")
                } else if editor
                    .project
                    .asset_root()
                    .is_some_and(std::path::Path::is_dir)
                {
                    CommandAvailability::enabled()
                } else {
                    CommandAvailability::disabled("Asset root unavailable")
                }
            }
            EditorCommand::Scene(SceneCommand::Load) => require_project(editor, "No project open"),
            EditorCommand::Scene(SceneCommand::Save) => {
                if editor.scene.snapshot().is_some() && editor.scene.is_project_document() {
                    CommandAvailability::enabled()
                } else {
                    CommandAvailability::disabled("No project scene loaded")
                }
            }
            EditorCommand::Scene(SceneCommand::Clear)
            | EditorCommand::Scene(SceneCommand::ClearSelection)
            | EditorCommand::Inspector(InspectorCommand::ShowSelected)
            | EditorCommand::Inspector(InspectorCommand::ShowPropertyCount) => {
                if editor.scene.snapshot().is_some() {
                    CommandAvailability::enabled()
                } else {
                    CommandAvailability::disabled("No scene loaded")
                }
            }
            EditorCommand::Inspector(InspectorCommand::Clear) => CommandAvailability::enabled(),
            EditorCommand::Edit(InspectorEditCommand::Undo) => {
                if editor.edit_history.undo_count() > 0 {
                    CommandAvailability::enabled()
                } else {
                    CommandAvailability::disabled("Nothing to undo")
                }
            }
            EditorCommand::Edit(InspectorEditCommand::Redo) => {
                if editor.edit_history.redo_count() > 0 {
                    CommandAvailability::enabled()
                } else {
                    CommandAvailability::disabled("Nothing to redo")
                }
            }
            EditorCommand::Adapter(AdapterCommand::Connect)
            | EditorCommand::Adapter(AdapterCommand::ShowStatus) => CommandAvailability::enabled(),
            EditorCommand::Adapter(AdapterCommand::Disconnect)
            | EditorCommand::Adapter(AdapterCommand::Handshake)
            | EditorCommand::Adapter(AdapterCommand::LoadProject)
            | EditorCommand::Adapter(AdapterCommand::LoadScene)
            | EditorCommand::Adapter(AdapterCommand::ShowDiagnostics) => {
                if adapter.is_connected() {
                    CommandAvailability::enabled()
                } else {
                    CommandAvailability::disabled("No adapter connected")
                }
            }
            EditorCommand::Viewport(ViewportCommand::RequestFrame) => {
                if !adapter.is_connected() {
                    CommandAvailability::disabled("No adapter connected")
                } else if !adapter.supports_viewport_preview() {
                    CommandAvailability::disabled("Adapter does not support viewport preview")
                } else {
                    CommandAvailability::enabled()
                }
            }
            EditorCommand::Viewport(ViewportCommand::Clear)
            | EditorCommand::Viewport(ViewportCommand::ShowStatus)
            | EditorCommand::Ui(_) => CommandAvailability::enabled(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolbarSnapshot {
    pub(crate) sections: Vec<ToolbarSection>,
    pub(crate) has_unsaved_scene: bool,
}

impl ToolbarSnapshot {
    pub(crate) fn actions(&self) -> impl Iterator<Item = &ToolbarAction> {
        self.sections
            .iter()
            .flat_map(|section| section.actions.iter())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn action_for_command(&self, command_id: &str) -> Option<&ToolbarAction> {
        self.actions()
            .find(|action| action.command_id.as_str() == command_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolbarSection {
    pub(crate) category: CommandCategory,
    pub(crate) actions: Vec<ToolbarAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolbarAction {
    pub(crate) command_id: String,
    pub(crate) title: String,
    pub(crate) label: String,
    pub(crate) shortcut: Option<String>,
    pub(crate) state: ToolbarButtonState,
}

impl ToolbarAction {
    pub(crate) fn button_label(&self) -> String {
        match &self.shortcut {
            Some(shortcut) if shortcut.len() <= 8 => format!("{} {}", self.label, shortcut),
            _ => self.label.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolbarButtonState {
    Enabled,
    Disabled { reason: String },
}

impl ToolbarButtonState {
    pub(crate) const fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn disabled_reason(&self) -> Option<&str> {
        match self {
            Self::Enabled => None,
            Self::Disabled { reason } => Some(reason.as_str()),
        }
    }
}

pub(crate) fn palette_entries(
    registry: &CommandRegistry,
    bindings: &CommandBindingRegistry,
    editor: &EditorSessionState,
    adapter: &AdapterState,
    viewport: &AppViewportState,
) -> Vec<CommandPaletteEntry> {
    registry
        .all()
        .into_iter()
        .filter(|command| command.presentation().palette_visible())
        .map(|command| palette_entry(command, bindings, editor, adapter, viewport))
        .collect()
}

pub(crate) fn toolbar_snapshot(
    registry: &CommandRegistry,
    bindings: &CommandBindingRegistry,
    editor: &EditorSessionState,
    adapter: &AdapterState,
    viewport: &AppViewportState,
) -> ToolbarSnapshot {
    let mut entries: Vec<_> = registry
        .all()
        .into_iter()
        .filter_map(|command| {
            let placement = command.presentation().toolbar()?;
            let availability = EditorCommandRouter::availability_for_id(
                command.id().as_str(),
                editor,
                adapter,
                viewport,
            );
            Some((
                placement.section,
                placement.order,
                ToolbarAction {
                    command_id: command.id().as_str().to_string(),
                    title: command.title().as_str().to_string(),
                    label: placement.short_label.clone(),
                    shortcut: shortcut_label(bindings, command.id()),
                    state: toolbar_state(availability),
                },
            ))
        })
        .collect();
    entries.sort_by_key(|(section, order, action)| (*order, *section, action.command_id.clone()));

    let mut sections = Vec::new();
    for (category, _order, action) in entries {
        if let Some(section) = sections
            .iter_mut()
            .find(|section: &&mut ToolbarSection| section.category == category)
        {
            section.actions.push(action);
        } else {
            sections.push(ToolbarSection {
                category,
                actions: vec![action],
            });
        }
    }
    ToolbarSnapshot {
        sections,
        has_unsaved_scene: editor.scene.has_unsaved_changes(),
    }
}

pub(crate) fn shortcut_summary(
    registry: &CommandRegistry,
    bindings: &CommandBindingRegistry,
) -> String {
    let summaries: Vec<_> = bindings
        .bindings()
        .into_iter()
        .filter_map(|(command_id, binding)| {
            let command = registry.get(command_id)?;
            Some(format!(
                "{}={}",
                binding.chord().display_label(),
                command.title().as_str()
            ))
        })
        .collect();
    format!(
        "Shortcuts: {} binding(s), {} conflict(s): {}",
        summaries.len(),
        bindings.diagnostics().len(),
        summaries.join(", ")
    )
}

pub(crate) fn command_summary(
    registry: &CommandRegistry,
    bindings: &CommandBindingRegistry,
) -> String {
    let mut counts: Vec<(CommandCategory, usize)> = Vec::new();
    for command in registry.all() {
        if let Some((_, count)) = counts
            .iter_mut()
            .find(|(category, _count)| *category == command.category())
        {
            *count += 1;
        } else {
            counts.push((command.category(), 1));
        }
    }
    counts.sort_by_key(|(category, _count)| *category);
    let category_summary = counts
        .into_iter()
        .map(|(category, count)| format!("{}={count}", category.label()))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Commands: {} command(s), {} conflicted binding(s): {}",
        registry.all().len(),
        bindings.diagnostics().len(),
        category_summary
    )
}

fn palette_entry(
    command: &RegisteredCommand,
    bindings: &CommandBindingRegistry,
    editor: &EditorSessionState,
    adapter: &AdapterState,
    viewport: &AppViewportState,
) -> CommandPaletteEntry {
    let availability =
        EditorCommandRouter::availability_for_id(command.id().as_str(), editor, adapter, viewport);
    CommandPaletteEntry::new(
        command.id().as_str(),
        command.title().as_str(),
        command.category().label(),
        command
            .description()
            .map(|description| description.as_str().to_string()),
        availability.is_enabled(),
    )
    .with_shortcut(shortcut_label(bindings, command.id()))
    .with_disabled_reason(availability.disabled_reason().map(str::to_string))
}

fn shortcut_label(bindings: &CommandBindingRegistry, command_id: &CommandId) -> Option<String> {
    bindings
        .shortcut_for_command(command_id)
        .map(|binding| binding.chord().display_label())
}

fn toolbar_state(availability: CommandAvailability) -> ToolbarButtonState {
    if availability.is_enabled() {
        ToolbarButtonState::Enabled
    } else {
        ToolbarButtonState::Disabled {
            reason: availability
                .disabled_reason()
                .unwrap_or("Command disabled")
                .to_string(),
        }
    }
}

fn require_project(editor: &EditorSessionState, reason: &str) -> CommandAvailability {
    if editor.project.is_project_loaded() {
        CommandAvailability::enabled()
    } else {
        CommandAvailability::disabled(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_commands::{CommandScope, KeyChord, KeyModifier, built_in_commands};
    use elcarax_scene_model::{ObjectSchema, SceneObject};

    use crate::inspector_state::InspectorPropertyCommit;

    fn registry_and_bindings() -> (CommandRegistry, CommandBindingRegistry) {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let bindings = CommandBindingRegistry::from_commands(&registry);
        (registry, bindings)
    }

    fn chord(modifiers: &[KeyModifier], key: &str) -> KeyChord {
        match KeyChord::new(modifiers.iter().cloned(), key) {
            Ok(chord) => chord,
            Err(error) => panic!("test chord should be valid: {error}"),
        }
    }

    #[test]
    fn default_shortcuts_map_to_commands() {
        let (registry, bindings) = registry_and_bindings();
        assert_eq!(
            bindings
                .command_for_chord(&chord(&[KeyModifier::Control], "S"), CommandScope::Global)
                .map(CommandId::as_str),
            Some(SCENE_SAVE_COMMAND)
        );
        assert_eq!(
            bindings
                .command_for_chord(&chord(&[KeyModifier::Control], "O"), CommandScope::Global)
                .map(CommandId::as_str),
            Some(PROJECT_OPEN_COMMAND)
        );
        assert_eq!(
            bindings
                .command_for_chord(&chord(&[KeyModifier::Control], "Z"), CommandScope::Global)
                .map(CommandId::as_str),
            Some(EDIT_UNDO_COMMAND)
        );
        assert_eq!(
            bindings
                .command_for_chord(&chord(&[KeyModifier::Control], "Y"), CommandScope::Global)
                .map(CommandId::as_str),
            Some(EDIT_REDO_COMMAND)
        );
        assert_eq!(
            bindings
                .command_for_chord(
                    &chord(&[KeyModifier::Control, KeyModifier::Shift], "Z"),
                    CommandScope::Global
                )
                .map(CommandId::as_str),
            Some(EDIT_REDO_COMMAND)
        );
        assert!(
            registry
                .get(
                    &CommandId::new(SCENE_SAVE_COMMAND)
                        .unwrap_or_else(|error| panic!("command id should be valid: {error}"))
                )
                .is_some()
        );
    }

    #[test]
    fn every_registered_command_has_one_typed_route() {
        let (registry, _) = registry_and_bindings();
        for command in registry.all() {
            assert!(
                EditorCommandRouter::parse(command.id().as_str()).is_some(),
                "missing typed route for {}",
                command.id().as_str()
            );
        }
    }

    #[test]
    fn disabled_command_reports_reason() {
        let editor = EditorSessionState::default();
        let availability = EditorCommandRouter::availability_for_id(
            SCENE_SAVE_COMMAND,
            &editor,
            &AdapterState::default(),
            &AppViewportState::default(),
        );
        assert!(!availability.is_enabled());
        assert_eq!(
            availability.disabled_reason(),
            Some("No project scene loaded")
        );
    }

    #[test]
    fn toolbar_snapshot_reflects_no_project_state() {
        let (registry, bindings) = registry_and_bindings();
        let snapshot = toolbar_snapshot(
            &registry,
            &bindings,
            &EditorSessionState::default(),
            &AdapterState::default(),
            &AppViewportState::default(),
        );
        let save = snapshot
            .action_for_command(SCENE_SAVE_COMMAND)
            .unwrap_or_else(|| panic!("save action should exist"));
        assert!(!save.state.is_enabled());
        assert_eq!(
            save.state.disabled_reason(),
            Some("No project scene loaded")
        );
    }

    #[test]
    fn toolbar_snapshot_reflects_loaded_project_state() {
        let temp = std::env::temp_dir().join(format!("elcarax-toolbar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let mut editor = EditorSessionState::new(crate::project_config::AppProjectConfig {
            create_root: Some(temp.clone()),
            ..crate::project_config::AppProjectConfig::default()
        });
        let _ = editor
            .session_mut()
            .execute_project_command(ProjectCommand::Create, None);
        let (registry, bindings) = registry_and_bindings();
        let snapshot = toolbar_snapshot(
            &registry,
            &bindings,
            &editor,
            &AdapterState::default(),
            &AppViewportState::default(),
        );
        let save = snapshot
            .action_for_command(SCENE_SAVE_COMMAND)
            .unwrap_or_else(|| panic!("save action should exist"));
        assert!(save.state.is_enabled());
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn dirty_scene_sets_unsaved_indicator_and_save_clears_it() {
        let temp = std::env::temp_dir().join(format!("elcarax-dirty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let mut editor = EditorSessionState::new(crate::project_config::AppProjectConfig {
            create_root: Some(temp.clone()),
            ..crate::project_config::AppProjectConfig::default()
        });
        let _ = editor
            .session_mut()
            .execute_project_command(ProjectCommand::Create, None);
        if let Some(snapshot) = editor.scene.snapshot_mut() {
            let schema = ObjectSchema::new("DirtyMarker");
            let object = SceneObject::new(
                "Dirty Root",
                elcarax_scene_model::SceneObjectKind::new(elcarax_scene_model::kinds::WORLD),
                schema.type_id,
            );
            snapshot.add_schema(schema);
            let _ = snapshot.add_object(
                None,
                0,
                object,
                &elcarax_scene_model::PropertyTypeRegistry::default(),
            );
        }
        editor.scene.mark_document_modified();
        let (registry, bindings) = registry_and_bindings();
        let dirty = toolbar_snapshot(
            &registry,
            &bindings,
            &editor,
            &AdapterState::default(),
            &AppViewportState::default(),
        );
        assert!(dirty.has_unsaved_scene);
        let _ = editor.session_mut().save_scene();
        let clean = toolbar_snapshot(
            &registry,
            &bindings,
            &editor,
            &AdapterState::default(),
            &AppViewportState::default(),
        );
        assert!(!clean.has_unsaved_scene);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn adapter_backed_scene_uses_edit_history_undo_availability() {
        use elcarax_adapter_api::{
            AdapterId, AdapterRequestId, AdapterResponseMessage, SetPropertyResponse,
            SetPropertyStatus,
        };
        use elcarax_adapter_host::{AdapterSession, FakeAdapterTransport, response_frame};
        use elcarax_scene_model::{
            ComponentInstance, ComponentSchema, PropertyEditKind, PropertyKind, PropertyPath,
            PropertySchema, PropertyValue, SceneName, SceneObject, SceneObjectKind, ScenePatch,
            SceneSnapshot, components, kinds,
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
        let _ = snapshot.add_object(
            None,
            0,
            object,
            &elcarax_scene_model::PropertyTypeRegistry::default(),
        );
        let mut editor = EditorSessionState::default();
        editor.scene.load_external_snapshot(
            snapshot,
            AdapterId::new("fixture-adapter"),
            "test",
            "Loaded adapter scene",
        );
        assert!(editor.scene.select_object(object_id));
        let scene_id = editor
            .scene
            .snapshot()
            .map(|value| value.scene_id())
            .unwrap_or_else(|| panic!("adapter fixture scene should be loaded"));
        let response = match response_frame(
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
            Ok(frame) => frame,
            Err(error) => panic!("response should serialize: {error}"),
        };
        let mut adapter = AdapterState::default();
        adapter.attach_fake_session_for_tests(AdapterSession::new(FakeAdapterTransport::new(
            vec![response],
        )));
        let _ = editor.session_mut().commit_inspector_property(
            &mut adapter,
            InspectorPropertyCommit {
                component_id,
                path: "health".to_string(),
                edit_kind: PropertyEditKind::Integer,
                extension_type_id: None,
                text: "65".to_string(),
                label: "Set Fixture Health".to_string(),
            },
        );
        let availability = EditorCommandRouter::availability_for_id(
            EDIT_UNDO_COMMAND,
            &editor,
            &adapter,
            &AppViewportState::default(),
        );
        assert!(availability.is_enabled());
        assert_eq!(editor.edit_history.undo_count(), 1);
    }
}
