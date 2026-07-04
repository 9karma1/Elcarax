use elcarax_core::{ViewportState, ViewportStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ViewportUiSnapshot {
    pub title: String,
    pub message: String,
    pub hint: String,
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
    let hint = viewport_hint_for_status(state.status);
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
        hint: hint.to_string(),
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

fn viewport_hint_for_status(status: ViewportStatus) -> &'static str {
    match status {
        ViewportStatus::NoSource => {
            "Open a project, connect an adapter, then run viewport.request_frame"
        }
        ViewportStatus::WaitingForFrame => {
            "Run viewport.request_frame from the command palette to load a preview"
        }
        ViewportStatus::FrameAvailable => {
            "Scroll to zoom, Alt+drag to pan, click to select scene objects"
        }
        ViewportStatus::Error => "Check adapter diagnostics and retry viewport.request_frame",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_core::{ViewportFrame, ViewportFrameFormat};

    #[test]
    fn no_source_snapshot_uses_actionable_hint() {
        let snapshot = viewport_ui_snapshot(&ViewportState::default_editor(), None);
        assert_eq!(snapshot.message, "No viewport source");
        assert!(snapshot.hint.contains("viewport.request_frame"));
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
        assert!(snapshot.hint.contains("Scroll to zoom"));
    }
}
