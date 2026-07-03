#![cfg(feature = "native-shell")]

use std::path::PathBuf;

use crate::project_config::AppProjectConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProjectPathResolution {
    Resolved(PathBuf),
    Cancelled,
}

pub(crate) fn resolve_open_project_path(config: &AppProjectConfig) -> ProjectPathResolution {
    if let Some(path) = &config.open_path {
        return ProjectPathResolution::Resolved(path.clone());
    }
    match elcarax_platform::pick_folder("Open Elcarax Project") {
        Some(path) => ProjectPathResolution::Resolved(path),
        None => ProjectPathResolution::Cancelled,
    }
}

pub(crate) fn resolve_create_project_root(config: &AppProjectConfig) -> ProjectPathResolution {
    if let Some(root) = &config.create_root {
        return ProjectPathResolution::Resolved(root.clone());
    }
    match elcarax_platform::pick_folder("Choose Folder for New Project") {
        Some(path) => ProjectPathResolution::Resolved(path),
        None => ProjectPathResolution::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_open_path_skips_dialog() {
        let config = AppProjectConfig {
            open_path: Some(PathBuf::from("/tmp/project")),
            ..AppProjectConfig::default()
        };
        assert_eq!(
            resolve_open_project_path(&config),
            ProjectPathResolution::Resolved(PathBuf::from("/tmp/project"))
        );
    }

    #[test]
    fn configured_create_root_skips_dialog() {
        let config = AppProjectConfig {
            create_root: Some(PathBuf::from("/tmp/new-project")),
            ..AppProjectConfig::default()
        };
        assert_eq!(
            resolve_create_project_root(&config),
            ProjectPathResolution::Resolved(PathBuf::from("/tmp/new-project"))
        );
    }
}
