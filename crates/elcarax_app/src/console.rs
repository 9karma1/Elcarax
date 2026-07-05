use elcarax_adapter_api::{HandshakeRequest, LoadProjectRequest};
use elcarax_adapter_host::{AdapterHost, AdapterProcessSpec};
use elcarax_commands::{
    CommandBindingRegistry, CommandScope, KeyChord, KeyModifier, built_in_commands,
};
use elcarax_core::{Result, ViewportStatus};
use elcarax_devtools::DevtoolsSnapshot;
use elcarax_gpu::FrameStats;
use elcarax_platform::NativeShellSpec;
use elcarax_render::{Rect, RenderStats, batch_scene, image_stats, text_stats};
use elcarax_ui::{PaintContext, Theme, UiContext, build_editor_shell_with_content};

use crate::adapter_state::AdapterState;
use crate::asset_state::{ASSET_REFRESH_COMMAND, ASSET_SCAN_COMMAND, ASSET_SHOW_SELECTED_COMMAND};
use crate::editor_commands::{command_summary, shortcut_summary, toolbar_snapshot};
use crate::editor_session::EditorSessionState;
use crate::project_config::AppProjectConfig;
use crate::project_state::{
    PROJECT_CREATE_COMMAND, PROJECT_OPEN_COMMAND, PROJECT_REOPEN_LAST_COMMAND,
    PROJECT_SHOW_RECENT_COMMAND, PROJECT_VALIDATE_COMMAND,
};
use crate::project_ui::editor_snapshots;
use crate::scene_state::{SCENE_LOAD_COMMAND, SCENE_SAVE_COMMAND};
use crate::scene_ui::shell_content_from_editor_state;
use crate::viewport_state::{
    AppViewportState, VIEWPORT_CLEAR_COMMAND, VIEWPORT_REQUEST_FRAME_COMMAND,
};

pub fn run_console_proof() -> Result<()> {
    let startup = build_startup_summary()?;
    println!("Elcarax v0.1 editor startup");
    println!("app_initialized: true");
    println!("command_registry: {} command(s)", startup.command_count);
    println!("keybindings: {} binding(s)", startup.binding_count);
    println!("keybinding_conflicts: {}", startup.binding_conflict_count);
    println!("toolbar_actions: {}", startup.toolbar_action_count);
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
    println!("help.shortcuts: {}", startup.shortcut_help);
    println!("help.commands: {}", startup.command_help);

    run_project_proof()?;
    run_viewport_proof()?;
    Ok(())
}

struct StartupSummary {
    command_count: usize,
    binding_count: usize,
    binding_conflict_count: usize,
    toolbar_action_count: usize,
    shortcut_help: String,
    command_help: String,
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
    let editor = EditorSessionState::default();
    let adapter_state = AdapterState::default();
    let viewport_state = AppViewportState::default();
    let registry = built_in_commands().map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to register commands: {error}"))
    })?;
    let bindings = CommandBindingRegistry::from_commands(&registry);
    let toolbar = toolbar_snapshot(
        &registry,
        &bindings,
        &editor,
        &adapter_state,
        &viewport_state,
    );
    let content = shell_content_from_editor_state(editor_snapshots(
        &editor.project.ui_snapshot(),
        &editor.assets.ui_snapshot(),
        &editor.scene.ui_snapshot(),
        &editor.inspector.ui_snapshot(&editor.scene),
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
        binding_count: bindings.bindings().len(),
        binding_conflict_count: bindings.diagnostics().len(),
        toolbar_action_count: toolbar.actions().count(),
        shortcut_help: shortcut_summary(&registry, &bindings),
        command_help: command_summary(&registry, &bindings),
        project_state: "No project open".to_string(),
        asset_state: "No project open".to_string(),
        adapter_state: "Disconnected; no adapter configured".to_string(),
        scene_state: "No scene loaded".to_string(),
        inspector_state: "No object selected".to_string(),
        viewport_state: "No viewport source".to_string(),
        undo_count: editor.edit_history.undo_count(),
        redo_count: editor.edit_history.redo_count(),
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
    let registry = built_in_commands().map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to register commands: {error}"))
    })?;
    let bindings = CommandBindingRegistry::from_commands(&registry);
    let config = AppProjectConfig {
        create_root: Some(project_root.clone()),
        create_name: Some("Console Proof Project".to_string()),
        recent_store_path: Some(recent_path.clone()),
        ..AppProjectConfig::default()
    };
    let mut session = EditorSessionState::new(config);

    assert!(!session.project.is_project_loaded());
    println!("project.startup: no project open");

    let create = dispatch_session_command(&mut session, PROJECT_CREATE_COMMAND);
    println!("project.create: {create}");
    assert!(session.project.is_project_loaded());
    assert!(session.scene.snapshot().is_some());

    let open_config = AppProjectConfig {
        open_path: Some(project_root.clone()),
        recent_store_path: Some(recent_path.clone()),
        ..AppProjectConfig::default()
    };
    session = EditorSessionState::new(open_config);
    let open = dispatch_session_command(&mut session, PROJECT_OPEN_COMMAND);
    println!("project.open: {open}");
    create_console_asset_files(project_root.join("assets").as_path())?;

    let validate = dispatch_session_command(&mut session, PROJECT_VALIDATE_COMMAND);
    println!("project.validate: {validate}");

    let scan = dispatch_session_command(&mut session, ASSET_SCAN_COMMAND);
    println!("asset.scan: {scan}");
    println!("asset.kinds: {}", session.assets.kind_summary());
    if session.assets.select_row(0) {
        let selected = session
            .assets
            .execute_command_id(
                ASSET_SHOW_SELECTED_COMMAND,
                session.project.is_project_loaded(),
            )
            .map(|result| result.message().to_string())
            .unwrap_or_else(|| "missing selected result".to_string());
        println!("asset.show_selected: {selected}");
    }

    let assets_root = project_root.join("assets");
    fs::write(assets_root.join("notes.txt"), "notes").map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to update console asset: {error}"))
    })?;
    let refresh = dispatch_session_command(&mut session, ASSET_REFRESH_COMMAND);
    println!("asset.refresh: {refresh}");

    let recent = dispatch_session_command(&mut session, PROJECT_SHOW_RECENT_COMMAND);
    println!("project.show_recent: {recent}");

    let close = session.session_mut().close_project(None);
    println!("project.close: {}", close.message());
    assert!(!session.project.is_project_loaded());
    assert!(session.assets.index().is_empty());
    assert!(session.assets.scanned_asset_count().is_none());

    let reopen_config = AppProjectConfig {
        recent_store_path: Some(recent_path),
        ..AppProjectConfig::default()
    };
    session = EditorSessionState::new(reopen_config);
    let reopen = dispatch_session_command(&mut session, PROJECT_REOPEN_LAST_COMMAND);
    println!("project.reopen_last: {reopen}");
    assert!(session.project.is_project_loaded());

    let scene_reload = dispatch_session_command(&mut session, SCENE_LOAD_COMMAND);
    println!("scene.reload: {scene_reload}");
    let scene_save_command = command_for_shortcut(&bindings, &[KeyModifier::Control], "S")?;
    let scene_save = dispatch_session_command(&mut session, scene_save_command.as_str());
    println!("scene.save: {scene_save}");

    prove_scene_document_round_trip(&mut session)?;

    let _ = fs::remove_dir_all(&temp);
    println!("project_proof: complete");
    Ok(())
}

fn command_for_shortcut(
    bindings: &CommandBindingRegistry,
    modifiers: &[KeyModifier],
    key: &str,
) -> Result<String> {
    let chord = KeyChord::new(modifiers.iter().cloned(), key).map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("invalid console shortcut: {error}"))
    })?;
    bindings
        .command_for_chord(&chord, CommandScope::Global)
        .map(|id| id.as_str().to_string())
        .ok_or_else(|| {
            elcarax_core::ElcaraxError::Internal(format!(
                "missing console shortcut binding for {}",
                chord.display_label()
            ))
        })
}

fn dispatch_session_command(session: &mut EditorSessionState, command_id: &str) -> String {
    if command_id == SCENE_SAVE_COMMAND {
        return session
            .session_mut()
            .save_scene()
            .map(|outcome| outcome.status_message())
            .unwrap_or_else(|| "missing scene save result".to_string());
    }
    if let Some(result) = session
        .session_mut()
        .execute_project_command(command_id, None)
    {
        return result.message().to_string();
    }
    if let Some(result) = session
        .assets
        .execute_command_id(command_id, session.project.is_project_loaded())
    {
        session.session_mut().after_asset_command(command_id);
        return result.message().to_string();
    }
    if let Some(outcome) = session.session_mut().execute_scene_command(command_id) {
        return outcome.status_message();
    }
    format!("missing command result: {command_id}")
}

fn prove_scene_document_round_trip(session: &mut EditorSessionState) -> Result<()> {
    use elcarax_scene_model::{ObjectSchema, SceneObject, SceneObjectKind};

    if let Some(snapshot) = session.scene.snapshot_mut() {
        let schema = ObjectSchema::new("RoundtripMarker");
        let object = SceneObject::new("Persisted Root", SceneObjectKind::World, schema.type_id);
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
    }
    session.scene.mark_document_modified();
    assert!(session.scene.has_unsaved_changes());
    let save = session
        .session_mut()
        .save_scene()
        .map(|outcome| outcome.status_message())
        .unwrap_or_else(|| "missing round-trip save result".to_string());
    println!("scene.round_trip_save: {save}");
    assert!(!session.scene.has_unsaved_changes());

    session.scene.on_project_closed();
    let scene_root = session
        .project
        .scene_root()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| elcarax_core::ElcaraxError::Internal("missing scene root".to_string()))?;
    session.scene.on_project_opened(
        scene_root.as_path(),
        session.project.active_scene_relative(),
    );
    let reload = session
        .session_mut()
        .execute_scene_command(SCENE_LOAD_COMMAND)
        .map(|outcome| outcome.status_message())
        .unwrap_or_else(|| "missing round-trip reload result".to_string());
    println!("scene.round_trip_reload: {reload}");
    assert_eq!(
        session
            .scene
            .snapshot()
            .map(|snapshot| snapshot.object_count()),
        Some(1)
    );
    Ok(())
}

fn create_console_asset_files(asset_root: &std::path::Path) -> Result<()> {
    use std::fs;

    fs::create_dir_all(asset_root.join("models")).map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to create model folder: {error}"))
    })?;
    fs::create_dir_all(asset_root.join("textures")).map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to create texture folder: {error}"))
    })?;
    fs::write(asset_root.join("models").join("hero.glb"), "model").map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to write model asset: {error}"))
    })?;
    fs::write(asset_root.join("textures").join("checker.png"), "image").map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to write image asset: {error}"))
    })?;
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
