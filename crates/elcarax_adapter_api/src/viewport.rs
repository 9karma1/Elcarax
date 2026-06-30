use serde::{Deserialize, Serialize};

use elcarax_core::{Severity, ViewportFrameFormat};

use crate::AdapterDiagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterViewportId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetViewportFrameRequest {
    pub viewport_id: AdapterViewportId,
    pub scene_id: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub format: ViewportFrameFormat,
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
    pub pixels: Vec<u8>,
    pub diagnostics: Vec<AdapterDiagnostic>,
    pub status: ViewportFrameResponseStatus,
}

impl GetViewportFrameResponse {
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
