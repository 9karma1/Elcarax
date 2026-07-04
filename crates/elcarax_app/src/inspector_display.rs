use elcarax_scene_model::{
    InspectorDiagnostic, InspectorObject, InspectorValueWidget, PropertyEditKind,
    build_inspector_for_selection, inspector_value_widget_for,
};
use elcarax_ui::MAX_VISIBLE_INSPECTOR_ROWS;

use crate::scene_state::SceneState;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InspectorUiSnapshot {
    pub(crate) has_selection: bool,
    pub(crate) empty_message: String,
    pub(crate) object_name: String,
    pub(crate) object_kind: String,
    pub(crate) row_labels: [String; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_values: [String; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_widgets: [InspectorValueWidget; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_editable: [bool; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_property_paths: [String; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_edit_kinds: [PropertyEditKind; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_command_ids: [String; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) property_count: usize,
    pub(crate) scroll_offset: usize,
    pub(crate) total_rows: usize,
    pub(crate) visible_rows: usize,
    pub(crate) summary: String,
}

pub(crate) fn inspector_ui_snapshot_with_scroll(
    scene: &SceneState,
    suppressed: bool,
    last_command_message: Option<&str>,
    scroll_offset: usize,
) -> InspectorUiSnapshot {
    if suppressed {
        let summary = last_command_message
            .map(ToString::to_string)
            .unwrap_or_else(|| "Inspector cleared".to_string());
        return empty_snapshot_with_summary(summary);
    }
    let Some(snapshot) = scene.snapshot() else {
        let summary = last_command_message
            .map(ToString::to_string)
            .unwrap_or_else(|| InspectorDiagnostic::NoSceneLoaded.message().to_string());
        return empty_snapshot_with_summary(summary);
    };
    let selected = scene.selection().selected();
    let mut view = match build_inspector_for_selection(snapshot, selected) {
        Ok(value) => build_selected_snapshot_with_scroll(value, snapshot, scroll_offset),
        Err(InspectorDiagnostic::NoObjectSelected) => {
            return empty_snapshot_with_message("No object selected");
        }
        Err(diagnostic) => {
            return empty_snapshot_with_summary(diagnostic.message().to_string());
        }
    };
    if let Some(message) = last_command_message {
        view.summary = message.to_string();
    }
    view
}

pub(crate) fn inspector_summary_for_object(inspector: &InspectorObject) -> String {
    format!(
        "{} ({}) | {} properties",
        inspector.name,
        inspector.kind.label(),
        inspector.property_count()
    )
}

fn build_selected_snapshot_with_scroll(
    inspector: InspectorObject,
    snapshot: &elcarax_scene_model::SceneSnapshot,
    scroll_offset: usize,
) -> InspectorUiSnapshot {
    let mut row_labels = empty_rows();
    let mut row_values = empty_rows();
    let mut row_widgets = empty_widgets();
    let mut row_editable = empty_editable_rows();
    let mut row_property_paths = empty_rows();
    let mut row_edit_kinds = empty_edit_kinds();
    let mut row_command_ids = empty_rows();
    let rows = inspector_rows(&inspector, snapshot);
    let total_rows = rows.len();
    let scroll_offset = clamp_scroll_offset(scroll_offset, total_rows, MAX_VISIBLE_INSPECTOR_ROWS);
    for (index, row) in rows
        .iter()
        .skip(scroll_offset)
        .take(MAX_VISIBLE_INSPECTOR_ROWS)
        .enumerate()
    {
        row_labels[index] = row.label.clone();
        row_values[index] = row.value.clone();
        row_widgets[index] = row.widget.clone();
        row_editable[index] = row.editable;
        row_property_paths[index] = row.property_path.clone();
        row_edit_kinds[index] = row.edit_kind;
        row_command_ids[index] = row.command_id.clone();
    }
    let property_count = inspector.property_count();
    InspectorUiSnapshot {
        has_selection: true,
        empty_message: String::new(),
        object_name: inspector.name.clone(),
        object_kind: format!("Kind: {}", inspector.kind.label()),
        row_labels,
        row_values,
        row_widgets,
        row_editable,
        row_property_paths,
        row_edit_kinds,
        row_command_ids,
        property_count,
        scroll_offset,
        total_rows,
        visible_rows: MAX_VISIBLE_INSPECTOR_ROWS,
        summary: String::new(),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct InspectorDisplayRow {
    label: String,
    value: String,
    widget: InspectorValueWidget,
    editable: bool,
    property_path: String,
    edit_kind: PropertyEditKind,
    command_id: String,
}

fn inspector_rows(
    inspector: &InspectorObject,
    snapshot: &elcarax_scene_model::SceneSnapshot,
) -> Vec<InspectorDisplayRow> {
    let object = match snapshot.object(inspector.object_id) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let _schema = snapshot.schema(object.type_id);
    let mut rows = Vec::new();
    for section in &inspector.sections {
        rows.push(InspectorDisplayRow {
            label: section.title.as_str().to_string(),
            value: String::new(),
            widget: InspectorValueWidget::Hidden,
            editable: false,
            property_path: String::new(),
            edit_kind: PropertyEditKind::Unsupported,
            command_id: String::new(),
        });
        for row in &section.rows {
            let widget = widget_for_row(snapshot, object.type_id, row, object);
            rows.push(InspectorDisplayRow {
                label: row.label.as_str().to_string(),
                value: if row.editable {
                    row.value.clone()
                } else {
                    read_only_value_label(row)
                },
                widget,
                editable: row.editable,
                property_path: row.path.to_string(),
                edit_kind: row.edit_kind,
                command_id: inspector_command_for_row(row),
            });
        }
    }
    rows
}

fn widget_for_row(
    snapshot: &elcarax_scene_model::SceneSnapshot,
    type_id: elcarax_scene_model::ObjectTypeId,
    row: &elcarax_scene_model::InspectorRow,
    object: &elcarax_scene_model::SceneObject,
) -> InspectorValueWidget {
    if !row.editable {
        return InspectorValueWidget::ReadOnly(read_only_value_label(row));
    }
    let Some(schema) = snapshot.schema(type_id) else {
        return InspectorValueWidget::Text(row.value.clone());
    };
    let Some(property_schema) = schema
        .properties
        .iter()
        .find(|property| property.path == row.path)
    else {
        return InspectorValueWidget::Text(row.value.clone());
    };
    let Some(value) = object.property(&row.path) else {
        return InspectorValueWidget::Text(row.value.clone());
    };
    inspector_value_widget_for(property_schema, value)
}

fn empty_snapshot_with_message(message: &str) -> InspectorUiSnapshot {
    InspectorUiSnapshot {
        has_selection: false,
        empty_message: message.to_string(),
        object_name: String::new(),
        object_kind: String::new(),
        row_labels: empty_rows(),
        row_values: empty_rows(),
        row_widgets: empty_widgets(),
        row_editable: empty_editable_rows(),
        row_property_paths: empty_rows(),
        row_edit_kinds: empty_edit_kinds(),
        row_command_ids: empty_rows(),
        property_count: 0,
        scroll_offset: 0,
        total_rows: 0,
        visible_rows: MAX_VISIBLE_INSPECTOR_ROWS,
        summary: message.to_string(),
    }
}

fn empty_snapshot_with_summary(summary: String) -> InspectorUiSnapshot {
    InspectorUiSnapshot {
        has_selection: false,
        empty_message: "No object selected".to_string(),
        object_name: String::new(),
        object_kind: String::new(),
        row_labels: empty_rows(),
        row_values: empty_rows(),
        row_widgets: empty_widgets(),
        row_editable: empty_editable_rows(),
        row_property_paths: empty_rows(),
        row_edit_kinds: empty_edit_kinds(),
        row_command_ids: empty_rows(),
        property_count: 0,
        scroll_offset: 0,
        total_rows: 0,
        visible_rows: MAX_VISIBLE_INSPECTOR_ROWS,
        summary,
    }
}

fn clamp_scroll_offset(scroll_offset: usize, total_rows: usize, visible_rows: usize) -> usize {
    scroll_offset.min(total_rows.saturating_sub(visible_rows))
}

fn empty_rows() -> [String; MAX_VISIBLE_INSPECTOR_ROWS] {
    std::array::from_fn(|_| String::new())
}

fn empty_widgets() -> [InspectorValueWidget; MAX_VISIBLE_INSPECTOR_ROWS] {
    std::array::from_fn(|_| InspectorValueWidget::Hidden)
}

fn empty_editable_rows() -> [bool; MAX_VISIBLE_INSPECTOR_ROWS] {
    [false; MAX_VISIBLE_INSPECTOR_ROWS]
}

fn empty_edit_kinds() -> [PropertyEditKind; MAX_VISIBLE_INSPECTOR_ROWS] {
    [PropertyEditKind::Unsupported; MAX_VISIBLE_INSPECTOR_ROWS]
}

fn read_only_value_label(row: &elcarax_scene_model::InspectorRow) -> String {
    match &row.read_only_reason {
        Some(reason) => format!("{}  [Read-only: {}]", row.value, reason),
        None => format!("{}  [Read-only]", row.value),
    }
}

fn inspector_command_for_row(_row: &elcarax_scene_model::InspectorRow) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_state::SceneState;
    use elcarax_scene_model::{
        ObjectSchema, PropertyGroup, PropertyKind, PropertyPath, PropertySchema, PropertyValue,
        SceneName, SceneObject, SceneObjectKind, SceneSnapshot,
    };

    #[test]
    fn selected_fixture_snapshot_contains_property_labels() {
        let scene = selected_fixture_scene();
        let snapshot = inspector_ui_snapshot_with_scroll(&scene, false, None, 0);
        assert!(snapshot.has_selection);
        assert_eq!(snapshot.object_name, "Fixture Actor");
        assert!(snapshot.row_labels.iter().any(|label| label == "Health"));
        assert!(snapshot.row_values.iter().any(|value| value == "100"));
        assert!(snapshot.row_editable.iter().any(|editable| *editable));
        assert!(snapshot
            .row_widgets
            .iter()
            .any(|widget| matches!(widget, InspectorValueWidget::Number { .. })));
    }

    fn selected_fixture_scene() -> SceneState {
        let health_path = fixture_path("gameplay.health");
        let schema = ObjectSchema::new("Actor").with_property(PropertySchema::editable(
            health_path.clone(),
            "Health",
            PropertyKind::I64,
            PropertyGroup::new("Gameplay"),
        ));
        let mut object =
            SceneObject::new("Fixture Actor", SceneObjectKind::Character, schema.type_id);
        object.set_property(health_path, PropertyValue::I64(100));
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Fixture Scene"));
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
        let mut scene = SceneState::default();
        scene.load_fixture_snapshot(snapshot);
        assert!(scene.select_object(object_id));
        scene
    }

    fn fixture_path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("fixture path should parse: {error}"),
        }
    }
}
