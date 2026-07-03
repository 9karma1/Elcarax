use std::num::NonZeroU64;
use std::path::{Component, Path, PathBuf};

use elcarax_core::Id;

use crate::error::AssetError;
use crate::kind::{AssetKind, detect_kind_from_path};
use crate::metadata::AssetMetadata;

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
        Ok(Self(normalize_asset_path(path.as_path())))
    }

    pub fn from_unvalidated(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self(normalize_asset_path(path.as_path()))
    }

    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub fn display(&self) -> String {
        normalized_asset_path_string(self.0.as_path())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRecord {
    pub id: AssetId,
    pub name: AssetName,
    pub path: AssetPath,
    pub source_path: Option<PathBuf>,
    pub kind: AssetKind,
    pub extension: Option<String>,
    pub file_size: Option<u64>,
    pub modified_time: Option<std::time::SystemTime>,
    pub diagnostics: Vec<crate::diagnostic::AssetDiagnostic>,
}

impl AssetRecord {
    pub fn new(id: AssetId, path: impl Into<PathBuf>, kind: AssetKind) -> Result<Self, AssetError> {
        let path = AssetPath::new(path)?;
        let name = AssetName::from_path(path.as_path())?;
        Ok(Self {
            id,
            name,
            path,
            source_path: None,
            kind,
            extension: None,
            file_size: None,
            modified_time: None,
            diagnostics: Vec::new(),
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
            source_path: None,
            kind,
            extension: None,
            file_size: None,
            modified_time: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: AssetMetadata) -> Self {
        self.source_path = metadata.source_path;
        self.extension = metadata.extension;
        self.file_size = metadata.file_size;
        self.modified_time = metadata.modified_time;
        self.diagnostics = metadata.diagnostics;
        self
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

pub fn stable_asset_id_from_path(path: &Path) -> AssetId {
    let normalized = normalized_asset_path_string(path);
    stable_asset_id(fnv1a64(normalized.as_bytes()))
}

pub fn normalized_asset_path_string(path: &Path) -> String {
    normalize_asset_path(path)
        .components()
        .filter_map(component_to_string)
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_asset_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => normalized.push(value),
            Component::ParentDir => normalized.push(".."),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => {}
        }
    }
    normalized
}

fn component_to_string(component: Component<'_>) -> Option<String> {
    match component {
        Component::Normal(value) => Some(value.to_string_lossy().to_string()),
        Component::ParentDir => Some("..".to_string()),
        Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_string()),
        Component::CurDir | Component::RootDir => None,
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash.max(1)
}
