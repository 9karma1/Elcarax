use std::time::{Duration, Instant};

use elcarax_commands::{
    CommandId, CommandInvocation, CommandRegistry, CommandResult, RegisteredCommand,
    built_in_commands,
};
use elcarax_core::{ElcaraxError, Result};
use elcarax_gpu::{GpuContext, GpuContextSpec, GpuSurface, RenderError, SurfaceSize};
use elcarax_platform::{
    ElementState, MouseButton, NativeApp, NativeAppError, NativeAppHandler, NativeShellSpec,
    PlatformCursor, PlatformEvent, run_native_app,
};
use elcarax_render::{Rect, RenderScene, Renderer, RendererConfig, RendererError, text_stats};
use elcarax_ui::{
    CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, EditorShellIds, KeyboardKey,
    LayoutConstraints, ModifierState, PaintContext, PointerButton, PointerPosition, Theme,
    UiContext, UiEvent, UiInputEvent, UiTree, build_editor_shell_with_layout,
    paint_command_palette_overlay,
};

use crate::adapter_state::{
    ADAPTER_HANDSHAKE_COMMAND, AdapterState, adapter_command_for_inspector_edit,
};
use crate::asset_ui::asset_row_index_for_widget;
use crate::editor_session::EditorSessionState;
use crate::inspector_ui::inspector_value_index_for_widget;
use crate::project_config::AppProjectConfig;
use crate::project_picker::{
    ProjectPathResolution, resolve_create_project_root, resolve_open_project_path,
};
use crate::project_state::{PROJECT_CLOSE_COMMAND, PROJECT_CREATE_COMMAND, PROJECT_OPEN_COMMAND};
use crate::project_ui::{apply_editor_snapshot, editor_snapshots};
use crate::scene_ui::{
    scene_expand_index_for_widget, scene_row_index_for_widget, shell_content_from_editor_state,
};
use crate::shell_layout::{
    MIN_PANEL_WIDTH, MIN_VIEWPORT_WIDTH, ShellLayout, default_shell_layout_path,
};
use crate::viewport_state::AppViewportState;

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
    editor: EditorSessionState,
    adapter_state: AdapterState,
    viewport_state: AppViewportState,
    scroll_offsets: ScrollOffsets,
    bounds: Rect,
    shell_layout: ShellLayout,
    shell_layout_path: std::path::PathBuf,
    panel_resize: Option<PanelResizeDrag>,
    last_pointer: Option<PointerPosition>,
    shell_cursor: PlatformCursor,
}

#[derive(Debug, Clone, Copy, Default)]
struct ScrollOffsets {
    asset: usize,
    scene: usize,
    inspector: usize,
}

#[derive(Debug, Clone, Copy)]
enum PanelResizeDrag {
    Left { start_x: f32, start_width: f32 },
    Right { start_x: f32, start_width: f32 },
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
            ui.shell_layout
                .clamp_for_body_width(body_width_for_bounds(bounds));
            apply_shell_layout(ui)?;
            ui.tree
                .resize_root(bounds)
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
        if poll_asset_watch(ui)? {
            ui.scene_dirty = true;
            app.request_redraw();
        }
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
        if poll_asset_watch(ui)? {
            app.request_redraw();
        }
        if handle_editor_shortcut(ui, &input, self.modifiers)? {
            app.request_redraw();
            return Ok(());
        }
        if handle_palette_shortcut(ui, &input, self.modifiers)? {
            app.request_redraw();
            return Ok(());
        }
        if handle_palette_input(ui, &input)? {
            app.request_redraw();
            return Ok(());
        }
        if handle_panel_resize(ui, &input)? {
            apply_shell_cursor(ui, app);
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
        apply_shell_cursor(ui, app);
        Ok(())
    }
}

fn build_ui_state(
    theme: Theme,
    width: f32,
    height: f32,
) -> std::result::Result<UiState, NativeAppError> {
    let context = UiContext::new(theme, Rect::new(0.0, 0.0, width, height));
    let editor = EditorSessionState::new(AppProjectConfig::from_env_and_args(
        &std::env::args().collect::<Vec<_>>(),
    ));
    let adapter_state = AdapterState::default();
    let viewport_state = AppViewportState::default();
    let content = shell_content_from_editor_state(editor_snapshots(
        &editor.project.ui_snapshot(),
        &editor.assets.ui_snapshot(),
        &editor.scene.ui_snapshot(),
        &editor.inspector.ui_snapshot(&editor.scene),
        &adapter_state.ui_snapshot(),
        &viewport_state.ui_snapshot(),
    ));
    let shell_layout_path = default_shell_layout_path();
    let shell_layout = ShellLayout::load(&shell_layout_path);
    let shell =
        build_editor_shell_with_layout(&context, &content, &shell_layout.editor_shell_layout())
            .map_err(|error| {
                NativeAppError::Window(format!("failed to build UI shell: {error}"))
            })?;
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
        editor,
        adapter_state,
        viewport_state,
        scroll_offsets: ScrollOffsets::default(),
        bounds,
        shell_layout,
        shell_layout_path,
        panel_resize: None,
        last_pointer: None,
        shell_cursor: PlatformCursor::Default,
    };
    apply_shell_layout(&mut ui)?;
    apply_editor_snapshot_to_ui(&mut ui)?;
    repaint_ui_scene(&mut ui)?;
    ui.scene_dirty = false;
    Ok(ui)
}

fn body_width_for_bounds(bounds: Rect) -> f32 {
    bounds.width
}

fn apply_shell_cursor(ui: &mut UiState, app: &NativeApp) {
    let cursor = desired_shell_cursor(ui);
    if ui.shell_cursor != cursor {
        ui.shell_cursor = cursor;
        app.set_cursor(cursor);
    }
}

fn desired_shell_cursor(ui: &UiState) -> PlatformCursor {
    if ui.panel_resize.is_some() || splitter_under_pointer(ui) {
        PlatformCursor::ResizeHorizontal
    } else {
        PlatformCursor::Default
    }
}

fn splitter_under_pointer(ui: &UiState) -> bool {
    let Some(position) = ui.last_pointer.or(ui.tree.pointer_position()) else {
        return false;
    };
    let Some(hit) = ui.tree.hit_test(position) else {
        return false;
    };
    hit.id == ui.ids.left_splitter || hit.id == ui.ids.right_splitter
}

fn apply_shell_layout(ui: &mut UiState) -> std::result::Result<(), NativeAppError> {
    ui.shell_layout
        .clamp_for_body_width(body_width_for_bounds(ui.bounds));
    ui.tree
        .set_fixed_panel_width(ui.ids.project_panel, ui.shell_layout.left_width)
        .map_err(|error| {
            NativeAppError::Window(format!("failed to resize project panel: {error}"))
        })?;
    ui.tree
        .set_fixed_panel_width(ui.ids.inspector_panel, ui.shell_layout.right_width)
        .map_err(|error| {
            NativeAppError::Window(format!("failed to resize inspector panel: {error}"))
        })?;
    ui.tree
        .layout(LayoutConstraints { bounds: ui.bounds })
        .map_err(|error| NativeAppError::Window(format!("failed to layout UI: {error}")))?;
    Ok(())
}

fn handle_panel_resize(
    ui: &mut UiState,
    input: &UiInputEvent,
) -> std::result::Result<bool, NativeAppError> {
    match input {
        UiInputEvent::PointerMoved(position) => {
            ui.last_pointer = Some(*position);
            if let Some(drag) = ui.panel_resize {
                apply_panel_resize_delta(ui, drag, position.x)?;
                return Ok(true);
            }
        }
        UiInputEvent::PointerButtonPressed(PointerButton::Primary) => {
            let Some(position) = ui.last_pointer.or(ui.tree.pointer_position()) else {
                return Ok(false);
            };
            let Some(hit) = ui.tree.hit_test(position) else {
                return Ok(false);
            };
            if hit.id == ui.ids.left_splitter {
                ui.panel_resize = Some(PanelResizeDrag::Left {
                    start_x: position.x,
                    start_width: ui.shell_layout.left_width,
                });
                return Ok(true);
            }
            if hit.id == ui.ids.right_splitter {
                ui.panel_resize = Some(PanelResizeDrag::Right {
                    start_x: position.x,
                    start_width: ui.shell_layout.right_width,
                });
                return Ok(true);
            }
        }
        UiInputEvent::PointerButtonReleased(PointerButton::Primary)
            if ui.panel_resize.take().is_some() =>
        {
            let _ = ui.shell_layout.save(&ui.shell_layout_path);
            apply_shell_layout(ui)?;
            return Ok(true);
        }
        _ => {}
    }
    Ok(false)
}

fn apply_panel_resize_delta(
    ui: &mut UiState,
    drag: PanelResizeDrag,
    current_x: f32,
) -> std::result::Result<(), NativeAppError> {
    match drag {
        PanelResizeDrag::Left {
            start_x,
            start_width,
        } => {
            let delta = current_x - start_x;
            let body_width = body_width_for_bounds(ui.bounds);
            let max_left = body_width
                - ui.shell_layout.splitter_width * 2.0
                - MIN_VIEWPORT_WIDTH
                - MIN_PANEL_WIDTH;
            ui.shell_layout.left_width =
                (start_width + delta).clamp(MIN_PANEL_WIDTH, max_left.max(MIN_PANEL_WIDTH));
        }
        PanelResizeDrag::Right {
            start_x,
            start_width,
        } => {
            let delta = start_x - current_x;
            let body_width = body_width_for_bounds(ui.bounds);
            let max_right = body_width
                - ui.shell_layout.splitter_width * 2.0
                - MIN_VIEWPORT_WIDTH
                - ui.shell_layout.left_width;
            ui.shell_layout.right_width =
                (start_width + delta).clamp(MIN_PANEL_WIDTH, max_right.max(MIN_PANEL_WIDTH));
        }
    }
    apply_shell_layout(ui)?;
    ui.scene_dirty = true;
    Ok(())
}

fn execute_project_open(ui: &mut UiState) -> std::result::Result<(), NativeAppError> {
    let result = match resolve_open_project_path(ui.editor.project.config()) {
        ProjectPathResolution::Resolved(path) => ui.editor.session_mut().open_project_at(path),
        ProjectPathResolution::Cancelled => {
            return set_status_text(ui, "Open project cancelled".to_string());
        }
    };
    set_status_text(ui, result.message().to_string())?;
    apply_editor_snapshot_to_ui(ui)?;
    Ok(())
}

fn execute_project_create(ui: &mut UiState) -> std::result::Result<(), NativeAppError> {
    let result = match resolve_create_project_root(ui.editor.project.config()) {
        ProjectPathResolution::Resolved(root) => ui.editor.session_mut().create_project_at(root),
        ProjectPathResolution::Cancelled => {
            return set_status_text(ui, "Create project cancelled".to_string());
        }
    };
    set_status_text(ui, result.message().to_string())?;
    apply_editor_snapshot_to_ui(ui)?;
    Ok(())
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

fn handle_editor_shortcut(
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
    if matches!(key, KeyboardKey::Character(value) if value.eq_ignore_ascii_case("s")) {
        execute_scene_save(ui)?;
        return Ok(true);
    }
    Ok(false)
}

fn execute_scene_save(ui: &mut UiState) -> std::result::Result<(), NativeAppError> {
    let Some(outcome) = ui.editor.session_mut().save_scene() else {
        return Ok(());
    };
    set_status_text(ui, outcome.status_message())?;
    apply_editor_snapshot_to_ui(ui)?;
    Ok(())
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
    if invocation.id.as_str() == PROJECT_OPEN_COMMAND {
        execute_project_open(ui)?;
        return Ok(());
    }
    if invocation.id.as_str() == PROJECT_CREATE_COMMAND {
        execute_project_create(ui)?;
        return Ok(());
    }
    if invocation.id.as_str() == PROJECT_CLOSE_COMMAND {
        let result = ui.editor.session_mut().close_project();
        set_status_text(ui, result.message().to_string())?;
        apply_editor_snapshot_to_ui(ui)?;
        return Ok(());
    }
    if let Some(result) = ui
        .editor
        .session_mut()
        .execute_project_command(invocation.id.as_str())
    {
        set_status_text(ui, result.message().to_string())?;
        apply_editor_snapshot_to_ui(ui)?;
        return Ok(());
    }
    if ui
        .editor
        .assets
        .execute_command_id(
            invocation.id.as_str(),
            ui.editor.project.is_project_loaded(),
        )
        .is_some()
    {
        ui.editor
            .session_mut()
            .after_asset_command(invocation.id.as_str());
        apply_editor_snapshot_to_ui(ui)?;
        return Ok(());
    }
    if let Some(outcome) = ui
        .editor
        .session_mut()
        .execute_scene_command(invocation.id.as_str())
    {
        set_status_text(ui, outcome.status_message())?;
        apply_editor_snapshot_to_ui(ui)?;
        return Ok(());
    }
    if ui
        .editor
        .inspector
        .execute_command_id(invocation.id.as_str(), &mut ui.editor.scene)
        .is_some()
    {
        apply_editor_snapshot_to_ui(ui)?;
        return Ok(());
    }
    if execute_editor_edit_command(ui, invocation.id.as_str())? {
        apply_editor_snapshot_to_ui(ui)?;
        return Ok(());
    }
    if ui
        .adapter_state
        .execute_command_id(invocation.id.as_str(), &mut ui.editor.scene)
        .is_some()
    {
        if invocation.id.as_str() == ADAPTER_HANDSHAKE_COMMAND
            && let Some((adapter_id, supports_preview)) = ui.adapter_state.connected_viewport_info()
        {
            ui.viewport_state
                .on_adapter_connected(&adapter_id, supports_preview);
        }
        if invocation.id.as_str() == "adapter.disconnect" {
            ui.viewport_state.on_adapter_disconnected();
        }
        ui.editor.inspector.on_scene_selection_changed();
        apply_editor_snapshot_to_ui(ui)?;
        return Ok(());
    }
    if ui
        .viewport_state
        .execute_command_id(invocation.id.as_str(), &mut ui.adapter_state)
        .is_some()
    {
        apply_editor_snapshot_to_ui(ui)?;
        ui.scene_dirty = true;
        return Ok(());
    }
    match invocation.id.as_str() {
        "elcarax.palette.open" => ui.command_palette.open(),
        "elcarax.palette.close" => ui.command_palette.close(),
        "elcarax.status.show_renderer_stats" => {
            set_status_text(ui, renderer_stats_status(&ui.scene))?
        }
        "elcarax.status.show_ready" => set_status_text(
            ui,
            "Ready - open a project or connect an adapter".to_string(),
        )?,
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

fn apply_editor_snapshot_to_ui(ui: &mut UiState) -> std::result::Result<(), NativeAppError> {
    let project = ui.editor.project.ui_snapshot();
    let assets = ui.editor.assets.ui_snapshot_at(ui.scroll_offsets.asset);
    ui.scroll_offsets.asset = assets.scroll_offset;
    let scene = ui.editor.scene.ui_snapshot_at(ui.scroll_offsets.scene);
    ui.scroll_offsets.scene = scene.scroll_offset;
    let inspector = ui
        .editor
        .inspector
        .ui_snapshot_at(&ui.editor.scene, ui.scroll_offsets.inspector);
    ui.scroll_offsets.inspector = inspector.scroll_offset;
    let adapter = ui.adapter_state.ui_snapshot();
    let viewport = ui.viewport_state.ui_snapshot();
    apply_editor_snapshot(
        &mut ui.tree,
        ui.ids,
        editor_snapshots(&project, &assets, &scene, &inspector, &adapter, &viewport),
        ui.bounds,
    )
    .map_err(|error| NativeAppError::Window(format!("failed to update editor UI: {error}")))
}

fn poll_asset_watch(ui: &mut UiState) -> std::result::Result<bool, NativeAppError> {
    if !ui.editor.assets.poll_watch_events() {
        return Ok(false);
    }
    apply_editor_snapshot_to_ui(ui)?;
    Ok(true)
}

fn apply_ui_events(
    ui: &mut UiState,
    events: &[UiEvent],
) -> std::result::Result<bool, NativeAppError> {
    let mut changed = false;
    for event in events {
        if matches!(event, UiEvent::Clicked { id } if *id == ui.ids.run_button) {
            execute_project_open(ui)?;
            changed = true;
            continue;
        }
        if let UiEvent::Clicked { id } = event
            && let Some(row_index) = asset_row_index_for_widget(ui.ids, *id)
            && ui
                .editor
                .assets
                .select_row(ui.scroll_offsets.asset.saturating_add(row_index))
        {
            apply_editor_snapshot_to_ui(ui)?;
            changed = true;
            continue;
        }
        if let UiEvent::Clicked { id } = event
            && let Some(row_index) = scene_expand_index_for_widget(ui.ids, *id)
            && ui
                .editor
                .scene
                .toggle_expand_row_at(row_index, ui.scroll_offsets.scene)
        {
            apply_editor_snapshot_to_ui(ui)?;
            changed = true;
            continue;
        }
        if let UiEvent::Clicked { id } = event
            && let Some(row_index) = scene_row_index_for_widget(ui.ids, *id)
        {
            let object_id = ui
                .editor
                .scene
                .ui_snapshot_at(ui.scroll_offsets.scene)
                .visible_object_ids[row_index];
            if let Some(object_id) = object_id
                && ui.editor.scene.select_object(object_id)
            {
                ui.editor.inspector.on_scene_selection_changed();
                apply_editor_snapshot_to_ui(ui)?;
                changed = true;
            }
        }
        if let UiEvent::TextCommitted { id, text } = event
            && let Some(row_index) = inspector_value_index_for_widget(ui.ids, *id)
        {
            commit_inspector_row(ui, row_index, text.clone())?;
            changed = true;
            continue;
        }
        if let UiEvent::TextCancelled { id } = event
            && inspector_value_index_for_widget(ui.ids, *id).is_some()
        {
            apply_editor_snapshot_to_ui(ui)?;
            changed = true;
            continue;
        }
        if let UiEvent::Scrolled { id, delta_rows } = event
            && apply_scroll_delta(ui, *id, *delta_rows)?
        {
            changed = true;
            continue;
        }
    }
    Ok(changed)
}

fn apply_scroll_delta(
    ui: &mut UiState,
    id: elcarax_ui::WidgetId,
    delta_rows: i32,
) -> std::result::Result<bool, NativeAppError> {
    let offset = if id == ui.ids.asset_scroll_view {
        &mut ui.scroll_offsets.asset
    } else if id == ui.ids.scene_scroll_view {
        &mut ui.scroll_offsets.scene
    } else if id == ui.ids.inspector_scroll_view {
        &mut ui.scroll_offsets.inspector
    } else {
        return Ok(false);
    };
    let next = offset.saturating_add_signed(delta_rows as isize);
    if next == *offset {
        return Ok(false);
    }
    *offset = next;
    apply_editor_snapshot_to_ui(ui)?;
    Ok(true)
}

fn commit_inspector_row(
    ui: &mut UiState,
    row_index: usize,
    text: String,
) -> std::result::Result<(), NativeAppError> {
    let snapshot = ui
        .editor
        .inspector
        .ui_snapshot_at(&ui.editor.scene, ui.scroll_offsets.inspector);
    let path = snapshot.row_property_paths[row_index].clone();
    let edit_kind = snapshot.row_edit_kinds[row_index];
    let label = snapshot.row_labels[row_index].clone();
    if path.is_empty() {
        return Ok(());
    }
    let result = ui.editor.inspector.commit_inspector_property(
        &mut ui.editor.scene,
        &mut ui.editor.edit_history,
        path.as_str(),
        edit_kind,
        text.as_str(),
        label.as_str(),
    );
    set_status_text(ui, result.message().to_string())?;
    apply_editor_snapshot_to_ui(ui)?;
    Ok(())
}

fn execute_editor_edit_command(
    ui: &mut UiState,
    command_id: &str,
) -> std::result::Result<bool, NativeAppError> {
    if ui.editor.scene.adapter_id().is_some()
        && let Some(adapter_command) = adapter_command_for_inspector_edit(command_id)
    {
        return Ok(ui
            .adapter_state
            .execute_command_id(adapter_command, &mut ui.editor.scene)
            .is_some());
    }
    Ok(ui
        .editor
        .inspector
        .execute_edit_command_id(
            command_id,
            &mut ui.editor.scene,
            &mut ui.editor.edit_history,
        )
        .is_some())
}

fn events_affect_paint(events: &[UiEvent]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            UiEvent::HoverChanged { .. }
                | UiEvent::FocusChanged(_)
                | UiEvent::ActiveChanged { .. }
                | UiEvent::Clicked { .. }
                | UiEvent::TextChanged { .. }
                | UiEvent::TextCommitted { .. }
                | UiEvent::TextCancelled { .. }
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
