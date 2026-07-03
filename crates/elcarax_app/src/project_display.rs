use elcarax_core::Severity;
use elcarax_project::{Project, ProjectStatus, ProjectValidation, RecentProjects};

use crate::project_state::ProjectCommandResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticTone {
    Neutral,
    Success,
    Warning,
    Danger,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectUiSnapshot {
    pub(crate) toolbar_title: String,
    pub(crate) project_name: String,
    pub(crate) project_path: String,
    pub(crate) project_status: String,
    pub(crate) project_recent: String,
    pub(crate) project_diagnostics: String,
    pub(crate) project_command: String,
    pub(crate) status: String,
    pub(crate) diagnostic_tone: DiagnosticTone,
}

pub(crate) fn project_ui_snapshot(
    current_project: Option<&Project>,
    recent_projects: &RecentProjects,
    validation: &ProjectValidation,
    last_command_result: Option<&ProjectCommandResult>,
    scanned_asset_count: Option<usize>,
) -> ProjectUiSnapshot {
    let command = command_label(last_command_result);
    match current_project {
        Some(project) => loaded_snapshot(
            project,
            recent_projects,
            validation,
            last_command_result,
            command,
            scanned_asset_count,
        ),
        None => no_project_snapshot(recent_projects, last_command_result, command),
    }
}

fn loaded_snapshot(
    project: &Project,
    recent_projects: &RecentProjects,
    validation: &ProjectValidation,
    _last_command_result: Option<&ProjectCommandResult>,
    command: String,
    scanned_asset_count: Option<usize>,
) -> ProjectUiSnapshot {
    let asset_label = format!("Asset root: {}", project.asset_root().display());
    let scene_label = format!("Scene root: {}", project.scene_root().display());
    let status = match scanned_asset_count {
        Some(count) => format!(
            "Project: Loaded | Diagnostics: {} | Assets: {}",
            validation.diagnostic_count(),
            count
        ),
        None => format!(
            "Project: Loaded | Diagnostics: {}",
            validation.diagnostic_count()
        ),
    };
    ProjectUiSnapshot {
        toolbar_title: format!("Elcarax — {}", project.name().as_str()),
        project_name: format!("Name: {}", project.name().as_str()),
        project_path: format!("Root: {}", project.root().display()),
        project_status: format!("{asset_label} | {scene_label}"),
        project_recent: format!("Recent: {}", recent_projects.len()),
        project_diagnostics: format!(
            "Validation: {} | {}",
            validation.status().label(),
            validation.summary_label()
        ),
        project_command: command,
        status,
        diagnostic_tone: diagnostic_tone(validation),
    }
}

fn no_project_snapshot(
    recent_projects: &RecentProjects,
    last_command_result: Option<&ProjectCommandResult>,
    command: String,
) -> ProjectUiSnapshot {
    let status = if last_command_result.is_some() {
        format!(
            "Project: No project open | Command: {}",
            command_id_label(last_command_result)
        )
    } else {
        "Project: No project open".to_string()
    };
    ProjectUiSnapshot {
        toolbar_title: "Elcarax — No Project".to_string(),
        project_name: "No project open".to_string(),
        project_path: "Open Project | Create Project".to_string(),
        project_status: "Assets unavailable until a project is open".to_string(),
        project_recent: format!("Recent: {}", recent_projects.len()),
        project_diagnostics: "Validation: No project open".to_string(),
        project_command: command,
        status,
        diagnostic_tone: DiagnosticTone::Neutral,
    }
}

fn command_label(last_command_result: Option<&ProjectCommandResult>) -> String {
    match last_command_result {
        Some(result) => format!("Command: {} - {}", result.command_id(), result.message()),
        None => "Command: None".to_string(),
    }
}

fn command_id_label(last_command_result: Option<&ProjectCommandResult>) -> &str {
    last_command_result.map_or("None", ProjectCommandResult::command_id)
}

fn diagnostic_tone(validation: &ProjectValidation) -> DiagnosticTone {
    match validation.max_severity() {
        Some(Severity::Error) => DiagnosticTone::Danger,
        Some(Severity::Warning) => DiagnosticTone::Warning,
        Some(Severity::Info) => DiagnosticTone::Neutral,
        None if validation.status() == ProjectStatus::Loaded => DiagnosticTone::Success,
        None => DiagnosticTone::Neutral,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_project::{ProjectId, ResolvedProjectPaths};
    use std::num::NonZeroU64;
    use std::path::PathBuf;

    #[test]
    fn no_project_state_paints_correctly() {
        let snapshot = project_ui_snapshot(
            None,
            &RecentProjects::default(),
            &ProjectValidation::no_project(),
            None,
            None,
        );
        assert_eq!(snapshot.toolbar_title, "Elcarax — No Project");
        assert_eq!(snapshot.project_name, "No project open");
        assert!(snapshot.project_status.contains("unavailable"));
    }

    #[test]
    fn loaded_project_state_paints_project_metadata() {
        let project = elcarax_project::Project::from_loaded_data(
            ProjectId::from_non_zero(NonZeroU64::MIN),
            "Loaded Project",
            PathBuf::from("/tmp/project"),
            ResolvedProjectPaths {
                asset_root: PathBuf::from("/tmp/project/assets"),
                scene_root: PathBuf::from("/tmp/project/scenes"),
                settings_dir: PathBuf::from("/tmp/project/.elcarax"),
            },
            elcarax_project::ProjectEditorSettings::default(),
        );
        let snapshot = project_ui_snapshot(
            Some(&project),
            &RecentProjects::default(),
            &ProjectValidation::clean_loaded(),
            None,
            Some(3),
        );
        assert_eq!(snapshot.toolbar_title, "Elcarax — Loaded Project");
        assert!(snapshot.project_path.contains("Root:"));
        assert!(snapshot.project_status.contains("Asset root:"));
        assert!(snapshot.status.contains("Assets: 3"));
    }

    #[test]
    fn validation_status_paints_in_diagnostics() {
        let project = elcarax_project::Project::from_loaded_data(
            ProjectId::from_non_zero(NonZeroU64::MIN),
            "Loaded Project",
            PathBuf::from("/tmp/project"),
            ResolvedProjectPaths {
                asset_root: PathBuf::from("/tmp/project/assets"),
                scene_root: PathBuf::from("/tmp/project/scenes"),
                settings_dir: PathBuf::from("/tmp/project/.elcarax"),
            },
            elcarax_project::ProjectEditorSettings::default(),
        );
        let snapshot = project_ui_snapshot(
            Some(&project),
            &RecentProjects::default(),
            &ProjectValidation::clean_loaded(),
            None,
            None,
        );
        assert!(snapshot.project_diagnostics.contains("Validation: Loaded"));
    }
}
