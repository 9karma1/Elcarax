use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::AssetDiagnostic;
use crate::index::AssetIndex;
use crate::record::{AssetId, AssetRecord, stable_asset_id};
use crate::selection::AssetSelection;

static SCAN_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetScan {
    pub root: Option<PathBuf>,
    pub index: AssetIndex,
    pub diagnostics: Vec<AssetDiagnostic>,
}

impl AssetScan {
    pub fn empty() -> Self {
        Self {
            root: None,
            index: AssetIndex::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn from_demo_index(index: AssetIndex) -> Self {
        Self {
            root: Some(PathBuf::from("assets")),
            index,
            diagnostics: Vec::new(),
        }
    }

    pub fn scan_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        if !root.exists() {
            let display = root.display().to_string();
            return Self {
                root: Some(root),
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
                index: AssetIndex::new(),
                diagnostics: vec![AssetDiagnostic::error(
                    "root",
                    format!("Asset root is not a directory: {display}"),
                )],
            };
        }
        let mut records = Vec::new();
        let mut diagnostics = Vec::new();
        collect_records(&root, &root, &mut records, &mut diagnostics);
        Self {
            root: Some(root),
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
    root: &Path,
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
    for entry in entries.flatten() {
        let path = entry.path();
        let is_directory = path.is_dir();
        if is_directory {
            collect_records(root, &path, records, diagnostics);
            continue;
        }
        let relative = match path.strip_prefix(root) {
            Ok(relative) => relative.to_path_buf(),
            Err(_) => path.clone(),
        };
        let id = next_scan_id();
        match AssetRecord::with_detected_kind(id, relative, false) {
            Ok(record) => records.push(record),
            Err(error) => diagnostics.push(AssetDiagnostic::warning("record", error.to_string())),
        }
    }
}

fn next_scan_id() -> AssetId {
    let value = SCAN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    stable_asset_id(value)
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

    #[test]
    fn missing_root_returns_diagnostic_not_panic() {
        let scan = AssetScan::scan_root(PathBuf::from("missing/asset/root/for/elcarax"));
        assert!(scan.index.is_empty());
        assert_eq!(scan.diagnostics.len(), 1);
        assert_eq!(scan.diagnostics[0].field(), "root");
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
}
