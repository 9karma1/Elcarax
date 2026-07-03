use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::AssetDiagnostic;
use crate::index::AssetIndex;
use crate::kind::detect_kind_from_path;
use crate::metadata::AssetMetadata;
use crate::record::{AssetRecord, stable_asset_id_from_path};
use crate::root::AssetRoot;
use crate::selection::AssetSelection;

pub type AssetScanResult = AssetScan;
pub type AssetScanDiagnostic = AssetDiagnostic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetScanRequest {
    root: AssetRoot,
}

impl AssetScanRequest {
    pub fn new(project_root: impl Into<PathBuf>, asset_root: impl Into<PathBuf>) -> Self {
        Self {
            root: AssetRoot::new(project_root, asset_root),
        }
    }

    pub fn from_asset_root(asset_root: impl Into<PathBuf>) -> Self {
        Self {
            root: AssetRoot::from_asset_root(asset_root),
        }
    }

    pub fn root(&self) -> &AssetRoot {
        &self.root
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetScan {
    pub root: Option<PathBuf>,
    pub request: Option<AssetScanRequest>,
    pub index: AssetIndex,
    pub diagnostics: Vec<AssetDiagnostic>,
}

impl AssetScan {
    pub fn empty() -> Self {
        Self {
            root: None,
            request: None,
            index: AssetIndex::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn from_demo_index(index: AssetIndex) -> Self {
        Self {
            root: Some(PathBuf::from("assets")),
            request: None,
            index,
            diagnostics: Vec::new(),
        }
    }

    pub fn scan_root(root: impl AsRef<Path>) -> Self {
        Self::scan(AssetScanRequest::from_asset_root(
            root.as_ref().to_path_buf(),
        ))
    }

    pub fn scan(request: AssetScanRequest) -> Self {
        let project_root = request.root().project_root().to_path_buf();
        let root = request.root().asset_root().to_path_buf();
        if !root.exists() {
            let display = root.display().to_string();
            return Self {
                root: Some(root),
                request: Some(request),
                index: AssetIndex::new(),
                diagnostics: vec![AssetDiagnostic::warning(
                    "root",
                    format!("Asset root does not exist: {display}"),
                )],
            };
        }
        if !root.is_dir() {
            let display = root.display().to_string();
            return Self {
                root: Some(root),
                request: Some(request),
                index: AssetIndex::new(),
                diagnostics: vec![AssetDiagnostic::error(
                    "root",
                    format!("Asset root is not a directory: {display}"),
                )],
            };
        }
        let mut records = Vec::new();
        let mut diagnostics = Vec::new();
        collect_records(
            project_root.as_path(),
            &root,
            &mut records,
            &mut diagnostics,
        );
        Self {
            root: Some(root),
            request: Some(request),
            index: AssetIndex::from_records(records),
            diagnostics,
        }
    }

    pub fn asset_count(&self) -> usize {
        self.index.len()
    }

    pub fn diagnostics(&self) -> &[AssetDiagnostic] {
        self.diagnostics.as_slice()
    }
}

fn collect_records(
    project_root: &Path,
    current: &Path,
    records: &mut Vec<AssetRecord>,
    diagnostics: &mut Vec<AssetDiagnostic>,
) {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(AssetDiagnostic::warning(
                "scan",
                format!("Failed to read {}: {error}", current.display()),
            ));
            return;
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => paths.push(entry.path()),
            Err(error) => diagnostics.push(AssetDiagnostic::warning(
                "scan",
                format!("Failed to read entry in {}: {error}", current.display()),
            )),
        }
    }
    paths.sort();
    for path in paths {
        if should_skip_path(&path) {
            continue;
        }
        let is_directory = path.is_dir();
        collect_record(project_root, &path, is_directory, records, diagnostics);
        if is_directory {
            collect_records(project_root, &path, records, diagnostics);
        }
    }
}

fn collect_record(
    project_root: &Path,
    path: &Path,
    is_directory: bool,
    records: &mut Vec<AssetRecord>,
    diagnostics: &mut Vec<AssetDiagnostic>,
) {
    let relative = project_relative_path(project_root, path);
    let id = stable_asset_id_from_path(relative.as_path());
    let kind = detect_kind_from_path(relative.as_path(), is_directory);
    let metadata = metadata_for_path(path, is_directory);
    match AssetRecord::new(id, relative, kind) {
        Ok(record) => records.push(record.with_metadata(metadata)),
        Err(error) => diagnostics.push(AssetDiagnostic::warning("record", error.to_string())),
    }
}

fn project_relative_path(project_root: &Path, path: &Path) -> PathBuf {
    match path.strip_prefix(project_root) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => path.to_path_buf(),
    }
}

fn metadata_for_path(path: &Path, is_directory: bool) -> AssetMetadata {
    let mut diagnostics = Vec::new();
    let (file_size, modified_time) = match fs::metadata(path) {
        Ok(metadata) => {
            let file_size = if is_directory {
                None
            } else {
                Some(metadata.len())
            };
            let modified_time = metadata.modified().ok();
            (file_size, modified_time)
        }
        Err(error) => {
            diagnostics.push(AssetDiagnostic::warning(
                "metadata",
                format!("Failed to read metadata for {}: {error}", path.display()),
            ));
            (None, None)
        }
    };
    AssetMetadata::empty()
        .with_source_path(path.to_path_buf())
        .with_extension(extension_for_path(path))
        .with_file_size(file_size)
        .with_modified_time(modified_time)
        .with_diagnostics(diagnostics)
}

fn extension_for_path(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

fn should_skip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".elcarax" || name.starts_with('.'))
}

pub fn scan_demo_assets() -> AssetScan {
    crate::demo::demo_asset_scan()
}

pub fn apply_selection_after_scan(scan: &AssetScan, selection: &mut AssetSelection) {
    if let Some(selected) = selection.selected()
        && scan.index.find(selected).is_some()
    {
        return;
    }
    selection.clear();
}

#[cfg(test)]
mod scan_tests {
    use super::*;
    use crate::demo::demo_asset_index;
    use crate::kind::AssetKind;
    use std::fs;

    #[test]
    fn missing_root_returns_diagnostic_not_panic() {
        let scan = AssetScan::scan_root(PathBuf::from("missing/asset/root/for/elcarax"));
        assert!(scan.index.is_empty());
        assert_eq!(scan.diagnostics.len(), 1);
        assert_eq!(scan.diagnostics[0].field(), "root");
    }

    #[test]
    fn scanning_empty_asset_root_succeeds() {
        let root = temp_root("empty");
        let assets = root.join("assets");
        assert!(fs::create_dir_all(&assets).is_ok());
        let scan = AssetScan::scan(AssetScanRequest::new(&root, &assets));
        assert_eq!(scan.asset_count(), 0);
        assert!(scan.diagnostics.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scan_returns_project_relative_paths_and_folder_records() {
        let root = temp_root("relative");
        let assets = root.join("assets");
        let models = assets.join("models");
        assert!(fs::create_dir_all(&models).is_ok());
        assert!(fs::write(models.join("hero.glb"), "model").is_ok());
        let scan = AssetScan::scan(AssetScanRequest::new(&root, &assets));
        let paths: Vec<_> = scan
            .index
            .records()
            .iter()
            .map(|record| record.path.display())
            .collect();
        assert_eq!(paths, vec!["assets/models", "assets/models/hero.glb"]);
        assert_eq!(scan.index.records()[0].kind, AssetKind::Folder);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stable_asset_ids_survive_repeated_scans() {
        let root = temp_root("stable");
        let assets = root.join("assets");
        assert!(fs::create_dir_all(&assets).is_ok());
        assert!(fs::write(assets.join("level.scene"), "scene").is_ok());
        let first = AssetScan::scan(AssetScanRequest::new(&root, &assets));
        let second = AssetScan::scan(AssetScanRequest::new(&root, &assets));
        assert_eq!(first.index.records()[0].id, second.index.records()[0].id);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sorted_order_is_deterministic() {
        let root = temp_root("sorted");
        let assets = root.join("assets");
        assert!(fs::create_dir_all(&assets).is_ok());
        assert!(fs::write(assets.join("z.txt"), "z").is_ok());
        assert!(fs::write(assets.join("a.txt"), "a").is_ok());
        let scan = AssetScan::scan(AssetScanRequest::new(&root, &assets));
        let paths: Vec<_> = scan
            .index
            .records()
            .iter()
            .map(|record| record.path.display())
            .collect();
        assert_eq!(paths, vec!["assets/a.txt", "assets/z.txt"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hidden_and_elcarax_folders_are_skipped() {
        let root = temp_root("hidden");
        let assets = root.join("assets");
        assert!(fs::create_dir_all(assets.join(".elcarax")).is_ok());
        assert!(fs::create_dir_all(assets.join(".hidden")).is_ok());
        assert!(fs::write(assets.join(".secret.txt"), "secret").is_ok());
        assert!(fs::write(assets.join("visible.txt"), "visible").is_ok());
        assert!(fs::write(assets.join(".elcarax").join("cache.txt"), "cache").is_ok());
        assert!(fs::write(assets.join(".hidden").join("skip.txt"), "skip").is_ok());
        let scan = AssetScan::scan(AssetScanRequest::new(&root, &assets));
        let paths: Vec<_> = scan
            .index
            .records()
            .iter()
            .map(|record| record.path.display())
            .collect();
        assert_eq!(paths, vec!["assets/visible.txt"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn demo_scan_has_expected_count() {
        let scan = scan_demo_assets();
        assert_eq!(scan.asset_count(), 7);
    }

    #[test]
    fn demo_index_kind_counts_are_correct() {
        let index = demo_asset_index();
        let counts = index.kind_counts();
        assert_eq!(counts.get(&AssetKind::Scene), Some(&1));
        assert_eq!(counts.get(&AssetKind::Image), Some(&1));
        assert_eq!(counts.get(&AssetKind::Unknown), None);
    }

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("elcarax-assets-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }
}
