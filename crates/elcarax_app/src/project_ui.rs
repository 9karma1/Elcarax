use elcarax_render::Rect;
use elcarax_ui::{EditorShellContent, EditorShellIds, TextRole, UiError, UiTree};

use crate::adapter_display::AdapterUiSnapshot;
use crate::asset_display::AssetUiSnapshot;
use crate::asset_ui::apply_asset_snapshot;
use crate::editor_status::editor_status_bar;
use crate::inspector_display::InspectorUiSnapshot;
use crate::inspector_ui::apply_inspector_snapshot;
use crate::project_display::{DiagnosticTone, ProjectUiSnapshot};
use crate::scene_display::SceneUiSnapshot;
use crate::scene_ui::apply_scene_snapshot;

pub(crate) use crate::scene_ui::shell_content_from_editor_state;

#[allow(dead_code)]
pub(crate) fn shell_content_from_project(snapshot: &ProjectUiSnapshot) -> EditorShellContent {
    shell_content_from_editor_state(editor_snapshots(
        snapshot,
        &empty_asset_snapshot(),
        &empty_scene_snapshot(),
        &empty_inspector_snapshot(),
        &empty_adapter_snapshot(),
    ))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn empty_inspector_snapshot() -> InspectorUiSnapshot {
    InspectorUiSnapshot {
        has_selection: false,
        empty_message: "No object selected".to_string(),
        object_name: String::new(),
        object_kind: String::new(),
        row_labels: std::array::from_fn(|_| String::new()),
        row_values: std::array::from_fn(|_| String::new()),
        row_editable: [false; elcarax_ui::MAX_VISIBLE_INSPECTOR_ROWS],
        row_command_ids: std::array::from_fn(|_| String::new()),
        property_count: 0,
        summary: String::new(),
    }
}

#[allow(dead_code)]
fn empty_adapter_snapshot() -> AdapterUiSnapshot {
    AdapterUiSnapshot {
        adapter_status: "Adapter: Disconnected".to_string(),
        adapter_diagnostics: "Adapter Diagnostics: 0".to_string(),
        adapter_command: "Adapter Command: None".to_string(),
        status_adapter_suffix: "Adapter: Disconnected".to_string(),
    }
}

#[allow(dead_code)]
pub(crate) fn apply_project_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &ProjectUiSnapshot,
    bounds: Rect,
) -> Result<(), UiError> {
    apply_editor_snapshot(
        tree,
        ids,
        editor_snapshots(
            snapshot,
            &empty_asset_snapshot(),
            &empty_scene_snapshot(),
            &empty_inspector_snapshot(),
            &empty_adapter_snapshot(),
        ),
        bounds,
    )
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EditorSnapshotRefs<'a> {
    pub(crate) project: &'a ProjectUiSnapshot,
    pub(crate) assets: &'a AssetUiSnapshot,
    pub(crate) scene: &'a SceneUiSnapshot,
    pub(crate) inspector: &'a InspectorUiSnapshot,
    pub(crate) adapter: &'a AdapterUiSnapshot,
}

pub(crate) const fn editor_snapshots<'a>(
    project: &'a ProjectUiSnapshot,
    assets: &'a AssetUiSnapshot,
    scene: &'a SceneUiSnapshot,
    inspector: &'a InspectorUiSnapshot,
    adapter: &'a AdapterUiSnapshot,
) -> EditorSnapshotRefs<'a> {
    EditorSnapshotRefs {
        project,
        assets,
        scene,
        inspector,
        adapter,
    }
}

pub(crate) fn apply_editor_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshots: EditorSnapshotRefs<'_>,
    bounds: Rect,
) -> Result<(), UiError> {
    let project = snapshots.project;
    let assets = snapshots.assets;
    let scene = snapshots.scene;
    let inspector = snapshots.inspector;
    let adapter = snapshots.adapter;
    tree.set_label_text(ids.toolbar_title, project.toolbar_title.clone())?;
    tree.set_label_text(ids.project_name, project.project_name.clone())?;
    tree.set_label_text(ids.project_path, project.project_path.clone())?;
    tree.set_label_text(ids.project_status, project.project_status.clone())?;
    tree.set_label_text(ids.project_recent, project.project_recent.clone())?;
    tree.set_label_text(ids.project_diagnostics, project.project_diagnostics.clone())?;
    tree.set_label_text(ids.project_command, project.project_command.clone())?;
    tree.set_label_text(ids.adapter_status, adapter.adapter_status.clone())?;
    tree.set_label_text(ids.adapter_diagnostics, adapter.adapter_diagnostics.clone())?;
    tree.set_label_text(ids.adapter_command, adapter.adapter_command.clone())?;
    tree.set_text_role(
        ids.project_diagnostics,
        text_role_for_diagnostic_tone(project.diagnostic_tone),
    )?;
    let status = editor_status_bar(project, assets, scene, adapter);
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
