use std::fs;
use std::path::PathBuf;

use crate::create::ensure_manifest_filename;
use crate::domain::{Project, project_id_from_root};
use crate::error::ProjectError;
use crate::manifest::{ProjectFile, manifest_path_for_root};
use crate::validate::{ProjectValidationResult, validate_opened_project};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectOpenRequest {
    pub root: PathBuf,
}

impl ProjectOpenRequest {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLoadResult {
    pub project: Project,
    pub validation: ProjectValidationResult,
}

pub fn open_project(request: &ProjectOpenRequest) -> Result<ProjectLoadResult, ProjectError> {
    let root = ensure_manifest_filename(&request.root);
    if root.as_os_str().is_empty() {
        return Err(ProjectError::EmptyProjectPath);
    }
    let manifest_path = manifest_path_for_root(&root);
    if !manifest_path.is_file() {
        return Err(ProjectError::ManifestMissing);
    }
    let content = fs::read_to_string(&manifest_path)
        .map_err(|error| ProjectError::Io(format!("failed to read manifest: {error}")))?;
    let file = ProjectFile::from_toml_str(&content)?;
    let resolved = file.manifest.paths.resolve(&root);
    let project = Project::new(
        project_id_from_root(&root),
        file.manifest.name.as_str(),
        &root,
        resolved,
    )?;
    let validation = validate_opened_project(&project, &manifest_path);
    Ok(ProjectLoadResult {
        project,
        validation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::{ProjectCreateRequest, create_project};
    use std::fs;

    #[test]
    fn opening_valid_project_succeeds() {
        let temp = std::env::temp_dir().join(format!("elcarax-open-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = create_project(&ProjectCreateRequest::new(&temp, "Open Test"));
        let loaded = open_project(&ProjectOpenRequest::new(&temp));
        let loaded = match loaded {
            Ok(value) => value,
            Err(error) => panic!("open should succeed: {error}"),
        };
        assert_eq!(loaded.project.name().as_str(), "Open Test");
        assert_eq!(loaded.project.asset_root(), temp.join("assets").as_path());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn opening_missing_manifest_fails_clearly() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-open-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(&temp);
        let result = open_project(&ProjectOpenRequest::new(&temp));
        assert!(matches!(result, Err(ProjectError::ManifestMissing)));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn invalid_toml_fails_clearly() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-open-invalid-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(&temp);
        let manifest = manifest_path_for_root(&temp);
        if let Err(error) = fs::write(&manifest, "not valid toml [[[") {
            panic!("write invalid manifest: {error}");
        }
        let result = open_project(&ProjectOpenRequest::new(&temp));
        assert!(matches!(result, Err(ProjectError::ManifestInvalid(_))));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn unsupported_schema_version_fails_clearly() {
        let temp = std::env::temp_dir().join(format!("elcarax-open-schema-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(&temp);
        let manifest = manifest_path_for_root(&temp);
        if let Err(error) = fs::write(
            &manifest,
            r#"
schema_version = 99
name = "Bad Version"

[paths]
asset_root = "assets"
scene_root = "scenes"
settings_dir = ".elcarax"
"#,
        ) {
            panic!("write manifest: {error}");
        }
        let result = open_project(&ProjectOpenRequest::new(&temp));
        assert!(matches!(
            result,
            Err(ProjectError::UnsupportedSchemaVersion(99))
        ));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn relative_paths_resolve_correctly() {
        let temp = std::env::temp_dir().join(format!("elcarax-open-paths-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = create_project(&ProjectCreateRequest::new(&temp, "Paths"));
        let loaded = open_project(&ProjectOpenRequest::new(&temp));
        let loaded = match loaded {
            Ok(value) => value,
            Err(error) => panic!("open should succeed: {error}"),
        };
        assert_eq!(loaded.project.scene_root(), temp.join("scenes").as_path());
        assert_eq!(
            loaded.project.settings_dir(),
            temp.join(".elcarax").as_path()
        );
        let _ = fs::remove_dir_all(&temp);
    }
}
