use elcarax_project::{Project, ProjectValidation, RecentProjects};

use crate::project_display::{ProjectUiSnapshot, project_ui_snapshot};

pub(crate) const PROJECT_NEW_DEMO_COMMAND: &str = "project.new_demo";
pub(crate) const PROJECT_OPEN_DEMO_COMMAND: &str = "project.open_demo";
pub(crate) const PROJECT_CLOSE_COMMAND: &str = "project.close";
pub(crate) const PROJECT_VALIDATE_COMMAND: &str = "project.validate";
pub(crate) const PROJECT_SHOW_RECENT_COMMAND: &str = "project.show_recent";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectState {
    current_project: Option<Project>,
    recent_projects: RecentProjects,
    validation: ProjectValidation,
    last_command_result: Option<ProjectCommandResult>,
}

impl ProjectState {
    pub(crate) fn execute_command_id(&mut self, id: &str) -> Option<ProjectCommandResult> {
        let command = ProjectCommand::from_id(id)?;
        let result = match command {
            ProjectCommand::NewDemo => self.create_new_demo_project(),
            ProjectCommand::OpenDemo => self.open_demo_project(),
            ProjectCommand::Close => self.close_project(),
            ProjectCommand::Validate => self.validate_current_project(),
            ProjectCommand::ShowRecent => self.show_recent_projects(),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    pub(crate) fn ui_snapshot(&self) -> ProjectUiSnapshot {
        project_ui_snapshot(
            self.current_project.as_ref(),
            &self.recent_projects,
            &self.validation,
            self.last_command_result.as_ref(),
        )
    }

    pub(crate) fn is_project_loaded(&self) -> bool {
        self.current_project.is_some()
    }

    fn create_new_demo_project(&mut self) -> ProjectCommandResult {
        let project = Project::demo();
        self.validation = project.validate();
        self.recent_projects.record(&project);
        self.current_project = Some(project);
        ProjectCommandResult::new(PROJECT_NEW_DEMO_COMMAND, "Created demo project")
    }

    fn open_demo_project(&mut self) -> ProjectCommandResult {
        let project = Project::sample();
        self.validation = project.validate();
        self.recent_projects.record(&project);
        self.current_project = Some(project);
        ProjectCommandResult::new(PROJECT_OPEN_DEMO_COMMAND, "Opened sample project")
    }

    fn close_project(&mut self) -> ProjectCommandResult {
        self.current_project = None;
        self.validation = ProjectValidation::no_project();
        ProjectCommandResult::new(PROJECT_CLOSE_COMMAND, "Closed current project")
    }

    fn validate_current_project(&mut self) -> ProjectCommandResult {
        let Some(project) = &self.current_project else {
            self.validation = ProjectValidation::no_project();
            return ProjectCommandResult::new(PROJECT_VALIDATE_COMMAND, "No project to validate");
        };
        self.validation = project.validate();
        if self.validation.diagnostic_count() == 0 {
            ProjectCommandResult::new(PROJECT_VALIDATE_COMMAND, "Project validation passed")
        } else {
            ProjectCommandResult::new(PROJECT_VALIDATE_COMMAND, self.validation.summary_label())
        }
    }

    fn show_recent_projects(&self) -> ProjectCommandResult {
        let count = self.recent_projects.len();
        if count == 0 {
            return ProjectCommandResult::new(PROJECT_SHOW_RECENT_COMMAND, "No recent projects");
        }
        let names = self
            .recent_projects
            .entries()
            .iter()
            .map(|project| project.name().as_str())
            .collect::<Vec<_>>()
            .join(", ");
        ProjectCommandResult::new(
            PROJECT_SHOW_RECENT_COMMAND,
            format!("Recent projects ({count}): {names}"),
        )
    }
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            current_project: None,
            recent_projects: RecentProjects::default(),
            validation: ProjectValidation::no_project(),
            last_command_result: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectCommand {
    NewDemo,
    OpenDemo,
    Close,
    Validate,
    ShowRecent,
}

impl ProjectCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            PROJECT_NEW_DEMO_COMMAND => Some(Self::NewDemo),
            PROJECT_OPEN_DEMO_COMMAND => Some(Self::OpenDemo),
            PROJECT_CLOSE_COMMAND => Some(Self::Close),
            PROJECT_VALIDATE_COMMAND => Some(Self::Validate),
            PROJECT_SHOW_RECENT_COMMAND => Some(Self::ShowRecent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectCommandResult {
    command_id: String,
    message: String,
}

impl ProjectCommandResult {
    fn new(command_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            command_id: command_id.into(),
            message: message.into(),
        }
    }

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
    use crate::project_display::DiagnosticTone;
    use elcarax_commands::{CommandId, CommandResult, RegisteredCommand, built_in_commands};
    use elcarax_project::ProjectStatus;
    use elcarax_ui::{CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey};

    #[test]
    fn project_new_demo_changes_project_state() {
        let mut state = ProjectState::default();
        let result = state.execute_command_id(PROJECT_NEW_DEMO_COMMAND);
        assert_eq!(
            result.as_ref().map(ProjectCommandResult::command_id),
            Some(PROJECT_NEW_DEMO_COMMAND)
        );
        assert!(state.current_project.is_some());
        assert_eq!(state.recent_projects.len(), 1);
        assert_eq!(state.validation.diagnostic_count(), 0);
    }

    #[test]
    fn project_close_clears_project_state() {
        let mut state = ProjectState::default();
        let _ = state.execute_command_id(PROJECT_NEW_DEMO_COMMAND);
        let _ = state.execute_command_id(PROJECT_CLOSE_COMMAND);
        assert!(state.current_project.is_none());
        assert_eq!(state.validation.status(), ProjectStatus::NoProject);
        assert_eq!(
            state
                .last_command_result
                .as_ref()
                .map(ProjectCommandResult::command_id),
            Some(PROJECT_CLOSE_COMMAND)
        );
    }

    #[test]
    fn project_validate_records_diagnostics() {
        let mut state = ProjectState::default();
        let _ = state.execute_command_id(PROJECT_NEW_DEMO_COMMAND);
        let result = state.execute_command_id(PROJECT_VALIDATE_COMMAND);
        assert_eq!(
            result.as_ref().map(ProjectCommandResult::message),
            Some("Project validation passed")
        );
        assert_eq!(state.validation.diagnostic_count(), 0);
    }

    #[test]
    fn unknown_command_does_not_mutate_project_state() {
        let mut state = ProjectState::default();
        assert_eq!(state.execute_command_id("elcarax.demo.run"), None);
        assert!(state.current_project.is_none());
        assert!(state.last_command_result.is_none());
    }

    #[test]
    fn ui_snapshot_formats_no_project_and_loaded_states() {
        let mut state = ProjectState::default();
        assert_eq!(state.ui_snapshot().toolbar_title, "Elcarax - No Project");
        let _ = state.execute_command_id(PROJECT_NEW_DEMO_COMMAND);
        let snapshot = state.ui_snapshot();
        assert_eq!(snapshot.toolbar_title, "Elcarax - Demo Project");
        assert_eq!(snapshot.project_recent, "Recent: 1");
        assert_eq!(snapshot.diagnostic_tone, DiagnosticTone::Success);
    }

    #[test]
    fn command_palette_can_execute_project_command() {
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
        for character in PROJECT_NEW_DEMO_COMMAND.chars() {
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
                Err(error) => panic!("selected project command ID should be valid: {error}"),
            },
            None => panic!("project command should be selected"),
        };
        assert!(matches!(
            registry.invoke(&selected_id),
            CommandResult::Invoked(_)
        ));
        let mut state = ProjectState::default();
        assert!(state.execute_command_id(selected_id.as_str()).is_some());
        assert!(state.current_project.is_some());
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
