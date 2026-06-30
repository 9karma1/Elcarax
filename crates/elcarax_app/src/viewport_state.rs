use elcarax_adapter_api::{
    AdapterViewportId, GetViewportFrameRequest, GetViewportFrameResponse,
    ViewportFrameResponseStatus,
};
use elcarax_core::{ViewportError, ViewportFrame, ViewportFrameFormat, ViewportSource};

use crate::adapter_state::AdapterState;
use crate::viewport_display::{ViewportUiSnapshot, viewport_ui_snapshot};

pub(crate) const VIEWPORT_REQUEST_FRAME_COMMAND: &str = "viewport.request_frame";
pub(crate) const VIEWPORT_CLEAR_COMMAND: &str = "viewport.clear";
pub(crate) const VIEWPORT_SHOW_STATUS_COMMAND: &str = "viewport.show_status";

pub(crate) struct AppViewportState {
    inner: elcarax_core::ViewportState,
    last_command_result: Option<ViewportCommandResult>,
}

impl AppViewportState {
    pub(crate) fn execute_command_id(
        &mut self,
        id: &str,
        adapter_state: &mut AdapterState,
    ) -> Option<ViewportCommandResult> {
        let command = ViewportCommand::from_id(id)?;
        let result = match command {
            ViewportCommand::RequestFrame => self.request_frame(adapter_state),
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

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
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

    fn request_frame(&mut self, adapter_state: &mut AdapterState) -> ViewportCommandResult {
        match adapter_state.request_viewport_frame(&mut self.inner) {
            Ok(message) => ViewportCommandResult::new(VIEWPORT_REQUEST_FRAME_COMMAND, message),
            Err(error) => ViewportCommandResult::new(VIEWPORT_REQUEST_FRAME_COMMAND, error),
        }
    }

    fn clear(&mut self) -> ViewportCommandResult {
        self.inner.clear_frame();
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
