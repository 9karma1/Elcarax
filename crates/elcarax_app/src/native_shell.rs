use std::time::{Duration, Instant};

use elcarax_commands::{
    CommandId, CommandInvocation, CommandRegistry, CommandResult, RegisteredCommand,
    built_in_commands,
};
use elcarax_core::{ElcaraxError, Result};
use elcarax_gpu::{GpuContext, GpuContextSpec, GpuSurface, RenderError, SurfaceSize};
use elcarax_platform::{
    ElementState, MouseButton, NativeApp, NativeAppError, NativeAppHandler, NativeShellSpec,
    PlatformEvent, run_native_app,
};
use elcarax_render::{Rect, RenderScene, Renderer, RendererConfig, RendererError, text_stats};
use elcarax_ui::{
    CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, EditorShellIds, KeyboardKey,
    LayoutConstraints, ModifierState, PaintContext, PointerButton, PointerPosition, Theme,
    UiContext, UiEvent, UiInputEvent, UiTree, build_editor_shell_with_content,
    paint_command_palette_overlay,
};

use crate::project_state::ProjectState;
use crate::project_ui::{apply_project_snapshot, shell_content_from_project};

pub fn run_native_shell() -> Result<()> {
    println!("Elcarax native shell: starting");
    run_native_app(NativeShellSpec::default_editor(), ShellState::default())
        .map_err(|error| ElcaraxError::Internal(error.to_string()))
}

#[derive(Default)]
struct ShellState {
    gpu: Option<GpuState>,
    ui: Option<UiState>,
    modifiers: elcarax_platform::ModifierState,
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
    command_registry: CommandRegistry,
    command_palette: CommandPaletteState,
    project_state: ProjectState,
    bounds: Rect,
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
            ui.bounds = bounds;
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
            repaint_ui_scene(ui)?;
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
        if let PlatformEvent::ModifiersChanged(modifiers) = event {
            self.modifiers = modifiers;
        }
        let Some(input) = platform_to_ui_input(event) else {
            return Ok(());
        };
        let Some(ui) = &mut self.ui else {
            return Ok(());
        };
        if handle_palette_shortcut(ui, &input, self.modifiers)? {
            app.request_redraw();
            return Ok(());
        }
        if handle_palette_input(ui, &input)? {
            app.request_redraw();
            return Ok(());
        }
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
    let project_state = ProjectState::default();
    let content = shell_content_from_project(&project_state.ui_snapshot());
    let shell = build_editor_shell_with_content(&context, &content)
        .map_err(|error| NativeAppError::Window(format!("failed to build UI shell: {error}")))?;
    let command_registry =
        built_in_commands().map_err(|error| NativeAppError::Window(error.to_string()))?;
    let command_palette =
        CommandPaletteState::new(palette_entries_from_registry(&command_registry));
    let bounds = context.root_bounds;
    let mut ui = UiState {
        tree: shell.tree,
        ids: shell.ids,
        theme,
        scene: RenderScene::new(),
        scene_dirty: true,
        command_registry,
        command_palette,
        project_state,
        bounds,
    };
    repaint_ui_scene(&mut ui)?;
    ui.scene_dirty = false;
    Ok(ui)
}

fn repaint_ui_scene(ui: &mut UiState) -> std::result::Result<(), NativeAppError> {
    let mut scene = ui
        .tree
        .paint(&PaintContext::new(ui.theme))
        .map_err(|error| NativeAppError::Window(format!("failed to paint UI: {error}")))?;
    paint_command_palette_overlay(
        &mut scene,
        &ui.command_palette,
        ui.bounds,
        &PaintContext::new(ui.theme),
    );
    ui.scene = scene;
    Ok(())
}

fn palette_entries_from_registry(registry: &CommandRegistry) -> Vec<CommandPaletteEntry> {
    registry
        .all()
        .into_iter()
        .map(palette_entry_from_command)
        .collect()
}

fn palette_entry_from_command(command: &RegisteredCommand) -> CommandPaletteEntry {
    CommandPaletteEntry::new(
        command.id().as_str(),
        command.name().as_str(),
        command.category().label(),
        command
            .description()
            .map(|description| description.as_str().to_string()),
        command.enabled(),
    )
}

fn handle_palette_shortcut(
    ui: &mut UiState,
    input: &UiInputEvent,
    modifiers: elcarax_platform::ModifierState,
) -> std::result::Result<bool, NativeAppError> {
    if !modifiers.control {
        return Ok(false);
    }
    let UiInputEvent::KeyPressed(key) = input else {
        return Ok(false);
    };
    if !is_command_palette_shortcut(key) {
        return Ok(false);
    }
    open_palette(ui)?;
    Ok(true)
}

fn is_command_palette_shortcut(key: &KeyboardKey) -> bool {
    matches!(key, KeyboardKey::Character(value) if value.eq_ignore_ascii_case("k") || value.eq_ignore_ascii_case("p"))
}

fn handle_palette_input(
    ui: &mut UiState,
    input: &UiInputEvent,
) -> std::result::Result<bool, NativeAppError> {
    if !ui.command_palette.is_open() {
        return Ok(false);
    }
    let UiInputEvent::KeyPressed(key) = input else {
        return Ok(true);
    };
    match ui.command_palette.handle_key(key.clone()) {
        CommandPaletteAction::None => {
            ui.scene_dirty = true;
            Ok(true)
        }
        CommandPaletteAction::Closed => {
            ui.scene_dirty = true;
            Ok(true)
        }
        CommandPaletteAction::Execute => {
            execute_selected_palette_command(ui)?;
            ui.scene_dirty = true;
            Ok(true)
        }
    }
}

fn open_palette(ui: &mut UiState) -> std::result::Result<(), NativeAppError> {
    let open_id = command_id("elcarax.palette.open")?;
    if matches!(
        ui.command_registry.invoke(&open_id),
        CommandResult::Invoked(_)
    ) {
        ui.command_palette
            .replace_entries(palette_entries_from_registry(&ui.command_registry));
        ui.command_palette.open();
        ui.scene_dirty = true;
    }
    Ok(())
}

fn execute_selected_palette_command(ui: &mut UiState) -> std::result::Result<(), NativeAppError> {
    let Some(entry) = ui.command_palette.selected_entry() else {
        return Ok(());
    };
    let id = command_id(entry.id.as_str())?;
    if let CommandResult::Invoked(invocation) = ui.command_registry.invoke(&id) {
        apply_command_invocation(ui, &invocation)?;
    }
    if ui.command_palette.is_open() {
        ui.command_palette.close();
    }
    Ok(())
}

fn apply_command_invocation(
    ui: &mut UiState,
    invocation: &CommandInvocation,
) -> std::result::Result<(), NativeAppError> {
    if ui
        .project_state
        .execute_command_id(invocation.id.as_str())
        .is_some()
    {
        apply_project_snapshot(
            &mut ui.tree,
            ui.ids,
            &ui.project_state.ui_snapshot(),
            ui.bounds,
        )
        .map_err(|error| NativeAppError::Window(format!("failed to update project UI: {error}")))?;
        return Ok(());
    }
    match invocation.id.as_str() {
        "elcarax.palette.open" => ui.command_palette.open(),
        "elcarax.palette.close" => ui.command_palette.close(),
        "elcarax.status.show_renderer_stats" => {
            set_status_text(ui, renderer_stats_status(&ui.scene))?
        }
        "elcarax.status.show_ready" => set_status_text(ui, "Status: Ready".to_string())?,
        "elcarax.demo.run" => set_status_text(ui, "Status: Run clicked".to_string())?,
        _ => {}
    }
    Ok(())
}

fn set_status_text(ui: &mut UiState, text: String) -> std::result::Result<(), NativeAppError> {
    ui.tree
        .set_label_text(ui.ids.status_label, text)
        .map_err(|error| NativeAppError::Window(format!("failed to update status: {error}")))?;
    ui.tree
        .layout(LayoutConstraints { bounds: ui.bounds })
        .map_err(|error| NativeAppError::Window(format!("failed to relayout status: {error}")))?;
    Ok(())
}

fn renderer_stats_status(scene: &RenderScene) -> String {
    let stats = text_stats(scene);
    format!(
        "Status: primitives={} text={} glyphs={}",
        scene.primitives().len(),
        stats.text_primitive_count,
        stats.glyph_count
    )
}

fn command_id(id: &str) -> std::result::Result<CommandId, NativeAppError> {
    CommandId::new(id).map_err(|error| NativeAppError::Window(error.to_string()))
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
            set_status_text(ui, "Status: Run clicked".to_string())?;
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
