use elcarax_adapter_api::{HandshakeRequest, LoadProjectRequest};
use elcarax_adapter_host::{AdapterHost, AdapterProcessSpec};
use elcarax_commands::{CommandHistory, built_in_commands};
use elcarax_core::{Result, ViewportStatus};
use elcarax_devtools::DevtoolsSnapshot;
use elcarax_gpu::FrameStats;
use elcarax_platform::NativeShellSpec;
use elcarax_render::{Rect, RenderStats, batch_scene, image_stats, text_stats};
use elcarax_ui::{PaintContext, Theme, UiContext, build_editor_shell_with_content};

use crate::adapter_state::AdapterState;
use crate::asset_state::AssetState;
use crate::inspector_state::InspectorState;
use crate::project_state::ProjectState;
use crate::project_ui::editor_snapshots;
use crate::scene_state::SceneState;
use crate::scene_ui::shell_content_from_editor_state;
use crate::viewport_state::{
    AppViewportState, VIEWPORT_CLEAR_COMMAND, VIEWPORT_REQUEST_FRAME_COMMAND,
};

pub fn run_console_proof() -> Result<()> {
    let startup = build_startup_summary()?;
    println!("Elcarax v0.1 editor startup");
    println!("app_initialized: true");
    println!("command_registry: {} command(s)", startup.command_count);
    println!("project_state: {}", startup.project_state);
    println!("asset_state: {}", startup.asset_state);
    println!("adapter_state: {}", startup.adapter_state);
    println!("scene_state: {}", startup.scene_state);
    println!("inspector_state: {}", startup.inspector_state);
    println!("viewport_state: {}", startup.viewport_state);
    println!("undo_stack: {}", startup.undo_count);
    println!("redo_stack: {}", startup.redo_count);
    println!(
        "ui_model: nodes={} layouts={} primitives={} text_primitives={} glyphs={} image_primitives={}",
        startup.node_count,
        startup.layout_count,
        startup.primitive_count,
        startup.text_primitive_count,
        startup.glyph_count,
        startup.image_primitive_count
    );
    println!("devtools: {}", startup.devtools);
    println!("status: Ready - open a project or connect an adapter");

    run_viewport_proof()?;
    Ok(())
}

struct StartupSummary {
    command_count: usize,
    project_state: String,
    asset_state: String,
    adapter_state: String,
    scene_state: String,
    inspector_state: String,
    viewport_state: String,
    undo_count: usize,
    redo_count: usize,
    node_count: usize,
    layout_count: usize,
    primitive_count: usize,
    text_primitive_count: usize,
    glyph_count: usize,
    image_primitive_count: usize,
    devtools: String,
}

fn build_startup_summary() -> Result<StartupSummary> {
    let shell = NativeShellSpec::default_editor();
    let theme = Theme::editor_dark();
    let context = UiContext::new(
        theme,
        Rect::new(0.0, 0.0, shell.width as f32, shell.height as f32),
    );
    let project_state = ProjectState::default();
    let asset_state = AssetState::default();
    let scene_state = SceneState::default();
    let inspector_state = InspectorState::default();
    let adapter_state = AdapterState::default();
    let viewport_state = AppViewportState::default();
    let history = CommandHistory::new();
    let registry = built_in_commands().map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to register commands: {error}"))
    })?;
    let content = shell_content_from_editor_state(editor_snapshots(
        &project_state.ui_snapshot(),
        &asset_state.ui_snapshot(),
        &scene_state.ui_snapshot(),
        &inspector_state.ui_snapshot(&scene_state),
        &adapter_state.ui_snapshot(),
        &viewport_state.ui_snapshot(),
    ));
    let shell = build_editor_shell_with_content(&context, &content).map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to build UI shell: {error}"))
    })?;
    let scene = shell
        .tree
        .paint(&PaintContext::new(theme))
        .map_err(|error| {
            elcarax_core::ElcaraxError::Internal(format!("failed to paint UI shell: {error}"))
        })?;
    let text = text_stats(&scene);
    let images = image_stats(&scene);
    let render = RenderStats {
        primitive_count: scene.primitives().len(),
        batch_count: batch_scene(&scene).len(),
        image_primitive_count: images.image_primitive_count,
        image_upload_bytes: images.image_upload_bytes,
        ..text
    };
    let devtools = DevtoolsSnapshot {
        frame: FrameStats::empty(),
        render,
        adapter_messages: 0,
    };
    Ok(StartupSummary {
        command_count: registry.all().len(),
        project_state: "No project open".to_string(),
        asset_state: "No asset root loaded".to_string(),
        adapter_state: "Disconnected; no adapter configured".to_string(),
        scene_state: "No scene loaded".to_string(),
        inspector_state: "No object selected".to_string(),
        viewport_state: "No viewport source".to_string(),
        undo_count: history.undo_count(),
        redo_count: history.redo_count(),
        node_count: shell.tree.node_count(),
        layout_count: shell.tree.node_count(),
        primitive_count: devtools.render.primitive_count,
        text_primitive_count: devtools.render.text_primitive_count,
        glyph_count: devtools.render.glyph_count,
        image_primitive_count: devtools.render.image_primitive_count,
        devtools: devtools.summary(),
    })
}

fn run_viewport_proof() -> Result<()> {
    let mut viewport_state = AppViewportState::default();
    let mut adapter_state = AdapterState::default();

    println!("viewport_proof: begin");
    assert_eq!(viewport_state.state().status, ViewportStatus::NoSource);

    let without_adapter = viewport_state
        .execute_command_id(VIEWPORT_REQUEST_FRAME_COMMAND, &mut adapter_state)
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing viewport command result".to_string());
    println!("viewport.request_frame_without_adapter: {without_adapter}");
    assert!(without_adapter.contains("No adapter connected"));

    let mut host = AdapterHost::spawn(AdapterProcessSpec::stdio_game_adapter(), None)
        .map_err(|error| elcarax_core::ElcaraxError::Adapter(error.to_string()))?;
    let info = host
        .handshake(HandshakeRequest::current("elcarax-console-proof", None))
        .map_err(|error| elcarax_core::ElcaraxError::Adapter(error.to_string()))?;
    host.load_project(LoadProjectRequest { project_path: None })
        .map_err(|error| elcarax_core::ElcaraxError::Adapter(error.to_string()))?;

    viewport_state.on_adapter_connected(
        info.id.as_str(),
        info.capabilities.supports_viewport_preview,
    );
    assert_eq!(
        viewport_state.state().status,
        ViewportStatus::WaitingForFrame
    );

    let frame_result = viewport_state
        .request_frame_from_host(&mut host, 64, 64)
        .map_err(|error| elcarax_core::ElcaraxError::Internal(error.to_string()))?;
    println!(
        "viewport.request_frame_with_adapter: {}",
        frame_result.message()
    );
    assert_eq!(
        viewport_state.state().status,
        ViewportStatus::FrameAvailable
    );
    let frame = viewport_state.state().frame.as_ref().ok_or_else(|| {
        elcarax_core::ElcaraxError::Internal("missing viewport frame".to_string())
    })?;
    println!(
        "viewport.frame_metadata: {}x{} bytes={}",
        frame.size.width,
        frame.size.height,
        frame.pixels.rgba.len()
    );

    let clear_result = viewport_state
        .execute_command_id(VIEWPORT_CLEAR_COMMAND, &mut adapter_state)
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing clear result".to_string());
    println!("viewport.clear: {clear_result}");
    assert_eq!(
        viewport_state.state().status,
        ViewportStatus::WaitingForFrame
    );

    host.shutdown()
        .map_err(|error| elcarax_core::ElcaraxError::Adapter(error.to_string()))?;
    viewport_state.on_adapter_disconnected();
    assert_eq!(viewport_state.state().status, ViewportStatus::NoSource);
    println!("viewport_proof: complete");
    Ok(())
}
