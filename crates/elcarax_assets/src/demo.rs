use std::path::PathBuf;

use crate::index::AssetIndex;
use crate::kind::AssetKind;
use crate::record::{AssetRecord, stable_asset_id};
use crate::scan::AssetScan;

pub fn demo_asset_records() -> Vec<AssetRecord> {
    [
        (1, "README.md", PathBuf::from("README.md"), AssetKind::Text),
        (
            2,
            "click.wav",
            PathBuf::from("assets/audio/click.wav"),
            AssetKind::Audio,
        ),
        (
            3,
            "default.material",
            PathBuf::from("assets/materials/default.material"),
            AssetKind::Material,
        ),
        (
            4,
            "cube.glb",
            PathBuf::from("assets/models/cube.glb"),
            AssetKind::Model,
        ),
        (
            5,
            "demo.scene",
            PathBuf::from("assets/scenes/demo.scene"),
            AssetKind::Scene,
        ),
        (
            6,
            "checker.png",
            PathBuf::from("assets/textures/checker.png"),
            AssetKind::Image,
        ),
        (
            7,
            "player.rs",
            PathBuf::from("scripts/player.rs"),
            AssetKind::Script,
        ),
    ]
    .into_iter()
    .map(|(id, name, path, kind)| AssetRecord::from_parts(stable_asset_id(id), name, path, kind))
    .collect()
}

pub fn demo_asset_index() -> AssetIndex {
    AssetIndex::from_records(demo_asset_records())
}

pub fn demo_asset_scan() -> AssetScan {
    AssetScan::from_demo_index(demo_asset_index())
}
