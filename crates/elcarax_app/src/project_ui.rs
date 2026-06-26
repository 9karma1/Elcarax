use elcarax_render::Rect;
use elcarax_ui::{
    EditorShellContent, EditorShellIds, LayoutConstraints, TextRole, UiError, UiTree,
};

use crate::project_display::{DiagnosticTone, ProjectUiSnapshot};

pub(crate) fn shell_content_from_project(snapshot: &ProjectUiSnapshot) -> EditorShellContent {
    EditorShellContent {
        toolbar_title: snapshot.toolbar_title.clone(),
        project_name: snapshot.project_name.clone(),
        project_path: snapshot.project_path.clone(),
        project_status: snapshot.project_status.clone(),
        project_recent: snapshot.project_recent.clone(),
        project_diagnostics: snapshot.project_diagnostics.clone(),
        project_command: snapshot.project_command.clone(),
        status: snapshot.status.clone(),
        ..EditorShellContent::default()
    }
}

pub(crate) fn apply_project_snapshot(
    tree: &mut UiTree,
    ids: EditorShellIds,
    snapshot: &ProjectUiSnapshot,
    bounds: Rect,
) -> Result<(), UiError> {
    tree.set_label_text(ids.toolbar_title, snapshot.toolbar_title.clone())?;
    tree.set_label_text(ids.project_name, snapshot.project_name.clone())?;
    tree.set_label_text(ids.project_path, snapshot.project_path.clone())?;
    tree.set_label_text(ids.project_status, snapshot.project_status.clone())?;
    tree.set_label_text(ids.project_recent, snapshot.project_recent.clone())?;
    tree.set_label_text(
        ids.project_diagnostics,
        snapshot.project_diagnostics.clone(),
    )?;
    tree.set_label_text(ids.project_command, snapshot.project_command.clone())?;
    tree.set_label_text(ids.status_label, snapshot.status.clone())?;
    tree.set_text_role(
        ids.project_diagnostics,
        text_role_for_diagnostic_tone(snapshot.diagnostic_tone),
    )?;
    tree.layout(LayoutConstraints { bounds })?;
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
