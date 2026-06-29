use elcarax_adapter_api::{AdapterCapabilities, AdapterDiagnostic, AdapterName, AdapterVersion};
use elcarax_adapter_host::AdapterHostState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdapterUiSnapshot {
    pub(crate) adapter_status: String,
    pub(crate) adapter_diagnostics: String,
    pub(crate) adapter_command: String,
    pub(crate) status_adapter_suffix: String,
}

pub(crate) fn adapter_ui_snapshot(
    state: AdapterHostState,
    name: Option<&AdapterName>,
    version: Option<&AdapterVersion>,
    capabilities: Option<&AdapterCapabilities>,
    diagnostics: &[AdapterDiagnostic],
    last_result: Option<&str>,
) -> AdapterUiSnapshot {
    let connected_label = match (state, name, version) {
        (AdapterHostState::Connected, Some(name), Some(version)) => {
            format!("Adapter: {} {} Connected", name.as_str(), version.as_str())
        }
        _ => format!("Adapter: {}", state_label(state)),
    };
    let capability_label = capabilities
        .map(capabilities_summary)
        .unwrap_or_else(|| "Capabilities: None".to_string());
    let diagnostic_label = format!("Adapter Diagnostics: {}", diagnostics.len());
    let command = last_result
        .map(|message| format!("Adapter Command: {message}"))
        .unwrap_or_else(|| "Adapter Command: None".to_string());
    AdapterUiSnapshot {
        adapter_status: format!("{connected_label} | {capability_label}"),
        adapter_diagnostics: diagnostic_label,
        adapter_command: command,
        status_adapter_suffix: connected_label,
    }
}

fn state_label(state: AdapterHostState) -> &'static str {
    match state {
        AdapterHostState::Disconnected => "Disconnected",
        AdapterHostState::Starting => "Starting",
        AdapterHostState::Connected => "Connected",
        AdapterHostState::Failed => "Failed",
        AdapterHostState::Stopped => "Stopped",
    }
}

fn capabilities_summary(capabilities: &AdapterCapabilities) -> String {
    format!(
        "Capabilities: project={} scene={} diagnostics={} writeback={} viewport={}",
        capabilities.provides_project_info,
        capabilities.provides_scene_snapshot,
        capabilities.provides_diagnostics,
        capabilities.supports_property_writeback,
        capabilities.supports_viewport_preview
    )
}
