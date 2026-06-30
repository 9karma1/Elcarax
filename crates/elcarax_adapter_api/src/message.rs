use std::error::Error;
use std::fmt;

use elcarax_core::{Diagnostic, DiagnosticSource, Severity};
use elcarax_scene_model::{
    PropertyPath, PropertyValue, SceneId, SceneObjectId, ScenePatch, SceneSnapshot,
};
use serde::{Deserialize, Serialize};

use crate::{
    GetViewportFrameRequest, GetViewportFrameResponse, HandshakeRequest, HandshakeResponse,
    LoadProjectRequest, LoadProjectResponse, ProtocolVersion,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AdapterRequestId(pub u64);

impl AdapterRequestId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdapterRequest {
    pub request_id: AdapterRequestId,
    pub message: AdapterRequestMessage,
}

impl AdapterRequest {
    pub const fn new(request_id: AdapterRequestId, message: AdapterRequestMessage) -> Self {
        Self {
            request_id,
            message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AdapterRequestMessage {
    Handshake(HandshakeRequest),
    LoadProject(LoadProjectRequest),
    GetSceneSnapshot(GetSceneSnapshotRequest),
    SetProperty(SetPropertyRequest),
    GetDiagnostics(GetDiagnosticsRequest),
    GetViewportFrame(GetViewportFrameRequest),
    Shutdown(ShutdownRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSceneSnapshotRequest {
    pub scene_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetPropertyRequest {
    pub scene_id: SceneId,
    pub object_id: SceneObjectId,
    pub path: PropertyPath,
    pub expected_old_value: Option<PropertyValue>,
    pub new_value: PropertyValue,
    pub transaction_id: String,
    pub edit_source: AdapterEditSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterEditSource {
    Inspector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetDiagnosticsRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownRequest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdapterResponse {
    pub request_id: AdapterRequestId,
    pub message: AdapterResponseMessage,
}

impl AdapterResponse {
    pub const fn new(request_id: AdapterRequestId, message: AdapterResponseMessage) -> Self {
        Self {
            request_id,
            message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AdapterResponseMessage {
    Handshake(HandshakeResponse),
    LoadProject(LoadProjectResponse),
    GetSceneSnapshot(GetSceneSnapshotResponse),
    SetProperty(SetPropertyResponse),
    GetDiagnostics(GetDiagnosticsResponse),
    GetViewportFrame(GetViewportFrameResponse),
    Shutdown(ShutdownResponse),
    Error(ErrorResponse),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetSceneSnapshotResponse {
    pub snapshot: SceneSnapshot,
    pub source_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetPropertyResponse {
    pub status: SetPropertyStatus,
    pub scene_id: SceneId,
    pub object_id: SceneObjectId,
    pub path: PropertyPath,
    pub old_value: Option<PropertyValue>,
    pub confirmed_new_value: Option<PropertyValue>,
    pub patch: Option<ScenePatch>,
    pub diagnostics: Vec<AdapterDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetPropertyStatus {
    Accepted,
    Rejected,
    ObjectNotFound,
    PropertyNotFound,
    ReadOnlyProperty,
    TypeMismatch,
    StaleValue,
    AdapterError,
}

impl SetPropertyStatus {
    pub const fn is_accepted(self) -> bool {
        matches!(self, Self::Accepted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetDiagnosticsResponse {
    pub diagnostics: Vec<AdapterDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownResponse {
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: AdapterError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterEvent {
    Diagnostic(AdapterDiagnostic),
    Log(AdapterLog),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterDiagnostic {
    pub severity: Severity,
    pub source: String,
    pub message: String,
}

impl AdapterDiagnostic {
    pub fn info(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            source: source.into(),
            message: message.into(),
        }
    }

    pub fn as_core_diagnostic(&self) -> Diagnostic {
        Diagnostic {
            severity: self.severity,
            source: DiagnosticSource::new(self.source.clone()),
            message: self.message.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterLog {
    pub level: String,
    pub message: String,
}

impl AdapterLog {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: "info".to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterError {
    UnsupportedProtocolVersion {
        requested: ProtocolVersion,
        supported: ProtocolVersion,
    },
    InvalidRequest(String),
    NotLoaded(String),
    Internal(String),
}

impl AdapterError {
    pub fn unsupported_protocol_version(requested: ProtocolVersion) -> Self {
        Self::UnsupportedProtocolVersion {
            requested,
            supported: ProtocolVersion::V0,
        }
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProtocolVersion {
                requested,
                supported,
            } => write!(
                formatter,
                "unsupported protocol version {}; supported version is {}",
                requested.0, supported.0
            ),
            Self::InvalidRequest(message) => write!(formatter, "invalid request: {message}"),
            Self::NotLoaded(message) => write!(formatter, "not loaded: {message}"),
            Self::Internal(message) => write!(formatter, "adapter internal error: {message}"),
        }
    }
}

impl Error for AdapterError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum AdapterLine {
    Response(AdapterResponse),
    Event(AdapterEvent),
}

pub type EditorToAdapter = AdapterRequestMessage;
pub type AdapterToEditor = AdapterResponseMessage;

pub fn encode_request_line(request: &AdapterRequest) -> Result<String, serde_json::Error> {
    serde_json::to_string(request)
}

pub fn decode_request_line(line: &str) -> Result<AdapterRequest, serde_json::Error> {
    serde_json::from_str(line)
}

pub fn encode_response_line(response: &AdapterResponse) -> Result<String, serde_json::Error> {
    serde_json::to_string(&AdapterLine::Response(response.clone()))
}

pub fn encode_event_line(event: &AdapterEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string(&AdapterLine::Event(event.clone()))
}

pub fn decode_adapter_line(line: &str) -> Result<AdapterLine, serde_json::Error> {
    serde_json::from_str(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdapterCapabilities, AdapterId, AdapterName, AdapterVersion};
    use elcarax_scene_model::demo_scene_snapshot;

    #[test]
    fn handshake_request_response_round_trip() -> Result<(), serde_json::Error> {
        let request = AdapterRequest::new(
            AdapterRequestId::new(7),
            AdapterRequestMessage::Handshake(HandshakeRequest::current("test-editor", None)),
        );
        let line = encode_request_line(&request)?;
        assert_eq!(decode_request_line(&line)?, request);

        let response = AdapterResponse::new(
            request.request_id,
            AdapterResponseMessage::Handshake(HandshakeResponse {
                adapter_id: AdapterId::new("mock"),
                adapter_name: AdapterName::new("Mock Adapter"),
                adapter_version: AdapterVersion::new("0.1.0"),
                protocol_version: ProtocolVersion::V0,
                capabilities: AdapterCapabilities::mock_milestone_12(),
            }),
        );
        let line = encode_response_line(&response)?;
        assert_eq!(decode_adapter_line(&line)?, AdapterLine::Response(response));
        Ok(())
    }

    #[test]
    fn request_ids_round_trip() -> Result<(), serde_json::Error> {
        let request = AdapterRequest::new(
            AdapterRequestId::new(42),
            AdapterRequestMessage::GetDiagnostics(GetDiagnosticsRequest),
        );
        let line = encode_request_line(&request)?;
        assert_eq!(decode_request_line(&line)?.request_id, AdapterRequestId(42));
        Ok(())
    }

    #[test]
    fn viewport_frame_request_round_trips() -> Result<(), serde_json::Error> {
        use crate::{AdapterViewportId, GetViewportFrameRequest, ViewportFrameResponseStatus};
        use elcarax_core::ViewportFrameFormat;

        let request = AdapterRequest::new(
            AdapterRequestId::new(11),
            AdapterRequestMessage::GetViewportFrame(GetViewportFrameRequest {
                viewport_id: AdapterViewportId(1),
                scene_id: Some(7),
                width: 64,
                height: 48,
                format: ViewportFrameFormat::Rgba8Unorm,
            }),
        );
        let line = encode_request_line(&request)?;
        assert_eq!(decode_request_line(&line)?, request);

        let response = AdapterResponse::new(
            request.request_id,
            AdapterResponseMessage::GetViewportFrame(GetViewportFrameResponse {
                viewport_id: AdapterViewportId(1),
                width: 64,
                height: 48,
                format: ViewportFrameFormat::Rgba8Unorm,
                pixels: vec![255, 0, 0, 255, 0, 255, 0, 255],
                diagnostics: Vec::new(),
                status: ViewportFrameResponseStatus::Available,
            }),
        );
        let line = encode_response_line(&response)?;
        assert_eq!(decode_adapter_line(&line)?, AdapterLine::Response(response));
        Ok(())
    }

    #[test]
    fn failed_viewport_response_round_trips() -> Result<(), serde_json::Error> {
        use crate::{AdapterViewportId, GetViewportFrameResponse, ViewportFrameResponseStatus};

        let response = AdapterResponse::new(
            AdapterRequestId::new(12),
            AdapterResponseMessage::GetViewportFrame(GetViewportFrameResponse::failed(
                AdapterViewportId(1),
                ViewportFrameResponseStatus::InvalidSize,
                "width and height must be positive",
            )),
        );
        let line = encode_response_line(&response)?;
        let AdapterLine::Response(AdapterResponse {
            message: AdapterResponseMessage::GetViewportFrame(decoded),
            ..
        }) = decode_adapter_line(&line)?
        else {
            panic!("decoded response should carry viewport frame");
        };
        assert_eq!(decoded.status, ViewportFrameResponseStatus::InvalidSize);
        assert_eq!(decoded.diagnostics.len(), 1);
        Ok(())
    }

    #[test]
    fn unsupported_protocol_version_error_is_clear() {
        let error = AdapterError::unsupported_protocol_version(ProtocolVersion(99));
        assert!(
            error
                .to_string()
                .contains("unsupported protocol version 99")
        );
    }

    #[test]
    fn scene_snapshot_response_can_carry_demo_scene() -> Result<(), serde_json::Error> {
        let response = AdapterResponse::new(
            AdapterRequestId::new(1),
            AdapterResponseMessage::GetSceneSnapshot(GetSceneSnapshotResponse {
                snapshot: demo_scene_snapshot(),
                source_label: "mock".to_string(),
            }),
        );
        let line = encode_response_line(&response)?;
        let decoded = decode_adapter_line(&line)?;
        let AdapterLine::Response(AdapterResponse {
            message: AdapterResponseMessage::GetSceneSnapshot(snapshot_response),
            ..
        }) = decoded
        else {
            panic!("decoded response should carry a scene snapshot");
        };
        assert_eq!(snapshot_response.snapshot.object_count(), 10);
        Ok(())
    }

    #[test]
    fn diagnostics_round_trip() -> Result<(), serde_json::Error> {
        let event = AdapterEvent::Diagnostic(AdapterDiagnostic::info(
            "mock-adapter",
            "diagnostic from adapter",
        ));
        let line = encode_event_line(&event)?;
        assert_eq!(decode_adapter_line(&line)?, AdapterLine::Event(event));
        Ok(())
    }

    #[test]
    fn set_property_request_round_trips() -> Result<(), serde_json::Error> {
        let snapshot = demo_scene_snapshot();
        let player = player(&snapshot);
        let request = AdapterRequest::new(
            AdapterRequestId::new(9),
            AdapterRequestMessage::SetProperty(SetPropertyRequest {
                scene_id: snapshot.scene_id(),
                object_id: player.id,
                path: path("gameplay.health"),
                expected_old_value: Some(PropertyValue::I64(100)),
                new_value: PropertyValue::I64(65),
                transaction_id: "adapter.inspector.set_player_health_demo".to_string(),
                edit_source: AdapterEditSource::Inspector,
            }),
        );
        let line = encode_request_line(&request)?;
        assert_eq!(decode_request_line(&line)?, request);
        Ok(())
    }

    #[test]
    fn set_property_response_round_trips() -> Result<(), serde_json::Error> {
        let snapshot = demo_scene_snapshot();
        let player = player(&snapshot);
        let response = AdapterResponse::new(
            AdapterRequestId::new(9),
            AdapterResponseMessage::SetProperty(SetPropertyResponse {
                status: SetPropertyStatus::Accepted,
                scene_id: snapshot.scene_id(),
                object_id: player.id,
                path: path("gameplay.health"),
                old_value: Some(PropertyValue::I64(100)),
                confirmed_new_value: Some(PropertyValue::I64(65)),
                patch: Some(ScenePatch::property_updated(
                    player.id,
                    path("gameplay.health"),
                    PropertyValue::I64(65),
                )),
                diagnostics: Vec::new(),
            }),
        );
        let line = encode_response_line(&response)?;
        assert_eq!(decode_adapter_line(&line)?, AdapterLine::Response(response));
        Ok(())
    }

    #[test]
    fn rejected_writeback_response_round_trips() -> Result<(), serde_json::Error> {
        let snapshot = demo_scene_snapshot();
        let response = rejected_response(&snapshot);
        let line = encode_response_line(&response)?;
        assert_eq!(decode_adapter_line(&line)?, AdapterLine::Response(response));
        Ok(())
    }

    #[test]
    fn diagnostics_round_trip_with_writeback_response() -> Result<(), serde_json::Error> {
        let snapshot = demo_scene_snapshot();
        let response = rejected_response(&snapshot);
        let line = encode_response_line(&response)?;
        let AdapterLine::Response(AdapterResponse {
            message: AdapterResponseMessage::SetProperty(decoded),
            ..
        }) = decode_adapter_line(&line)?
        else {
            panic!("decoded response should be set property");
        };
        assert_eq!(decoded.diagnostics.len(), 1);
        Ok(())
    }

    fn rejected_response(snapshot: &SceneSnapshot) -> AdapterResponse {
        let player = player(snapshot);
        AdapterResponse::new(
            AdapterRequestId::new(10),
            AdapterResponseMessage::SetProperty(SetPropertyResponse {
                status: SetPropertyStatus::ReadOnlyProperty,
                scene_id: snapshot.scene_id(),
                object_id: player.id,
                path: path("references.mesh"),
                old_value: None,
                confirmed_new_value: None,
                patch: None,
                diagnostics: vec![AdapterDiagnostic::info(
                    "mock-adapter",
                    "Property is read-only",
                )],
            }),
        )
    }

    fn player(snapshot: &SceneSnapshot) -> &elcarax_scene_model::SceneObject {
        match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        }
    }

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("test path should parse: {error}"),
        }
    }
}
