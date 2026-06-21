//! Platform abstraction for windows, input, frame scheduling, and shell events.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeShellSpec {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl NativeShellSpec {
    pub fn default_editor() -> Self {
        Self {
            title: "Elcarax".to_owned(),
            width: 1440,
            height: 900,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlatformEvent {
    CloseRequested,
    RedrawRequested,
    Resized { width: u32, height: u32 },
    ScaleFactorChanged { scale_factor: f64 },
    KeyboardInput { key: String, pressed: bool },
    PointerMoved { x: f32, y: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramePolicy {
    WaitWhenIdle,
    Continuous,
}
