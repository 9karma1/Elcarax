use std::path::{Path, PathBuf};

use crate::domain::{Project, ProjectDiagnostic, ProjectValidation};
use crate::manifest::MANIFEST_FILENAME;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectValidationResult {
    pub validation: ProjectValidation,
    pub manifest_path: PathBuf,
    pub asset_root_exists: bool,
    pub scene_root_exists: bool,
}

impl ProjectValidationResult {
    pub fn diagnostic_count(&self) -> usize {
        self.validation.diagnostic_count()
    }

    pub fn summary_label(&self) -> String {
        self.validation.summary_label()
    }

    pub fn is_valid(&self) -> bool {
        self.validation.is_valid()
    }
}

pub fn validate_opened_project(project: &Project, manifest_path: &Path) -> ProjectValidationResult {
    let mut diagnostics = project.validate().diagnostics().to_vec();
    let asset_root_exists = project.asset_root().exists();
    let scene_root_exists = project.scene_root().exists();
    if !asset_root_exists {
        diagnostics.push(ProjectDiagnostic::warning(
            "asset_root",
            format!(
                "Asset root does not exist: {}",
                project.asset_root().display()
            ),
        ));
    } else if !project.asset_root().is_dir() {
        diagnostics.push(ProjectDiagnostic::error(
            "asset_root",
            format!(
                "Asset root is not a directory: {}",
                project.asset_root().display()
            ),
        ));
    }
    if !scene_root_exists {
        diagnostics.push(ProjectDiagnostic::warning(
            "scene_root",
            format!(
                "Scene root does not exist: {}",
                project.scene_root().display()
            ),
        ));
    } else if !project.scene_root().is_dir() {
        diagnostics.push(ProjectDiagnostic::error(
            "scene_root",
            format!(
                "Scene root is not a directory: {}",
                project.scene_root().display()
            ),
        ));
    }
    if !manifest_path.is_file() {
        diagnostics.push(ProjectDiagnostic::error(
            "manifest",
            format!("Project manifest is missing: {}", MANIFEST_FILENAME),
        ));
    }
    ProjectValidationResult {
        validation: ProjectValidation::from_project_diagnostics(diagnostics),
        manifest_path: manifest_path.to_path_buf(),
        asset_root_exists,
        scene_root_exists,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::{ProjectCreateRequest, create_project};
    use crate::domain::project_id_from_root;
    use crate::manifest::{ProjectEditorSettings, ResolvedProjectPaths, manifest_path_for_root};
    use std::fs;

    #[test]
    fn validation_reports_missing_asset_root() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-validate-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(&temp);
        let manifest = manifest_path_for_root(&temp);
        if let Err(error) = fs::write(
            &manifest,
            r#"
schema_version = 1
name = "Missing Asset Root"

[paths]
asset_root = "missing_assets"
scene_root = "scenes"
settings_dir = ".elcarax"
"#,
        ) {
            panic!("write manifest: {error}");
        }
        let paths = ResolvedProjectPaths {
            asset_root: temp.join("missing_assets"),
            scene_root: temp.join("scenes"),
            settings_dir: temp.join(".elcarax"),
        };
        let project = Project::from_loaded_data(
            project_id_from_root(&temp),
            "Missing Asset Root",
            &temp,
            paths,
            ProjectEditorSettings::default(),
        );
        let validation = validate_opened_project(&project, &manifest);
        assert!(!validation.asset_root_exists);
        assert!(
            validation
                .validation
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.field() == "asset_root")
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn created_project_validates_cleanly() {
        let temp = std::env::temp_dir().join(format!("elcarax-validate-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let created = create_project(&ProjectCreateRequest::new(&temp, "Valid"));
        let created = match created {
            Ok(value) => value,
            Err(error) => panic!("create should succeed: {error}"),
        };
        assert!(created.validation.is_valid());
        let _ = fs::remove_dir_all(&temp);
    }
}
