use elcarax_scene_model::{InspectorDiagnostic, InspectorObject, build_inspector_for_selection};
use elcarax_ui::MAX_VISIBLE_INSPECTOR_ROWS;

use crate::scene_state::SceneState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectorUiSnapshot {
    pub(crate) has_selection: bool,
    pub(crate) empty_message: String,
    pub(crate) object_name: String,
    pub(crate) object_kind: String,
    pub(crate) row_labels: [String; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_values: [String; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_editable: [bool; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) row_command_ids: [String; MAX_VISIBLE_INSPECTOR_ROWS],
    pub(crate) property_count: usize,
    pub(crate) summary: String,
}

pub(crate) fn inspector_ui_snapshot(
    scene: &SceneState,
    suppressed: bool,
    last_command_message: Option<&str>,
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
        Ok(value) => build_selected_snapshot(value),
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

fn build_selected_snapshot(inspector: InspectorObject) -> InspectorUiSnapshot {
    let mut row_labels = empty_rows();
    let mut row_values = empty_rows();
    let mut row_editable = empty_editable_rows();
    let mut row_command_ids = empty_rows();
    let mut index = 0usize;
    for section in &inspector.sections {
        if index >= MAX_VISIBLE_INSPECTOR_ROWS {
            break;
        }
        row_labels[index] = section.title.as_str().to_string();
        row_values[index].clear();
        index += 1;
        for row in &section.rows {
            if index >= MAX_VISIBLE_INSPECTOR_ROWS {
                break;
            }
            row_labels[index] = row.label.as_str().to_string();
            row_values[index] = inspector_value_label(row);
            row_editable[index] = row.editable;
            row_command_ids[index] = inspector_command_for_row(row);
            index += 1;
        }
    }
    let property_count = inspector.property_count();
    InspectorUiSnapshot {
        has_selection: true,
        empty_message: String::new(),
        object_name: inspector.name.clone(),
        object_kind: format!("Kind: {}", inspector.kind.label()),
        row_labels,
        row_values,
        row_editable,
        row_command_ids,
        property_count,
        summary: String::new(),
    }
}

fn empty_snapshot_with_message(message: &str) -> InspectorUiSnapshot {
    InspectorUiSnapshot {
        has_selection: false,
        empty_message: message.to_string(),
        object_name: String::new(),
        object_kind: String::new(),
        row_labels: empty_rows(),
        row_values: empty_rows(),
        row_editable: empty_editable_rows(),
        row_command_ids: empty_rows(),
        property_count: 0,
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
        row_editable: empty_editable_rows(),
        row_command_ids: empty_rows(),
        property_count: 0,
        summary,
    }
}

fn empty_rows() -> [String; MAX_VISIBLE_INSPECTOR_ROWS] {
    std::array::from_fn(|_| String::new())
}

fn empty_editable_rows() -> [bool; MAX_VISIBLE_INSPECTOR_ROWS] {
    [false; MAX_VISIBLE_INSPECTOR_ROWS]
}

fn inspector_value_label(row: &elcarax_scene_model::InspectorRow) -> String {
    if row.editable {
        return format!("{}  [Set]", row.value);
    }
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
        let snapshot = inspector_ui_snapshot(&scene, false, None);
        assert!(snapshot.has_selection);
        assert_eq!(snapshot.object_name, "Fixture Actor");
        assert!(snapshot.row_labels.iter().any(|label| label == "Health"));
        assert!(
            snapshot
                .row_values
                .iter()
                .any(|value| value == "100  [Set]")
        );
        assert!(snapshot.row_editable.iter().any(|editable| *editable));
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
