#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use std::path::PathBuf;

use crate::asset_state::{ASSET_SCAN_COMMAND, AssetState};
use crate::inspector_state::InspectorState;
use crate::project_state::{
    PROJECT_CLOSE_COMMAND, PROJECT_CREATE_COMMAND, PROJECT_OPEN_COMMAND,
    PROJECT_REOPEN_LAST_COMMAND, ProjectState,
};
use crate::scene_state::SceneState;

pub(crate) fn apply_project_open_side_effects(
    project_state: &mut ProjectState,
    asset_state: &mut AssetState,
    scene_state: &mut SceneState,
    inspector_state: &mut InspectorState,
) {
    let asset_root = project_state.asset_root().map(PathBuf::from);
    if let Some(root) = asset_root {
        asset_state.on_project_opened(root.as_path());
    }
    scene_state.on_project_closed();
    inspector_state.on_project_closed();
    project_state.set_scanned_asset_count(None);
}

pub(crate) fn apply_project_close_side_effects(
    asset_state: &mut AssetState,
    scene_state: &mut SceneState,
    inspector_state: &mut InspectorState,
) {
    asset_state.on_project_closed();
    scene_state.on_project_closed();
    inspector_state.on_project_closed();
}

pub(crate) fn apply_project_command_side_effects(
    command_id: &str,
    project_state: &mut ProjectState,
    asset_state: &mut AssetState,
    scene_state: &mut SceneState,
    inspector_state: &mut InspectorState,
) {
    match command_id {
        PROJECT_CLOSE_COMMAND => {
            apply_project_close_side_effects(asset_state, scene_state, inspector_state)
        }
        PROJECT_CREATE_COMMAND | PROJECT_OPEN_COMMAND | PROJECT_REOPEN_LAST_COMMAND => {
            if project_state.is_project_loaded() {
                apply_project_open_side_effects(
                    project_state,
                    asset_state,
                    scene_state,
                    inspector_state,
                );
            }
        }
        ASSET_SCAN_COMMAND => {
            project_state.set_scanned_asset_count(asset_state.scanned_asset_count());
        }
        _ => {}
    }
}
