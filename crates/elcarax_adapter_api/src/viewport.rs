use serde::{Deserialize, Serialize};

use elcarax_core::{Severity, ViewportFrameFormat};
use elcarax_scene_model::SceneObjectId;

use crate::AdapterDiagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterViewportId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewportCameraInput {
    pub orbit_delta_x: f32,
    pub orbit_delta_y: f32,
    pub pan_delta_x: f32,
    pub pan_delta_y: f32,
    pub dolly_factor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewportEditorInput {
    pub pointer_x: f32,
    pub pointer_y: f32,
    pub primary_down: bool,
    pub secondary_down: bool,
    pub middle_down: bool,
    pub wheel_delta_y: f32,
}

impl ViewportEditorInput {
    pub const fn pointer(
        pointer_x: f32,
        pointer_y: f32,
        primary_down: bool,
        secondary_down: bool,
        middle_down: bool,
    ) -> Self {
        Self {
            pointer_x,
            pointer_y,
            primary_down,
            secondary_down,
            middle_down,
            wheel_delta_y: 0.0,
        }
    }
}

impl ViewportCameraInput {
    pub const fn neutral() -> Self {
        Self {
            orbit_delta_x: 0.0,
            orbit_delta_y: 0.0,
            pan_delta_x: 0.0,
            pan_delta_y: 0.0,
            dolly_factor: 1.0,
        }
    }

    pub fn is_neutral(self) -> bool {
        self.orbit_delta_x == 0.0
            && self.orbit_delta_y == 0.0
            && self.pan_delta_x == 0.0
            && self.pan_delta_y == 0.0
            && self.dolly_factor == 1.0
    }

    pub fn combine(&mut self, input: Self) {
        self.orbit_delta_x += input.orbit_delta_x;
        self.orbit_delta_y += input.orbit_delta_y;
        self.pan_delta_x += input.pan_delta_x;
        self.pan_delta_y += input.pan_delta_y;
        self.dolly_factor *= input.dolly_factor;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetViewportFrameRequest {
    pub viewport_id: AdapterViewportId,
    pub scene_id: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub format: ViewportFrameFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_input: Option<ViewportCameraInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_input: Option<ViewportEditorInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportFrameResponseStatus {
    Available,
    NoSceneLoaded,
    InvalidSize,
    UnsupportedFormat,
    AdapterError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetViewportFrameResponse {
    pub viewport_id: AdapterViewportId,
    pub width: u32,
    pub height: u32,
    pub format: ViewportFrameFormat,
    /// Declared binary payload length; pixel bytes travel in the frame binary segment.
    pub byte_len: u32,
    #[serde(skip)]
    pub pixels: Vec<u8>,
    pub diagnostics: Vec<AdapterDiagnostic>,
    pub status: ViewportFrameResponseStatus,
}

impl GetViewportFrameResponse {
    pub fn available(
        viewport_id: AdapterViewportId,
        width: u32,
        height: u32,
        format: ViewportFrameFormat,
        pixels: Vec<u8>,
        diagnostics: Vec<AdapterDiagnostic>,
    ) -> Self {
        let byte_len = u32::try_from(pixels.len()).unwrap_or(u32::MAX);
        Self {
            viewport_id,
            width,
            height,
            format,
            byte_len,
            pixels,
            diagnostics,
            status: ViewportFrameResponseStatus::Available,
        }
    }

    pub fn failed(
        viewport_id: AdapterViewportId,
        status: ViewportFrameResponseStatus,
        message: impl Into<String>,
    ) -> Self {
        Self {
            viewport_id,
            width: 0,
            height: 0,
            format: ViewportFrameFormat::Rgba8Unorm,
            byte_len: 0,
            pixels: Vec::new(),
            diagnostics: vec![AdapterDiagnostic {
                severity: Severity::Error,
                source: "adapter".to_string(),
                message: message.into(),
            }],
            status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PickViewportObjectRequest {
    pub viewport_id: AdapterViewportId,
    pub scene_id: Option<u64>,
    pub u: f32,
    pub v: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportPickResponseStatus {
    Picked,
    Missed,
    NoSceneLoaded,
    InvalidCoordinate,
    AdapterError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PickViewportObjectResponse {
    pub viewport_id: AdapterViewportId,
    pub object_id: Option<SceneObjectId>,
    pub diagnostics: Vec<AdapterDiagnostic>,
    pub status: ViewportPickResponseStatus,
}

impl PickViewportObjectResponse {
    pub fn failed(
        viewport_id: AdapterViewportId,
        status: ViewportPickResponseStatus,
        message: impl Into<String>,
    ) -> Self {
        Self {
            viewport_id,
            object_id: None,
            diagnostics: vec![AdapterDiagnostic {
                severity: Severity::Error,
                source: "adapter".to_string(),
                message: message.into(),
            }],
            status,
        }
    }
}
