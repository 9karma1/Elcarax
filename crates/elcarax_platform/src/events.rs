#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl WindowSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn is_drawable(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierState {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl ModifierState {
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        alt: false,
        super_key: false,
    };
}

impl Default for ModifierState {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    pub key: String,
    pub state: ElementState,
}

impl KeyInput {
    pub fn new(key: impl Into<String>, state: ElementState) -> Self {
        Self {
            key: key.into(),
            state,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlatformEvent {
    CloseRequested,
    RedrawRequested,
    Resized(WindowSize),
    ScaleFactorChanged {
        scale_factor: f64,
    },
    KeyboardInput(KeyInput),
    PointerMoved {
        x: f64,
        y: f64,
    },
    PointerEntered,
    PointerLeft,
    MouseInput {
        button: MouseButton,
        state: ElementState,
    },
    MouseWheel {
        delta_x: f64,
        delta_y: f64,
    },
    ModifiersChanged(ModifierState),
    WindowFocused,
    WindowUnfocused,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sized_window_is_not_drawable() {
        assert!(!WindowSize::new(0, 480).is_drawable());
        assert!(WindowSize::new(640, 480).is_drawable());
    }

    #[test]
    fn key_input_keeps_platform_neutral_state() {
        let input = KeyInput::new("A", ElementState::Pressed);
        assert_eq!(input.key, "A");
        assert_eq!(input.state, ElementState::Pressed);
    }

    #[test]
    fn modifier_state_can_represent_no_modifiers() {
        assert_eq!(
            ModifierState::NONE,
            ModifierState {
                shift: false,
                control: false,
                alt: false,
                super_key: false,
            }
        );
    }
}
