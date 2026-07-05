use elcarax_adapter_api::{
    AdapterViewportId, GetViewportFrameRequest, GetViewportFrameResponse, ViewportCameraInput,
    ViewportEditorInput, ViewportFrameResponseStatus,
};
use elcarax_core::{
    ViewportCamera, ViewportError, ViewportFrame, ViewportFrameFormat, ViewportSource,
};

use crate::adapter_state::AdapterState;
use crate::viewport_display::{ViewportUiSnapshot, viewport_ui_snapshot};

pub(crate) const VIEWPORT_REQUEST_FRAME_COMMAND: &str = "viewport.request_frame";
pub(crate) const VIEWPORT_CLEAR_COMMAND: &str = "viewport.clear";
pub(crate) const VIEWPORT_SHOW_STATUS_COMMAND: &str = "viewport.show_status";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ViewportFrameRequestSize {
    width: u32,
    height: u32,
}

impl ViewportFrameRequestSize {
    const DEFAULT_SIZE: u32 = 256;
    #[cfg(feature = "native-shell")]
    const MAX_SIZE: u32 = 1024;

    pub(crate) const fn default_editor() -> Self {
        Self {
            width: Self::DEFAULT_SIZE,
            height: Self::DEFAULT_SIZE,
        }
    }

    #[cfg(feature = "native-shell")]
    pub(crate) fn from_content_size(width: f32, height: f32) -> Self {
        if width <= 0.0 || height <= 0.0 {
            return Self::default_editor();
        }
        let width = width.round().clamp(1.0, Self::MAX_SIZE as f32) as u32;
        let height = height.round().clamp(1.0, Self::MAX_SIZE as f32) as u32;
        Self { width, height }
    }

    pub(crate) const fn width(self) -> u32 {
        self.width
    }

    pub(crate) const fn height(self) -> u32 {
        self.height
    }
}

pub(crate) struct AppViewportState {
    inner: elcarax_core::ViewportState,
    camera: ViewportCamera,
    pending_camera_input: ViewportCameraInput,
    pending_editor_input: Option<ViewportEditorInput>,
    last_command_result: Option<ViewportCommandResult>,
}

impl AppViewportState {
    #[cfg(feature = "native-shell")]
    pub(crate) fn viewport_id(&self) -> u64 {
        self.inner.id.get()
    }

    pub(crate) fn camera(&self) -> ViewportCamera {
        self.camera
    }

    #[cfg(feature = "native-shell")]
    pub(crate) fn pan_camera_by(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan_by(delta_x, delta_y);
        self.pending_camera_input.combine(ViewportCameraInput {
            pan_delta_x: delta_x,
            pan_delta_y: delta_y,
            ..ViewportCameraInput::neutral()
        });
    }

    #[cfg(feature = "native-shell")]
    pub(crate) fn orbit_camera_by(&mut self, delta_x: f32, delta_y: f32) {
        // Local editor camera is pan/zoom only; track orbit as pan so pick UVs stay aligned.
        self.camera.pan_by(delta_x, delta_y);
        self.pending_camera_input.combine(ViewportCameraInput {
            orbit_delta_x: delta_x,
            orbit_delta_y: delta_y,
            ..ViewportCameraInput::neutral()
        });
    }

    #[cfg(feature = "native-shell")]
    pub(crate) fn zoom_camera_by(&mut self, factor: f32) {
        self.camera.zoom_by(factor);
        self.pending_camera_input.combine(ViewportCameraInput {
            dolly_factor: factor,
            ..ViewportCameraInput::neutral()
        });
    }

    #[cfg(feature = "native-shell")]
    pub(crate) fn set_editor_input(&mut self, input: ViewportEditorInput) {
        self.pending_editor_input = Some(input);
    }

    pub(crate) fn execute_command_id(
        &mut self,
        id: &str,
        adapter_state: &mut AdapterState,
    ) -> Option<ViewportCommandResult> {
        self.execute_command_id_with_size(
            id,
            adapter_state,
            ViewportFrameRequestSize::default_editor(),
        )
    }

    pub(crate) fn execute_command_id_with_size(
        &mut self,
        id: &str,
        adapter_state: &mut AdapterState,
        request_size: ViewportFrameRequestSize,
    ) -> Option<ViewportCommandResult> {
        let command = ViewportCommand::from_id(id)?;
        let result = match command {
            ViewportCommand::RequestFrame => self.request_frame(adapter_state, request_size),
            ViewportCommand::Clear => self.clear(),
            ViewportCommand::ShowStatus => self.show_status(),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    pub(crate) fn on_adapter_connected(&mut self, adapter_id: &str, supports_preview: bool) {
        if supports_preview {
            self.inner.set_adapter_source(adapter_id);
        }
    }

    pub(crate) fn on_adapter_disconnected(&mut self) {
        self.inner.clear_source();
    }

    #[cfg_attr(all(feature = "native-shell", not(test)), allow(dead_code))]
    pub(crate) fn state(&self) -> &elcarax_core::ViewportState {
        &self.inner
    }

    pub(crate) fn ui_snapshot(&self) -> ViewportUiSnapshot {
        viewport_ui_snapshot(&self.inner, self.last_command_result.as_ref())
    }
    pub(crate) fn apply_host_response(
        &mut self,
        response: GetViewportFrameResponse,
    ) -> Result<(), ViewportError> {
        if response.status != ViewportFrameResponseStatus::Available {
            let message = response
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "adapter viewport frame unavailable".to_string());
            self.inner
                .apply_error(ViewportError::AdapterFailed(message));
            return Err(ViewportError::AdapterFailed(
                self.inner
                    .last_diagnostic
                    .as_ref()
                    .map(|diagnostic| diagnostic.message.clone())
                    .unwrap_or_default(),
            ));
        }
        let frame = ViewportFrame::new(
            response.width,
            response.height,
            response.format,
            response.pixels,
        )?;
        self.inner.apply_frame(frame)
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn request_frame_from_host(
        &mut self,
        host: &mut elcarax_adapter_host::AdapterHost,
        width: u32,
        height: u32,
    ) -> Result<ViewportCommandResult, ViewportError> {
        if let Err(error) = self.inner.begin_frame_request() {
            return Ok(ViewportCommandResult::new(
                VIEWPORT_REQUEST_FRAME_COMMAND,
                error.to_string(),
            ));
        }
        let request = GetViewportFrameRequest {
            viewport_id: AdapterViewportId(self.inner.id.get()),
            scene_id: None,
            width,
            height,
            format: ViewportFrameFormat::Rgba8Unorm,
            camera_input: None,
            editor_input: None,
        };
        match host.get_viewport_frame(request) {
            Ok(response) => {
                if let Err(error) = self.apply_host_response(response.clone()) {
                    return Ok(ViewportCommandResult::new(
                        VIEWPORT_REQUEST_FRAME_COMMAND,
                        error.to_string(),
                    ));
                }
                Ok(ViewportCommandResult::new(
                    VIEWPORT_REQUEST_FRAME_COMMAND,
                    format!(
                        "viewport frame {}x{} {}",
                        response.width,
                        response.height,
                        response.format_label()
                    ),
                ))
            }
            Err(error) => {
                self.inner
                    .apply_error(ViewportError::AdapterFailed(error.to_string()));
                Ok(ViewportCommandResult::new(
                    VIEWPORT_REQUEST_FRAME_COMMAND,
                    format!("Diagnostic: {error}"),
                ))
            }
        }
    }

    fn request_frame(
        &mut self,
        adapter_state: &mut AdapterState,
        request_size: ViewportFrameRequestSize,
    ) -> ViewportCommandResult {
        let camera_input = self.take_pending_camera_input();
        let editor_input = self.pending_editor_input.take();
        match adapter_state.request_viewport_frame(
            &mut self.inner,
            request_size,
            camera_input,
            editor_input,
        ) {
            Ok(message) => ViewportCommandResult::new(VIEWPORT_REQUEST_FRAME_COMMAND, message),
            Err(error) => ViewportCommandResult::new(VIEWPORT_REQUEST_FRAME_COMMAND, error),
        }
    }

    fn take_pending_camera_input(&mut self) -> Option<ViewportCameraInput> {
        if self.pending_camera_input.is_neutral() {
            return None;
        }
        let input = self.pending_camera_input;
        self.pending_camera_input = ViewportCameraInput::neutral();
        Some(input)
    }

    fn clear(&mut self) -> ViewportCommandResult {
        self.inner.clear_frame();
        self.camera.reset();
        self.pending_camera_input = ViewportCameraInput::neutral();
        self.pending_editor_input = None;
        ViewportCommandResult::new(VIEWPORT_CLEAR_COMMAND, "viewport cleared")
    }

    fn show_status(&mut self) -> ViewportCommandResult {
        let message = format!(
            "viewport status={:?} source={}",
            self.inner.status,
            match &self.inner.source {
                ViewportSource::None => "none".to_string(),
                ViewportSource::Adapter(id) => id.clone(),
            }
        );
        ViewportCommandResult::new(VIEWPORT_SHOW_STATUS_COMMAND, message)
    }
}

impl Default for AppViewportState {
    fn default() -> Self {
        Self {
            inner: elcarax_core::ViewportState::default_editor(),
            camera: ViewportCamera::default_editor(),
            pending_camera_input: ViewportCameraInput::neutral(),
            pending_editor_input: None,
            last_command_result: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ViewportCommandResult {
    command_id: String,
    message: String,
}

impl ViewportCommandResult {
    fn new(command_id: &str, message: impl Into<String>) -> Self {
        Self {
            command_id: command_id.to_string(),
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        self.message.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewportCommand {
    RequestFrame,
    Clear,
    ShowStatus,
}

impl ViewportCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            VIEWPORT_REQUEST_FRAME_COMMAND => Some(Self::RequestFrame),
            VIEWPORT_CLEAR_COMMAND => Some(Self::Clear),
            VIEWPORT_SHOW_STATUS_COMMAND => Some(Self::ShowStatus),
            _ => None,
        }
    }
}

#[cfg_attr(feature = "native-shell", allow(dead_code))]
trait ViewportFrameFormatLabel {
    fn format_label(&self) -> &'static str;
}

impl ViewportFrameFormatLabel for GetViewportFrameResponse {
    fn format_label(&self) -> &'static str {
        match self.format {
            ViewportFrameFormat::Rgba8Unorm => "Rgba8Unorm",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_adapter_api::{
        AdapterCapabilities, AdapterId, AdapterName, AdapterRequestId, AdapterResponseMessage,
        AdapterVersion, HandshakeResponse, ProtocolVersion,
    };
    use elcarax_adapter_host::{AdapterSession, FakeAdapterTransport, response_line};
    use elcarax_core::{ViewportFrameFormat, ViewportStatus};

    #[test]
    fn request_frame_without_adapter_fails_clearly() {
        let mut viewport = AppViewportState::default();
        let mut adapter = AdapterState::default();
        let result = match viewport.execute_command_id(VIEWPORT_REQUEST_FRAME_COMMAND, &mut adapter)
        {
            Some(result) => result,
            None => panic!("command should run"),
        };
        assert!(result.message().contains("No adapter connected"));
        assert_eq!(viewport.state().status, ViewportStatus::NoSource);
    }

    #[test]
    fn request_frame_with_fake_adapter_updates_viewport_state() {
        let mut viewport = AppViewportState::default();
        let mut adapter = adapter_with_viewport_response();
        viewport.on_adapter_connected("fixture-adapter", true);
        let result = match viewport.execute_command_id(VIEWPORT_REQUEST_FRAME_COMMAND, &mut adapter)
        {
            Some(result) => result,
            None => panic!("command should run"),
        };
        assert!(result.message().contains("viewport frame"));
        assert_eq!(viewport.state().status, ViewportStatus::FrameAvailable);
    }

    #[test]
    fn clear_clears_frame_status() {
        let mut viewport = AppViewportState::default();
        let mut adapter = adapter_with_viewport_response();
        viewport.on_adapter_connected("fixture-adapter", true);
        let _ = viewport.execute_command_id(VIEWPORT_REQUEST_FRAME_COMMAND, &mut adapter);
        let _ = viewport.execute_command_id(VIEWPORT_CLEAR_COMMAND, &mut adapter);
        assert_eq!(viewport.state().status, ViewportStatus::WaitingForFrame);
        assert!(viewport.state().frame.is_none());
    }

    #[test]
    fn adapter_disconnect_clears_viewport_source() {
        let mut viewport = AppViewportState::default();
        viewport.on_adapter_connected("fixture-adapter", true);
        viewport.on_adapter_disconnected();
        assert_eq!(viewport.state().status, ViewportStatus::NoSource);
    }

    #[cfg(feature = "native-shell")]
    #[test]
    fn pan_camera_updates_local_camera_for_pick_layout() {
        let mut viewport = AppViewportState::default();
        viewport.pan_camera_by(12.0, -4.0);
        assert_eq!(viewport.camera().pan_x, 12.0);
        assert_eq!(viewport.camera().pan_y, -4.0);
        viewport.zoom_camera_by(2.0);
        assert_eq!(viewport.camera().zoom, 2.0);
    }

    fn adapter_with_viewport_response() -> AdapterState {
        let response = GetViewportFrameResponse {
            viewport_id: AdapterViewportId(1),
            width: 2,
            height: 2,
            format: ViewportFrameFormat::Rgba8Unorm,
            pixels: vec![0; 16],
            diagnostics: Vec::new(),
            status: ViewportFrameResponseStatus::Available,
        };
        let handshake = match response_line(
            AdapterRequestId(1),
            AdapterResponseMessage::Handshake(HandshakeResponse {
                adapter_id: AdapterId::new("fixture-adapter"),
                adapter_name: AdapterName::new("Fixture"),
                adapter_version: AdapterVersion::new("0.1.0"),
                protocol_version: ProtocolVersion::V0,
                capabilities: AdapterCapabilities {
                    provides_project_info: true,
                    provides_scene_snapshot: true,
                    provides_diagnostics: true,
                    supports_property_writeback: true,
                    supports_viewport_preview: true,
                    supports_viewport_picking: true,
                },
            }),
        ) {
            Ok(line) => line,
            Err(error) => panic!("handshake line should serialize: {error}"),
        };
        let frame = match response_line(
            AdapterRequestId(2),
            AdapterResponseMessage::GetViewportFrame(response),
        ) {
            Ok(line) => line,
            Err(error) => panic!("frame line should serialize: {error}"),
        };
        let mut adapter = AdapterState::default();
        adapter.attach_fake_session_for_tests(AdapterSession::new(FakeAdapterTransport::new(
            vec![handshake, frame],
        )));
        let _ = adapter.handshake_for_tests();
        adapter
    }
}
