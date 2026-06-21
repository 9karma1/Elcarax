use std::{error::Error, fmt, sync::Arc};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState as WinitElementState, MouseButton as WinitMouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::Key,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{ElementState, KeyInput, MouseButton, NativeShellSpec, PlatformEvent, WindowSize};

#[derive(Debug)]
pub enum NativeAppError {
    EventLoop(String),
    Window(String),
}

impl fmt::Display for NativeAppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventLoop(message) => write!(formatter, "event loop error: {message}"),
            Self::Window(message) => write!(formatter, "window error: {message}"),
        }
    }
}

impl Error for NativeAppError {}

pub trait NativeAppHandler {
    fn resumed(&mut self, app: &NativeApp) -> Result<(), NativeAppError>;
    fn event(&mut self, event: PlatformEvent, app: &NativeApp) -> Result<(), NativeAppError>;
}

pub struct NativeApp {
    window: Arc<Window>,
}

impl NativeApp {
    pub fn window(&self) -> Arc<Window> {
        Arc::clone(&self.window)
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

pub fn run_native_app<H: NativeAppHandler + 'static>(
    spec: NativeShellSpec,
    handler: H,
) -> Result<(), NativeAppError> {
    let event_loop =
        EventLoop::new().map_err(|error| NativeAppError::EventLoop(error.to_string()))?;
    let mut adapter = WinitAppAdapter {
        spec,
        handler,
        app: None,
        failed: None,
    };
    event_loop
        .run_app(&mut adapter)
        .map_err(|error| NativeAppError::EventLoop(error.to_string()))?;
    if let Some(error) = adapter.failed {
        return Err(error);
    }
    Ok(())
}

struct WinitAppAdapter<H> {
    spec: NativeShellSpec,
    handler: H,
    app: Option<NativeApp>,
    failed: Option<NativeAppError>,
}

impl<H: NativeAppHandler> ApplicationHandler for WinitAppAdapter<H> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_none() {
            match create_window(event_loop, &self.spec) {
                Ok(window) => self.app = Some(NativeApp { window }),
                Err(error) => {
                    self.fail(event_loop, error);
                    return;
                }
            }
        }
        if let Some(app) = &self.app
            && let Err(error) = self.handler.resumed(app)
        {
            self.fail(event_loop, error);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(app) = &self.app else {
            return;
        };
        if app.window.id() != window_id {
            return;
        }
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
        }
        if let Some(event) = translate_window_event(event)
            && let Err(error) = self.handler.event(event, app)
        {
            self.fail(event_loop, error);
        }
    }
}

impl<H> WinitAppAdapter<H> {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: NativeAppError) {
        self.failed = Some(error);
        event_loop.exit();
    }
}

fn create_window(
    event_loop: &ActiveEventLoop,
    spec: &NativeShellSpec,
) -> Result<Arc<Window>, NativeAppError> {
    let attributes = WindowAttributes::default()
        .with_title(spec.title.clone())
        .with_inner_size(LogicalSize::new(
            f64::from(spec.width),
            f64::from(spec.height),
        ));
    event_loop
        .create_window(attributes)
        .map(Arc::new)
        .map_err(|error| NativeAppError::Window(error.to_string()))
}

fn translate_window_event(event: WindowEvent) -> Option<PlatformEvent> {
    match event {
        WindowEvent::CloseRequested => Some(PlatformEvent::CloseRequested),
        WindowEvent::RedrawRequested => Some(PlatformEvent::RedrawRequested),
        WindowEvent::Resized(size) => Some(PlatformEvent::Resized(WindowSize::new(
            size.width,
            size.height,
        ))),
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            Some(PlatformEvent::ScaleFactorChanged { scale_factor })
        }
        WindowEvent::CursorMoved { position, .. } => Some(PlatformEvent::PointerMoved {
            x: position.x,
            y: position.y,
        }),
        WindowEvent::MouseInput { state, button, .. } => Some(PlatformEvent::MouseInput {
            button: translate_mouse_button(button),
            state: translate_element_state(state),
        }),
        WindowEvent::KeyboardInput { event, .. } => {
            Some(PlatformEvent::KeyboardInput(KeyInput::new(
                translate_key(event.logical_key),
                translate_element_state(event.state),
            )))
        }
        _ => None,
    }
}

fn translate_element_state(state: WinitElementState) -> ElementState {
    match state {
        WinitElementState::Pressed => ElementState::Pressed,
        WinitElementState::Released => ElementState::Released,
    }
}

fn translate_mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(value) => MouseButton::Other(value),
    }
}

fn translate_key(key: Key) -> String {
    match key {
        Key::Character(value) => value.to_string(),
        other => format!("{other:?}"),
    }
}
