use elcarax_assets::{AssetId, AssetIndex, AssetRecord, AssetScan, AssetSelection};
use elcarax_ui::MAX_VISIBLE_ASSET_ROWS;

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
    scan: Option<&AssetScan>,
    last_command_message: Option<&str>,
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
    let status_asset_suffix = status_asset_suffix(index, selection, scan, last_command_message);
    AssetUiSnapshot {
        asset_section_title: "Assets".to_string(),
        asset_count: asset_count_label(index),
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
    format!("{} ({})", record.name.as_str(), record.kind.label())
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
    scan: Option<&AssetScan>,
    last_command_message: Option<&str>,
) -> String {
    if let Some(message) = last_command_message {
        return format!("Asset: {message}");
    }
    if let Some(scan) = scan
        && !scan.diagnostics().is_empty()
    {
        return format!("Asset: {}", scan.diagnostics()[0].summary());
    }
    if let Some(id) = selection.selected()
        && let Some(record) = index.find(id)
    {
        return format!("Asset: {} ({})", record.name.as_str(), record.kind.label());
    }
    if index.is_empty() {
        "Asset: No asset root loaded".to_string()
    } else {
        format!("Assets: {}", index.len())
    }
}

fn asset_count_label(index: &AssetIndex) -> String {
    if index.is_empty() {
        "Assets: No asset root loaded".to_string()
    } else {
        format!("Assets: {}", index.len())
    }
}
