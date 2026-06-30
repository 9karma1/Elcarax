//! Engine-neutral viewport state and frame types.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, DiagnosticSource, Severity};
use crate::id::Id;

pub struct ViewportMarker;

pub type ViewportId = Id<ViewportMarker>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportFrameFormat {
    Rgba8Unorm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewportFrameSize {
    pub width: u32,
    pub height: u32,
}

impl ViewportFrameSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn pixel_count(self) -> usize {
        (self.width as usize).saturating_mul(self.height as usize)
    }

    pub const fn rgba_byte_len(self) -> usize {
        self.pixel_count().saturating_mul(4)
    }

    pub const fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewportFramePixels {
    pub rgba: Vec<u8>,
}

impl ViewportFramePixels {
    pub fn from_rgba8(size: ViewportFrameSize, rgba: Vec<u8>) -> Result<Self, ViewportError> {
        let expected = size.rgba_byte_len();
        if rgba.len() != expected {
            return Err(ViewportError::InvalidPixelLength {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self { rgba })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewportFrame {
    pub size: ViewportFrameSize,
    pub format: ViewportFrameFormat,
    pub pixels: ViewportFramePixels,
}

impl ViewportFrame {
    pub fn new(
        width: u32,
        height: u32,
        format: ViewportFrameFormat,
        rgba: Vec<u8>,
    ) -> Result<Self, ViewportError> {
        let size = ViewportFrameSize::new(width, height);
        if !size.is_valid() {
            return Err(ViewportError::InvalidDimensions { width, height });
        }
        Ok(Self {
            size,
            format,
            pixels: ViewportFramePixels::from_rgba8(size, rgba)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewportSource {
    None,
    Adapter(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportStatus {
    NoSource,
    WaitingForFrame,
    FrameAvailable,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportDiagnostic {
    pub severity: Severity,
    pub message: String,
}

impl ViewportDiagnostic {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }

    pub fn as_core_diagnostic(&self) -> Diagnostic {
        Diagnostic {
            severity: self.severity,
            source: DiagnosticSource::new("viewport"),
            message: self.message.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewportError {
    InvalidDimensions { width: u32, height: u32 },
    InvalidPixelLength { expected: usize, actual: usize },
    NoAdapterConnected,
    AdapterUnsupported,
    AdapterFailed(String),
    Cleared,
}

impl fmt::Display for ViewportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => {
                write!(
                    formatter,
                    "invalid viewport frame dimensions {width}x{height}"
                )
            }
            Self::InvalidPixelLength { expected, actual } => write!(
                formatter,
                "invalid viewport pixel length: expected {expected} bytes, received {actual}"
            ),
            Self::NoAdapterConnected => write!(formatter, "No adapter connected"),
            Self::AdapterUnsupported => {
                write!(formatter, "Adapter does not support viewport preview")
            }
            Self::AdapterFailed(message) => write!(formatter, "adapter viewport error: {message}"),
            Self::Cleared => write!(formatter, "viewport cleared"),
        }
    }
}

impl Error for ViewportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportState {
    pub id: ViewportId,
    pub source: ViewportSource,
    pub status: ViewportStatus,
    pub frame: Option<ViewportFrame>,
    pub last_diagnostic: Option<ViewportDiagnostic>,
}

impl ViewportState {
    pub fn new(id: ViewportId) -> Self {
        Self {
            id,
            source: ViewportSource::None,
            status: ViewportStatus::NoSource,
            frame: None,
            last_diagnostic: None,
        }
    }

    pub fn default_editor() -> Self {
        Self::new(ViewportId::from_non_zero(std::num::NonZeroU64::MIN))
    }

    pub fn set_adapter_source(&mut self, adapter_id: impl Into<String>) {
        self.source = ViewportSource::Adapter(adapter_id.into());
        if self.status == ViewportStatus::NoSource {
            self.status = ViewportStatus::WaitingForFrame;
        }
        self.frame = None;
    }

    pub fn clear_source(&mut self) {
        self.source = ViewportSource::None;
        self.status = ViewportStatus::NoSource;
        self.frame = None;
        self.last_diagnostic = None;
    }

    pub fn begin_frame_request(&mut self) -> Result<(), ViewportError> {
        match self.source {
            ViewportSource::None => {
                let diagnostic =
                    ViewportDiagnostic::error(ViewportError::NoAdapterConnected.to_string());
                self.last_diagnostic = Some(diagnostic);
                self.status = ViewportStatus::NoSource;
                Err(ViewportError::NoAdapterConnected)
            }
            ViewportSource::Adapter(_) => {
                self.status = ViewportStatus::WaitingForFrame;
                self.frame = None;
                Ok(())
            }
        }
    }

    pub fn apply_frame(&mut self, frame: ViewportFrame) -> Result<(), ViewportError> {
        if !frame.size.is_valid() {
            return Err(ViewportError::InvalidDimensions {
                width: frame.size.width,
                height: frame.size.height,
            });
        }
        let expected = frame.size.rgba_byte_len();
        if frame.pixels.rgba.len() != expected {
            return Err(ViewportError::InvalidPixelLength {
                expected,
                actual: frame.pixels.rgba.len(),
            });
        }
        self.frame = Some(frame);
        self.status = ViewportStatus::FrameAvailable;
        self.last_diagnostic = None;
        Ok(())
    }

    pub fn apply_error(&mut self, error: ViewportError) {
        self.status = ViewportStatus::Error;
        self.frame = None;
        self.last_diagnostic = Some(ViewportDiagnostic::error(error.to_string()));
    }

    pub fn clear_frame(&mut self) {
        self.frame = None;
        match self.source {
            ViewportSource::None => {
                self.status = ViewportStatus::NoSource;
            }
            ViewportSource::Adapter(_) => {
                self.status = ViewportStatus::WaitingForFrame;
            }
        }
        self.last_diagnostic = Some(ViewportDiagnostic::info("viewport frame cleared"));
    }

    pub fn status_message(&self) -> &'static str {
        match self.status {
            ViewportStatus::NoSource => "No viewport source",
            ViewportStatus::WaitingForFrame => "Waiting for preview frame",
            ViewportStatus::FrameAvailable => "Adapter Preview",
            ViewportStatus::Error => "Viewport error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_source_state_is_valid() {
        let state = ViewportState::default_editor();
        assert_eq!(state.status, ViewportStatus::NoSource);
        assert!(state.frame.is_none());
    }

    #[test]
    fn waiting_for_frame_transition() {
        let mut state = ViewportState::default_editor();
        state.set_adapter_source("adapter-a");
        assert_eq!(state.status, ViewportStatus::WaitingForFrame);
        assert!(state.begin_frame_request().is_ok());
        assert_eq!(state.status, ViewportStatus::WaitingForFrame);
    }

    #[test]
    fn frame_available_requires_matching_pixels() {
        let mut state = ViewportState::default_editor();
        state.set_adapter_source("adapter-a");
        let frame = match ViewportFrame::new(2, 2, ViewportFrameFormat::Rgba8Unorm, vec![0; 16]) {
            Ok(frame) => frame,
            Err(error) => panic!("valid frame should construct: {error}"),
        };
        assert!(state.apply_frame(frame).is_ok());
        assert_eq!(state.status, ViewportStatus::FrameAvailable);
    }

    #[test]
    fn invalid_pixel_length_returns_error() {
        let result = ViewportFrame::new(2, 2, ViewportFrameFormat::Rgba8Unorm, vec![0; 8]);
        assert!(matches!(
            result,
            Err(ViewportError::InvalidPixelLength { .. })
        ));
    }

    #[test]
    fn clear_returns_expected_state() {
        let mut state = ViewportState::default_editor();
        state.set_adapter_source("adapter-a");
        let frame = match ViewportFrame::new(1, 1, ViewportFrameFormat::Rgba8Unorm, vec![0; 4]) {
            Ok(frame) => frame,
            Err(error) => panic!("valid frame should construct: {error}"),
        };
        let _ = state.apply_frame(frame);
        state.clear_frame();
        assert_eq!(state.status, ViewportStatus::WaitingForFrame);
        assert!(state.frame.is_none());
    }

    #[test]
    fn diagnostics_attach_cleanly() {
        let mut state = ViewportState::default_editor();
        let error = state.begin_frame_request();
        assert!(matches!(error, Err(ViewportError::NoAdapterConnected)));
        let diagnostic = match state.last_diagnostic.as_ref() {
            Some(diagnostic) => diagnostic,
            None => panic!("diagnostic should be set"),
        };
        assert_eq!(diagnostic.severity, Severity::Error);
    }
}
