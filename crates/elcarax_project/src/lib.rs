//! Project file and project-state domain model for Elcarax.

mod create;
mod domain;
mod error;
mod manifest;
mod open;
mod recent;
mod validate;

pub use create::{ProjectCreateRequest, create_project, ensure_manifest_filename};
pub use domain::{
    Project, ProjectDiagnostic, ProjectId, ProjectName, ProjectPath, ProjectStatus,
    ProjectValidation, project_id_from_root,
};
pub use error::ProjectError;
pub use manifest::{
    MANIFEST_FILENAME, ProjectFile, ProjectFileVersion, ProjectManifest, ProjectPaths,
    ProjectSettings, ResolvedProjectPaths, manifest_path_for_root,
};
pub use open::{ProjectLoadResult, ProjectOpenRequest, open_project};
pub use recent::{RecentProjectEntry, RecentProjects, RecentProjectsError, RecentProjectsStore};
pub use validate::{ProjectValidationResult, validate_opened_project};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn project_status_labels_are_stable() {
        assert_eq!(ProjectStatus::NoProject.label(), "None");
        assert_eq!(ProjectStatus::Loaded.label(), "Loaded");
        assert_eq!(ProjectStatus::Invalid.label(), "Invalid");
    }

    #[test]
    fn invalid_project_name_produces_diagnostics() {
        let project = Project::from_loaded_data(
            project_id_from_root(PathBuf::from("/tmp/invalid").as_path()),
            "",
            PathBuf::from("/tmp/invalid"),
            ResolvedProjectPaths {
                asset_root: PathBuf::from("/tmp/invalid/assets"),
                scene_root: PathBuf::from("/tmp/invalid/scenes"),
                settings_dir: PathBuf::from("/tmp/invalid/.elcarax"),
            },
        );
        let validation = project.validate();
        assert_eq!(validation.status(), ProjectStatus::Invalid);
        assert_eq!(validation.error_count(), 1);
    }

    #[test]
    fn recent_project_list_preserves_recency_order() {
        let mut recent = RecentProjects::default();
        let first = Project::from_loaded_data(
            project_id_from_root(PathBuf::from("/tmp/a").as_path()),
            "First",
            PathBuf::from("/tmp/a"),
            ResolvedProjectPaths {
                asset_root: PathBuf::from("/tmp/a/assets"),
                scene_root: PathBuf::from("/tmp/a/scenes"),
                settings_dir: PathBuf::from("/tmp/a/.elcarax"),
            },
        );
        let second = Project::from_loaded_data(
            project_id_from_root(PathBuf::from("/tmp/b").as_path()),
            "Second",
            PathBuf::from("/tmp/b"),
            ResolvedProjectPaths {
                asset_root: PathBuf::from("/tmp/b/assets"),
                scene_root: PathBuf::from("/tmp/b/scenes"),
                settings_dir: PathBuf::from("/tmp/b/.elcarax"),
            },
        );
        recent.record(&first);
        recent.record(&second);
        recent.record(&first);
        assert_eq!(recent.entries()[0].name, "First");
        assert_eq!(recent.entries()[1].name, "Second");
    }
}
