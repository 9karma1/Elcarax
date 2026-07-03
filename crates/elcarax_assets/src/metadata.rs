use std::path::PathBuf;
use std::time::SystemTime;

use crate::diagnostic::AssetDiagnostic;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssetMetadata {
    pub source_path: Option<PathBuf>,
    pub extension: Option<String>,
    pub file_size: Option<u64>,
    pub modified_time: Option<SystemTime>,
    pub diagnostics: Vec<AssetDiagnostic>,
}

impl AssetMetadata {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with_source_path(mut self, source_path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(source_path.into());
        self
    }

    pub fn with_extension(mut self, extension: Option<String>) -> Self {
        self.extension = extension;
        self
    }

    pub fn with_file_size(mut self, file_size: Option<u64>) -> Self {
        self.file_size = file_size;
        self
    }

    pub fn with_modified_time(mut self, modified_time: Option<SystemTime>) -> Self {
        self.modified_time = modified_time;
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: Vec<AssetDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }
}
