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
) -> ProjectUiSnapshot {
    let command = command_label(last_command_result);
    match current_project {
        Some(project) => loaded_snapshot(
            project,
            recent_projects,
            validation,
            last_command_result,
            command,
        ),
        None => no_project_snapshot(recent_projects, last_command_result, command),
    }
}

fn loaded_snapshot(
    project: &Project,
    recent_projects: &RecentProjects,
    validation: &ProjectValidation,
    last_command_result: Option<&ProjectCommandResult>,
    command: String,
) -> ProjectUiSnapshot {
    ProjectUiSnapshot {
        toolbar_title: format!("Elcarax - {}", project.name().as_str()),
        project_name: format!("Name: {}", project.name().as_str()),
        project_path: format!("Path: {}", project.path().display()),
        project_status: format!("Status: {}", validation.status().label()),
        project_recent: format!("Recent: {}", recent_projects.len()),
        project_diagnostics: format!("Diagnostics: {}", validation.summary_label()),
        project_command: command,
        status: format!(
            "Project: {} | Diagnostics: {} | Command: {}",
            validation.status().label(),
            validation.diagnostic_count(),
            command_id_label(last_command_result)
        ),
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
            "Project: None | Command: {}",
            command_id_label(last_command_result)
        )
    } else {
        "Project: None".to_string()
    };
    ProjectUiSnapshot {
        toolbar_title: "Elcarax - No Project".to_string(),
        project_name: "Name: No project".to_string(),
        project_path: "Path: None".to_string(),
        project_status: format!("Status: {}", ProjectStatus::NoProject.label()),
        project_recent: format!("Recent: {}", recent_projects.len()),
        project_diagnostics: "Diagnostics: No diagnostics".to_string(),
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
