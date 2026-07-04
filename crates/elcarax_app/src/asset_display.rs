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
    pub(crate) scroll_offset: usize,
    pub(crate) total_rows: usize,
    pub(crate) visible_rows: usize,
    pub(crate) status_asset_suffix: String,
}

pub(crate) fn asset_ui_snapshot_with_scroll(
    index: &AssetIndex,
    selection: &AssetSelection,
    state: AssetUiState<'_>,
    scroll_offset: usize,
) -> AssetUiSnapshot {
    let total_rows = index.len();
    let scroll_offset = clamp_scroll_offset(scroll_offset, total_rows, MAX_VISIBLE_ASSET_ROWS);
    let mut asset_row_labels = empty_row_labels();
    let records: Vec<_> = index
        .records()
        .iter()
        .skip(scroll_offset)
        .take(MAX_VISIBLE_ASSET_ROWS)
        .collect();
    for (index, record) in records.iter().enumerate() {
        asset_row_labels[index] = asset_row_label(record);
    }
    let selected_row_index = selection
        .selected()
        .and_then(|id| row_index_for_asset(index, id, scroll_offset));
    let selected_summary = selected_asset_summary(index, selection);
    let status_asset_suffix = status_asset_suffix(index, selection, &state);
    AssetUiSnapshot {
        asset_section_title: "Assets".to_string(),
        asset_count: asset_count_label(index, &state),
        asset_row_labels,
        asset_selected_summary: selected_summary,
        selected_row_index,
        scroll_offset,
        total_rows,
        visible_rows: MAX_VISIBLE_ASSET_ROWS,
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

fn row_index_for_asset(index: &AssetIndex, id: AssetId, scroll_offset: usize) -> Option<usize> {
    index
        .records()
        .iter()
        .skip(scroll_offset)
        .take(MAX_VISIBLE_ASSET_ROWS)
        .position(|record| record.id == id)
}

fn clamp_scroll_offset(scroll_offset: usize, total_rows: usize, visible_rows: usize) -> usize {
    scroll_offset.min(total_rows.saturating_sub(visible_rows))
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

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_assets::{AssetKind, AssetRecord, stable_asset_id_from_path};
    use std::path::{Path, PathBuf};

    #[test]
    fn asset_snapshot_uses_scroll_offset_window() {
        let records: Vec<_> = (0..(MAX_VISIBLE_ASSET_ROWS + 2))
            .map(|index| {
                let path = PathBuf::from(format!("asset-{index}.txt"));
                AssetRecord::from_parts(
                    stable_asset_id_from_path(Path::new(&path)),
                    format!("asset-{index}.txt"),
                    path,
                    AssetKind::Text,
                )
            })
            .collect();
        let index = AssetIndex::from_records(records);
        let snapshot = asset_ui_snapshot_with_scroll(
            &index,
            &AssetSelection::none(),
            AssetUiState {
                project_loaded: true,
                scan_status: AssetScanStatus::Ready,
                watch_status: &AssetWatchStatus::Stopped,
                diagnostics: &[],
                last_command_message: None,
                dirty: false,
                scanned: true,
            },
            2,
        );
        assert_eq!(snapshot.scroll_offset, 2);
        assert_eq!(snapshot.total_rows, MAX_VISIBLE_ASSET_ROWS + 2);
        assert!(snapshot.asset_row_labels[0].contains("asset-2.txt"));
        assert!(snapshot.asset_row_labels[MAX_VISIBLE_ASSET_ROWS - 1].contains("asset-9.txt"));
    }
}
