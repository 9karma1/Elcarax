//! Platform abstraction for windows, input, frame scheduling, and shell events.

mod events;
#[cfg(feature = "native")]
mod native;
mod spec;

pub use events::{ElementState, KeyInput, MouseButton, PlatformEvent, WindowSize};
pub use spec::{FramePolicy, NativeShellSpec};

#[cfg(feature = "native")]
pub use native::{NativeApp, NativeAppError, NativeAppHandler, run_native_app};
