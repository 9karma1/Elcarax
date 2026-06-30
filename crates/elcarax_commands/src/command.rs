use elcarax_core::Result;
use elcarax_scene_model::SceneSnapshot;
use std::collections::BTreeMap;
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
pub struct CommandName(String);

impl CommandName {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, CommandRegistryError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CommandRegistryError::EmptyCommandName);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Palette,
    Project,
    Asset,
    Scene,
    Inspector,
    Adapter,
    Viewport,
    Status,
}

impl CommandCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Palette => "Palette",
            Self::Project => "Project",
            Self::Asset => "Asset",
            Self::Scene => "Scene",
            Self::Inspector => "Inspector",
            Self::Adapter => "Adapter",
            Self::Viewport => "Viewport",
            Self::Status => "Status",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredCommand {
    id: CommandId,
    name: CommandName,
    description: Option<CommandDescription>,
    category: CommandCategory,
    enabled: bool,
    order: usize,
}

impl RegisteredCommand {
    pub fn new(
        id: CommandId,
        name: CommandName,
        description: Option<CommandDescription>,
        category: CommandCategory,
    ) -> Self {
        Self {
            id,
            name,
            description,
            category,
            enabled: true,
            order: 0,
        }
    }

    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn id(&self) -> &CommandId {
        &self.id
    }

    pub fn name(&self) -> &CommandName {
        &self.name
    }

    pub fn description(&self) -> Option<&CommandDescription> {
        self.description.as_ref()
    }

    pub const fn category(&self) -> CommandCategory {
        self.category
    }

    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    pub const fn order(&self) -> usize {
        self.order
    }
}

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
    DuplicateCommandId(CommandId),
}

impl fmt::Display for CommandRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCommandId => write!(formatter, "command ID cannot be empty"),
            Self::EmptyCommandName => write!(formatter, "command name cannot be empty"),
            Self::DuplicateCommandId(id) => {
                write!(formatter, "duplicate command ID {}", id.as_str())
            }
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

pub fn built_in_commands() -> std::result::Result<CommandRegistry, CommandRegistryError> {
    let mut registry = CommandRegistry::new();
    for command in [
        registered(
            "elcarax.palette.open",
            "Open Command Palette",
            "Open the command palette overlay",
            CommandCategory::Palette,
        )?,
        registered(
            "elcarax.palette.close",
            "Close Command Palette",
            "Close the command palette overlay",
            CommandCategory::Palette,
        )?,
        registered(
            "project.create",
            "Create Project",
            "Create a project when project creation is implemented",
            CommandCategory::Project,
        )?,
        registered(
            "project.open",
            "Open Project",
            "Open a project when file loading is implemented",
            CommandCategory::Project,
        )?,
        registered(
            "project.close",
            "Close Project",
            "Return to no-project state",
            CommandCategory::Project,
        )?,
        registered(
            "project.validate",
            "Validate Project",
            "Validate the current project model",
            CommandCategory::Project,
        )?,
        registered(
            "asset.scan",
            "Scan Assets",
            "Scan assets when an asset root is loaded",
            CommandCategory::Asset,
        )?,
        registered(
            "asset.clear_selection",
            "Clear Asset Selection",
            "Clear the current asset selection",
            CommandCategory::Asset,
        )?,
        registered(
            "scene.load",
            "Load Scene",
            "Load a scene from a project or adapter when available",
            CommandCategory::Scene,
        )?,
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
            CommandCategory::Inspector,
        )?,
        registered(
            "edit.redo",
            "Redo",
            "Redo the last undone editor command",
            CommandCategory::Inspector,
        )?,
        registered(
            "adapter.connect",
            "Connect Adapter",
            "Connect an adapter when adapter configuration is available",
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
            "elcarax.status.show_renderer_stats",
            "Show Renderer Stats",
            "Show current primitive, text, and glyph counts",
            CommandCategory::Status,
        )?,
        registered(
            "elcarax.status.show_ready",
            "Show Ready Status",
            "Set the status label to ready",
            CommandCategory::Status,
        )?,
    ] {
        registry.register(command)?;
    }
    Ok(registry)
}

fn registered(
    id: &str,
    name: &str,
    description: &str,
    category: CommandCategory,
) -> std::result::Result<RegisteredCommand, CommandRegistryError> {
    Ok(RegisteredCommand::new(
        CommandId::new(id)?,
        CommandName::new(name)?,
        Some(CommandDescription::new(description)),
        category,
    ))
}

fn command_matches(command: &RegisteredCommand, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    command.id().as_str().to_lowercase().contains(query)
        || command.name().as_str().to_lowercase().contains(query)
        || command
            .description()
            .is_some_and(|description| description.as_str().to_lowercase().contains(query))
        || command.category().label().to_lowercase().contains(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(id: &str, name: &str) -> RegisteredCommand {
        match registered(id, name, "description", CommandCategory::Status) {
            Ok(command) => command,
            Err(error) => panic!("test command should be valid: {error}"),
        }
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
            registry.get(&id).map(|command| command.name().as_str()),
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
        assert!(ids.contains(&"project.close"));
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
                "asset.scan",
                "asset.clear_selection",
                "scene.load",
                "scene.clear",
                "scene.clear_selection",
                "inspector.clear",
                "edit.undo",
                "edit.redo",
                "adapter.connect",
                "adapter.disconnect",
                "adapter.show_status",
                "adapter.show_diagnostics",
                "adapter.load_scene",
                "viewport.request_frame",
                "viewport.clear",
                "viewport.show_status",
                "elcarax.status.show_renderer_stats",
                "elcarax.status.show_ready"
            ]
        );
    }

    #[test]
    fn asset_commands_are_discoverable() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let matches = registry.filter("asset");
        let ids: Vec<_> = matches
            .into_iter()
            .map(|command| command.id().as_str())
            .collect();
        assert!(ids.contains(&"asset.scan"));
        assert!(ids.contains(&"asset.clear_selection"));
    }

    #[test]
    fn scene_commands_are_discoverable() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let matches = registry.filter("scene");
        let ids: Vec<_> = matches
            .into_iter()
            .map(|command| command.id().as_str())
            .collect();
        assert!(ids.contains(&"scene.load"));
        assert!(ids.contains(&"scene.clear"));
        assert!(ids.contains(&"scene.clear_selection"));
    }

    #[test]
    fn inspector_commands_are_discoverable() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let matches = registry.filter("inspector");
        let ids: Vec<_> = matches
            .into_iter()
            .map(|command| command.id().as_str())
            .collect();
        assert!(ids.contains(&"inspector.clear"));
        assert!(ids.contains(&"edit.undo"));
        assert!(ids.contains(&"edit.redo"));
    }

    #[test]
    fn adapter_commands_are_discoverable() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let matches = registry.filter("adapter");
        let ids: Vec<_> = matches
            .into_iter()
            .map(|command| command.id().as_str())
            .collect();
        assert!(ids.contains(&"adapter.connect"));
        assert!(ids.contains(&"adapter.load_scene"));
        assert!(ids.contains(&"adapter.disconnect"));
    }

    #[test]
    fn disabled_command_does_not_execute() {
        let mut registry = CommandRegistry::new();
        let disabled = command("elcarax.disabled", "Disabled").disabled();
        let id = disabled.id().clone();
        assert!(registry.register(disabled).is_ok());
        assert_eq!(registry.invoke(&id), CommandResult::Disabled(id));
    }
}
