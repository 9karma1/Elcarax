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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramePolicy {
    WaitWhenIdle,
    Continuous,
}
