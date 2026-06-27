use elcarax_commands::{
    CommandContext, CommandHistory, CommandId, CommandRegistry, CommandResult,
    PropertyChangeCommand, RegisteredCommand, built_in_commands,
};
use elcarax_core::Result;
use elcarax_devtools::DevtoolsSnapshot;
use elcarax_gpu::FrameStats;
use elcarax_platform::NativeShellSpec;
use elcarax_project::ProjectFile;
use elcarax_render::{Rect, RenderScene, RenderStats, batch_scene, text_stats};
use elcarax_scene_model::{
    ObjectSchema, PropertyKind, PropertyPath, PropertySchema, PropertyValue, SceneObject,
    SceneSnapshot,
};
use elcarax_ui::{
    CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey, PaintContext,
    PointerButton, PointerPosition, Theme, UiContext, UiEvent, UiInputEvent,
    build_editor_shell_with_content,
};

use crate::asset_state::{
    ASSET_CLEAR_SELECTION_COMMAND, ASSET_SCAN_DEMO_COMMAND, ASSET_SELECT_FIRST_COMMAND, AssetState,
};
use crate::project_state::{
    PROJECT_CLOSE_COMMAND, PROJECT_NEW_DEMO_COMMAND, PROJECT_VALIDATE_COMMAND, ProjectState,
};
use crate::project_ui::{apply_editor_snapshot, shell_content_from_editor_state};

pub fn run_console_proof() -> Result<()> {
    let shell = NativeShellSpec::default_editor();
    let project = ProjectFile::new("Elcarax Sample", "elcarax-game");
    project.validate()?;
    let position_path = PropertyPath::parse("transform.position")?;
    let schema = ObjectSchema::new("Entity").with_property(PropertySchema::editable(
        position_path.clone(),
        "Position",
        PropertyKind::Vec3,
    ));
    let mut object = SceneObject::new("Player", schema.type_id);
    object.set_property(position_path.clone(), PropertyValue::Vec3([0.0, 1.0, 0.0]));
    let object_id = object.id;
    let mut scene = SceneSnapshot::empty();
    scene.add_schema(schema);
    scene.add_root_object(object);
    let mut history = CommandHistory::new();
    let mut context = CommandContext { scene: &mut scene };
    history.execute(
        Box::new(PropertyChangeCommand::new(
            object_id,
            position_path.clone(),
            PropertyValue::Vec3([2.0, 4.0, 6.0]),
        )),
        &mut context,
    )?;
    history.undo(&mut context)?;
    let proof = build_console_ui(&shell)?;
    let text_stats = text_stats(&proof.scene);
    let snapshot = DevtoolsSnapshot {
        frame: FrameStats::empty(),
        render: RenderStats {
            primitive_count: proof.scene.primitives().len(),
            batch_count: batch_scene(&proof.scene).len(),
            ..text_stats
        },
        adapter_messages: 0,
    };
    println!("Elcarax v0.1 core scaffold");
    println!("window: {} {}x{}", shell.title, shell.width, shell.height);
    println!("project: {} via {}", project.name, project.adapter);
    println!(
        "undo_count: {} redo_count: {}",
        history.undo_count(),
        history.redo_count()
    );
    println!("devtools: {}", snapshot.summary());
    println!(
        "ui: nodes={} layouts={} primitives={} text_primitives={} dirty(layout={}, paint={}, text={}, hit_test={}, accessibility={})",
        proof.node_count,
        proof.layout_count,
        snapshot.render.primitive_count,
        snapshot.render.text_primitive_count,
        proof.dirty.layout,
        proof.dirty.paint,
        proof.dirty.text,
        proof.dirty.hit_test,
        proof.dirty.accessibility
    );
    println!("interaction: run_clicked={}", proof.run_clicked);
    println!(
        "command_palette: ready_executed={} status=\"{}\"",
        proof.ready_executed, proof.status_text
    );
    println!("project_ui: initial=\"{}\"", proof.project_initial_status);
    println!(
        "project_command: new_demo_executed={} status=\"{}\"",
        proof.project_new_demo_executed, proof.project_loaded_status
    );
    println!(
        "project_command: validate_executed={} diagnostics=\"{}\"",
        proof.project_validate_executed, proof.project_validation_summary
    );
    println!(
        "project_command: close_executed={} status=\"{}\"",
        proof.project_close_executed, proof.project_closed_status
    );
    println!(
        "asset_command: scan_demo_executed={} count={}",
        proof.asset_scan_demo_executed, proof.asset_count
    );
    println!("asset_kinds: {}", proof.asset_kinds_summary);
    println!(
        "asset_command: select_first_executed={} selected=\"{}\"",
        proof.asset_select_first_executed, proof.asset_selected_summary
    );
    println!(
        "asset_command: clear_selection_executed={} selected=\"{}\"",
        proof.asset_clear_selection_executed, proof.asset_cleared_summary
    );
    Ok(())
}

struct ConsoleUiProof {
    scene: RenderScene,
    node_count: usize,
    layout_count: usize,
    dirty: elcarax_ui::DirtySummary,
    run_clicked: bool,
    ready_executed: bool,
    status_text: String,
    project_initial_status: String,
    project_loaded_status: String,
    project_validation_summary: String,
    project_closed_status: String,
    project_new_demo_executed: bool,
    project_validate_executed: bool,
    project_close_executed: bool,
    asset_scan_demo_executed: bool,
    asset_select_first_executed: bool,
    asset_clear_selection_executed: bool,
    asset_count: usize,
    asset_kinds_summary: String,
    asset_selected_summary: String,
    asset_cleared_summary: String,
}

fn build_console_ui(shell: &NativeShellSpec) -> Result<ConsoleUiProof> {
    let theme = Theme::editor_dark();
    let context = UiContext::new(
        theme,
        Rect::new(0.0, 0.0, shell.width as f32, shell.height as f32),
    );
    let mut project_state = ProjectState::default();
    let mut asset_state = AssetState::default();
    let initial_content =
        shell_content_from_editor_state(&project_state.ui_snapshot(), &asset_state.ui_snapshot());
    let shell = build_editor_shell_with_content(&context, &initial_content).map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to build UI shell: {error}"))
    })?;
    let mut tree = shell.tree;
    let button_rect = tree
        .get(shell.ids.run_button)
        .map(|node| node.rect)
        .ok_or_else(|| {
            elcarax_core::ElcaraxError::Internal("missing Run button in UI shell".to_string())
        })?;
    let pointer = PointerPosition::new(button_rect.x + 4.0, button_rect.y + 4.0);
    let mut interaction_events = Vec::new();
    for input in [
        UiInputEvent::PointerMoved(pointer),
        UiInputEvent::PointerButtonPressed(PointerButton::Primary),
        UiInputEvent::PointerButtonReleased(PointerButton::Primary),
    ] {
        interaction_events.extend(tree.process_input(input).map_err(|error| {
            elcarax_core::ElcaraxError::Internal(format!("failed to process UI input: {error}"))
        })?);
    }
    let run_clicked = interaction_events
        .iter()
        .any(|event| matches!(event, UiEvent::Clicked { id } if *id == shell.ids.run_button));
    if run_clicked {
        tree.set_label_text(shell.ids.status_label, "Status: Run clicked")
            .map_err(|error| {
                elcarax_core::ElcaraxError::Internal(format!("failed to update status: {error}"))
            })?;
    }
    let registry = built_in_commands().map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to register commands: {error}"))
    })?;
    let mut palette = CommandPaletteState::new(palette_entries_from_registry(&registry));
    let mut ready_executed = false;
    if let Some(id) = execute_palette_query(&registry, &mut palette, "ready")?
        && id.as_str() == "elcarax.status.show_ready"
    {
        tree.set_label_text(shell.ids.status_label, "Status: Ready")
            .map_err(|error| {
                elcarax_core::ElcaraxError::Internal(format!("failed to update status: {error}"))
            })?;
        ready_executed = true;
    }
    let ready_status_text = label_text(&tree, shell.ids.status_label);

    let bounds = context.root_bounds;
    let project_initial_status = project_state.ui_snapshot().status;

    let project_new_demo_executed = execute_project_command_from_palette(
        &registry,
        &mut palette,
        &mut project_state,
        PROJECT_NEW_DEMO_COMMAND,
    )?;
    apply_editor_snapshot(
        &mut tree,
        shell.ids,
        &project_state.ui_snapshot(),
        &asset_state.ui_snapshot(),
        bounds,
    )
    .map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!(
            "failed to apply project state to UI: {error}"
        ))
    })?;
    let project_loaded_status = project_state.ui_snapshot().status;

    let asset_scan_demo_executed = execute_asset_command_from_palette(
        &registry,
        &mut palette,
        &mut project_state,
        &mut asset_state,
        ASSET_SCAN_DEMO_COMMAND,
    )?;
    apply_editor_snapshot(
        &mut tree,
        shell.ids,
        &project_state.ui_snapshot(),
        &asset_state.ui_snapshot(),
        bounds,
    )
    .map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to apply asset scan to UI: {error}"))
    })?;
    let asset_count = asset_state.index().len();
    let asset_kinds_summary = asset_state.kind_summary();

    let asset_select_first_executed = execute_asset_command_from_palette(
        &registry,
        &mut palette,
        &mut project_state,
        &mut asset_state,
        ASSET_SELECT_FIRST_COMMAND,
    )?;
    apply_editor_snapshot(
        &mut tree,
        shell.ids,
        &project_state.ui_snapshot(),
        &asset_state.ui_snapshot(),
        bounds,
    )
    .map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!(
            "failed to apply asset selection to UI: {error}"
        ))
    })?;
    let asset_selected_summary = asset_state.ui_snapshot().asset_selected_summary;

    let asset_clear_selection_executed = execute_asset_command_from_palette(
        &registry,
        &mut palette,
        &mut project_state,
        &mut asset_state,
        ASSET_CLEAR_SELECTION_COMMAND,
    )?;
    apply_editor_snapshot(
        &mut tree,
        shell.ids,
        &project_state.ui_snapshot(),
        &asset_state.ui_snapshot(),
        bounds,
    )
    .map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!(
            "failed to apply cleared asset selection to UI: {error}"
        ))
    })?;
    let asset_cleared_summary = asset_state.ui_snapshot().asset_selected_summary;

    let project_validate_executed = execute_project_command_from_palette(
        &registry,
        &mut palette,
        &mut project_state,
        PROJECT_VALIDATE_COMMAND,
    )?;
    apply_editor_snapshot(
        &mut tree,
        shell.ids,
        &project_state.ui_snapshot(),
        &asset_state.ui_snapshot(),
        bounds,
    )
    .map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!(
            "failed to apply project validation to UI: {error}"
        ))
    })?;
    let project_validation_summary = project_state
        .ui_snapshot()
        .project_diagnostics
        .trim_start_matches("Diagnostics: ")
        .to_string();

    let project_close_executed = execute_project_command_from_palette(
        &registry,
        &mut palette,
        &mut project_state,
        PROJECT_CLOSE_COMMAND,
    )?;
    apply_editor_snapshot(
        &mut tree,
        shell.ids,
        &project_state.ui_snapshot(),
        &asset_state.ui_snapshot(),
        bounds,
    )
    .map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!(
            "failed to apply closed project state to UI: {error}"
        ))
    })?;
    let project_closed_status = project_state.ui_snapshot().status;
    let scene = tree.paint(&PaintContext::new(theme)).map_err(|error| {
        elcarax_core::ElcaraxError::Internal(format!("failed to paint UI shell: {error}"))
    })?;
    Ok(ConsoleUiProof {
        scene,
        node_count: tree.node_count(),
        layout_count: tree.node_count(),
        dirty: tree.dirty_summary(),
        run_clicked,
        ready_executed,
        status_text: ready_status_text,
        project_initial_status,
        project_loaded_status,
        project_validation_summary,
        project_closed_status,
        project_new_demo_executed,
        project_validate_executed,
        project_close_executed,
        asset_scan_demo_executed,
        asset_select_first_executed,
        asset_clear_selection_executed,
        asset_count,
        asset_kinds_summary,
        asset_selected_summary,
        asset_cleared_summary,
    })
}

fn execute_project_command_from_palette(
    registry: &CommandRegistry,
    palette: &mut CommandPaletteState,
    project_state: &mut ProjectState,
    query: &str,
) -> Result<bool> {
    let Some(id) = execute_palette_query(registry, palette, query)? else {
        return Ok(false);
    };
    Ok(project_state.execute_command_id(id.as_str()).is_some())
}

fn execute_asset_command_from_palette(
    registry: &CommandRegistry,
    palette: &mut CommandPaletteState,
    project_state: &ProjectState,
    asset_state: &mut AssetState,
    query: &str,
) -> Result<bool> {
    let Some(id) = execute_palette_query(registry, palette, query)? else {
        return Ok(false);
    };
    let project_loaded = project_state.is_project_loaded();
    Ok(asset_state
        .execute_command_id(id.as_str(), project_loaded)
        .is_some())
}

fn execute_palette_query(
    registry: &CommandRegistry,
    palette: &mut CommandPaletteState,
    query: &str,
) -> Result<Option<CommandId>> {
    palette.open();
    for character in query.chars() {
        let action = palette.handle_key(KeyboardKey::Character(character.to_string()));
        if action != CommandPaletteAction::None {
            return Err(elcarax_core::ElcaraxError::Internal(
                "typing should not execute command palette action".to_string(),
            ));
        }
    }
    if palette.handle_key(KeyboardKey::Enter) != CommandPaletteAction::Execute {
        palette.close();
        return Ok(None);
    }
    let id = palette
        .selected_entry()
        .map(|entry| CommandId::new(entry.id.as_str()))
        .transpose()
        .map_err(|error| {
            elcarax_core::ElcaraxError::Internal(format!("invalid command ID: {error}"))
        })?;
    palette.close();
    let Some(id) = id else {
        return Ok(None);
    };
    match registry.invoke(&id) {
        CommandResult::Invoked(invocation) => Ok(Some(invocation.id)),
        CommandResult::Disabled(_) | CommandResult::NotFound(_) => Ok(None),
    }
}

fn label_text(tree: &elcarax_ui::UiTree, id: elcarax_ui::WidgetId) -> String {
    tree.get(id)
        .and_then(|node| match &node.kind {
            elcarax_ui::WidgetKind::Label(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "Status: Unknown".to_string())
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
