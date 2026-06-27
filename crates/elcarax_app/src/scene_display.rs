use elcarax_scene_model::{
    SceneDiagnostic, SceneExpansion, SceneHierarchy, SceneObjectId, SceneSelection, SceneSnapshot,
    SceneTreeRow,
};
use elcarax_ui::MAX_VISIBLE_SCENE_ROWS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneUiSnapshot {
    pub(crate) scene_section_title: String,
    pub(crate) scene_name: String,
    pub(crate) scene_expand_labels: [String; MAX_VISIBLE_SCENE_ROWS],
    pub(crate) scene_row_labels: [String; MAX_VISIBLE_SCENE_ROWS],
    pub(crate) scene_selected_summary: String,
    pub(crate) selected_row_index: Option<usize>,
    pub(crate) visible_object_ids: [Option<SceneObjectId>; MAX_VISIBLE_SCENE_ROWS],
    pub(crate) status_scene_suffix: String,
}

pub(crate) fn scene_ui_snapshot(
    snapshot: Option<&SceneSnapshot>,
    selection: &SceneSelection,
    expansion: &SceneExpansion,
    diagnostics: &[SceneDiagnostic],
    last_command_message: Option<&str>,
) -> SceneUiSnapshot {
    let mut scene_expand_labels = empty_labels();
    let mut scene_row_labels = empty_labels();
    let mut visible_object_ids = empty_object_ids();
    let visible_rows = snapshot
        .map(|scene| SceneHierarchy::visible_rows(scene, expansion))
        .unwrap_or_default();
    for (index, row) in visible_rows.iter().take(MAX_VISIBLE_SCENE_ROWS).enumerate() {
        scene_expand_labels[index] = row.expand_marker();
        scene_row_labels[index] = row.name_label();
        visible_object_ids[index] = Some(row.object_id);
    }
    let selected_row_index = selection
        .selected()
        .and_then(|id| row_index_for_object(&visible_rows, id));
    SceneUiSnapshot {
        scene_section_title: "Scene".to_string(),
        scene_name: snapshot
            .map(|scene| scene.name().as_str().to_string())
            .unwrap_or_else(|| "No scene".to_string()),
        scene_expand_labels,
        scene_row_labels,
        scene_selected_summary: selected_object_summary(snapshot, selection),
        selected_row_index,
        visible_object_ids,
        status_scene_suffix: status_scene_suffix(
            snapshot,
            selection,
            diagnostics,
            last_command_message,
        ),
    }
}

fn empty_labels() -> [String; MAX_VISIBLE_SCENE_ROWS] {
    std::array::from_fn(|_| String::new())
}

fn empty_object_ids() -> [Option<SceneObjectId>; MAX_VISIBLE_SCENE_ROWS] {
    std::array::from_fn(|_| None)
}

fn row_index_for_object(rows: &[SceneTreeRow], id: SceneObjectId) -> Option<usize> {
    rows.iter()
        .take(MAX_VISIBLE_SCENE_ROWS)
        .position(|row| row.object_id == id)
}

fn selected_object_summary(snapshot: Option<&SceneSnapshot>, selection: &SceneSelection) -> String {
    let Some(snapshot) = snapshot else {
        return "Selected: None".to_string();
    };
    let Some(id) = selection.selected() else {
        return "Selected: None".to_string();
    };
    let Ok(object) = snapshot.object(id) else {
        return "Selected: None".to_string();
    };
    format!(
        "Selected: {} ({})",
        object.display_name,
        object.kind.label()
    )
}

fn status_scene_suffix(
    snapshot: Option<&SceneSnapshot>,
    selection: &SceneSelection,
    diagnostics: &[SceneDiagnostic],
    last_command_message: Option<&str>,
) -> String {
    if let Some(message) = last_command_message {
        return format!("Scene: {message}");
    }
    if let Some(diagnostic) = diagnostics.first() {
        return format!("Scene: {}", diagnostic.summary());
    }
    let scene_name = snapshot
        .map(|scene| scene.name().as_str().to_string())
        .unwrap_or_else(|| "None".to_string());
    let object = match selection
        .selected()
        .and_then(|id| snapshot.and_then(|scene| scene.object(id).ok()))
    {
        Some(object) => object.display_name.clone(),
        None => "None".to_string(),
    };
    format!("Scene: {scene_name} | Object: {object}")
}

pub(crate) fn selected_object_label(snapshot: &SceneUiSnapshot) -> String {
    if snapshot.scene_selected_summary == "Selected: None" {
        return "None".to_string();
    }
    match snapshot
        .scene_selected_summary
        .strip_prefix("Selected: ")
        .and_then(|value| value.split(" (").next())
    {
        Some(label) => label.to_string(),
        None => "None".to_string(),
    }
}
