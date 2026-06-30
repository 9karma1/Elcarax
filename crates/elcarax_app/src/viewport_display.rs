use elcarax_core::{ViewportState, ViewportStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ViewportUiSnapshot {
    pub title: String,
    pub message: String,
    pub status: ViewportStatus,
    pub show_preview_label: bool,
    pub frame_width: u32,
    pub frame_height: u32,
    pub frame_rgba: Vec<u8>,
    pub command_message: String,
}

pub(crate) fn viewport_ui_snapshot(
    state: &ViewportState,
    last_result: Option<&crate::viewport_state::ViewportCommandResult>,
) -> ViewportUiSnapshot {
    let message = if state.status == ViewportStatus::Error {
        state
            .last_diagnostic
            .as_ref()
            .map(|diagnostic| diagnostic.message.clone())
            .unwrap_or_else(|| state.status_message().to_string())
    } else {
        state.status_message().to_string()
    };
    let (frame_width, frame_height, frame_rgba) = match &state.frame {
        Some(frame) => (
            frame.size.width,
            frame.size.height,
            frame.pixels.rgba.clone(),
        ),
        None => (0, 0, Vec::new()),
    };
    ViewportUiSnapshot {
        title: "Viewport".to_string(),
        message,
        status: state.status,
        show_preview_label: state.status == ViewportStatus::FrameAvailable,
        frame_width,
        frame_height,
        frame_rgba,
        command_message: last_result
            .map(|result| result.message().to_string())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_core::{ViewportFrame, ViewportFrameFormat};

    #[test]
    fn no_source_snapshot_uses_empty_message() {
        let snapshot = viewport_ui_snapshot(&ViewportState::default_editor(), None);
        assert_eq!(snapshot.message, "No viewport source");
    }

    #[test]
    fn frame_available_snapshot_includes_pixels() {
        let mut state = ViewportState::default_editor();
        state.set_adapter_source("adapter-a");
        let frame =
            match ViewportFrame::new(1, 1, ViewportFrameFormat::Rgba8Unorm, vec![1, 2, 3, 4]) {
                Ok(frame) => frame,
                Err(error) => panic!("frame should be valid: {error}"),
            };
        let _ = state.apply_frame(frame);
        let snapshot = viewport_ui_snapshot(&state, None);
        assert_eq!(snapshot.frame_rgba.len(), 4);
        assert!(snapshot.show_preview_label);
    }
}
