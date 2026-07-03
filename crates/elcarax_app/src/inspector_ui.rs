use elcarax_render::Rect;
use elcarax_ui::{
    EditorShellIds, INSPECTOR_EDITABLE_ROW_HEIGHT, INSPECTOR_READONLY_ROW_HEIGHT, TextRole,
    UiError, UiTree,
};

use crate::inspector_display::InspectorUiSnapshot;

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
            INSPECTOR_READONLY_ROW_HEIGHT,
        )?;
        tree.set_sized_label_text(
            ids.inspector_object_kind,
            String::new(),
            INSPECTOR_READONLY_ROW_HEIGHT,
        )?;
        tree.set_sized_label_text(
            ids.inspector_empty_message,
            snapshot.empty_message.clone(),
            INSPECTOR_READONLY_ROW_HEIGHT,
        )?;
    }
    for (index, row_id) in ids.inspector_row_labels.iter().enumerate() {
        let row_height = if snapshot.row_editable[index] {
            INSPECTOR_EDITABLE_ROW_HEIGHT
        } else {
            INSPECTOR_READONLY_ROW_HEIGHT
        };
        tree.set_sized_label_text(*row_id, snapshot.row_labels[index].clone(), row_height)?;
    }
    for (index, value_id) in ids.inspector_row_values.iter().enumerate() {
        if snapshot.row_editable[index] {
            tree.set_text_field(*value_id, snapshot.row_values[index].clone())?;
            tree.set_text_role(*value_id, TextRole::Accent)?;
        } else {
            tree.set_sized_label_text(
                *value_id,
                snapshot.row_values[index].clone(),
                INSPECTOR_READONLY_ROW_HEIGHT,
            )?;
            tree.set_text_role(*value_id, TextRole::Muted)?;
        }
    }
    tree.set_sized_label_text(
        ids.inspector_summary,
        snapshot.summary.clone(),
        INSPECTOR_READONLY_ROW_HEIGHT,
    )?;
    tree.layout(elcarax_ui::LayoutConstraints { bounds })?;
    Ok(())
}

#[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
pub(crate) fn inspector_value_index_for_widget(
    ids: EditorShellIds,
    widget_id: elcarax_ui::WidgetId,
) -> Option<usize> {
    ids.inspector_row_values
        .iter()
        .position(|value_id| *value_id == widget_id)
}
