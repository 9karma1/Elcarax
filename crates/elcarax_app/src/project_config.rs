#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AppProjectConfig {
    pub(crate) open_path: Option<PathBuf>,
    pub(crate) create_root: Option<PathBuf>,
    pub(crate) create_name: Option<String>,
    pub(crate) recent_store_path: Option<PathBuf>,
}

impl AppProjectConfig {
    pub(crate) fn from_env_and_args(args: &[String]) -> Self {
        let mut config = Self {
            open_path: env::var_os("ELCARAX_PROJECT_PATH").map(PathBuf::from),
            create_root: env::var_os("ELCARAX_PROJECT_CREATE_PATH").map(PathBuf::from),
            create_name: env::var("ELCARAX_PROJECT_NAME").ok(),
            recent_store_path: env::var_os("ELCARAX_RECENT_PROJECTS_PATH").map(PathBuf::from),
        };
        let mut index = 1usize;
        while index < args.len() {
            match args[index].as_str() {
                "--project" => {
                    if let Some(path) = args.get(index + 1) {
                        config.open_path = Some(PathBuf::from(path));
                        index += 2;
                        continue;
                    }
                }
                "--create-project" => {
                    if let Some(path) = args.get(index + 1) {
                        config.create_root = Some(PathBuf::from(path));
                        index += 2;
                        continue;
                    }
                }
                "--project-name" => {
                    if let Some(name) = args.get(index + 1) {
                        config.create_name = Some(name.clone());
                        index += 2;
                        continue;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        config
    }

    pub(crate) fn default_recent_store_path() -> PathBuf {
        PathBuf::from(".elcarax/recent-projects.toml")
    }

    pub(crate) fn recent_store_path(&self) -> PathBuf {
        self.recent_store_path
            .clone()
            .unwrap_or_else(Self::default_recent_store_path)
    }

    pub(crate) fn create_name(&self) -> &str {
        match self.create_name.as_deref() {
            Some(name) if !name.trim().is_empty() => name.trim(),
            _ => "My Elcarax Project",
        }
    }

    pub(crate) fn create_root_or_open_parent(&self) -> Option<PathBuf> {
        if let Some(root) = &self.create_root {
            return Some(root.clone());
        }
        self.open_path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_project_cli_argument() {
        let args = vec![
            "elcarax_app".to_string(),
            "--project".to_string(),
            "/tmp/project".to_string(),
        ];
        let config = AppProjectConfig::from_env_and_args(&args);
        assert_eq!(config.open_path, Some(PathBuf::from("/tmp/project")));
    }

    #[test]
    fn parses_create_project_cli_arguments() {
        let args = vec![
            "elcarax_app".to_string(),
            "--create-project".to_string(),
            "/tmp/new-project".to_string(),
            "--project-name".to_string(),
            "Named Project".to_string(),
        ];
        let config = AppProjectConfig::from_env_and_args(&args);
        assert_eq!(config.create_root, Some(PathBuf::from("/tmp/new-project")));
        assert_eq!(config.create_name(), "Named Project");
    }
}
