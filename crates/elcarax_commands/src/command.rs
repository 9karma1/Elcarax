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
    Status,
    Demo,
}

impl CommandCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Palette => "Palette",
            Self::Project => "Project",
            Self::Status => "Status",
            Self::Demo => "Demo",
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
            "project.new_demo",
            "New Demo Project",
            "Create an in-memory demo project",
            CommandCategory::Project,
        )?,
        registered(
            "project.open_demo",
            "Open Demo Project",
            "Load a sample demo project path",
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
            "project.show_recent",
            "Show Recent Projects",
            "Report recent project entries",
            CommandCategory::Project,
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
        registered(
            "elcarax.demo.run",
            "Run Demo Action",
            "Run the demo status action",
            CommandCategory::Demo,
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
        match registered(id, name, "description", CommandCategory::Demo) {
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
        assert!(ids.contains(&"project.new_demo"));
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
                "project.new_demo",
                "project.open_demo",
                "project.close",
                "project.validate",
                "project.show_recent",
                "elcarax.status.show_renderer_stats",
                "elcarax.status.show_ready",
                "elcarax.demo.run"
            ]
        );
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
