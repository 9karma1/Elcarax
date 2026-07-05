#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AppProjectConfig {
    pub(crate) open_path: Option<PathBuf>,
    pub(crate) create_root: Option<PathBuf>,
    pub(crate) create_name: Option<String>,
    pub(crate) recent_store_path: Option<PathBuf>,
    pub(crate) adapter_executable: Option<PathBuf>,
    pub(crate) adapter_project_path: Option<PathBuf>,
    pub(crate) auto_connect_adapter: bool,
}

impl AppProjectConfig {
    pub(crate) fn from_env_and_args(args: &[String]) -> Self {
        let mut config = Self {
            open_path: env::var_os("ELCARAX_PROJECT_PATH").map(PathBuf::from),
            create_root: env::var_os("ELCARAX_PROJECT_CREATE_PATH").map(PathBuf::from),
            create_name: env::var("ELCARAX_PROJECT_NAME").ok(),
            recent_store_path: env::var_os("ELCARAX_RECENT_PROJECTS_PATH").map(PathBuf::from),
            adapter_executable: env::var_os("ELCARAX_ADAPTER_EXE").map(PathBuf::from),
            adapter_project_path: env::var_os("ELCARAX_ADAPTER_PROJECT_PATH").map(PathBuf::from),
            auto_connect_adapter: env_flag("ELCARAX_ADAPTER_AUTO_CONNECT"),
        };
        if config.adapter_executable.is_some() {
            config.auto_connect_adapter = true;
        }
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
                "--adapter" => {
                    if let Some(path) = args.get(index + 1) {
                        config.adapter_executable = Some(PathBuf::from(path));
                        config.auto_connect_adapter = true;
                        index += 2;
                        continue;
                    }
                }
                "--adapter-project" => {
                    if let Some(path) = args.get(index + 1) {
                        config.adapter_project_path = Some(PathBuf::from(path));
                        index += 2;
                        continue;
                    }
                }
                "--auto-connect-adapter" => {
                    config.auto_connect_adapter = true;
                    index += 1;
                    continue;
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

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
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

    #[test]
    fn parses_adapter_cli_arguments() {
        let args = vec![
            "elcarax_app".to_string(),
            "--adapter".to_string(),
            "/tmp/adapter".to_string(),
            "--adapter-project".to_string(),
            "/tmp/game/assets".to_string(),
        ];
        let config = AppProjectConfig::from_env_and_args(&args);
        assert_eq!(
            config.adapter_executable,
            Some(PathBuf::from("/tmp/adapter"))
        );
        assert_eq!(
            config.adapter_project_path,
            Some(PathBuf::from("/tmp/game/assets"))
        );
        assert!(config.auto_connect_adapter);
    }
}
