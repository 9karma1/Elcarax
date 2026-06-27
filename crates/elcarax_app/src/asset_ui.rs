use elcarax_render::Rect;
use elcarax_ui::{EditorShellContent, EditorShellIds, LayoutConstraints, UiError, UiTree};

use crate::asset_display::AssetUiSnapshot;
use crate::project_display::ProjectUiSnapshot;

pub(crate) fn shell_content_from_editor_state(
    project: &ProjectUiSnapshot,
    assets: &AssetUiSnapshot,
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
        status: combine_status(&project.status, &assets.status_asset_suffix),
    }
}

pub(crate) fn apply_asset_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &AssetUiSnapshot,
    project_status: &str,
    bounds: Rect,
) -> Result<(), UiError> {
    tree.set_label_text(
        ids.asset_section_title,
        snapshot.asset_section_title.clone(),
    )?;
    tree.set_label_text(ids.asset_count, snapshot.asset_count.clone())?;
    for (index, row_id) in ids.asset_rows.iter().enumerate() {
        tree.set_button_text(*row_id, snapshot.asset_row_labels[index].clone())?;
    }
    let focused_row = snapshot
        .selected_row_index
        .map(|index| ids.asset_rows[index]);
    tree.set_focused(focused_row)?;
    tree.set_label_text(
        ids.asset_selected_summary,
        snapshot.asset_selected_summary.clone(),
    )?;
    tree.set_label_text(
        ids.status_label,
        combine_status(project_status, &snapshot.status_asset_suffix),
    )?;
    tree.layout(LayoutConstraints { bounds })?;
    Ok(())
}

#[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
pub(crate) fn asset_row_index_for_widget(
    ids: EditorShellIds,
    widget_id: elcarax_ui::WidgetId,
) -> Option<usize> {
    ids.asset_rows
        .iter()
        .position(|row_id| *row_id == widget_id)
}

fn combine_status(project_status: &str, asset_suffix: &str) -> String {
    if asset_suffix.is_empty() {
        return project_status.to_string();
    }
    format!("{project_status} | {asset_suffix}")
}
