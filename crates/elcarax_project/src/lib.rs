//! Project file model for Elcarax.

use std::path::PathBuf;

use elcarax_core::{ElcaraxError, Result};

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
