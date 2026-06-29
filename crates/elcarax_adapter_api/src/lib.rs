//! Versioned protocol between Elcarax and domain adapters.

mod capability;
mod message;
mod protocol;

pub use capability::AdapterCapabilities;
pub use message::{
    AdapterDiagnostic, AdapterError, AdapterEvent, AdapterLine, AdapterLog, AdapterRequest,
    AdapterRequestId, AdapterRequestMessage, AdapterResponse, AdapterResponseMessage,
    AdapterToEditor, EditorToAdapter, ErrorResponse, GetDiagnosticsRequest, GetDiagnosticsResponse,
    GetSceneSnapshotRequest, GetSceneSnapshotResponse, ShutdownRequest, ShutdownResponse,
    decode_adapter_line, decode_request_line, encode_event_line, encode_request_line,
    encode_response_line,
};
pub use protocol::{
    AdapterId, AdapterName, AdapterVersion, HandshakeRequest, HandshakeResponse,
    LoadProjectRequest, LoadProjectResponse, ProtocolVersion,
};
