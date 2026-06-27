//! File-based asset indexing foundation for Elcarax.

mod demo;
mod diagnostic;
mod error;
mod index;
mod kind;
mod record;
mod scan;
mod selection;

pub use demo::{demo_asset_index, demo_asset_records, demo_asset_scan};
pub use diagnostic::AssetDiagnostic;
pub use error::AssetError;
pub use index::AssetIndex;
pub use kind::{AssetKind, detect_kind_from_extension, detect_kind_from_path};
pub use record::{AssetId, AssetName, AssetPath, AssetRecord, stable_asset_id};
pub use scan::{AssetScan, apply_selection_after_scan, scan_demo_assets};
pub use selection::AssetSelection;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn empty_asset_index_is_valid() {
        let index = AssetIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert_eq!(index.kind_summary(), "none");
    }

    #[test]
    fn demo_asset_index_has_stable_order() {
        let records: Vec<_> = demo_asset_index()
            .records()
            .iter()
            .map(|record| record.path.display())
            .collect();
        assert_eq!(
            records,
            vec![
                "README.md",
                "assets/audio/click.wav",
                "assets/materials/default.material",
                "assets/models/cube.glb",
                "assets/scenes/demo.scene",
                "assets/textures/checker.png",
                "scripts/player.rs",
            ]
        );
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
    fn selecting_first_asset_works() {
        let index = demo_asset_index();
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
        let index = demo_asset_index();
        let mut selection = AssetSelection::none();
        assert!(selection.select_first(&index));
        selection.clear();
        assert_eq!(selection.selected(), None);
    }

    #[test]
    fn missing_root_returns_diagnostic_not_panic() {
        let scan = AssetScan::scan_root(PathBuf::from("definitely/missing/root"));
        assert!(scan.index.is_empty());
        assert!(!scan.diagnostics.is_empty());
    }
}
