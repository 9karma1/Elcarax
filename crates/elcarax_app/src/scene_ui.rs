use elcarax_render::Rect;
use elcarax_ui::{EditorShellContent, EditorShellIds, LayoutConstraints, UiError, UiTree};

use crate::asset_display::AssetUiSnapshot;
use crate::editor_status::editor_status_bar;
use crate::inspector_display::InspectorUiSnapshot;
use crate::project_display::ProjectUiSnapshot;
use crate::scene_display::SceneUiSnapshot;

pub(crate) fn shell_content_from_editor_state(
    project: &ProjectUiSnapshot,
    assets: &AssetUiSnapshot,
    scene: &SceneUiSnapshot,
    inspector: &InspectorUiSnapshot,
) -> EditorShellContent {
    EditorShellContent {
        toolbar_title: project.toolbar_title.clone(),
        project_title: "Project".to_string(),
        project_name: project.project_name.clone(),
        project_path: project.project_path.clone(),
        project_status: project.project_status.clone(),
        project_recent: project.project_recent.clone(),
        project_diagnostics: project.project_diagnostics.clone(),
        project_command: project.project_command.clone(),
        asset_section_title: assets.asset_section_title.clone(),
        asset_count: assets.asset_count.clone(),
        asset_row_labels: assets.asset_row_labels.clone(),
        asset_selected_summary: assets.asset_selected_summary.clone(),
        scene_section_title: scene.scene_section_title.clone(),
        scene_name: scene.scene_name.clone(),
        scene_expand_labels: scene.scene_expand_labels.clone(),
        scene_row_labels: scene.scene_row_labels.clone(),
        scene_selected_summary: scene.scene_selected_summary.clone(),
        inspector_object_name: inspector.object_name.clone(),
        inspector_object_kind: inspector.object_kind.clone(),
        inspector_empty_message: if inspector.has_selection {
            String::new()
        } else {
            inspector.empty_message.clone()
        },
        inspector_row_labels: inspector.row_labels.clone(),
        inspector_row_values: inspector.row_values.clone(),
        inspector_summary: inspector.summary.clone(),
        status: editor_status_bar(project, assets, scene),
    }
}

pub(crate) fn apply_scene_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &SceneUiSnapshot,
    status: &str,
    bounds: Rect,
) -> Result<(), UiError> {
    tree.set_label_text(
        ids.scene_section_title,
        snapshot.scene_section_title.clone(),
    )?;
    tree.set_label_text(ids.scene_name, snapshot.scene_name.clone())?;
    for (index, expand_id) in ids.scene_expand_rows.iter().enumerate() {
        tree.set_icon_button_text(*expand_id, snapshot.scene_expand_labels[index].clone())?;
    }
    for (index, row_id) in ids.scene_rows.iter().enumerate() {
        tree.set_button_text(*row_id, snapshot.scene_row_labels[index].clone())?;
    }
    let focused_row = snapshot
        .selected_row_index
        .map(|index| ids.scene_rows[index]);
    tree.set_focused(focused_row)?;
    tree.set_label_text(
        ids.scene_selected_summary,
        snapshot.scene_selected_summary.clone(),
    )?;
    tree.set_label_text(ids.status_label, status.to_string())?;
    tree.layout(LayoutConstraints { bounds })?;
    Ok(())
}

#[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
pub(crate) fn scene_row_index_for_widget(
    ids: EditorShellIds,
    widget_id: elcarax_ui::WidgetId,
) -> Option<usize> {
    ids.scene_rows
        .iter()
        .position(|row_id| *row_id == widget_id)
}

#[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
pub(crate) fn scene_expand_index_for_widget(
    ids: EditorShellIds,
    widget_id: elcarax_ui::WidgetId,
) -> Option<usize> {
    ids.scene_expand_rows
        .iter()
        .position(|expand_id| *expand_id == widget_id)
}
