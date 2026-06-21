use std::time::{Duration, Instant};

use elcarax_core::{ElcaraxError, Result};
use elcarax_gpu::{GpuContext, GpuContextSpec, GpuSurface, RenderError, SurfaceSize};
use elcarax_platform::{
    NativeApp, NativeAppError, NativeAppHandler, NativeShellSpec, PlatformEvent, run_native_app,
};
use elcarax_render::{
    Border, Color, Rect, RenderLayer, RenderPrimitive, RenderScene, Renderer, RendererConfig,
    RendererError,
};

pub fn run_native_shell() -> Result<()> {
    println!("Elcarax native shell: starting");
    run_native_app(NativeShellSpec::default_editor(), ShellState::default())
        .map_err(|error| ElcaraxError::Internal(error.to_string()))
}

#[derive(Default)]
struct ShellState {
    gpu: Option<GpuState>,
}

struct GpuState {
    context: GpuContext,
    surface: GpuSurface<'static>,
    renderer: Renderer,
    last_stats_log: Option<Instant>,
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
        println!("Elcarax native shell: GPU renderer initialized");
        self.gpu = Some(GpuState {
            context,
            surface,
            renderer,
            last_stats_log: None,
        });
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
            PlatformEvent::Resized(size) => self.resize(size.width, size.height, app),
            PlatformEvent::ScaleFactorChanged { .. } => {
                let size = app.window().inner_size();
                self.resize(size.width, size.height, app);
            }
            PlatformEvent::RedrawRequested => self.render(app)?,
            PlatformEvent::KeyboardInput(_)
            | PlatformEvent::PointerMoved { .. }
            | PlatformEvent::MouseInput { .. } => {}
        }
        Ok(())
    }
}

impl ShellState {
    fn resize(&mut self, width: u32, height: u32, app: &NativeApp) {
        if let Some(gpu) = &mut self.gpu {
            gpu.surface.resize(SurfaceSize::new(width, height));
            app.request_redraw();
        }
    }

    fn render(&mut self, app: &NativeApp) -> std::result::Result<(), NativeAppError> {
        let Some(gpu) = &mut self.gpu else {
            return Ok(());
        };
        gpu.context.keep_alive();
        let size = gpu.surface.size();
        let scene = demo_scene(size.width as f32, size.height as f32);
        match gpu.renderer.render(&mut gpu.surface, &scene) {
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
}

fn demo_scene(width: f32, height: f32) -> RenderScene {
    let mut scene = RenderScene::new();
    let toolbar_h = 56.0;
    let status_h = 28.0;
    let sidebar_w = 248.0;
    let inspector_w = 292.0;
    let chrome = Color::srgb(0.075, 0.082, 0.11, 1.0);
    let panel = Color::srgb(0.095, 0.105, 0.14, 1.0);
    let viewport = Color::srgb(0.045, 0.05, 0.07, 1.0);
    let line = Color::srgb(0.18, 0.20, 0.26, 1.0);

    scene.push(
        RenderLayer::Background,
        RenderPrimitive::solid_rect(Rect::new(0.0, 0.0, width, height), Color::ELCARAX_DARK)
            .with_debug_label("background"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::solid_rect(Rect::new(0.0, 0.0, width, toolbar_h), chrome)
            .with_debug_label("top toolbar"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::solid_rect(
            Rect::new(0.0, toolbar_h, sidebar_w, height - toolbar_h - status_h),
            panel,
        )
        .with_debug_label("left sidebar"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::solid_rect(
            Rect::new(
                sidebar_w,
                toolbar_h,
                width - sidebar_w - inspector_w,
                height - toolbar_h - status_h,
            ),
            viewport,
        )
        .with_debug_label("central viewport"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::solid_rect(
            Rect::new(
                width - inspector_w,
                toolbar_h,
                inspector_w,
                height - toolbar_h - status_h,
            ),
            panel,
        )
        .with_debug_label("right inspector"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::solid_rect(Rect::new(0.0, height - status_h, width, status_h), chrome)
            .with_debug_label("bottom status bar"),
    );

    for x in [sidebar_w, width - inspector_w] {
        scene.push(
            RenderLayer::Overlay,
            RenderPrimitive::line([x, toolbar_h], [x, height - status_h], 1.0, line)
                .with_debug_label("vertical separator"),
        );
    }
    for y in [toolbar_h, height - status_h] {
        scene.push(
            RenderLayer::Overlay,
            RenderPrimitive::line([0.0, y], [width, y], 1.0, line)
                .with_debug_label("horizontal separator"),
        );
    }
    scene.push(
        RenderLayer::Overlay,
        RenderPrimitive::border_rect(
            Rect::new(sidebar_w + 24.0, toolbar_h + 24.0, 320.0, 180.0),
            Border::new(2.0, Color::srgb(0.26, 0.34, 0.55, 1.0)),
        )
        .with_debug_label("viewport sample border"),
    );
    scene
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
