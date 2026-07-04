use elcarax_core::Result;
use elcarax_scene_model::SceneSnapshot;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEffect {
    NoChange,
    SceneChanged,
    ProjectChanged,
    UiChanged,
}

pub struct CommandContext<'a> {
    pub scene: &'a mut SceneSnapshot,
}

pub trait EditorCommand {
    fn label(&self) -> &str;
    fn apply(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect>;
    fn revert(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect>;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(String);

impl CommandId {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, CommandRegistryError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CommandRegistryError::EmptyCommandId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandTitle(String);

pub type CommandName = CommandTitle;

impl CommandTitle {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, CommandRegistryError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CommandRegistryError::EmptyCommandTitle);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDescription(String);

impl CommandDescription {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandCategory {
    Project,
    Asset,
    Scene,
    Edit,
    Viewport,
    Adapter,
    Window,
    Help,
    Developer,
    Palette,
    Inspector,
    Status,
}

impl CommandCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Asset => "Asset",
            Self::Scene => "Scene",
            Self::Edit => "Edit",
            Self::Viewport => "Viewport",
            Self::Adapter => "Adapter",
            Self::Window => "Window",
            Self::Help => "Help",
            Self::Developer => "Developer",
            Self::Palette => "Palette",
            Self::Inspector => "Inspector",
            Self::Status => "Status",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandScope {
    Global,
    Overlay,
    TextEdit,
}

impl CommandScope {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Overlay => "Overlay",
            Self::TextEdit => "Text Edit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAvailability {
    enabled: bool,
    disabled_reason: Option<String>,
}

impl CommandAvailability {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            disabled_reason: None,
        }
    }

    pub fn disabled(reason: impl Into<String>) -> Self {
        Self {
            enabled: false,
            disabled_reason: Some(reason.into()),
        }
    }

    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn disabled_reason(&self) -> Option<&str> {
        self.disabled_reason.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPresentation {
    palette_visible: bool,
    toolbar: Option<CommandToolbarPlacement>,
}

impl CommandPresentation {
    pub fn palette_only() -> Self {
        Self {
            palette_visible: true,
            toolbar: None,
        }
    }

    pub fn hidden() -> Self {
        Self {
            palette_visible: false,
            toolbar: None,
        }
    }

    pub fn with_toolbar(mut self, placement: CommandToolbarPlacement) -> Self {
        self.toolbar = Some(placement);
        self
    }

    pub const fn palette_visible(&self) -> bool {
        self.palette_visible
    }

    pub const fn toolbar(&self) -> Option<&CommandToolbarPlacement> {
        self.toolbar.as_ref()
    }
}

impl Default for CommandPresentation {
    fn default() -> Self {
        Self::palette_only()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandToolbarPlacement {
    pub section: CommandCategory,
    pub order: usize,
    pub short_label: String,
}

impl CommandToolbarPlacement {
    pub fn new(section: CommandCategory, order: usize, short_label: impl Into<String>) -> Self {
        Self {
            section,
            order,
            short_label: short_label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyModifier {
    Control,
    Shift,
    Alt,
    Super,
}

impl KeyModifier {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Control => "Ctrl",
            Self::Shift => "Shift",
            Self::Alt => "Alt",
            Self::Super => "Super",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyChord {
    modifiers: Vec<KeyModifier>,
    key: String,
}

impl KeyChord {
    pub fn new(
        modifiers: impl IntoIterator<Item = KeyModifier>,
        key: impl Into<String>,
    ) -> std::result::Result<Self, CommandRegistryError> {
        let key = normalize_key_name(key.into().as_str());
        if key.is_empty() {
            return Err(CommandRegistryError::EmptyKeyChord);
        }
        if !is_supported_key_name(key.as_str()) {
            return Err(CommandRegistryError::UnsupportedKeyName(key));
        }
        Ok(Self {
            modifiers: sorted_modifiers(modifiers),
            key,
        })
    }

    pub fn key(&self) -> &str {
        self.key.as_str()
    }

    pub fn modifiers(&self) -> &[KeyModifier] {
        self.modifiers.as_slice()
    }

    pub fn display_label(&self) -> String {
        let mut parts: Vec<String> = self
            .modifiers
            .iter()
            .map(|modifier| modifier.label().to_string())
            .collect();
        parts.push(self.key.clone());
        parts.join("+")
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.display_label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandShortcut {
    binding: KeyBinding,
}

impl CommandShortcut {
    pub fn new(binding: KeyBinding) -> Self {
        Self { binding }
    }

    pub const fn binding(&self) -> &KeyBinding {
        &self.binding
    }

    pub fn display_label(&self) -> String {
        self.binding.chord().display_label()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    chord: KeyChord,
    scope: CommandScope,
}

impl KeyBinding {
    pub fn global(chord: KeyChord) -> Self {
        Self {
            chord,
            scope: CommandScope::Global,
        }
    }

    pub fn new(chord: KeyChord, scope: CommandScope) -> Self {
        Self { chord, scope }
    }

    pub const fn chord(&self) -> &KeyChord {
        &self.chord
    }

    pub const fn scope(&self) -> CommandScope {
        self.scope
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMetadata {
    id: CommandId,
    title: CommandTitle,
    description: Option<CommandDescription>,
    category: CommandCategory,
    shortcuts: Vec<CommandShortcut>,
    availability: CommandAvailability,
    presentation: CommandPresentation,
    order: usize,
}

impl CommandMetadata {
    pub fn new(
        id: CommandId,
        title: CommandTitle,
        description: Option<CommandDescription>,
        category: CommandCategory,
    ) -> Self {
        Self {
            id,
            title,
            description,
            category,
            shortcuts: Vec::new(),
            availability: CommandAvailability::enabled(),
            presentation: CommandPresentation::default(),
            order: 0,
        }
    }

    pub fn with_shortcut(mut self, shortcut: CommandShortcut) -> Self {
        self.shortcuts.push(shortcut);
        self
    }

    pub fn with_presentation(mut self, presentation: CommandPresentation) -> Self {
        self.presentation = presentation;
        self
    }

    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.availability = CommandAvailability::disabled(reason);
        self
    }

    pub fn id(&self) -> &CommandId {
        &self.id
    }

    pub fn title(&self) -> &CommandTitle {
        &self.title
    }

    pub fn name(&self) -> &CommandTitle {
        &self.title
    }

    pub fn description(&self) -> Option<&CommandDescription> {
        self.description.as_ref()
    }

    pub const fn category(&self) -> CommandCategory {
        self.category
    }

    pub fn shortcut(&self) -> Option<&CommandShortcut> {
        self.shortcuts.first()
    }

    pub fn shortcuts(&self) -> &[CommandShortcut] {
        self.shortcuts.as_slice()
    }

    pub const fn availability(&self) -> &CommandAvailability {
        &self.availability
    }

    pub const fn presentation(&self) -> &CommandPresentation {
        &self.presentation
    }

    pub const fn enabled(&self) -> bool {
        self.availability.enabled
    }

    pub const fn order(&self) -> usize {
        self.order
    }
}

pub type RegisteredCommand = CommandMetadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    pub id: CommandId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    Invoked(CommandInvocation),
    Disabled(CommandId),
    NotFound(CommandId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandRegistryError {
    EmptyCommandId,
    EmptyCommandName,
    EmptyCommandTitle,
    DuplicateCommandId(CommandId),
    EmptyKeyChord,
    UnsupportedKeyName(String),
}

impl fmt::Display for CommandRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCommandId => write!(formatter, "command ID cannot be empty"),
            Self::EmptyCommandName | Self::EmptyCommandTitle => {
                write!(formatter, "command title cannot be empty")
            }
            Self::DuplicateCommandId(id) => {
                write!(formatter, "duplicate command ID {}", id.as_str())
            }
            Self::EmptyKeyChord => write!(formatter, "key chord cannot be empty"),
            Self::UnsupportedKeyName(key) => write!(formatter, "unsupported key name {key}"),
        }
    }
}

impl Error for CommandRegistryError {}

#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    commands: BTreeMap<CommandId, RegisteredCommand>,
    next_order: usize,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        mut command: RegisteredCommand,
    ) -> std::result::Result<(), CommandRegistryError> {
        if self.commands.contains_key(command.id()) {
            return Err(CommandRegistryError::DuplicateCommandId(
                command.id().clone(),
            ));
        }
        command.order = self.next_order;
        self.next_order += 1;
        self.commands.insert(command.id().clone(), command);
        Ok(())
    }

    pub fn get(&self, id: &CommandId) -> Option<&RegisteredCommand> {
        self.commands.get(id)
    }

    pub fn all(&self) -> Vec<&RegisteredCommand> {
        let mut commands: Vec<_> = self.commands.values().collect();
        commands.sort_by_key(|command| command.order());
        commands
    }

    pub fn filter(&self, query: &str) -> Vec<&RegisteredCommand> {
        let query = query.trim().to_lowercase();
        self.all()
            .into_iter()
            .filter(|command| command_matches(command, &query))
            .collect()
    }

    pub fn invoke(&self, id: &CommandId) -> CommandResult {
        let Some(command) = self.commands.get(id) else {
            return CommandResult::NotFound(id.clone());
        };
        if !command.enabled() {
            return CommandResult::Disabled(id.clone());
        }
        CommandResult::Invoked(CommandInvocation { id: id.clone() })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandBindingDiagnostic {
    EmptyKeyChord {
        command_id: CommandId,
    },
    UnsupportedKeyName {
        command_id: CommandId,
        key: String,
    },
    Conflict {
        scope: CommandScope,
        chord: KeyChord,
        command_ids: Vec<CommandId>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct CommandBindingRegistry {
    bindings: Vec<RegisteredBinding>,
    diagnostics: Vec<CommandBindingDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegisteredBinding {
    command_id: CommandId,
    binding: KeyBinding,
    order: usize,
}

impl CommandBindingRegistry {
    pub fn from_commands(commands: &CommandRegistry) -> Self {
        let mut registry = Self::default();
        for command in commands.all() {
            for shortcut in command.shortcuts() {
                registry.register(
                    command.id().clone(),
                    shortcut.binding().clone(),
                    command.order,
                );
            }
        }
        registry.detect_conflicts();
        registry
    }

    pub fn register(&mut self, command_id: CommandId, binding: KeyBinding, order: usize) {
        if binding.chord().key().is_empty() {
            self.diagnostics
                .push(CommandBindingDiagnostic::EmptyKeyChord { command_id });
            return;
        }
        if !is_supported_key_name(binding.chord().key()) {
            self.diagnostics
                .push(CommandBindingDiagnostic::UnsupportedKeyName {
                    command_id,
                    key: binding.chord().key().to_string(),
                });
            return;
        }
        self.bindings.push(RegisteredBinding {
            command_id,
            binding,
            order,
        });
    }

    pub fn command_for_chord(&self, chord: &KeyChord, scope: CommandScope) -> Option<&CommandId> {
        self.bindings
            .iter()
            .filter(|binding| binding.binding.scope() == scope && binding.binding.chord() == chord)
            .min_by_key(|binding| binding.order)
            .map(|binding| &binding.command_id)
    }

    pub fn shortcut_for_command(&self, command_id: &CommandId) -> Option<&KeyBinding> {
        self.bindings
            .iter()
            .find(|binding| &binding.command_id == command_id)
            .map(|binding| &binding.binding)
    }

    pub fn bindings(&self) -> Vec<(&CommandId, &KeyBinding)> {
        let mut bindings: Vec<_> = self
            .bindings
            .iter()
            .map(|binding| (&binding.command_id, &binding.binding))
            .collect();
        bindings.sort_by_key(|(command_id, binding)| {
            (
                binding.scope(),
                binding.chord().clone(),
                (*command_id).clone(),
            )
        });
        bindings
    }

    pub fn diagnostics(&self) -> &[CommandBindingDiagnostic] {
        self.diagnostics.as_slice()
    }

    fn detect_conflicts(&mut self) {
        let mut grouped: BTreeMap<(CommandScope, KeyChord), Vec<&RegisteredBinding>> =
            BTreeMap::new();
        for binding in &self.bindings {
            grouped
                .entry((binding.binding.scope(), binding.binding.chord().clone()))
                .or_default()
                .push(binding);
        }
        for ((scope, chord), mut conflicts) in grouped {
            if conflicts.len() < 2 {
                continue;
            }
            conflicts.sort_by_key(|binding| binding.order);
            self.diagnostics.push(CommandBindingDiagnostic::Conflict {
                scope,
                chord,
                command_ids: conflicts
                    .into_iter()
                    .map(|binding| binding.command_id.clone())
                    .collect(),
            });
        }
    }
}

pub fn built_in_commands() -> std::result::Result<CommandRegistry, CommandRegistryError> {
    let mut registry = CommandRegistry::new();
    for command in default_commands()? {
        registry.register(command)?;
    }
    Ok(registry)
}

fn default_commands() -> std::result::Result<Vec<RegisteredCommand>, CommandRegistryError> {
    Ok(vec![
        registered(
            "elcarax.palette.open",
            "Open Command Palette",
            "Open the command palette overlay",
            CommandCategory::Window,
        )?
        .with_shortcut(global_shortcut(&[KeyModifier::Control], "K")?),
        registered(
            "elcarax.palette.close",
            "Close Command Palette",
            "Close the command palette overlay",
            CommandCategory::Window,
        )?,
        registered(
            "project.create",
            "Create Project",
            "Create a project at the configured project path",
            CommandCategory::Project,
        )?
        .with_shortcut(global_shortcut(&[KeyModifier::Control], "N")?)
        .with_presentation(toolbar(CommandCategory::Project, 0, "New")),
        registered(
            "project.open",
            "Open Project",
            "Open a project from the configured project path",
            CommandCategory::Project,
        )?
        .with_shortcut(global_shortcut(&[KeyModifier::Control], "O")?)
        .with_presentation(toolbar(CommandCategory::Project, 1, "Open")),
        registered(
            "project.close",
            "Close Project",
            "Return to no-project state",
            CommandCategory::Project,
        )?,
        registered(
            "project.validate",
            "Validate Project",
            "Validate the current project manifest and paths",
            CommandCategory::Project,
        )?,
        registered(
            "project.show_recent",
            "Show Recent Projects",
            "Show the recent projects summary",
            CommandCategory::Project,
        )?,
        registered(
            "project.reopen_last",
            "Reopen Last Project",
            "Open the most recent project when available",
            CommandCategory::Project,
        )?
        .with_shortcut(global_shortcut(
            &[KeyModifier::Control, KeyModifier::Shift],
            "O",
        )?),
        registered(
            "asset.scan",
            "Scan Assets",
            "Scan assets when an asset root is loaded",
            CommandCategory::Asset,
        )?
        .with_presentation(toolbar(CommandCategory::Asset, 5, "Scan")),
        registered(
            "asset.refresh",
            "Refresh Assets",
            "Rescan the loaded project's asset root and clear dirty state",
            CommandCategory::Asset,
        )?
        .with_presentation(toolbar(CommandCategory::Asset, 6, "Refresh")),
        registered(
            "asset.start_watching",
            "Start Asset Watcher",
            "Watch the loaded project's asset root for filesystem changes",
            CommandCategory::Asset,
        )?,
        registered(
            "asset.stop_watching",
            "Stop Asset Watcher",
            "Stop watching the current project asset root",
            CommandCategory::Asset,
        )?,
        registered(
            "asset.clear_selection",
            "Clear Asset Selection",
            "Clear the current asset selection",
            CommandCategory::Asset,
        )?,
        registered(
            "asset.show_selected",
            "Show Selected Asset",
            "Report the current selected asset",
            CommandCategory::Asset,
        )?,
        registered(
            "asset.reveal_root",
            "Reveal Asset Root",
            "Report the current project asset root path",
            CommandCategory::Asset,
        )?,
        registered(
            "scene.load",
            "Load Scene",
            "Load the active scene from the open project scene root",
            CommandCategory::Scene,
        )?,
        registered(
            "scene.save",
            "Save Scene",
            "Save the loaded project scene back to disk",
            CommandCategory::Scene,
        )?
        .with_shortcut(global_shortcut(&[KeyModifier::Control], "S")?)
        .with_presentation(toolbar(CommandCategory::Scene, 2, "Save")),
        registered(
            "scene.clear",
            "Clear Scene",
            "Unload the current scene",
            CommandCategory::Scene,
        )?,
        registered(
            "scene.clear_selection",
            "Clear Scene Selection",
            "Clear the current scene object selection",
            CommandCategory::Scene,
        )?,
        registered(
            "inspector.clear",
            "Clear Inspector",
            "Clear the inspector view",
            CommandCategory::Inspector,
        )?,
        registered(
            "edit.undo",
            "Undo",
            "Undo the last editor command",
            CommandCategory::Edit,
        )?
        .with_shortcut(global_shortcut(&[KeyModifier::Control], "Z")?)
        .with_presentation(toolbar(CommandCategory::Edit, 3, "Undo")),
        registered(
            "edit.redo",
            "Redo",
            "Redo the last undone editor command",
            CommandCategory::Edit,
        )?
        .with_shortcut(global_shortcut(&[KeyModifier::Control], "Y")?)
        .with_shortcut(global_shortcut(
            &[KeyModifier::Control, KeyModifier::Shift],
            "Z",
        )?)
        .with_presentation(toolbar(CommandCategory::Edit, 4, "Redo")),
        registered(
            "adapter.connect",
            "Connect Adapter",
            "Connect an adapter when adapter configuration is available",
            CommandCategory::Adapter,
        )?
        .with_presentation(toolbar(CommandCategory::Adapter, 7, "Connect")),
        registered(
            "adapter.handshake",
            "Handshake Adapter",
            "Run the adapter handshake for the current adapter session",
            CommandCategory::Adapter,
        )?,
        registered(
            "adapter.load_project",
            "Load Adapter Project",
            "Send the current project path to the connected adapter",
            CommandCategory::Adapter,
        )?,
        registered(
            "adapter.disconnect",
            "Disconnect Adapter",
            "Disconnect the current adapter session",
            CommandCategory::Adapter,
        )?,
        registered(
            "adapter.show_status",
            "Show Adapter Status",
            "Report the current adapter connection state",
            CommandCategory::Adapter,
        )?,
        registered(
            "adapter.show_diagnostics",
            "Show Adapter Diagnostics",
            "Request and report adapter diagnostics",
            CommandCategory::Adapter,
        )?,
        registered(
            "adapter.load_scene",
            "Load Adapter Scene",
            "Request a scene snapshot from a connected adapter",
            CommandCategory::Adapter,
        )?,
        registered(
            "viewport.request_frame",
            "Request Viewport Frame",
            "Request a preview frame from the connected adapter",
            CommandCategory::Viewport,
        )?,
        registered(
            "viewport.clear",
            "Clear Viewport",
            "Clear the current viewport preview frame",
            CommandCategory::Viewport,
        )?,
        registered(
            "viewport.show_status",
            "Show Viewport Status",
            "Report the current viewport source and status",
            CommandCategory::Viewport,
        )?,
        registered(
            "help.shortcuts",
            "Show Shortcuts",
            "Report the default keybinding summary",
            CommandCategory::Help,
        )?,
        registered(
            "help.commands",
            "Show Commands",
            "Report the command registry summary",
            CommandCategory::Help,
        )?,
        registered(
            "elcarax.status.show_renderer_stats",
            "Show Renderer Stats",
            "Show current primitive, text, and glyph counts",
            CommandCategory::Developer,
        )?,
        registered(
            "elcarax.status.show_ready",
            "Show Ready Status",
            "Set the status label to ready",
            CommandCategory::Status,
        )?,
    ])
}

fn registered(
    id: &str,
    title: &str,
    description: &str,
    category: CommandCategory,
) -> std::result::Result<RegisteredCommand, CommandRegistryError> {
    Ok(RegisteredCommand::new(
        CommandId::new(id)?,
        CommandTitle::new(title)?,
        Some(CommandDescription::new(description)),
        category,
    ))
}

fn global_shortcut(
    modifiers: &[KeyModifier],
    key: &str,
) -> std::result::Result<CommandShortcut, CommandRegistryError> {
    Ok(CommandShortcut::new(KeyBinding::global(KeyChord::new(
        modifiers.iter().cloned(),
        key,
    )?)))
}

fn toolbar(section: CommandCategory, order: usize, label: &str) -> CommandPresentation {
    CommandPresentation::palette_only()
        .with_toolbar(CommandToolbarPlacement::new(section, order, label))
}

fn sorted_modifiers(modifiers: impl IntoIterator<Item = KeyModifier>) -> Vec<KeyModifier> {
    let mut unique = BTreeSet::new();
    for modifier in modifiers {
        unique.insert(modifier);
    }
    unique.into_iter().collect()
}

fn normalize_key_name(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.chars().count() == 1 {
        return trimmed.to_ascii_uppercase();
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "esc" | "escape" => "Escape".to_string(),
        "enter" | "return" => "Enter".to_string(),
        "space" => "Space".to_string(),
        "tab" => "Tab".to_string(),
        "backspace" => "Backspace".to_string(),
        "arrowleft" | "left" => "ArrowLeft".to_string(),
        "arrowright" | "right" => "ArrowRight".to_string(),
        "arrowup" | "up" => "ArrowUp".to_string(),
        "arrowdown" | "down" => "ArrowDown".to_string(),
        _ => trimmed.to_string(),
    }
}

fn is_supported_key_name(key: &str) -> bool {
    if key.chars().count() == 1 {
        return key
            .chars()
            .all(|character| character.is_ascii_alphanumeric());
    }
    matches!(
        key,
        "Escape"
            | "Enter"
            | "Space"
            | "Tab"
            | "Backspace"
            | "ArrowLeft"
            | "ArrowRight"
            | "ArrowUp"
            | "ArrowDown"
    )
}

fn command_matches(command: &RegisteredCommand, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    command.id().as_str().to_lowercase().contains(query)
        || command.title().as_str().to_lowercase().contains(query)
        || command
            .description()
            .is_some_and(|description| description.as_str().to_lowercase().contains(query))
        || command.category().label().to_lowercase().contains(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(id: &str, title: &str) -> RegisteredCommand {
        match registered(id, title, "description", CommandCategory::Status) {
            Ok(command) => command,
            Err(error) => panic!("test command should be valid: {error}"),
        }
    }

    fn chord(modifiers: &[KeyModifier], key: &str) -> KeyChord {
        match KeyChord::new(modifiers.iter().cloned(), key) {
            Ok(chord) => chord,
            Err(error) => panic!("test chord should be valid: {error}"),
        }
    }

    #[test]
    fn command_metadata_creation_preserves_fields() {
        let command = command("elcarax.test", "Test Command").with_shortcut(
            global_shortcut(&[KeyModifier::Control], "T")
                .unwrap_or_else(|error| panic!("shortcut should be valid: {error}")),
        );
        assert_eq!(command.id().as_str(), "elcarax.test");
        assert_eq!(command.title().as_str(), "Test Command");
        assert_eq!(command.category(), CommandCategory::Status);
        assert_eq!(
            command.shortcut().map(CommandShortcut::display_label),
            Some("Ctrl+T".to_string())
        );
    }

    #[test]
    fn command_registration_preserves_lookup() {
        let mut registry = CommandRegistry::new();
        let id = match CommandId::new("elcarax.test") {
            Ok(id) => id,
            Err(error) => panic!("test ID should be valid: {error}"),
        };
        assert!(
            registry
                .register(command(id.as_str(), "Test Command"))
                .is_ok()
        );
        assert_eq!(
            registry.get(&id).map(|command| command.title().as_str()),
            Some("Test Command")
        );
    }

    #[test]
    fn duplicate_command_ids_are_rejected() {
        let mut registry = CommandRegistry::new();
        assert!(registry.register(command("elcarax.test", "One")).is_ok());
        assert!(matches!(
            registry.register(command("elcarax.test", "Two")),
            Err(CommandRegistryError::DuplicateCommandId(_))
        ));
    }

    #[test]
    fn categories_cover_editor_domains() {
        let labels = [
            CommandCategory::Project.label(),
            CommandCategory::Asset.label(),
            CommandCategory::Scene.label(),
            CommandCategory::Edit.label(),
            CommandCategory::Viewport.label(),
            CommandCategory::Adapter.label(),
            CommandCategory::Window.label(),
            CommandCategory::Help.label(),
            CommandCategory::Developer.label(),
        ];
        assert_eq!(
            labels,
            [
                "Project",
                "Asset",
                "Scene",
                "Edit",
                "Viewport",
                "Adapter",
                "Window",
                "Help",
                "Developer"
            ]
        );
    }

    #[test]
    fn shortcut_display_format_is_stable() {
        assert_eq!(
            chord(&[KeyModifier::Shift, KeyModifier::Control], "z").display_label(),
            "Ctrl+Shift+Z"
        );
    }

    #[test]
    fn unsupported_shortcut_key_is_rejected() {
        assert!(matches!(
            KeyChord::new([KeyModifier::Control], "F13"),
            Err(CommandRegistryError::UnsupportedKeyName(_))
        ));
    }

    #[test]
    fn default_keybindings_are_conflict_free() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let bindings = CommandBindingRegistry::from_commands(&registry);
        assert_eq!(bindings.diagnostics(), &[]);
    }

    #[test]
    fn conflict_detection_is_deterministic() {
        let mut registry = CommandRegistry::new();
        assert!(
            registry
                .register(
                    command("a.first", "First").with_shortcut(
                        global_shortcut(&[KeyModifier::Control], "A").unwrap_or_else(|error| {
                            panic!("shortcut should be valid: {error}")
                        })
                    )
                )
                .is_ok()
        );
        assert!(
            registry
                .register(
                    command("b.second", "Second").with_shortcut(
                        global_shortcut(&[KeyModifier::Control], "A").unwrap_or_else(|error| {
                            panic!("shortcut should be valid: {error}")
                        })
                    )
                )
                .is_ok()
        );
        let bindings = CommandBindingRegistry::from_commands(&registry);
        assert_eq!(bindings.diagnostics().len(), 1);
        let CommandBindingDiagnostic::Conflict { command_ids, .. } = &bindings.diagnostics()[0]
        else {
            panic!("expected conflict diagnostic");
        };
        let ids: Vec<_> = command_ids.iter().map(CommandId::as_str).collect();
        assert_eq!(ids, vec!["a.first", "b.second"]);
    }

    #[test]
    fn command_lookup_by_key_chord_uses_defaults() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let bindings = CommandBindingRegistry::from_commands(&registry);
        let save =
            bindings.command_for_chord(&chord(&[KeyModifier::Control], "S"), CommandScope::Global);
        assert_eq!(save.map(CommandId::as_str), Some("scene.save"));
    }

    #[test]
    fn missing_binding_returns_none() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let bindings = CommandBindingRegistry::from_commands(&registry);
        assert!(
            bindings
                .command_for_chord(&chord(&[KeyModifier::Control], "B"), CommandScope::Global)
                .is_none()
        );
    }

    #[test]
    fn command_filtering_uses_query() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let matches = registry.filter("ready");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id().as_str(), "elcarax.status.show_ready");
    }

    #[test]
    fn project_commands_are_discoverable() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let matches = registry.filter("project");
        let ids: Vec<_> = matches
            .into_iter()
            .map(|command| command.id().as_str())
            .collect();
        assert!(ids.contains(&"project.create"));
        assert!(ids.contains(&"project.open"));
        assert!(ids.contains(&"project.validate"));
        assert!(ids.contains(&"project.show_recent"));
        assert!(ids.contains(&"project.reopen_last"));
    }

    #[test]
    fn empty_query_returns_stable_order() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let ids: Vec<_> = registry
            .filter("")
            .into_iter()
            .map(|command| command.id().as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "elcarax.palette.open",
                "elcarax.palette.close",
                "project.create",
                "project.open",
                "project.close",
                "project.validate",
                "project.show_recent",
                "project.reopen_last",
                "asset.scan",
                "asset.refresh",
                "asset.start_watching",
                "asset.stop_watching",
                "asset.clear_selection",
                "asset.show_selected",
                "asset.reveal_root",
                "scene.load",
                "scene.save",
                "scene.clear",
                "scene.clear_selection",
                "inspector.clear",
                "edit.undo",
                "edit.redo",
                "adapter.connect",
                "adapter.handshake",
                "adapter.load_project",
                "adapter.disconnect",
                "adapter.show_status",
                "adapter.show_diagnostics",
                "adapter.load_scene",
                "viewport.request_frame",
                "viewport.clear",
                "viewport.show_status",
                "help.shortcuts",
                "help.commands",
                "elcarax.status.show_renderer_stats",
                "elcarax.status.show_ready"
            ]
        );
    }

    #[test]
    fn disabled_command_does_not_execute() {
        let mut registry = CommandRegistry::new();
        let disabled = command("elcarax.disabled", "Disabled").disabled("disabled");
        let id = disabled.id().clone();
        assert!(registry.register(disabled).is_ok());
        assert_eq!(registry.invoke(&id), CommandResult::Disabled(id));
    }
}
