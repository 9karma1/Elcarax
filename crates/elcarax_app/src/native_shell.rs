use std::time::{Duration, Instant};

use elcarax_core::{ElcaraxError, Result};
use elcarax_gpu::{GpuContext, GpuContextSpec, GpuSurface, RenderError, SurfaceSize};
use elcarax_platform::{
    ElementState, MouseButton, NativeApp, NativeAppError, NativeAppHandler, NativeShellSpec,
    PlatformEvent, run_native_app,
};
use elcarax_render::{Rect, RenderScene, Renderer, RendererConfig, RendererError};
use elcarax_ui::{
    EditorShellIds, KeyboardKey, ModifierState, PaintContext, PointerButton, PointerPosition,
    Theme, UiContext, UiEvent, UiInputEvent, UiTree, build_editor_shell_with_ids,
};

pub fn run_native_shell() -> Result<()> {
    println!("Elcarax native shell: starting");
    run_native_app(NativeShellSpec::default_editor(), ShellState::default())
        .map_err(|error| ElcaraxError::Internal(error.to_string()))
}

#[derive(Default)]
struct ShellState {
    gpu: Option<GpuState>,
    ui: Option<UiState>,
}

struct GpuState {
    context: GpuContext,
    surface: GpuSurface<'static>,
    renderer: Renderer,
    last_stats_log: Option<Instant>,
}

struct UiState {
    tree: UiTree,
    ids: EditorShellIds,
    theme: Theme,
    scene: RenderScene,
    scene_dirty: bool,
}

impl NativeAppHandler for ShellState {
    fn resumed(&mut self, app: &NativeApp) -> std::result::Result<(), NativeAppError> {
        if self.gpu.is_some() {
            return Ok(());
        }
        println!("Elcarax native shell: window created");
        let window = app.window();
        let size = window.inner_size();
        let surface_size = SurfaceSize::new(size.width, size.height);
        let (context, surface) = pollster::block_on(GpuContext::for_window(
            window,
            surface_size,
            &GpuContextSpec::editor_default(),
        ))
        .map_err(to_native_gpu_error)?;
        let renderer = Renderer::new(&context, &surface, RendererConfig::default())
            .map_err(to_native_renderer_error)?;
        let theme = Theme::editor_dark();
        let ui = build_ui_state(theme, size.width as f32, size.height as f32)?;
        println!("Elcarax native shell: GPU renderer initialized");
        self.gpu = Some(GpuState {
            context,
            surface,
            renderer,
            last_stats_log: None,
        });
        self.ui = Some(ui);
        app.request_redraw();
        Ok(())
    }

    fn event(
        &mut self,
        event: PlatformEvent,
        app: &NativeApp,
    ) -> std::result::Result<(), NativeAppError> {
        match event {
            PlatformEvent::CloseRequested => println!("Elcarax native shell: close requested"),
            PlatformEvent::Resized(size) => self.resize(size.width, size.height, app)?,
            PlatformEvent::ScaleFactorChanged { .. } => {
                let size = app.window().inner_size();
                self.resize(size.width, size.height, app)?;
            }
            PlatformEvent::RedrawRequested => self.render(app)?,
            PlatformEvent::KeyboardInput(_)
            | PlatformEvent::PointerMoved { .. }
            | PlatformEvent::PointerEntered
            | PlatformEvent::PointerLeft
            | PlatformEvent::MouseInput { .. }
            | PlatformEvent::MouseWheel { .. }
            | PlatformEvent::ModifiersChanged(_)
            | PlatformEvent::WindowFocused
            | PlatformEvent::WindowUnfocused => self.handle_platform_input(event, app)?,
        }
        Ok(())
    }
}

impl ShellState {
    fn resize(
        &mut self,
        width: u32,
        height: u32,
        app: &NativeApp,
    ) -> std::result::Result<(), NativeAppError> {
        if let Some(gpu) = &mut self.gpu {
            gpu.surface.resize(SurfaceSize::new(width, height));
        }
        if let Some(ui) = &mut self.ui {
            let bounds = Rect::new(0.0, 0.0, width as f32, height as f32);
            ui.tree
                .resize_root(bounds)
                .and_then(|()| ui.tree.layout(elcarax_ui::LayoutConstraints { bounds }))
                .map_err(|error| NativeAppError::Window(format!("failed to resize UI: {error}")))?;
            ui.scene_dirty = true;
        }
        app.request_redraw();
        Ok(())
    }

    fn render(&mut self, app: &NativeApp) -> std::result::Result<(), NativeAppError> {
        let Some(gpu) = &mut self.gpu else {
            return Ok(());
        };
        let Some(ui) = &mut self.ui else {
            return Ok(());
        };
        if ui.scene_dirty {
            ui.scene = ui
                .tree
                .paint(&PaintContext::new(ui.theme))
                .map_err(|error| NativeAppError::Window(format!("failed to paint UI: {error}")))?;
            ui.scene_dirty = false;
        }
        gpu.context.keep_alive();
        match gpu.renderer.render(&mut gpu.surface, &ui.scene) {
            Ok(()) => {
                log_stats_periodically(gpu);
                Ok(())
            }
            Err(RendererError::Gpu(RenderError::SurfaceLost)) => {
                let size = app.window().inner_size();
                gpu.surface
                    .resize(SurfaceSize::new(size.width, size.height));
                app.request_redraw();
                Ok(())
            }
            Err(error) => Err(to_native_renderer_error(error)),
        }
    }

    fn handle_platform_input(
        &mut self,
        event: PlatformEvent,
        app: &NativeApp,
    ) -> std::result::Result<(), NativeAppError> {
        let Some(input) = platform_to_ui_input(event) else {
            return Ok(());
        };
        let Some(ui) = &mut self.ui else {
            return Ok(());
        };
        let events = ui.tree.process_input(input).map_err(|error| {
            NativeAppError::Window(format!("failed to process UI input: {error}"))
        })?;
        if apply_ui_events(ui, &events)? || events_affect_paint(&events) {
            ui.scene_dirty = true;
            app.request_redraw();
        }
        Ok(())
    }
}

fn build_ui_state(
    theme: Theme,
    width: f32,
    height: f32,
) -> std::result::Result<UiState, NativeAppError> {
    let context = UiContext::new(theme, Rect::new(0.0, 0.0, width, height));
    let shell = build_editor_shell_with_ids(&context)
        .map_err(|error| NativeAppError::Window(format!("failed to build UI shell: {error}")))?;
    let scene = shell
        .tree
        .paint(&PaintContext::new(theme))
        .map_err(|error| NativeAppError::Window(format!("failed to paint UI shell: {error}")))?;
    Ok(UiState {
        tree: shell.tree,
        ids: shell.ids,
        theme,
        scene,
        scene_dirty: false,
    })
}

fn platform_to_ui_input(event: PlatformEvent) -> Option<UiInputEvent> {
    match event {
        PlatformEvent::PointerMoved { x, y } => Some(UiInputEvent::PointerMoved(
            PointerPosition::new(x as f32, y as f32),
        )),
        PlatformEvent::PointerEntered => Some(UiInputEvent::PointerEntered),
        PlatformEvent::PointerLeft => Some(UiInputEvent::PointerLeft),
        PlatformEvent::MouseInput { button, state } => pointer_button_event(button, state),
        PlatformEvent::MouseWheel { delta_x, delta_y } => Some(UiInputEvent::MouseWheel {
            delta_x: delta_x as f32,
            delta_y: delta_y as f32,
        }),
        PlatformEvent::KeyboardInput(input) => {
            let key = KeyboardKey::from_platform_key(input.key);
            match input.state {
                ElementState::Pressed => Some(UiInputEvent::KeyPressed(key)),
                ElementState::Released => Some(UiInputEvent::KeyReleased(key)),
            }
        }
        PlatformEvent::ModifiersChanged(modifiers) => {
            Some(UiInputEvent::ModifiersChanged(ModifierState {
                shift: modifiers.shift,
                control: modifiers.control,
                alt: modifiers.alt,
                super_key: modifiers.super_key,
            }))
        }
        PlatformEvent::WindowFocused => Some(UiInputEvent::WindowFocused),
        PlatformEvent::WindowUnfocused => Some(UiInputEvent::WindowUnfocused),
        PlatformEvent::CloseRequested
        | PlatformEvent::RedrawRequested
        | PlatformEvent::Resized(_)
        | PlatformEvent::ScaleFactorChanged { .. } => None,
    }
}

fn pointer_button_event(button: MouseButton, state: ElementState) -> Option<UiInputEvent> {
    let button = match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Back => PointerButton::Back,
        MouseButton::Forward => PointerButton::Forward,
        MouseButton::Other(value) => PointerButton::Other(value),
    };
    match state {
        ElementState::Pressed => Some(UiInputEvent::PointerButtonPressed(button)),
        ElementState::Released => Some(UiInputEvent::PointerButtonReleased(button)),
    }
}

fn apply_ui_events(
    ui: &mut UiState,
    events: &[UiEvent],
) -> std::result::Result<bool, NativeAppError> {
    let mut changed = false;
    for event in events {
        if matches!(event, UiEvent::Clicked { id } if *id == ui.ids.run_button) {
            ui.tree
                .set_label_text(ui.ids.status_label, "Status: Run clicked")
                .map_err(|error| {
                    NativeAppError::Window(format!("failed to update status: {error}"))
                })?;
            changed = true;
        }
    }
    Ok(changed)
}

fn events_affect_paint(events: &[UiEvent]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            UiEvent::HoverChanged { .. }
                | UiEvent::FocusChanged(_)
                | UiEvent::ActiveChanged { .. }
                | UiEvent::Clicked { .. }
        )
    })
}

fn log_stats_periodically(gpu: &mut GpuState) {
    let now = Instant::now();
    if gpu
        .last_stats_log
        .is_some_and(|last_log| now.duration_since(last_log) < Duration::from_secs(5))
    {
        return;
    }
    gpu.last_stats_log = Some(now);
    let stats = gpu.renderer.stats();
    println!(
        "Elcarax render stats: primitives={}, batches={}, uploaded_bytes={}, frames={}",
        stats.primitive_count, stats.batch_count, stats.uploaded_bytes, stats.frame_count
    );
}

fn to_native_gpu_error(error: RenderError) -> NativeAppError {
    NativeAppError::Window(error.to_string())
}

fn to_native_renderer_error(error: RendererError) -> NativeAppError {
    NativeAppError::Window(error.to_string())
}
