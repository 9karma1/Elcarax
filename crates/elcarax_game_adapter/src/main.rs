use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterError, AdapterEvent, AdapterId, AdapterLog,
    AdapterName, AdapterRequest, AdapterRequestMessage, AdapterResponse, AdapterResponseMessage,
    AdapterVersion, ErrorResponse, GetDiagnosticsResponse, GetSceneSnapshotResponse,
    HandshakeResponse, LoadProjectResponse, ProtocolVersion, ShutdownResponse, decode_request_line,
    encode_event_line, encode_response_line,
};
use elcarax_core::{ElcaraxError, Result};
use elcarax_scene_model::{SceneSnapshot, demo_scene_snapshot};

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
                        capabilities: AdapterCapabilities::mock_milestone_12(),
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
