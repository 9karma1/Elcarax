use elcarax_render::Rect;
use elcarax_ui::{EditorShellContent, EditorShellIds, TextRole, UiError, UiTree};

use crate::asset_display::AssetUiSnapshot;
use crate::asset_ui::apply_asset_snapshot;
use crate::editor_status::editor_status_bar;
use crate::inspector_display::InspectorUiSnapshot;
use crate::inspector_ui::apply_inspector_snapshot;
use crate::project_display::{DiagnosticTone, ProjectUiSnapshot};
use crate::scene_display::SceneUiSnapshot;
use crate::scene_ui::apply_scene_snapshot;

pub(crate) use crate::scene_ui::shell_content_from_editor_state;

#[cfg_attr(feature = "native-shell", allow(dead_code))]
pub(crate) fn shell_content_from_project(snapshot: &ProjectUiSnapshot) -> EditorShellContent {
    shell_content_from_editor_state(
        snapshot,
        &empty_asset_snapshot(),
        &empty_scene_snapshot(),
        &empty_inspector_snapshot(),
    )
}

#[cfg_attr(feature = "native-shell", allow(dead_code))]
fn empty_asset_snapshot() -> AssetUiSnapshot {
    AssetUiSnapshot {
        asset_section_title: "Assets".to_string(),
        asset_count: "Assets: 0".to_string(),
        asset_row_labels: std::array::from_fn(|_| String::new()),
        asset_selected_summary: "Selected: None".to_string(),
        selected_row_index: None,
        status_asset_suffix: "Asset: None".to_string(),
    }
}

#[cfg_attr(feature = "native-shell", allow(dead_code))]
fn empty_scene_snapshot() -> SceneUiSnapshot {
    SceneUiSnapshot {
        scene_section_title: "Scene".to_string(),
        scene_name: "No scene".to_string(),
        scene_expand_labels: std::array::from_fn(|_| String::new()),
        scene_row_labels: std::array::from_fn(|_| String::new()),
        scene_selected_summary: "Selected: None".to_string(),
        selected_row_index: None,
        visible_object_ids: std::array::from_fn(|_| None),
        status_scene_suffix: "Scene: None | Object: None".to_string(),
    }
}

fn empty_inspector_snapshot() -> InspectorUiSnapshot {
    InspectorUiSnapshot {
        has_selection: false,
        empty_message: "No object selected".to_string(),
        object_name: String::new(),
        object_kind: String::new(),
        row_labels: std::array::from_fn(|_| String::new()),
        row_values: std::array::from_fn(|_| String::new()),
        property_count: 0,
        summary: String::new(),
    }
}

#[cfg_attr(feature = "native-shell", allow(dead_code))]
pub(crate) fn apply_project_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &ProjectUiSnapshot,
    bounds: Rect,
) -> Result<(), UiError> {
    apply_editor_snapshot(
        tree,
        ids,
        snapshot,
        &empty_asset_snapshot(),
        &empty_scene_snapshot(),
        &empty_inspector_snapshot(),
        bounds,
    )
}

pub(crate) fn apply_editor_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    project: &ProjectUiSnapshot,
    assets: &AssetUiSnapshot,
    scene: &SceneUiSnapshot,
    inspector: &InspectorUiSnapshot,
    bounds: Rect,
) -> Result<(), UiError> {
    tree.set_label_text(ids.toolbar_title, project.toolbar_title.clone())?;
    tree.set_label_text(ids.project_name, project.project_name.clone())?;
    tree.set_label_text(ids.project_path, project.project_path.clone())?;
    tree.set_label_text(ids.project_status, project.project_status.clone())?;
    tree.set_label_text(ids.project_recent, project.project_recent.clone())?;
    tree.set_label_text(ids.project_diagnostics, project.project_diagnostics.clone())?;
    tree.set_label_text(ids.project_command, project.project_command.clone())?;
    tree.set_text_role(
        ids.project_diagnostics,
        text_role_for_diagnostic_tone(project.diagnostic_tone),
    )?;
    let status = editor_status_bar(project, assets, scene);
    apply_asset_snapshot(tree, ids, assets, &status, bounds)?;
    apply_scene_snapshot(tree, ids, scene, &status, bounds)?;
    apply_inspector_snapshot(tree, ids, inspector, bounds)?;
    Ok(())
}

fn text_role_for_diagnostic_tone(tone: DiagnosticTone) -> TextRole {
    match tone {
        DiagnosticTone::Neutral => TextRole::Muted,
        DiagnosticTone::Success => TextRole::Success,
        DiagnosticTone::Warning => TextRole::Warning,
        DiagnosticTone::Danger => TextRole::Danger,
    }
}
