use elcarax_render::Rect;
use elcarax_ui::{EditorShellIds, LayoutConstraints, ScrollViewState, UiError, UiTree};

use crate::asset_display::AssetUiSnapshot;

pub(crate) fn apply_asset_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &AssetUiSnapshot,
    status: &str,
    bounds: Rect,
) -> Result<(), UiError> {
    tree.set_label_text(
        ids.asset_section_title,
        snapshot.asset_section_title.clone(),
    )?;
    tree.set_label_text(ids.asset_count, snapshot.asset_count.clone())?;
    tree.set_scroll_view_state(
        ids.asset_scroll_view,
        ScrollViewState::with_metrics(
            snapshot.scroll_offset,
            snapshot.visible_rows,
            snapshot.total_rows,
        ),
    )?;
    for (index, row_id) in ids.asset_rows.iter().enumerate() {
        tree.set_button_text(*row_id, snapshot.asset_row_labels[index].clone())?;
    }
    tree.set_label_text(
        ids.asset_selected_summary,
        snapshot.asset_selected_summary.clone(),
    )?;
    tree.set_label_text(ids.status_label, status.to_string())?;
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
