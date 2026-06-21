//! GPU boundary types for Elcarax.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackendPreference {
    Auto,
    Vulkan,
    Metal,
    Direct3D12,
    OpenGl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuContextSpec {
    pub backend: GpuBackendPreference,
    pub enable_timestamps: bool,
    pub enable_validation: bool,
}

impl GpuContextSpec {
    pub const fn editor_default() -> Self {
        Self {
            backend: GpuBackendPreference::Auto,
            enable_timestamps: true,
            enable_validation: cfg!(debug_assertions),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    pub cpu_frame_ms: f32,
    pub gpu_frame_ms: Option<f32>,
    pub uploaded_bytes: u64,
}

impl FrameStats {
    pub const fn empty() -> Self {
        Self {
            cpu_frame_ms: 0.0,
            gpu_frame_ms: None,
            uploaded_bytes: 0,
        }
    }
}
