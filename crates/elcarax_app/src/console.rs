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
use crate::asset_state::{ASSET_SCAN_COMMAND, AssetState};
use crate::inspector_state::InspectorState;
use crate::project_config::AppProjectConfig;
use crate::project_effects::apply_project_command_side_effects;
use crate::project_state::{
    PROJECT_CLOSE_COMMAND, PROJECT_CREATE_COMMAND, PROJECT_OPEN_COMMAND,
    PROJECT_REOPEN_LAST_COMMAND, PROJECT_SHOW_RECENT_COMMAND, PROJECT_VALIDATE_COMMAND,
    ProjectState,
};
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

    run_project_proof()?;
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

fn run_project_proof() -> Result<()> {
    use std::fs;

    let temp = std::env::temp_dir().join(format!("elcarax-console-project-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let recent_path = temp.join("recent-projects.toml");
    let project_root = temp.join("project");

    println!("project_proof: begin");
    let config = AppProjectConfig {
        create_root: Some(project_root.clone()),
        create_name: Some("Console Proof Project".to_string()),
        recent_store_path: Some(recent_path.clone()),
        ..AppProjectConfig::default()
    };
    let mut project_state = ProjectState::new(config);
    let mut asset_state = AssetState::default();
    let mut scene_state = SceneState::default();
    let mut inspector_state = InspectorState::default();

    assert!(!project_state.is_project_loaded());
    println!("project.startup: no project open");

    let create = project_state
        .execute_command_id(PROJECT_CREATE_COMMAND)
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing create result".to_string());
    println!("project.create: {create}");
    apply_project_command_side_effects(
        PROJECT_CREATE_COMMAND,
        &mut project_state,
        &mut asset_state,
        &mut scene_state,
        &mut inspector_state,
    );
    assert!(project_state.is_project_loaded());

    let open_config = AppProjectConfig {
        open_path: Some(project_root.clone()),
        recent_store_path: Some(recent_path.clone()),
        ..AppProjectConfig::default()
    };
    project_state = ProjectState::new(open_config);
    let open = project_state
        .execute_command_id(PROJECT_OPEN_COMMAND)
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing open result".to_string());
    println!("project.open: {open}");
    apply_project_command_side_effects(
        PROJECT_OPEN_COMMAND,
        &mut project_state,
        &mut asset_state,
        &mut scene_state,
        &mut inspector_state,
    );

    let validate = project_state
        .execute_command_id(PROJECT_VALIDATE_COMMAND)
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing validate result".to_string());
    println!("project.validate: {validate}");

    let scan = asset_state
        .execute_command_id(ASSET_SCAN_COMMAND, project_state.is_project_loaded())
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing scan result".to_string());
    println!("asset.scan: {scan}");
    apply_project_command_side_effects(
        ASSET_SCAN_COMMAND,
        &mut project_state,
        &mut asset_state,
        &mut scene_state,
        &mut inspector_state,
    );

    let recent = project_state
        .execute_command_id(PROJECT_SHOW_RECENT_COMMAND)
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing recent result".to_string());
    println!("project.show_recent: {recent}");

    let close = project_state
        .execute_command_id(PROJECT_CLOSE_COMMAND)
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing close result".to_string());
    println!("project.close: {close}");
    apply_project_command_side_effects(
        PROJECT_CLOSE_COMMAND,
        &mut project_state,
        &mut asset_state,
        &mut scene_state,
        &mut inspector_state,
    );
    assert!(!project_state.is_project_loaded());
    assert!(asset_state.index().is_empty());

    let reopen_config = AppProjectConfig {
        recent_store_path: Some(recent_path),
        ..AppProjectConfig::default()
    };
    project_state = ProjectState::new(reopen_config);
    let reopen = project_state
        .execute_command_id(PROJECT_REOPEN_LAST_COMMAND)
        .map(|result| result.message().to_string())
        .unwrap_or_else(|| "missing reopen result".to_string());
    println!("project.reopen_last: {reopen}");
    apply_project_command_side_effects(
        PROJECT_REOPEN_LAST_COMMAND,
        &mut project_state,
        &mut asset_state,
        &mut scene_state,
        &mut inspector_state,
    );
    assert!(project_state.is_project_loaded());

    let _ = fs::remove_dir_all(&temp);
    println!("project_proof: complete");
    Ok(())
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
