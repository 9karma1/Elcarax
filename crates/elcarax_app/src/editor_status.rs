use crate::adapter_display::AdapterUiSnapshot;
use crate::asset_display::AssetUiSnapshot;
use crate::project_display::ProjectUiSnapshot;
use crate::scene_display::{SceneUiSnapshot, selected_object_label};

pub(crate) fn editor_status_bar(
    project: &ProjectUiSnapshot,
    assets: &AssetUiSnapshot,
    scene: &SceneUiSnapshot,
    adapter: &AdapterUiSnapshot,
) -> String {
    let project_label = project
        .project_name
        .trim_start_matches("Name: ")
        .to_string();
    let asset_label = asset_status_label(assets);
    let scene_label = if scene.scene_name == "No scene" {
        "None".to_string()
    } else {
        scene.scene_name.clone()
    };
    let object_label = selected_object_label(scene);
    let base = format!(
        "Project: {project_label} | Asset: {asset_label} | Scene: {scene_label} | Object: {object_label} | {}",
        adapter.status_adapter_suffix
    );
    if scene.status_scene_suffix.starts_with("Scene: Command:")
        || scene.status_scene_suffix.starts_with("Scene: Diagnostic:")
    {
        return format!("{base} | {}", scene.status_scene_suffix);
    }
    base
}

fn asset_status_label(assets: &AssetUiSnapshot) -> String {
    if assets.asset_count == "Assets: 0" {
        return "None".to_string();
    }
    if let Some(index) = assets.selected_row_index
        && let Some(label) = assets.asset_row_labels.get(index)
        && !label.is_empty()
    {
        return match label.split(" (").next() {
            Some(name) => name.to_string(),
            None => "None".to_string(),
        };
    }
    assets
        .asset_count
        .trim_start_matches("Assets: ")
        .to_string()
}
