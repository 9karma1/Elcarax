use std::fs;
use std::path::{Path, PathBuf};

use elcarax_scene_model::{
    DEFAULT_SCENE_FILENAME, create_default_scene_file, scene_file_path_in_root,
};

use crate::domain::{Project, project_id_from_root};
use crate::error::ProjectError;
use crate::manifest::{MANIFEST_FILENAME, ProjectFile, manifest_path_for_root};
use crate::open::ProjectLoadResult;
use crate::validate::validate_opened_project;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCreateRequest {
    pub root: PathBuf,
    pub name: String,
    pub overwrite: bool,
}

impl ProjectCreateRequest {
    pub fn new(root: impl Into<PathBuf>, name: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            name: name.into(),
            overwrite: false,
        }
    }

    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }
}

pub fn create_project(request: &ProjectCreateRequest) -> Result<ProjectLoadResult, ProjectError> {
    if request.name.trim().is_empty() {
        return Err(ProjectError::EmptyProjectName);
    }
    if request.root.as_os_str().is_empty() {
        return Err(ProjectError::EmptyProjectPath);
    }
    let manifest_path = manifest_path_for_root(&request.root);
    if manifest_path.exists() && !request.overwrite {
        return Err(ProjectError::AlreadyExists);
    }
    fs::create_dir_all(&request.root)
        .map_err(|error| ProjectError::Io(format!("failed to create project root: {error}")))?;
    let file = ProjectFile::new(request.name.trim());
    let manifest = file.manifest.clone();
    let resolved = manifest.paths.resolve(&request.root);
    for directory in [
        resolved.asset_root.as_path(),
        resolved.scene_root.as_path(),
        resolved.settings_dir.as_path(),
    ] {
        fs::create_dir_all(directory).map_err(|error| {
            ProjectError::Io(format!(
                "failed to create project directory {}: {error}",
                directory.display()
            ))
        })?;
    }
    let toml = file.to_toml_string()?;
    fs::write(&manifest_path, toml).map_err(|error| {
        ProjectError::Io(format!(
            "failed to write {}: {error}",
            manifest_path.display()
        ))
    })?;
    let default_scene_path =
        scene_file_path_in_root(resolved.scene_root.as_path(), DEFAULT_SCENE_FILENAME);
    create_default_scene_file(&default_scene_path, "Main Scene").map_err(|error| {
        ProjectError::Io(format!(
            "failed to write default scene {}: {error}",
            default_scene_path.display()
        ))
    })?;
    let project = Project::new(
        project_id_from_root(&request.root),
        manifest.name.as_str(),
        &request.root,
        resolved,
        manifest.editor.clone(),
    )?;
    let validation = validate_opened_project(&project, &manifest_path);
    Ok(ProjectLoadResult {
        project,
        validation,
    })
}

pub fn ensure_manifest_filename(path: &Path) -> PathBuf {
    if path
        .file_name()
        .is_some_and(|name| name == MANIFEST_FILENAME)
    {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_scene_model::DEFAULT_SCENE_FILENAME;
    use std::fs;

    #[test]
    fn creating_project_writes_manifest_and_folders() {
        let temp = std::env::temp_dir().join(format!("elcarax-create-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let result = create_project(&ProjectCreateRequest::new(&temp, "Created Project"));
        let result = match result {
            Ok(value) => value,
            Err(error) => panic!("create should succeed: {error}"),
        };
        assert_eq!(result.project.name().as_str(), "Created Project");
        assert!(manifest_path_for_root(&temp).is_file());
        assert!(temp.join("assets").is_dir());
        assert!(temp.join("scenes").is_dir());
        assert!(temp.join(".elcarax").is_dir());
        assert!(temp.join("scenes").join(DEFAULT_SCENE_FILENAME).is_file());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn creating_project_without_overwrite_fails_when_manifest_exists() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-create-exists-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let first = create_project(&ProjectCreateRequest::new(&temp, "First"));
        assert!(first.is_ok());
        let second = create_project(&ProjectCreateRequest::new(&temp, "Second"));
        assert!(matches!(second, Err(ProjectError::AlreadyExists)));
        let _ = fs::remove_dir_all(&temp);
    }
}
