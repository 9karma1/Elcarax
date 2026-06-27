use std::num::NonZeroU64;
use std::path::{Path, PathBuf};

use elcarax_core::Id;

use crate::error::AssetError;
use crate::kind::{AssetKind, detect_kind_from_path};

pub enum AssetMarker {}
pub type AssetId = Id<AssetMarker>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetName(String);

impl AssetName {
    pub fn new(value: impl Into<String>) -> Result<Self, AssetError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AssetError::EmptyAssetName);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn from_path(path: &Path) -> Result<Self, AssetError> {
        if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
            return Self::new(name);
        }
        Self::new(path.display().to_string())
    }

    pub fn from_unvalidated(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetPath(PathBuf);

impl AssetPath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, AssetError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(AssetError::EmptyAssetPath);
        }
        Ok(Self(path))
    }

    pub fn from_unvalidated(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub fn display(&self) -> String {
        self.0.display().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRecord {
    pub id: AssetId,
    pub name: AssetName,
    pub path: AssetPath,
    pub kind: AssetKind,
}

impl AssetRecord {
    pub fn new(id: AssetId, path: impl Into<PathBuf>, kind: AssetKind) -> Result<Self, AssetError> {
        let path = AssetPath::new(path)?;
        let name = AssetName::from_path(path.as_path())?;
        Ok(Self {
            id,
            name,
            path,
            kind,
        })
    }

    pub fn from_parts(
        id: AssetId,
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        kind: AssetKind,
    ) -> Self {
        Self {
            id,
            name: AssetName::from_unvalidated(name),
            path: AssetPath::from_unvalidated(path),
            kind,
        }
    }

    pub fn with_detected_kind(
        id: AssetId,
        path: impl Into<PathBuf>,
        is_directory: bool,
    ) -> Result<Self, AssetError> {
        let path_buf = path.into();
        let kind = detect_kind_from_path(path_buf.as_path(), is_directory);
        Self::new(id, path_buf, kind)
    }
}

pub fn stable_asset_id(value: u64) -> AssetId {
    match NonZeroU64::new(value) {
        Some(value) => AssetId::from_non_zero(value),
        None => AssetId::from_non_zero(NonZeroU64::MIN),
    }
}
