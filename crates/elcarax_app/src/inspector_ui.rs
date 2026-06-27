use elcarax_render::Rect;
use elcarax_ui::{EditorShellIds, UiError, UiTree};

use crate::inspector_display::InspectorUiSnapshot;

const INSPECTOR_ROW_HEIGHT: f32 = 20.0;

pub(crate) fn apply_inspector_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &InspectorUiSnapshot,
    bounds: Rect,
) -> Result<(), UiError> {
    if snapshot.has_selection {
        tree.set_label_text(ids.inspector_empty_message, String::new())?;
        tree.set_label_text(ids.inspector_object_name, snapshot.object_name.clone())?;
        tree.set_label_text(ids.inspector_object_kind, snapshot.object_kind.clone())?;
    } else {
        tree.set_sized_label_text(
            ids.inspector_object_name,
            String::new(),
            INSPECTOR_ROW_HEIGHT,
        )?;
        tree.set_sized_label_text(
            ids.inspector_object_kind,
            String::new(),
            INSPECTOR_ROW_HEIGHT,
        )?;
        tree.set_sized_label_text(
            ids.inspector_empty_message,
            snapshot.empty_message.clone(),
            INSPECTOR_ROW_HEIGHT,
        )?;
    }
    for (index, row_id) in ids.inspector_row_labels.iter().enumerate() {
        tree.set_sized_label_text(
            *row_id,
            snapshot.row_labels[index].clone(),
            INSPECTOR_ROW_HEIGHT,
        )?;
    }
    for (index, value_id) in ids.inspector_row_values.iter().enumerate() {
        tree.set_sized_label_text(
            *value_id,
            snapshot.row_values[index].clone(),
            INSPECTOR_ROW_HEIGHT,
        )?;
    }
    tree.set_sized_label_text(
        ids.inspector_summary,
        snapshot.summary.clone(),
        INSPECTOR_ROW_HEIGHT,
    )?;
    tree.layout(elcarax_ui::LayoutConstraints { bounds })?;
    Ok(())
}
