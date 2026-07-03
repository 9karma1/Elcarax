use elcarax_assets::{
    AssetDiagnostic, AssetId, AssetIndex, AssetRecord, AssetSelection, AssetWatchStatus,
};
use elcarax_ui::MAX_VISIBLE_ASSET_ROWS;

use crate::asset_state::{AssetScanStatus, AssetUiState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetUiSnapshot {
    pub(crate) asset_section_title: String,
    pub(crate) asset_count: String,
    pub(crate) asset_row_labels: [String; MAX_VISIBLE_ASSET_ROWS],
    pub(crate) asset_selected_summary: String,
    pub(crate) selected_row_index: Option<usize>,
    pub(crate) status_asset_suffix: String,
}

pub(crate) fn asset_ui_snapshot(
    index: &AssetIndex,
    selection: &AssetSelection,
    state: AssetUiState<'_>,
) -> AssetUiSnapshot {
    let mut asset_row_labels = empty_row_labels();
    let records: Vec<_> = index
        .records()
        .iter()
        .take(MAX_VISIBLE_ASSET_ROWS)
        .collect();
    for (index, record) in records.iter().enumerate() {
        asset_row_labels[index] = asset_row_label(record);
    }
    let selected_row_index = selection
        .selected()
        .and_then(|id| row_index_for_asset(index, id));
    let selected_summary = selected_asset_summary(index, selection);
    let status_asset_suffix = status_asset_suffix(index, selection, &state);
    AssetUiSnapshot {
        asset_section_title: "Assets".to_string(),
        asset_count: asset_count_label(index, &state),
        asset_row_labels,
        asset_selected_summary: selected_summary,
        selected_row_index,
        status_asset_suffix,
    }
}

fn empty_row_labels() -> [String; MAX_VISIBLE_ASSET_ROWS] {
    std::array::from_fn(|_| String::new())
}

fn asset_row_label(record: &AssetRecord) -> String {
    format!(
        "{} - {} ({})",
        record.name.as_str(),
        record.path.display(),
        record.kind.label()
    )
}

fn row_index_for_asset(index: &AssetIndex, id: AssetId) -> Option<usize> {
    index
        .records()
        .iter()
        .take(MAX_VISIBLE_ASSET_ROWS)
        .position(|record| record.id == id)
}

fn selected_asset_summary(index: &AssetIndex, selection: &AssetSelection) -> String {
    let Some(id) = selection.selected() else {
        return "Selected: None".to_string();
    };
    let Some(record) = index.find(id) else {
        return "Selected: None".to_string();
    };
    format!(
        "Selected: {} | {} | {}",
        record.name.as_str(),
        record.kind.label(),
        record.path.display()
    )
}

fn status_asset_suffix(
    index: &AssetIndex,
    selection: &AssetSelection,
    state: &AssetUiState<'_>,
) -> String {
    if let Some(message) = state.last_command_message {
        return format!("Asset: {message}");
    }
    if state.dirty {
        return "Asset: Asset index dirty - refresh recommended".to_string();
    }
    if let Some(diagnostic) = first_diagnostic(state.diagnostics) {
        return format!("Asset: {}", diagnostic.summary());
    }
    if let Some(id) = selection.selected()
        && let Some(record) = index.find(id)
    {
        return format!("Asset: {} ({})", record.name.as_str(), record.kind.label());
    }
    match state.scan_status {
        AssetScanStatus::UnavailableNoProject => "Asset: No project open".to_string(),
        AssetScanStatus::Scanning => "Asset: Scanning assets...".to_string(),
        AssetScanStatus::Dirty => "Asset: Asset index dirty - refresh recommended".to_string(),
        AssetScanStatus::Error => "Asset: Error".to_string(),
        AssetScanStatus::Ready => {
            if index.is_empty() && !state.scanned {
                "Asset: Assets not scanned".to_string()
            } else {
                format!("Assets: {}", index.len())
            }
        }
    }
}

fn asset_count_label(index: &AssetIndex, state: &AssetUiState<'_>) -> String {
    if !state.project_loaded {
        return "Assets unavailable - no project open".to_string();
    }
    if state.dirty || state.scan_status == AssetScanStatus::Dirty {
        return format!(
            "Asset index dirty - refresh recommended | {}",
            watch_label(state.watch_status)
        );
    }
    if let Some(diagnostic) = first_diagnostic(state.diagnostics)
        && state.scan_status == AssetScanStatus::Error
    {
        return format!("Asset error: {}", diagnostic.summary());
    }
    match state.scan_status {
        AssetScanStatus::UnavailableNoProject => "Assets unavailable - no project open".to_string(),
        AssetScanStatus::Scanning => "Scanning assets...".to_string(),
        AssetScanStatus::Error => "Asset error".to_string(),
        AssetScanStatus::Dirty => {
            format!(
                "Asset index dirty - refresh recommended | {}",
                watch_label(state.watch_status)
            )
        }
        AssetScanStatus::Ready => {
            if index.is_empty() && !state.scanned {
                format!(
                    "Assets not scanned - Run asset.scan | {}",
                    watch_label(state.watch_status)
                )
            } else {
                format!(
                    "Assets: {} | {} | {}",
                    index.len(),
                    index.kind_summary(),
                    watch_label(state.watch_status)
                )
            }
        }
    }
}

fn watch_label(status: &AssetWatchStatus) -> String {
    match status {
        AssetWatchStatus::Stopped => "Watch: stopped".to_string(),
        AssetWatchStatus::Watching(_) => "Watch: watching".to_string(),
        AssetWatchStatus::Error(_) => "Watch: error".to_string(),
    }
}

fn first_diagnostic(diagnostics: &[AssetDiagnostic]) -> Option<&AssetDiagnostic> {
    diagnostics.first()
}
