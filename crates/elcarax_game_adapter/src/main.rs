use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterError, AdapterEvent, AdapterId, AdapterLog,
    AdapterName, AdapterRequest, AdapterRequestMessage, AdapterResponse, AdapterResponseMessage,
    AdapterVersion, ErrorResponse, GetDiagnosticsResponse, GetSceneSnapshotResponse,
    HandshakeResponse, LoadProjectResponse, ProtocolVersion, SetPropertyRequest,
    SetPropertyResponse, SetPropertyStatus, ShutdownResponse, decode_request_line,
    encode_event_line, encode_response_line,
};
use elcarax_core::{ElcaraxError, Result};
use elcarax_scene_model::{
    PropertyEditError, ScenePatch, SceneSnapshot, demo_scene_snapshot, prepare_property_change,
};

fn main() -> Result<()> {
    run_stdio_adapter()
}

fn run_stdio_adapter() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut adapter = MockAdapter::new();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| ElcaraxError::Adapter(error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let request = match decode_request_line(&line) {
            Ok(request) => request,
            Err(error) => {
                write_event(
                    &mut stdout,
                    AdapterEvent::Diagnostic(AdapterDiagnostic::info(
                        "mock-adapter",
                        format!("invalid request JSON: {error}"),
                    )),
                )?;
                continue;
            }
        };
        let should_stop = matches!(request.message, AdapterRequestMessage::Shutdown(_));
        adapter.handle_request(request, &mut stdout)?;
        if should_stop {
            break;
        }
    }
    Ok(())
}

struct MockAdapter {
    scene: SceneSnapshot,
    project_loaded: bool,
    diagnostics: Vec<AdapterDiagnostic>,
}

impl MockAdapter {
    fn new() -> Self {
        Self {
            scene: demo_scene_snapshot(),
            project_loaded: false,
            diagnostics: vec![AdapterDiagnostic::info(
                "mock-adapter",
                "Mock adapter ready with deterministic demo scene",
            )],
        }
    }

    fn handle_request<W: Write>(&mut self, request: AdapterRequest, writer: &mut W) -> Result<()> {
        let message = match request.message {
            AdapterRequestMessage::Handshake(handshake) => {
                write_event(
                    writer,
                    AdapterEvent::Log(AdapterLog::info("handshake requested")),
                )?;
                if !handshake.protocol_version.is_supported() {
                    AdapterResponseMessage::Error(ErrorResponse {
                        error: AdapterError::unsupported_protocol_version(
                            handshake.protocol_version,
                        ),
                    })
                } else {
                    AdapterResponseMessage::Handshake(HandshakeResponse {
                        adapter_id: AdapterId::new("elcarax-mock-adapter"),
                        adapter_name: AdapterName::new("Mock Adapter"),
                        adapter_version: AdapterVersion::new(env!("CARGO_PKG_VERSION")),
                        protocol_version: ProtocolVersion::V0,
                        capabilities: AdapterCapabilities::mock_milestone_13(),
                    })
                }
            }
            AdapterRequestMessage::LoadProject(request) => {
                self.project_loaded = true;
                AdapterResponseMessage::LoadProject(LoadProjectResponse {
                    display_name: "Mock Adapter Demo Project".to_string(),
                    root_path: request
                        .project_path
                        .or_else(|| Some(PathBuf::from("adapter/mock-demo"))),
                })
            }
            AdapterRequestMessage::GetSceneSnapshot(_) => {
                if !self.project_loaded {
                    self.project_loaded = true;
                }
                write_event(
                    writer,
                    AdapterEvent::Diagnostic(AdapterDiagnostic::info(
                        "mock-adapter",
                        "Scene snapshot served from mock adapter",
                    )),
                )?;
                AdapterResponseMessage::GetSceneSnapshot(GetSceneSnapshotResponse {
                    snapshot: self.scene.clone(),
                    source_label: "Mock Adapter".to_string(),
                })
            }
            AdapterRequestMessage::SetProperty(request) => {
                AdapterResponseMessage::SetProperty(self.set_property(request))
            }
            AdapterRequestMessage::GetDiagnostics(_) => {
                AdapterResponseMessage::GetDiagnostics(GetDiagnosticsResponse {
                    diagnostics: self.diagnostics.clone(),
                })
            }
            AdapterRequestMessage::Shutdown(_) => {
                AdapterResponseMessage::Shutdown(ShutdownResponse { accepted: true })
            }
        };
        let response = AdapterResponse::new(request.request_id, message);
        write_response(writer, response)
    }

    fn set_property(&mut self, request: SetPropertyRequest) -> SetPropertyResponse {
        let path = request.path.clone();
        let result = prepare_property_change(
            &self.scene,
            request.object_id,
            &request.path,
            &request.new_value,
        );
        let change = match result {
            Ok(change) => change,
            Err(error) => {
                return rejected_property_response(
                    request,
                    status_for_edit_error(&error),
                    error.message(),
                );
            }
        };
        if let Some(expected) = &request.expected_old_value
            && *expected != change.old_value
        {
            let message = format!(
                "Stale expected value for '{}': expected {}, adapter has {}",
                path,
                expected.display_label(),
                change.old_value.display_label()
            );
            return rejected_property_response(request, SetPropertyStatus::StaleValue, message);
        }
        let old_value = change.old_value.clone();
        let new_value = change.new_value.clone();
        let patch = ScenePatch::property_updated(request.object_id, path, new_value.clone());
        if let Err(error) = patch.apply(&mut self.scene) {
            return rejected_property_response(
                request,
                status_for_edit_error(&error),
                error.message(),
            );
        }
        SetPropertyResponse {
            status: SetPropertyStatus::Accepted,
            scene_id: request.scene_id,
            object_id: request.object_id,
            path: request.path,
            old_value: Some(old_value),
            confirmed_new_value: Some(new_value),
            patch: Some(patch),
            diagnostics: Vec::new(),
        }
    }
}

fn rejected_property_response(
    request: SetPropertyRequest,
    status: SetPropertyStatus,
    message: impl Into<String>,
) -> SetPropertyResponse {
    SetPropertyResponse {
        status,
        scene_id: request.scene_id,
        object_id: request.object_id,
        path: request.path,
        old_value: None,
        confirmed_new_value: None,
        patch: None,
        diagnostics: vec![AdapterDiagnostic::info("mock-adapter", message)],
    }
}

fn status_for_edit_error(error: &PropertyEditError) -> SetPropertyStatus {
    match error {
        PropertyEditError::ObjectNotFound { .. } => SetPropertyStatus::ObjectNotFound,
        PropertyEditError::PropertyNotFound { .. } => SetPropertyStatus::PropertyNotFound,
        PropertyEditError::ReadOnly { .. } => SetPropertyStatus::ReadOnlyProperty,
        PropertyEditError::TypeMismatch { .. } => SetPropertyStatus::TypeMismatch,
    }
}

fn write_response<W: Write>(writer: &mut W, response: AdapterResponse) -> Result<()> {
    let line = encode_response_line(&response)
        .map_err(|error| ElcaraxError::Adapter(error.to_string()))?;
    writeln!(writer, "{line}").map_err(|error| ElcaraxError::Adapter(error.to_string()))?;
    writer
        .flush()
        .map_err(|error| ElcaraxError::Adapter(error.to_string()))
}

fn write_event<W: Write>(writer: &mut W, event: AdapterEvent) -> Result<()> {
    let line =
        encode_event_line(&event).map_err(|error| ElcaraxError::Adapter(error.to_string()))?;
    writeln!(writer, "{line}").map_err(|error| ElcaraxError::Adapter(error.to_string()))?;
    writer
        .flush()
        .map_err(|error| ElcaraxError::Adapter(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_adapter_api::{AdapterEditSource, GetSceneSnapshotRequest};
    use elcarax_scene_model::{PropertyPath, PropertyValue};

    #[test]
    fn set_editable_int_property_succeeds() {
        let mut adapter = MockAdapter::new();
        let response = adapter.set_property(request("gameplay.health", PropertyValue::I64(65)));
        assert_eq!(response.status, SetPropertyStatus::Accepted);
        assert_eq!(
            player_property(&adapter.scene, "gameplay.health"),
            PropertyValue::I64(65)
        );
    }

    #[test]
    fn set_editable_float_property_succeeds() {
        let mut adapter = MockAdapter::new();
        let response = adapter.set_property(request("gameplay.speed", PropertyValue::F64(9.0)));
        assert_eq!(response.status, SetPropertyStatus::Accepted);
        assert_eq!(
            player_property(&adapter.scene, "gameplay.speed"),
            PropertyValue::F64(9.0)
        );
    }

    #[test]
    fn set_editable_string_property_succeeds() {
        let mut adapter = MockAdapter::new();
        let response = adapter.set_property(request(
            "general.name",
            PropertyValue::String("Adapter Hero".to_string()),
        ));
        assert_eq!(response.status, SetPropertyStatus::Accepted);
        let player = match adapter.scene.object_by_name("Adapter Hero") {
            Some(player) => player,
            None => panic!("renamed player should exist"),
        };
        assert_eq!(player.display_name, "Adapter Hero");
    }

    #[test]
    fn set_read_only_property_fails() {
        let mut adapter = MockAdapter::new();
        let response = adapter.set_property(request(
            "references.mesh",
            PropertyValue::AssetRef("assets/models/hero.glb".to_string()),
        ));
        assert_eq!(response.status, SetPropertyStatus::ReadOnlyProperty);
    }

    #[test]
    fn set_missing_property_fails() {
        let mut adapter = MockAdapter::new();
        let response = adapter.set_property(request("gameplay.mana", PropertyValue::I64(10)));
        assert_eq!(response.status, SetPropertyStatus::PropertyNotFound);
    }

    #[test]
    fn set_type_mismatch_fails() {
        let mut adapter = MockAdapter::new();
        let response = adapter.set_property(request(
            "gameplay.health",
            PropertyValue::String("high".to_string()),
        ));
        assert_eq!(response.status, SetPropertyStatus::TypeMismatch);
    }

    #[test]
    fn stale_expected_old_value_fails() {
        let mut adapter = MockAdapter::new();
        let mut request = request("gameplay.health", PropertyValue::I64(65));
        request.expected_old_value = Some(PropertyValue::I64(50));
        let response = adapter.set_property(request);
        assert_eq!(response.status, SetPropertyStatus::StaleValue);
        assert_eq!(
            player_property(&adapter.scene, "gameplay.health"),
            PropertyValue::I64(100)
        );
    }

    #[test]
    fn scene_snapshot_after_edit_contains_updated_value() {
        let mut adapter = MockAdapter::new();
        let _ = adapter.set_property(request("gameplay.health", PropertyValue::I64(65)));
        let mut writer = Vec::new();
        let adapter_request = AdapterRequest::new(
            elcarax_adapter_api::AdapterRequestId(1),
            AdapterRequestMessage::GetSceneSnapshot(GetSceneSnapshotRequest { scene_id: None }),
        );
        if let Err(error) = adapter.handle_request(adapter_request, &mut writer) {
            panic!("snapshot request should succeed: {error}");
        }
        let output = match String::from_utf8(writer) {
            Ok(value) => value,
            Err(error) => panic!("response should be UTF-8: {error}"),
        };
        assert!(output.contains("65"));
    }

    fn request(path: &str, new_value: PropertyValue) -> SetPropertyRequest {
        let snapshot = demo_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        };
        SetPropertyRequest {
            scene_id: snapshot.scene_id(),
            object_id: player.id,
            path: property_path(path),
            expected_old_value: player.property(&property_path(path)).cloned(),
            new_value,
            transaction_id: "test".to_string(),
            edit_source: AdapterEditSource::Inspector,
        }
    }

    fn player_property(snapshot: &SceneSnapshot, path: &str) -> PropertyValue {
        let player = match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        };
        match player.property(&property_path(path)) {
            Some(value) => value.clone(),
            None => panic!("property should exist"),
        }
    }

    fn property_path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("test path should parse: {error}"),
        }
    }
}
