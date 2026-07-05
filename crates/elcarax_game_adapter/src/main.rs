use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterError, AdapterEvent, AdapterId, AdapterLog,
    AdapterName, AdapterRequest, AdapterRequestMessage, AdapterResponse, AdapterResponseMessage,
    AdapterVersion, ErrorResponse, GetDiagnosticsResponse, GetSceneSnapshotResponse,
    GetViewportFrameRequest, GetViewportFrameResponse, HandshakeResponse, LoadProjectResponse,
    PickViewportObjectRequest, PickViewportObjectResponse, ProtocolVersion, SetPropertyRequest,
    SetPropertyResponse, SetPropertyStatus, ShutdownResponse, ViewportCameraInput,
    ViewportFrameResponseStatus, ViewportPickResponseStatus, decode_request_line, encode_event_line,
    encode_response_line,
};
use elcarax_core::{ElcaraxError, Result, ViewportCamera, ViewportFrameFormat};
use elcarax_scene_model::{
    PropertyEditError, ScenePatch, SceneSnapshot, ViewportPickCoord, pick_object_at,
    prepare_property_change, reference_scene_snapshot,
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
    viewport_camera: ViewportCamera,
}

impl MockAdapter {
    fn new() -> Self {
        Self {
            scene: reference_scene_snapshot(),
            project_loaded: false,
            diagnostics: vec![AdapterDiagnostic::info(
                "mock-adapter",
                "Reference stdio adapter ready with deterministic scene snapshot",
            )],
            viewport_camera: ViewportCamera::default_editor(),
        }
    }

    fn apply_camera_input(&mut self, input: Option<ViewportCameraInput>) {
        let Some(input) = input else {
            return;
        };
        self.viewport_camera.pan_by(
            input.pan_delta_x + input.orbit_delta_x,
            input.pan_delta_y + input.orbit_delta_y,
        );
        if input.dolly_factor != 1.0 {
            self.viewport_camera.zoom_by(input.dolly_factor);
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
                        capabilities: AdapterCapabilities::stdio_game_adapter(),
                    })
                }
            }
            AdapterRequestMessage::LoadProject(request) => {
                self.project_loaded = true;
                AdapterResponseMessage::LoadProject(LoadProjectResponse {
                    display_name: "Mock Adapter Demo Project".to_string(),
                    root_path: request
                        .project_path
                        .or_else(|| Some(PathBuf::from("adapter/reference"))),
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
            AdapterRequestMessage::GetViewportFrame(request) => {
                AdapterResponseMessage::GetViewportFrame(self.viewport_frame(request))
            }
            AdapterRequestMessage::PickViewportObject(request) => {
                AdapterResponseMessage::PickViewportObject(self.pick_viewport_object(request))
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

    fn viewport_frame(&mut self, request: GetViewportFrameRequest) -> GetViewportFrameResponse {
        let viewport_id = request.viewport_id;
        if request.format != ViewportFrameFormat::Rgba8Unorm {
            return GetViewportFrameResponse::failed(
                viewport_id,
                ViewportFrameResponseStatus::UnsupportedFormat,
                "only Rgba8Unorm is supported",
            );
        }
        const MAX_VIEWPORT_SIZE: u32 = 1024;
        if request.width == 0
            || request.height == 0
            || request.width > MAX_VIEWPORT_SIZE
            || request.height > MAX_VIEWPORT_SIZE
        {
            return GetViewportFrameResponse::failed(
                viewport_id,
                ViewportFrameResponseStatus::InvalidSize,
                format!("viewport size must be between 1 and {MAX_VIEWPORT_SIZE} pixels"),
            );
        }
        if !self.project_loaded {
            return GetViewportFrameResponse::failed(
                viewport_id,
                ViewportFrameResponseStatus::NoSceneLoaded,
                "load project before requesting viewport frame",
            );
        }
        self.apply_camera_input(request.camera_input);
        let width = request.width;
        let height = request.height;
        let pixels = procedural_viewport_rgba(
            width,
            height,
            self.viewport_camera,
            request.editor_input,
        );
        GetViewportFrameResponse {
            viewport_id,
            width,
            height,
            format: ViewportFrameFormat::Rgba8Unorm,
            pixels,
            diagnostics: vec![AdapterDiagnostic::info(
                "game-adapter",
                format!("generated {width}x{height} preview frame"),
            )],
            status: ViewportFrameResponseStatus::Available,
        }
    }

    fn pick_viewport_object(
        &self,
        request: PickViewportObjectRequest,
    ) -> PickViewportObjectResponse {
        if !self.project_loaded {
            return PickViewportObjectResponse::failed(
                request.viewport_id,
                ViewportPickResponseStatus::NoSceneLoaded,
                "load project before picking viewport objects",
            );
        }
        if !(0.0..=1.0).contains(&request.u) || !(0.0..=1.0).contains(&request.v) {
            return PickViewportObjectResponse::failed(
                request.viewport_id,
                ViewportPickResponseStatus::InvalidCoordinate,
                "viewport pick coordinates must be normalized between 0 and 1",
            );
        }
        let object_id = pick_object_at(
            &self.scene,
            ViewportPickCoord {
                u: request.u,
                v: request.v,
            },
        );
        PickViewportObjectResponse {
            viewport_id: request.viewport_id,
            object_id,
            diagnostics: Vec::new(),
            status: if object_id.is_some() {
                ViewportPickResponseStatus::Picked
            } else {
                ViewportPickResponseStatus::Missed
            },
        }
    }
}

fn procedural_viewport_rgba(
    width: u32,
    height: u32,
    camera: ViewportCamera,
    editor_input: Option<elcarax_adapter_api::ViewportEditorInput>,
) -> Vec<u8> {
    let mut pixels = vec![0_u8; (width * height * 4) as usize];
    let pan_x = camera.pan_x.round() as i32;
    let pan_y = camera.pan_y.round() as i32;
    let zoom = camera.zoom.max(ViewportCamera::MIN_ZOOM);
    let pointer = editor_input.map(|input| (input.pointer_x.round() as i32, input.pointer_y.round() as i32));
    for y in 0..height {
        for x in 0..width {
            let sample_x = ((x as f32 / zoom).round() as i32).saturating_sub(pan_x);
            let sample_y = ((y as f32 / zoom).round() as i32).saturating_sub(pan_y);
            let sample_x = sample_x.unsigned_abs();
            let sample_y = sample_y.unsigned_abs();
            let index = ((y * width + x) * 4) as usize;
            let checker = ((sample_x / 8) + (sample_y / 8)).is_multiple_of(2);
            let gradient = ((sample_x + sample_y) % 256) as u8;
            let near_pointer = pointer.is_some_and(|(px, py)| {
                let dx = (x as i32 - px).abs();
                let dy = (y as i32 - py).abs();
                dx <= 2 && dy <= 2
            });
            pixels[index] = if near_pointer {
                220
            } else if checker {
                40 + gradient / 4
            } else {
                90 + gradient / 3
            };
            pixels[index + 1] = if near_pointer { 180 } else { 50 + (gradient / 2) };
            pixels[index + 2] = if near_pointer { 80 } else { 120 + (gradient / 5) };
            pixels[index + 3] = 255;
        }
    }
    pixels
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
    use elcarax_adapter_api::{
        AdapterEditSource, AdapterViewportId, GetSceneSnapshotRequest, GetViewportFrameRequest,
        ViewportCameraInput, ViewportEditorInput, ViewportFrameResponseStatus,
    };
    use elcarax_core::ViewportFrameFormat;
    use elcarax_scene_model::{PropertyPath, PropertyValue};

    #[test]
    fn valid_viewport_request_returns_rgba_frame() {
        let mut adapter = MockAdapter::new();
        adapter.project_loaded = true;
        let response = adapter.viewport_frame(GetViewportFrameRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            width: 16,
            height: 16,
            format: ViewportFrameFormat::Rgba8Unorm,
            camera_input: None,
            editor_input: None,
        });
        assert_eq!(response.status, ViewportFrameResponseStatus::Available);
        assert_eq!(response.width, 16);
        assert_eq!(response.height, 16);
        assert_eq!(response.pixels.len(), 16 * 16 * 4);
    }

    #[test]
    fn invalid_viewport_size_returns_error() {
        let mut adapter = MockAdapter::new();
        adapter.project_loaded = true;
        let response = adapter.viewport_frame(GetViewportFrameRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            width: 0,
            height: 16,
            format: ViewportFrameFormat::Rgba8Unorm,
            camera_input: None,
            editor_input: None,
        });
        assert_eq!(response.status, ViewportFrameResponseStatus::InvalidSize);
    }

    #[test]
    fn camera_input_changes_viewport_frame() {
        let mut adapter = MockAdapter::new();
        adapter.project_loaded = true;
        let base = adapter.viewport_frame(GetViewportFrameRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            width: 16,
            height: 16,
            format: ViewportFrameFormat::Rgba8Unorm,
            camera_input: None,
            editor_input: None,
        });
        let shifted = adapter.viewport_frame(GetViewportFrameRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            width: 16,
            height: 16,
            format: ViewportFrameFormat::Rgba8Unorm,
            camera_input: Some(ViewportCameraInput {
                pan_delta_x: 16.0,
                pan_delta_y: 0.0,
                ..ViewportCameraInput::neutral()
            }),
            editor_input: None,
        });
        assert_ne!(base.pixels, shifted.pixels);
    }

    #[test]
    fn editor_input_highlights_pointer_pixel() {
        let mut adapter = MockAdapter::new();
        adapter.project_loaded = true;
        let response = adapter.viewport_frame(GetViewportFrameRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            width: 16,
            height: 16,
            format: ViewportFrameFormat::Rgba8Unorm,
            camera_input: None,
            editor_input: Some(ViewportEditorInput::pointer(8.0, 8.0, false, false, false)),
        });
        let index = ((8 * 16 + 8) * 4) as usize;
        assert_eq!(response.pixels[index], 220);
    }

    #[test]
    fn repeated_viewport_requests_are_deterministic() {
        let mut adapter = MockAdapter::new();
        adapter.project_loaded = true;
        let request = GetViewportFrameRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            width: 8,
            height: 8,
            format: ViewportFrameFormat::Rgba8Unorm,
            camera_input: None,
            editor_input: None,
        };
        let first = adapter.viewport_frame(request.clone());
        let second = adapter.viewport_frame(request);
        assert_eq!(first.pixels, second.pixels);
    }

    #[test]
    fn viewport_pick_selects_reference_object() {
        let mut adapter = MockAdapter::new();
        adapter.project_loaded = true;
        let response = adapter.pick_viewport_object(PickViewportObjectRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            u: 0.5,
            v: 0.5,
        });
        assert_eq!(response.status, ViewportPickResponseStatus::Picked);
        assert!(response.object_id.is_some());
    }

    #[test]
    fn viewport_pick_rejects_invalid_coordinates() {
        let mut adapter = MockAdapter::new();
        adapter.project_loaded = true;
        let response = adapter.pick_viewport_object(PickViewportObjectRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            u: 1.5,
            v: 0.5,
        });
        assert_eq!(
            response.status,
            ViewportPickResponseStatus::InvalidCoordinate
        );
    }

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
        let snapshot = reference_scene_snapshot();
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
