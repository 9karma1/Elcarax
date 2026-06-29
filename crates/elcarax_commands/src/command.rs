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
    Status,
    Demo,
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
            "asset.scan_demo",
            "Scan Demo Assets",
            "Populate the asset index with demo assets",
            CommandCategory::Asset,
        )?,
        registered(
            "asset.select_first",
            "Select First Asset",
            "Select the first asset in stable order",
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
            "Report the currently selected asset",
            CommandCategory::Asset,
        )?,
        registered(
            "scene.load_demo",
            "Load Demo Scene",
            "Load the deterministic demo scene snapshot",
            CommandCategory::Scene,
        )?,
        registered(
            "scene.select_root",
            "Select Root Object",
            "Select the root scene object",
            CommandCategory::Scene,
        )?,
        registered(
            "scene.select_player",
            "Select Player Object",
            "Select the Player scene object",
            CommandCategory::Scene,
        )?,
        registered(
            "scene.clear_selection",
            "Clear Scene Selection",
            "Clear the current scene object selection",
            CommandCategory::Scene,
        )?,
        registered(
            "scene.expand_all",
            "Expand Scene Tree",
            "Expand all scene tree nodes",
            CommandCategory::Scene,
        )?,
        registered(
            "scene.collapse_all",
            "Collapse Scene Tree",
            "Collapse all scene tree nodes",
            CommandCategory::Scene,
        )?,
        registered(
            "scene.show_selected",
            "Show Selected Scene Object",
            "Report the currently selected scene object",
            CommandCategory::Scene,
        )?,
        registered(
            "inspector.show_selected",
            "Show Selected Inspector",
            "Build inspector rows from the current scene selection",
            CommandCategory::Inspector,
        )?,
        registered(
            "inspector.clear",
            "Clear Inspector",
            "Clear the inspector view",
            CommandCategory::Inspector,
        )?,
        registered(
            "inspector.inspect_player",
            "Inspect Player",
            "Select and inspect the Player object",
            CommandCategory::Inspector,
        )?,
        registered(
            "inspector.inspect_root",
            "Inspect Root Object",
            "Select and inspect the root scene object",
            CommandCategory::Inspector,
        )?,
        registered(
            "inspector.show_property_count",
            "Show Inspector Property Count",
            "Report the visible inspector property count",
            CommandCategory::Inspector,
        )?,
        registered(
            "inspector.set_player_health_demo",
            "Set Player Health Demo",
            "Set the selected Player health property through undoable inspector edit flow",
            CommandCategory::Inspector,
        )?,
        registered(
            "inspector.set_player_speed_demo",
            "Set Player Speed Demo",
            "Set the selected Player speed property through undoable inspector edit flow",
            CommandCategory::Inspector,
        )?,
        registered(
            "inspector.rename_player_demo",
            "Rename Player Demo",
            "Rename the selected Player through undoable inspector edit flow",
            CommandCategory::Inspector,
        )?,
        registered(
            "inspector.reset_player_transform_demo",
            "Reset Player Transform Demo",
            "Reset selected Player transform properties through inspector edit flow",
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
            "adapter.start_mock",
            "Start Mock Adapter",
            "Spawn the mock adapter process and perform the JSON-line handshake",
            CommandCategory::Adapter,
        )?,
        registered(
            "adapter.handshake",
            "Handshake Adapter",
            "Perform the versioned adapter handshake",
            CommandCategory::Adapter,
        )?,
        registered(
            "adapter.load_demo_project",
            "Load Adapter Demo Project",
            "Request demo project information from the adapter",
            CommandCategory::Adapter,
        )?,
        registered(
            "adapter.load_demo_scene",
            "Load Adapter Demo Scene",
            "Request a demo scene snapshot from the adapter",
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
            "adapter.stop_mock",
            "Stop Mock Adapter",
            "Gracefully shut down the mock adapter process",
            CommandCategory::Adapter,
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
                "asset.scan_demo",
                "asset.select_first",
                "asset.clear_selection",
                "asset.show_selected",
                "scene.load_demo",
                "scene.select_root",
                "scene.select_player",
                "scene.clear_selection",
                "scene.expand_all",
                "scene.collapse_all",
                "scene.show_selected",
                "inspector.show_selected",
                "inspector.clear",
                "inspector.inspect_player",
                "inspector.inspect_root",
                "inspector.show_property_count",
                "inspector.set_player_health_demo",
                "inspector.set_player_speed_demo",
                "inspector.rename_player_demo",
                "inspector.reset_player_transform_demo",
                "edit.undo",
                "edit.redo",
                "adapter.start_mock",
                "adapter.handshake",
                "adapter.load_demo_project",
                "adapter.load_demo_scene",
                "adapter.show_status",
                "adapter.show_diagnostics",
                "adapter.stop_mock",
                "elcarax.status.show_renderer_stats",
                "elcarax.status.show_ready",
                "elcarax.demo.run"
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
        assert!(ids.contains(&"asset.scan_demo"));
        assert!(ids.contains(&"asset.select_first"));
        assert!(ids.contains(&"asset.clear_selection"));
        assert!(ids.contains(&"asset.show_selected"));
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
        assert!(ids.contains(&"scene.load_demo"));
        assert!(ids.contains(&"scene.select_player"));
        assert!(ids.contains(&"scene.expand_all"));
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
        assert!(ids.contains(&"inspector.show_selected"));
        assert!(ids.contains(&"inspector.inspect_player"));
        assert!(ids.contains(&"inspector.show_property_count"));
        assert!(ids.contains(&"inspector.set_player_health_demo"));
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
        assert!(ids.contains(&"adapter.start_mock"));
        assert!(ids.contains(&"adapter.load_demo_scene"));
        assert!(ids.contains(&"adapter.stop_mock"));
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
