use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRoot {
    project_root: PathBuf,
    asset_root: PathBuf,
}

impl AssetRoot {
    pub fn new(project_root: impl Into<PathBuf>, asset_root: impl Into<PathBuf>) -> Self {
        let project_root = project_root.into();
        let asset_root = asset_root.into();
        let asset_root = if asset_root.is_absolute() {
            asset_root
        } else {
            project_root.join(asset_root)
        };
        Self {
            project_root,
            asset_root,
        }
    }

    pub fn from_asset_root(asset_root: impl Into<PathBuf>) -> Self {
        let asset_root = asset_root.into();
        let project_root = asset_root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Self::new(project_root, asset_root)
    }

    pub fn project_root(&self) -> &Path {
        self.project_root.as_path()
    }

    pub fn asset_root(&self) -> &Path {
        self.asset_root.as_path()
    }
}
