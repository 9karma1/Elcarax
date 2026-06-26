//! Project file and project-state domain model for Elcarax.

use std::error::Error;
use std::fmt;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};

use elcarax_core::{ElcaraxError, Id, Result, Severity};

pub enum ProjectMarker {}
pub type ProjectId = Id<ProjectMarker>;

const DEFAULT_RECENT_PROJECT_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    pub schema_version: u32,
    pub name: String,
    pub adapter: String,
    pub asset_root: PathBuf,
    pub scene_root: PathBuf,
}

impl ProjectFile {
    pub fn new(name: impl Into<String>, adapter: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            name: name.into(),
            adapter: adapter.into(),
            asset_root: PathBuf::from("assets"),
            scene_root: PathBuf::from("scenes"),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(ElcaraxError::Project(
                "project name cannot be empty".to_owned(),
            ));
        }
        if self.adapter.trim().is_empty() {
            return Err(ElcaraxError::Project(
                "project adapter cannot be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    id: ProjectId,
    name: ProjectName,
    path: ProjectPath,
    adapter: String,
}

impl Project {
    pub fn new(
        id: ProjectId,
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        adapter: impl Into<String>,
    ) -> std::result::Result<Self, ProjectError> {
        Ok(Self {
            id,
            name: ProjectName::new(name)?,
            path: ProjectPath::new(path)?,
            adapter: adapter.into(),
        })
    }

    pub fn from_loaded_data(
        id: ProjectId,
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        adapter: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: ProjectName::from_unvalidated(name),
            path: ProjectPath::from_unvalidated(path),
            adapter: adapter.into(),
        }
    }

    pub fn demo() -> Self {
        Self::from_loaded_data(
            stable_project_id(1),
            "Demo Project",
            PathBuf::from("samples/demo_project.elcarax"),
            "elcarax-game",
        )
    }

    pub fn sample() -> Self {
        Self::from_loaded_data(
            stable_project_id(2),
            "Elcarax Sample",
            PathBuf::from("samples/elcarax_sample.elcarax"),
            "elcarax-game",
        )
    }

    pub const fn id(&self) -> ProjectId {
        self.id
    }

    pub fn name(&self) -> &ProjectName {
        &self.name
    }

    pub fn path(&self) -> &ProjectPath {
        &self.path
    }

    pub fn adapter(&self) -> &str {
        self.adapter.as_str()
    }

    pub fn validate(&self) -> ProjectValidation {
        let mut diagnostics = Vec::new();
        if self.name.as_str().trim().is_empty() {
            diagnostics.push(ProjectDiagnostic::error(
                "name",
                "Project name cannot be empty",
            ));
        }
        if self.path.as_path().as_os_str().is_empty() {
            diagnostics.push(ProjectDiagnostic::error(
                "path",
                "Project path cannot be empty",
            ));
        }
        if self.adapter.trim().is_empty() {
            diagnostics.push(ProjectDiagnostic::warning(
                "adapter",
                "Project adapter is not selected",
            ));
        }
        ProjectValidation::from_project_diagnostics(diagnostics)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectName(String);

impl ProjectName {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, ProjectError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ProjectError::EmptyProjectName);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn from_unvalidated(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectPath(PathBuf);

impl ProjectPath {
    pub fn new(path: impl Into<PathBuf>) -> std::result::Result<Self, ProjectError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(ProjectError::EmptyProjectPath);
        }
        Ok(Self(path))
    }

    pub fn from_unvalidated(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub fn display(&self) -> String {
        self.0.display().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    NoProject,
    Loading,
    Loaded,
    Invalid,
}

impl ProjectStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::NoProject => "None",
            Self::Loading => "Loading",
            Self::Loaded => "Loaded",
            Self::Invalid => "Invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiagnostic {
    severity: Severity,
    field: String,
    message: String,
}

impl ProjectDiagnostic {
    pub fn new(severity: Severity, field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, field, message)
    }

    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, field, message)
    }

    pub const fn severity(&self) -> Severity {
        self.severity
    }

    pub fn field(&self) -> &str {
        self.field.as_str()
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    pub fn summary(&self) -> String {
        format!("{}: {}", self.field, self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectValidation {
    status: ProjectStatus,
    diagnostics: Vec<ProjectDiagnostic>,
}

impl ProjectValidation {
    pub fn no_project() -> Self {
        Self {
            status: ProjectStatus::NoProject,
            diagnostics: Vec::new(),
        }
    }

    pub fn clean_loaded() -> Self {
        Self {
            status: ProjectStatus::Loaded,
            diagnostics: Vec::new(),
        }
    }

    pub fn from_project_diagnostics(diagnostics: Vec<ProjectDiagnostic>) -> Self {
        let status = if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error)
        {
            ProjectStatus::Invalid
        } else {
            ProjectStatus::Loaded
        };
        Self {
            status,
            diagnostics,
        }
    }

    pub const fn status(&self) -> ProjectStatus {
        self.status
    }

    pub fn diagnostics(&self) -> &[ProjectDiagnostic] {
        self.diagnostics.as_slice()
    }

    pub fn diagnostic_count(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Warning)
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
            .count()
    }

    pub fn is_valid(&self) -> bool {
        self.status != ProjectStatus::Invalid
    }

    pub fn max_severity(&self) -> Option<Severity> {
        if self.error_count() > 0 {
            Some(Severity::Error)
        } else if self.warning_count() > 0 {
            Some(Severity::Warning)
        } else if self.diagnostic_count() > 0 {
            Some(Severity::Info)
        } else {
            None
        }
    }

    pub fn summary_label(&self) -> String {
        match (
            self.error_count(),
            self.warning_count(),
            self.diagnostic_count(),
        ) {
            (0, 0, 0) => "No diagnostics".to_string(),
            (errors, warnings, _) => format!("{errors} error(s), {warnings} warning(s)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentProject {
    name: ProjectName,
    path: ProjectPath,
}

impl RecentProject {
    pub fn new(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> std::result::Result<Self, ProjectError> {
        Ok(Self {
            name: ProjectName::new(name)?,
            path: ProjectPath::new(path)?,
        })
    }

    pub fn from_project(project: &Project) -> Self {
        Self {
            name: project.name().clone(),
            path: project.path().clone(),
        }
    }

    pub fn name(&self) -> &ProjectName {
        &self.name
    }

    pub fn path(&self) -> &ProjectPath {
        &self.path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentProjects {
    entries: Vec<RecentProject>,
    max_len: usize,
}

impl RecentProjects {
    pub fn new(max_len: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_len,
        }
    }

    pub fn record(&mut self, project: &Project) {
        let recent = RecentProject::from_project(project);
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.path() == recent.path())
        {
            self.entries.remove(index);
        }
        self.entries.insert(0, recent);
        self.entries.truncate(self.max_len);
    }

    pub fn entries(&self) -> &[RecentProject] {
        self.entries.as_slice()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for RecentProjects {
    fn default() -> Self {
        Self::new(DEFAULT_RECENT_PROJECT_LIMIT)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    EmptyProjectName,
    EmptyProjectPath,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProjectName => write!(formatter, "project name cannot be empty"),
            Self::EmptyProjectPath => write!(formatter, "project path cannot be empty"),
        }
    }
}

impl Error for ProjectError {}

fn stable_project_id(value: u64) -> ProjectId {
    match NonZeroU64::new(value) {
        Some(value) => ProjectId::from_non_zero(value),
        None => ProjectId::from_non_zero(NonZeroU64::MIN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(recent: &RecentProjects) -> Vec<&str> {
        recent
            .entries()
            .iter()
            .map(|project| project.name().as_str())
            .collect()
    }

    #[test]
    fn valid_demo_project_validates_cleanly() {
        let validation = Project::demo().validate();
        assert_eq!(validation.status(), ProjectStatus::Loaded);
        assert_eq!(validation.diagnostic_count(), 0);
        assert!(validation.is_valid());
    }

    #[test]
    fn invalid_project_path_and_name_produce_diagnostics() {
        let project = Project::from_loaded_data(stable_project_id(3), "", PathBuf::new(), "");
        let validation = project.validate();
        assert_eq!(validation.status(), ProjectStatus::Invalid);
        assert_eq!(validation.error_count(), 2);
        assert_eq!(validation.warning_count(), 1);
        assert_eq!(validation.diagnostics()[0].field(), "name");
        assert_eq!(validation.diagnostics()[1].field(), "path");
        assert_eq!(validation.diagnostics()[2].field(), "adapter");
    }

    #[test]
    fn recent_project_list_preserves_recency_order() {
        let mut recent = RecentProjects::default();
        let demo = Project::demo();
        let sample = Project::sample();
        recent.record(&demo);
        recent.record(&sample);
        recent.record(&demo);
        assert_eq!(names(&recent), vec!["Demo Project", "Elcarax Sample"]);
    }

    #[test]
    fn recent_project_list_respects_limit() {
        let mut recent = RecentProjects::new(1);
        recent.record(&Project::demo());
        recent.record(&Project::sample());
        assert_eq!(recent.len(), 1);
        assert_eq!(names(&recent), vec!["Elcarax Sample"]);
    }

    #[test]
    fn project_status_labels_are_stable() {
        assert_eq!(ProjectStatus::NoProject.label(), "None");
        assert_eq!(ProjectStatus::Loaded.label(), "Loaded");
        assert_eq!(ProjectStatus::Invalid.label(), "Invalid");
    }
}
