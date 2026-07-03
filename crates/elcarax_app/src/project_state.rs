#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_project::{
    Project, ProjectCreateRequest, ProjectError, ProjectOpenRequest, ProjectValidation,
    RecentProjects, RecentProjectsStore, create_project, open_project,
};

use crate::project_config::AppProjectConfig;
use crate::project_display::{ProjectUiSnapshot, project_ui_snapshot};

pub(crate) const PROJECT_CREATE_COMMAND: &str = "project.create";
pub(crate) const PROJECT_OPEN_COMMAND: &str = "project.open";
pub(crate) const PROJECT_CLOSE_COMMAND: &str = "project.close";
pub(crate) const PROJECT_VALIDATE_COMMAND: &str = "project.validate";
pub(crate) const PROJECT_SHOW_RECENT_COMMAND: &str = "project.show_recent";
pub(crate) const PROJECT_REOPEN_LAST_COMMAND: &str = "project.reopen_last";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectState {
    current_project: Option<Project>,
    recent_store: RecentProjectsStore,
    recent_view: RecentProjects,
    validation: ProjectValidation,
    last_command_result: Option<ProjectCommandResult>,
    config: AppProjectConfig,
    scanned_asset_count: Option<usize>,
}

impl ProjectState {
    pub(crate) fn new(config: AppProjectConfig) -> Self {
        let recent_store = match RecentProjectsStore::load(config.recent_store_path()) {
            Ok(store) => store,
            Err(_) => RecentProjectsStore::new(config.recent_store_path(), 20),
        };
        let recent_view = RecentProjects::from_store(&recent_store);
        Self {
            current_project: None,
            recent_store,
            recent_view,
            validation: ProjectValidation::no_project(),
            last_command_result: None,
            config,
            scanned_asset_count: None,
        }
    }

    pub(crate) fn execute_command_id(&mut self, id: &str) -> Option<ProjectCommandResult> {
        let command = ProjectCommand::from_id(id)?;
        let result = match command {
            ProjectCommand::Create => self.create_project(),
            ProjectCommand::Open => self.open_project(),
            ProjectCommand::Close => self.close_project(),
            ProjectCommand::Validate => self.validate_current_project(),
            ProjectCommand::ShowRecent => self.show_recent_projects(),
            ProjectCommand::ReopenLast => self.reopen_last_project(),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    pub(crate) fn ui_snapshot(&self) -> ProjectUiSnapshot {
        project_ui_snapshot(
            self.current_project.as_ref(),
            &self.recent_view,
            &self.validation,
            self.last_command_result.as_ref(),
            self.scanned_asset_count,
        )
    }

    pub(crate) fn is_project_loaded(&self) -> bool {
        self.current_project.is_some()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn current_project(&self) -> Option<&Project> {
        self.current_project.as_ref()
    }

    pub(crate) fn asset_root(&self) -> Option<&std::path::Path> {
        self.current_project
            .as_ref()
            .map(|project| project.asset_root())
    }

    pub(crate) fn project_root(&self) -> Option<&std::path::Path> {
        self.current_project
            .as_ref()
            .map(|project| project.root().as_path())
    }

    pub(crate) fn set_scanned_asset_count(&mut self, count: Option<usize>) {
        self.scanned_asset_count = count;
    }

    fn create_project(&mut self) -> ProjectCommandResult {
        let Some(root) = self.config.create_root_or_open_parent() else {
            return ProjectCommandResult::new(
                PROJECT_CREATE_COMMAND,
                "No project path configured. Set ELCARAX_PROJECT_CREATE_PATH or pass --create-project <path>",
            );
        };
        let request = ProjectCreateRequest::new(&root, self.config.create_name());
        match create_project(&request) {
            Ok(loaded) => {
                self.apply_loaded_project(loaded, PROJECT_CREATE_COMMAND, "Created project")
            }
            Err(error) => {
                ProjectCommandResult::new(PROJECT_CREATE_COMMAND, format_project_error(error))
            }
        }
    }

    fn open_project(&mut self) -> ProjectCommandResult {
        let Some(path) = self.config.open_path.clone() else {
            return ProjectCommandResult::new(
                PROJECT_OPEN_COMMAND,
                "No project path configured. Set ELCARAX_PROJECT_PATH or pass --project <path>",
            );
        };
        self.open_project_at(path, PROJECT_OPEN_COMMAND)
    }

    fn reopen_last_project(&mut self) -> ProjectCommandResult {
        let Some(entry) = self.recent_store.most_recent().cloned() else {
            return ProjectCommandResult::new(
                PROJECT_REOPEN_LAST_COMMAND,
                "No recent project available",
            );
        };
        self.open_project_at(entry.path.clone(), PROJECT_REOPEN_LAST_COMMAND)
    }

    fn open_project_at(
        &mut self,
        path: std::path::PathBuf,
        command_id: &'static str,
    ) -> ProjectCommandResult {
        let request = ProjectOpenRequest::new(path);
        match open_project(&request) {
            Ok(loaded) => {
                let message = if command_id == PROJECT_REOPEN_LAST_COMMAND {
                    "Reopened last project"
                } else {
                    "Opened project"
                };
                self.apply_loaded_project(loaded, command_id, message)
            }
            Err(error) => ProjectCommandResult::new(command_id, format_project_error(error)),
        }
    }

    fn apply_loaded_project(
        &mut self,
        loaded: elcarax_project::ProjectLoadResult,
        command_id: &'static str,
        message: &str,
    ) -> ProjectCommandResult {
        self.validation = loaded.validation.validation.clone();
        self.recent_store.add_project(&loaded.project);
        self.recent_view.record(&loaded.project);
        let _ = self.recent_store.save();
        self.current_project = Some(loaded.project);
        self.scanned_asset_count = None;
        ProjectCommandResult::new(command_id, message)
    }

    fn close_project(&mut self) -> ProjectCommandResult {
        self.current_project = None;
        self.validation = ProjectValidation::no_project();
        self.scanned_asset_count = None;
        ProjectCommandResult::new(PROJECT_CLOSE_COMMAND, "Closed current project")
    }

    fn validate_current_project(&mut self) -> ProjectCommandResult {
        let Some(project) = self.current_project.clone() else {
            self.validation = ProjectValidation::no_project();
            return ProjectCommandResult::new(PROJECT_VALIDATE_COMMAND, "No project to validate");
        };
        let manifest_path = elcarax_project::manifest_path_for_root(project.root().as_path());
        let validation = elcarax_project::validate_opened_project(&project, &manifest_path);
        self.validation = validation.validation.clone();
        if self.validation.diagnostic_count() == 0 {
            ProjectCommandResult::new(PROJECT_VALIDATE_COMMAND, "Project validation passed")
        } else {
            ProjectCommandResult::new(PROJECT_VALIDATE_COMMAND, self.validation.summary_label())
        }
    }

    fn show_recent_projects(&self) -> ProjectCommandResult {
        ProjectCommandResult::new(PROJECT_SHOW_RECENT_COMMAND, self.recent_store.summary())
    }

    #[cfg(test)]
    fn load_fixture_project(&mut self, project: Project) {
        self.validation = project.validate();
        self.recent_view.record(&project);
        self.recent_store.add_project(&project);
        self.current_project = Some(project);
    }

    #[cfg(test)]
    fn with_recent_store_path(mut self, path: std::path::PathBuf) -> Self {
        self.recent_store = RecentProjectsStore::new(path, 20);
        self.recent_view = RecentProjects::from_store(&self.recent_store);
        self
    }
}

impl Default for ProjectState {
    fn default() -> Self {
        Self::new(AppProjectConfig::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectCommand {
    Create,
    Open,
    Close,
    Validate,
    ShowRecent,
    ReopenLast,
}

impl ProjectCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            PROJECT_CREATE_COMMAND => Some(Self::Create),
            PROJECT_OPEN_COMMAND => Some(Self::Open),
            PROJECT_CLOSE_COMMAND => Some(Self::Close),
            PROJECT_VALIDATE_COMMAND => Some(Self::Validate),
            PROJECT_SHOW_RECENT_COMMAND => Some(Self::ShowRecent),
            PROJECT_REOPEN_LAST_COMMAND => Some(Self::ReopenLast),
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

fn format_project_error(error: ProjectError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_display::DiagnosticTone;
    use elcarax_commands::{CommandId, CommandResult, RegisteredCommand, built_in_commands};
    use elcarax_project::{ProjectId, ProjectStatus, ResolvedProjectPaths};
    use elcarax_ui::{CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey};
    use std::fs;
    use std::num::NonZeroU64;
    use std::path::PathBuf;

    #[test]
    fn project_create_creates_real_temp_project() {
        let temp = std::env::temp_dir().join(format!("elcarax-app-create-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let config = AppProjectConfig {
            create_root: Some(temp.clone()),
            create_name: Some("App Create Test".to_string()),
            ..AppProjectConfig::default()
        };
        let mut state = ProjectState::new(config);
        let result = state.execute_command_id(PROJECT_CREATE_COMMAND);
        assert!(result.is_some_and(|value| value.message().contains("Created project")));
        assert!(state.current_project.is_some());
        assert!(elcarax_project::manifest_path_for_root(&temp).is_file());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn project_open_opens_real_temp_project() {
        let temp = std::env::temp_dir().join(format!("elcarax-app-open-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let create_config = AppProjectConfig {
            create_root: Some(temp.clone()),
            create_name: Some("App Open Test".to_string()),
            ..AppProjectConfig::default()
        };
        let mut create_state = ProjectState::new(create_config);
        let _ = create_state.execute_command_id(PROJECT_CREATE_COMMAND);
        let _ = create_state.execute_command_id(PROJECT_CLOSE_COMMAND);
        let open_config = AppProjectConfig {
            open_path: Some(temp.clone()),
            ..AppProjectConfig::default()
        };
        let mut state = ProjectState::new(open_config);
        let result = state.execute_command_id(PROJECT_OPEN_COMMAND);
        assert!(result.is_some_and(|value| value.message().contains("Opened project")));
        assert_eq!(
            state
                .current_project()
                .map(|project| project.name().as_str()),
            Some("App Open Test")
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn project_validate_reports_current_validation() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-app-validate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let config = AppProjectConfig {
            create_root: Some(temp.clone()),
            ..AppProjectConfig::default()
        };
        let mut state = ProjectState::new(config);
        let _ = state.execute_command_id(PROJECT_CREATE_COMMAND);
        let result = state.execute_command_id(PROJECT_VALIDATE_COMMAND);
        assert_eq!(
            result.as_ref().map(ProjectCommandResult::message),
            Some("Project validation passed")
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn project_close_clears_project_state() {
        let temp = std::env::temp_dir().join(format!("elcarax-app-close-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let config = AppProjectConfig {
            create_root: Some(temp.clone()),
            ..AppProjectConfig::default()
        };
        let mut state = ProjectState::new(config);
        let _ = state.execute_command_id(PROJECT_CREATE_COMMAND);
        let _ = state.execute_command_id(PROJECT_CLOSE_COMMAND);
        assert!(state.current_project.is_none());
        assert_eq!(state.validation.status(), ProjectStatus::NoProject);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn project_reopen_last_opens_last_project() {
        let temp = std::env::temp_dir().join(format!("elcarax-app-reopen-{}", std::process::id()));
        let recent_path = temp.join("recent.toml");
        let _ = fs::remove_dir_all(&temp);
        let config = AppProjectConfig {
            create_root: Some(temp.join("project")),
            recent_store_path: Some(recent_path.clone()),
            ..AppProjectConfig::default()
        };
        let mut state = ProjectState::new(config).with_recent_store_path(recent_path.clone());
        let _ = state.execute_command_id(PROJECT_CREATE_COMMAND);
        let _ = state.execute_command_id(PROJECT_CLOSE_COMMAND);
        let mut reopen_state = ProjectState::new(AppProjectConfig {
            recent_store_path: Some(recent_path),
            ..AppProjectConfig::default()
        });
        let result = reopen_state.execute_command_id(PROJECT_REOPEN_LAST_COMMAND);
        assert!(result.is_some_and(|value| value.message().contains("Reopened last project")));
        assert!(reopen_state.current_project.is_some());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn unknown_command_does_not_mutate_project_state() {
        let mut state = ProjectState::default();
        assert_eq!(state.execute_command_id("elcarax.unknown"), None);
        assert!(state.current_project.is_none());
        assert!(state.last_command_result.is_none());
    }

    #[test]
    fn ui_snapshot_formats_no_project_and_loaded_states() {
        let temp = std::env::temp_dir().join(format!("elcarax-ui-snapshot-{}", std::process::id()));
        let recent_path = temp.join("recent.toml");
        let mut state = ProjectState::new(AppProjectConfig {
            recent_store_path: Some(recent_path),
            ..AppProjectConfig::default()
        });
        assert_eq!(state.ui_snapshot().toolbar_title, "Elcarax — No Project");
        state.load_fixture_project(fixture_project());
        let snapshot = state.ui_snapshot();
        assert_eq!(snapshot.toolbar_title, "Elcarax — Fixture Project");
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
        for character in PROJECT_CREATE_COMMAND.chars() {
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
    }

    fn fixture_project() -> elcarax_project::Project {
        elcarax_project::Project::from_loaded_data(
            ProjectId::from_non_zero(NonZeroU64::MIN),
            "Fixture Project",
            PathBuf::from("fixtures/project"),
            ResolvedProjectPaths {
                asset_root: PathBuf::from("fixtures/project/assets"),
                scene_root: PathBuf::from("fixtures/project/scenes"),
                settings_dir: PathBuf::from("fixtures/project/.elcarax"),
            },
        )
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
