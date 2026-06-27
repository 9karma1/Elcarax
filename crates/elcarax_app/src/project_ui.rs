use elcarax_render::Rect;
use elcarax_ui::{EditorShellContent, EditorShellIds, TextRole, UiError, UiTree};

use crate::asset_display::AssetUiSnapshot;
use crate::asset_ui::apply_asset_snapshot;
use crate::project_display::{DiagnosticTone, ProjectUiSnapshot};

pub(crate) use crate::asset_ui::shell_content_from_editor_state;

#[cfg_attr(feature = "native-shell", allow(dead_code))]
pub(crate) fn shell_content_from_project(snapshot: &ProjectUiSnapshot) -> EditorShellContent {
    shell_content_from_editor_state(snapshot, &empty_asset_snapshot())
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
pub(crate) fn apply_project_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &ProjectUiSnapshot,
    bounds: Rect,
) -> Result<(), UiError> {
    apply_editor_snapshot(tree, ids, snapshot, &empty_asset_snapshot(), bounds)
}

pub(crate) fn apply_editor_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    project: &ProjectUiSnapshot,
    assets: &AssetUiSnapshot,
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
    apply_asset_snapshot(tree, ids, assets, &project.status, bounds)?;
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
