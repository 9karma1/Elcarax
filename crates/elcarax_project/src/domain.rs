use std::path::{Path, PathBuf};

use elcarax_core::{Id, Severity};
use std::num::NonZeroU64;

use crate::manifest::ProjectEditorSettings;
use crate::manifest::ResolvedProjectPaths;

pub enum ProjectMarker {}
pub type ProjectId = Id<ProjectMarker>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    id: ProjectId,
    name: ProjectName,
    root: ProjectPath,
    paths: ResolvedProjectPaths,
    editor: ProjectEditorSettings,
}

impl Project {
    pub fn new(
        id: ProjectId,
        name: impl Into<String>,
        root: impl Into<PathBuf>,
        paths: ResolvedProjectPaths,
        editor: ProjectEditorSettings,
    ) -> std::result::Result<Self, super::error::ProjectError> {
        Ok(Self {
            id,
            name: ProjectName::new(name)?,
            root: ProjectPath::new(root)?,
            paths,
            editor,
        })
    }

    pub fn from_loaded_data(
        id: ProjectId,
        name: impl Into<String>,
        root: impl Into<PathBuf>,
        paths: ResolvedProjectPaths,
        editor: ProjectEditorSettings,
    ) -> Self {
        Self {
            id,
            name: ProjectName::from_unvalidated(name),
            root: ProjectPath::from_unvalidated(root),
            paths,
            editor,
        }
    }

    pub const fn id(&self) -> ProjectId {
        self.id
    }

    pub fn name(&self) -> &ProjectName {
        &self.name
    }

    pub fn root(&self) -> &ProjectPath {
        &self.root
    }

    /// Project root directory (alias for `root()`).
    pub fn path(&self) -> &ProjectPath {
        &self.root
    }

    pub fn asset_root(&self) -> &Path {
        self.paths.asset_root.as_path()
    }

    pub fn scene_root(&self) -> &Path {
        self.paths.scene_root.as_path()
    }

    pub fn settings_dir(&self) -> &Path {
        self.paths.settings_dir.as_path()
    }

    pub fn editor_settings(&self) -> &ProjectEditorSettings {
        &self.editor
    }

    pub fn set_active_scene(&mut self, relative_path: Option<PathBuf>) {
        self.editor.active_scene = relative_path;
    }

    pub fn resolved_paths(&self) -> &ResolvedProjectPaths {
        &self.paths
    }

    pub fn validate(&self) -> ProjectValidation {
        let mut diagnostics = Vec::new();
        if self.name.as_str().trim().is_empty() {
            diagnostics.push(ProjectDiagnostic::error(
                "name",
                "Project name cannot be empty",
            ));
        }
        if self.root.as_path().as_os_str().is_empty() {
            diagnostics.push(ProjectDiagnostic::error(
                "root",
                "Project root cannot be empty",
            ));
        }
        ProjectValidation::from_project_diagnostics(diagnostics)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectName(String);

impl ProjectName {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, super::error::ProjectError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(super::error::ProjectError::EmptyProjectName);
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
    pub fn new(path: impl Into<PathBuf>) -> std::result::Result<Self, super::error::ProjectError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(super::error::ProjectError::EmptyProjectPath);
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

pub fn project_id_from_root(root: &Path) -> ProjectId {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in root.display().to_string().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let value = hash.max(1);
    match NonZeroU64::new(value) {
        Some(value) => ProjectId::from_non_zero(value),
        None => ProjectId::from_non_zero(NonZeroU64::MIN),
    }
}
