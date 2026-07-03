use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ProjectError;

pub const MANIFEST_FILENAME: &str = "elcarax.project.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectFileVersion(pub u32);

impl ProjectFileVersion {
    pub const CURRENT: Self = Self(1);

    pub const fn value(self) -> u32 {
        self.0
    }

    pub fn validate(self) -> Result<(), ProjectError> {
        if self.0 == Self::CURRENT.0 {
            Ok(())
        } else {
            Err(ProjectError::UnsupportedSchemaVersion(self.0))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPaths {
    pub asset_root: PathBuf,
    pub scene_root: PathBuf,
    pub settings_dir: PathBuf,
}

impl ProjectPaths {
    pub fn defaults() -> Self {
        Self {
            asset_root: PathBuf::from("assets"),
            scene_root: PathBuf::from("scenes"),
            settings_dir: PathBuf::from(".elcarax"),
        }
    }

    pub fn resolve(&self, project_root: &Path) -> ResolvedProjectPaths {
        ResolvedProjectPaths {
            asset_root: project_root.join(&self.asset_root),
            scene_root: project_root.join(&self.scene_root),
            settings_dir: project_root.join(&self.settings_dir),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProjectPaths {
    pub asset_root: PathBuf,
    pub scene_root: PathBuf,
    pub settings_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectEditorSettings {
    pub active_scene: Option<PathBuf>,
}

impl ProjectEditorSettings {
    pub fn with_default_active_scene() -> Self {
        Self {
            active_scene: Some(PathBuf::from(elcarax_scene_model::DEFAULT_SCENE_FILENAME)),
        }
    }

    pub fn active_scene_relative(&self) -> Option<&Path> {
        self.active_scene.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectSettings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectManifest {
    pub schema_version: ProjectFileVersion,
    pub name: String,
    pub paths: ProjectPaths,
    pub settings: ProjectSettings,
    pub editor: ProjectEditorSettings,
}

impl ProjectManifest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: ProjectFileVersion::CURRENT,
            name: name.into(),
            paths: ProjectPaths::defaults(),
            settings: ProjectSettings,
            editor: ProjectEditorSettings::with_default_active_scene(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    pub manifest: ProjectManifest,
}

impl ProjectFile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            manifest: ProjectManifest::new(name),
        }
    }

    pub fn validate(&self) -> Result<(), ProjectError> {
        self.manifest.schema_version.validate()?;
        if self.manifest.name.trim().is_empty() {
            return Err(ProjectError::EmptyProjectName);
        }
        Ok(())
    }

    pub fn to_toml_string(&self) -> Result<String, ProjectError> {
        self.validate()?;
        let document = ManifestDocument::from_manifest(&self.manifest);
        toml::to_string_pretty(&document)
            .map_err(|error| ProjectError::ManifestInvalid(error.to_string()))
    }

    pub fn from_toml_str(content: &str) -> Result<Self, ProjectError> {
        let document: ManifestDocument = toml::from_str(content)
            .map_err(|error| ProjectError::ManifestInvalid(error.to_string()))?;
        let manifest = document.into_manifest()?;
        let file = Self { manifest };
        file.validate()?;
        Ok(file)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestDocument {
    schema_version: u32,
    name: String,
    paths: ManifestPathsDocument,
    #[serde(default)]
    editor: ManifestEditorDocument,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ManifestEditorDocument {
    active_scene: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestPathsDocument {
    asset_root: String,
    scene_root: String,
    settings_dir: String,
}

impl ManifestDocument {
    fn from_manifest(manifest: &ProjectManifest) -> Self {
        Self {
            schema_version: manifest.schema_version.value(),
            name: manifest.name.clone(),
            paths: ManifestPathsDocument {
                asset_root: manifest.paths.asset_root.display().to_string(),
                scene_root: manifest.paths.scene_root.display().to_string(),
                settings_dir: manifest.paths.settings_dir.display().to_string(),
            },
            editor: ManifestEditorDocument {
                active_scene: manifest
                    .editor
                    .active_scene
                    .as_ref()
                    .map(|path| path.display().to_string()),
            },
        }
    }

    fn into_manifest(self) -> Result<ProjectManifest, ProjectError> {
        let schema_version = ProjectFileVersion(self.schema_version);
        schema_version.validate()?;
        Ok(ProjectManifest {
            schema_version,
            name: self.name,
            paths: ProjectPaths {
                asset_root: PathBuf::from(self.paths.asset_root),
                scene_root: PathBuf::from(self.paths.scene_root),
                settings_dir: PathBuf::from(self.paths.settings_dir),
            },
            settings: ProjectSettings,
            editor: ProjectEditorSettings {
                active_scene: self.editor.active_scene.map(PathBuf::from),
            },
        })
    }
}

pub fn manifest_path_for_root(project_root: &Path) -> PathBuf {
    project_root.join(MANIFEST_FILENAME)
}
