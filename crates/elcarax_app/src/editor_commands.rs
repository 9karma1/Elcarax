#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_commands::{
    CommandAvailability, CommandBindingRegistry, CommandCategory, CommandId, CommandRegistry,
    RegisteredCommand,
};
use elcarax_ui::CommandPaletteEntry;

use crate::adapter_state::{
    ADAPTER_CONNECT_COMMAND, ADAPTER_DISCONNECT_COMMAND, ADAPTER_HANDSHAKE_COMMAND,
    ADAPTER_LOAD_PROJECT_COMMAND, ADAPTER_LOAD_SCENE_COMMAND, ADAPTER_SHOW_DIAGNOSTICS_COMMAND,
    ADAPTER_SHOW_STATUS_COMMAND, AdapterState,
};
use crate::asset_state::{
    ASSET_CLEAR_SELECTION_COMMAND, ASSET_REFRESH_COMMAND, ASSET_REVEAL_ROOT_COMMAND,
    ASSET_SCAN_COMMAND, ASSET_SHOW_SELECTED_COMMAND, ASSET_START_WATCHING_COMMAND,
    ASSET_STOP_WATCHING_COMMAND,
};
use crate::editor_session::EditorSessionState;
use crate::inspector_state::{
    EDIT_REDO_COMMAND, EDIT_UNDO_COMMAND, INSPECTOR_CLEAR_COMMAND,
    INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND, INSPECTOR_SHOW_SELECTED_COMMAND,
};
use crate::project_state::{
    PROJECT_CLOSE_COMMAND, PROJECT_CREATE_COMMAND, PROJECT_OPEN_COMMAND,
    PROJECT_REOPEN_LAST_COMMAND, PROJECT_SHOW_RECENT_COMMAND, PROJECT_VALIDATE_COMMAND,
};
use crate::scene_state::{
    SCENE_CLEAR_COMMAND, SCENE_CLEAR_SELECTION_COMMAND, SCENE_LOAD_COMMAND, SCENE_SAVE_COMMAND,
    UNSAVED_SCENE_MESSAGE,
};
use crate::viewport_state::{
    AppViewportState, VIEWPORT_CLEAR_COMMAND, VIEWPORT_REQUEST_FRAME_COMMAND,
    VIEWPORT_SHOW_STATUS_COMMAND,
};

pub(crate) const HELP_SHORTCUTS_COMMAND: &str = "help.shortcuts";
pub(crate) const HELP_COMMANDS_COMMAND: &str = "help.commands";
pub(crate) const PALETTE_OPEN_COMMAND: &str = "elcarax.palette.open";
pub(crate) const PALETTE_CLOSE_COMMAND: &str = "elcarax.palette.close";
pub(crate) const SHOW_RENDERER_STATS_COMMAND: &str = "elcarax.status.show_renderer_stats";
pub(crate) const SHOW_READY_STATUS_COMMAND: &str = "elcarax.status.show_ready";

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
            let availability =
                command_availability(command.id().as_str(), editor, adapter, viewport);
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

pub(crate) fn command_availability(
    command_id: &str,
    editor: &EditorSessionState,
    adapter: &AdapterState,
    _viewport: &AppViewportState,
) -> CommandAvailability {
    match command_id {
        PROJECT_CREATE_COMMAND | PROJECT_OPEN_COMMAND | PROJECT_REOPEN_LAST_COMMAND => {
            if editor.scene.has_unsaved_changes() {
                CommandAvailability::disabled(UNSAVED_SCENE_MESSAGE)
            } else {
                CommandAvailability::enabled()
            }
        }
        PROJECT_CLOSE_COMMAND => {
            if !editor.project.is_project_loaded() {
                CommandAvailability::disabled("No project open")
            } else if editor.scene.has_unsaved_changes() {
                CommandAvailability::disabled(UNSAVED_SCENE_MESSAGE)
            } else {
                CommandAvailability::enabled()
            }
        }
        PROJECT_VALIDATE_COMMAND => require_project(editor, "No project open"),
        PROJECT_SHOW_RECENT_COMMAND => CommandAvailability::enabled(),
        ASSET_SCAN_COMMAND
        | ASSET_START_WATCHING_COMMAND
        | ASSET_STOP_WATCHING_COMMAND
        | ASSET_CLEAR_SELECTION_COMMAND
        | ASSET_SHOW_SELECTED_COMMAND
        | ASSET_REVEAL_ROOT_COMMAND => require_project(editor, "No project open"),
        ASSET_REFRESH_COMMAND => {
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
        SCENE_LOAD_COMMAND => require_project(editor, "No project open"),
        SCENE_SAVE_COMMAND => {
            if editor.scene.snapshot().is_some() && editor.scene.is_project_document() {
                CommandAvailability::enabled()
            } else {
                CommandAvailability::disabled("No project scene loaded")
            }
        }
        SCENE_CLEAR_COMMAND => {
            if editor.scene.snapshot().is_some() {
                CommandAvailability::enabled()
            } else {
                CommandAvailability::disabled("No scene loaded")
            }
        }
        SCENE_CLEAR_SELECTION_COMMAND => {
            if editor.scene.snapshot().is_some() {
                CommandAvailability::enabled()
            } else {
                CommandAvailability::disabled("No scene loaded")
            }
        }
        INSPECTOR_CLEAR_COMMAND => CommandAvailability::enabled(),
        INSPECTOR_SHOW_SELECTED_COMMAND | INSPECTOR_SHOW_PROPERTY_COUNT_COMMAND => {
            if editor.scene.snapshot().is_some() {
                CommandAvailability::enabled()
            } else {
                CommandAvailability::disabled("No scene loaded")
            }
        }
        EDIT_UNDO_COMMAND => {
            if editor.scene.is_adapter_backed() {
                if adapter.undo_count() > 0 {
                    CommandAvailability::enabled()
                } else {
                    CommandAvailability::disabled("Nothing to undo")
                }
            } else if editor.edit_history.undo_count() > 0 {
                CommandAvailability::enabled()
            } else {
                CommandAvailability::disabled("Nothing to undo")
            }
        }
        EDIT_REDO_COMMAND => {
            if editor.scene.is_adapter_backed() {
                if adapter.redo_count() > 0 {
                    CommandAvailability::enabled()
                } else {
                    CommandAvailability::disabled("Nothing to redo")
                }
            } else if editor.edit_history.redo_count() > 0 {
                CommandAvailability::enabled()
            } else {
                CommandAvailability::disabled("Nothing to redo")
            }
        }
        ADAPTER_CONNECT_COMMAND | ADAPTER_SHOW_STATUS_COMMAND => CommandAvailability::enabled(),
        ADAPTER_DISCONNECT_COMMAND
        | ADAPTER_HANDSHAKE_COMMAND
        | ADAPTER_LOAD_PROJECT_COMMAND
        | ADAPTER_LOAD_SCENE_COMMAND
        | ADAPTER_SHOW_DIAGNOSTICS_COMMAND => {
            if adapter.is_connected() {
                CommandAvailability::enabled()
            } else {
                CommandAvailability::disabled("No adapter connected")
            }
        }
        VIEWPORT_REQUEST_FRAME_COMMAND => {
            if !adapter.is_connected() {
                CommandAvailability::disabled("No adapter connected")
            } else if !adapter.supports_viewport_preview() {
                CommandAvailability::disabled("Adapter does not support viewport preview")
            } else {
                CommandAvailability::enabled()
            }
        }
        VIEWPORT_CLEAR_COMMAND | VIEWPORT_SHOW_STATUS_COMMAND => CommandAvailability::enabled(),
        HELP_SHORTCUTS_COMMAND
        | HELP_COMMANDS_COMMAND
        | PALETTE_OPEN_COMMAND
        | PALETTE_CLOSE_COMMAND
        | SHOW_RENDERER_STATS_COMMAND
        | SHOW_READY_STATUS_COMMAND => CommandAvailability::enabled(),
        _ => CommandAvailability::enabled(),
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
    let availability = command_availability(command.id().as_str(), editor, adapter, viewport);
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
    use elcarax_scene_model::{ObjectSchema, SceneObject, SceneObjectKind};

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
    fn disabled_command_reports_reason() {
        let editor = EditorSessionState::default();
        let availability = command_availability(
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
            .execute_project_command(PROJECT_CREATE_COMMAND, None);
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
            .execute_project_command(PROJECT_CREATE_COMMAND, None);
        if let Some(snapshot) = editor.scene.snapshot_mut() {
            let schema = ObjectSchema::new("DirtyMarker");
            let object = SceneObject::new("Dirty Root", SceneObjectKind::World, schema.type_id);
            snapshot.add_schema(schema);
            snapshot.add_root_object(object);
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
    fn adapter_backed_scene_uses_adapter_undo_availability() {
        use elcarax_adapter_api::{
            AdapterId, AdapterRequestId, AdapterResponseMessage, SetPropertyResponse,
            SetPropertyStatus,
        };
        use elcarax_adapter_host::{AdapterSession, FakeAdapterTransport, response_line};
        use elcarax_scene_model::{
            PropertyEditKind, PropertyGroup, PropertyKind, PropertyPath, PropertySchema,
            PropertyValue, SceneName, SceneObject, SceneObjectKind, ScenePatch, SceneSnapshot,
        };

        use crate::adapter_state::AdapterState;

        let health_path = match PropertyPath::parse("gameplay.health") {
            Ok(path) => path,
            Err(error) => panic!("fixture path should parse: {error}"),
        };
        let schema = ObjectSchema::new("Actor").with_property(PropertySchema::editable(
            health_path.clone(),
            "Health",
            PropertyKind::I64,
            PropertyGroup::new("Gameplay"),
        ));
        let mut object =
            SceneObject::new("Fixture Actor", SceneObjectKind::Character, schema.type_id);
        object.set_property(health_path.clone(), PropertyValue::I64(100));
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Fixture Scene"));
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
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
        let response = match response_line(
            AdapterRequestId(1),
            AdapterResponseMessage::SetProperty(SetPropertyResponse {
                status: SetPropertyStatus::Accepted,
                scene_id,
                object_id,
                path: health_path.clone(),
                old_value: Some(PropertyValue::I64(100)),
                confirmed_new_value: Some(PropertyValue::I64(65)),
                patch: Some(ScenePatch::property_updated(
                    object_id,
                    health_path,
                    PropertyValue::I64(65),
                )),
                diagnostics: Vec::new(),
            }),
        ) {
            Ok(line) => line,
            Err(error) => panic!("response should serialize: {error}"),
        };
        let mut adapter = AdapterState::default();
        adapter.attach_fake_session_for_tests(AdapterSession::new(FakeAdapterTransport::new(vec![
            response,
        ])));
        let _ = adapter.commit_inspector_property(
            &mut editor.scene,
            "gameplay.health",
            PropertyEditKind::Integer,
            "65",
            "Set Fixture Health",
        );
        let availability = command_availability(
            EDIT_UNDO_COMMAND,
            &editor,
            &adapter,
            &AppViewportState::default(),
        );
        assert!(availability.is_enabled());
    }
}
