//! Versioned protocol between Elcarax and domain adapters.

mod capability;
mod frame;
mod message;
mod protocol;
mod viewport;

pub use capability::AdapterCapabilities;
pub use frame::{
    AdapterFrame, FRAME_HEADER_LEN, FRAME_MAGIC, FrameError, FrameKind, decode_adapter_frame,
    decode_request_frame, encode_event_frame, encode_request_frame, encode_response_frame,
    read_frame, write_frame,
};
pub use message::{
    AdapterDiagnostic, AdapterEditSource, AdapterError, AdapterEvent, AdapterLine, AdapterLog,
    AdapterRequest, AdapterRequestId, AdapterRequestMessage, AdapterResponse,
    AdapterResponseMessage, AdapterToEditor, EditorToAdapter, ErrorResponse, GetDiagnosticsRequest,
    GetDiagnosticsResponse, GetSceneSnapshotRequest, GetSceneSnapshotResponse, SetPropertyRequest,
    SetPropertyResponse, SetPropertyStatus, ShutdownRequest, ShutdownResponse,
};
pub use protocol::{
    AdapterId, AdapterName, AdapterVersion, HandshakeRequest, HandshakeResponse,
    LoadProjectRequest, LoadProjectResponse, ProtocolVersion,
};
pub use viewport::{
    AdapterViewportId, GetViewportFrameRequest, GetViewportFrameResponse,
    PickViewportObjectRequest, PickViewportObjectResponse, ViewportCameraInput,
    ViewportEditorInput, ViewportFrameResponseStatus, ViewportPickResponseStatus,
};
