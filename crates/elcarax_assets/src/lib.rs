//! File-based asset indexing foundation for Elcarax.

mod diagnostic;
mod error;
mod index;
mod kind;
mod metadata;
mod record;
mod root;
mod scan;
mod selection;
mod watch;

pub use diagnostic::AssetDiagnostic;
pub use error::AssetError;
pub use index::{AssetIndex, AssetIndexSnapshot};
pub use kind::{AssetKind, detect_kind_from_extension, detect_kind_from_path};
pub use metadata::AssetMetadata;
pub use record::{
    AssetId, AssetName, AssetPath, AssetRecord, normalized_asset_path_string,
    stable_asset_id_from_path,
};
pub use root::AssetRoot;
pub use scan::{
    AssetScan, AssetScanDiagnostic, AssetScanRequest, AssetScanResult, apply_selection_after_scan,
};
pub use selection::AssetSelection;
pub use watch::{
    AssetWatchError, AssetWatchEvent, AssetWatchEventKind, AssetWatchService, AssetWatchStatus,
};

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn empty_asset_index_is_valid() {
        let index = AssetIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert_eq!(index.kind_summary(), "none");
    }

    #[test]
    fn extension_detection_maps_known_types() {
        assert_eq!(detect_kind_from_extension(Some("scene")), AssetKind::Scene);
        assert_eq!(detect_kind_from_extension(Some("png")), AssetKind::Image);
        assert_eq!(detect_kind_from_extension(Some("wav")), AssetKind::Audio);
        assert_eq!(detect_kind_from_extension(Some("glb")), AssetKind::Model);
        assert_eq!(detect_kind_from_extension(Some("rs")), AssetKind::Script);
        assert_eq!(
            detect_kind_from_extension(Some("material")),
            AssetKind::Material
        );
        assert_eq!(detect_kind_from_extension(Some("md")), AssetKind::Text);
    }

    #[test]
    fn unknown_extension_maps_to_unknown() {
        assert_eq!(detect_kind_from_extension(Some("xyz")), AssetKind::Unknown);
        assert_eq!(detect_kind_from_extension(None), AssetKind::Unknown);
    }

    #[test]
    fn path_normalization_uses_forward_slashes_for_ids() {
        let windows = PathBuf::from("assets\\models\\hero.glb");
        let portable = PathBuf::from("assets/models/hero.glb");
        assert_eq!(
            normalized_asset_path_string(windows.as_path()),
            "assets/models/hero.glb"
        );
        assert_eq!(
            stable_asset_id_from_path(windows.as_path()),
            stable_asset_id_from_path(portable.as_path())
        );
    }

    #[test]
    fn selecting_first_asset_works() {
        let index = fixture_asset_index();
        let mut selection = AssetSelection::none();
        assert!(selection.select_first(&index));
        let selected = match selection.selected() {
            Some(id) => id,
            None => panic!("first asset should be selected"),
        };
        assert_eq!(
            index.find(selected).map(|record| record.path.display()),
            Some("README.md".to_string())
        );
    }

    #[test]
    fn clearing_selection_works() {
        let index = fixture_asset_index();
        let mut selection = AssetSelection::none();
        assert!(selection.select_first(&index));
        selection.clear();
        assert_eq!(selection.selected(), None);
    }

    #[test]
    fn asset_index_selection_by_id_works() {
        let index = fixture_asset_index();
        let first = match index.first() {
            Some(record) => record.id,
            None => panic!("fixture index should contain records"),
        };
        assert_eq!(
            index.find(first).map(|record| record.path.display()),
            Some("README.md".to_string())
        );
    }

    #[test]
    fn missing_root_returns_diagnostic_not_panic() {
        let scan = AssetScan::scan_root(PathBuf::from("definitely/missing/root"));
        assert!(scan.index.is_empty());
        assert!(!scan.diagnostics.is_empty());
    }

    fn fixture_asset_index() -> AssetIndex {
        AssetIndex::from_records(vec![
            AssetRecord::from_parts(
                stable_asset_id_from_path(Path::new("README.md")),
                "README.md",
                PathBuf::from("README.md"),
                AssetKind::Text,
            ),
            AssetRecord::from_parts(
                stable_asset_id_from_path(Path::new("assets/models/cube.glb")),
                "cube.glb",
                PathBuf::from("assets/models/cube.glb"),
                AssetKind::Model,
            ),
        ])
    }
}
