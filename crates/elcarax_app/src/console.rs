use elcarax_commands::{
    CommandContext, CommandHistory, CommandRegistry, CommandResult, PropertyChangeCommand,
    RegisteredCommand, built_in_commands,
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
    build_editor_shell_with_ids,
};

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
}

fn build_console_ui(shell: &NativeShellSpec) -> Result<ConsoleUiProof> {
    let theme = Theme::editor_dark();
    let context = UiContext::new(
        theme,
        Rect::new(0.0, 0.0, shell.width as f32, shell.height as f32),
    );
    let shell = build_editor_shell_with_ids(&context).map_err(|error| {
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
    palette.open();
    for character in ["r", "e", "a", "d", "y"] {
        let action = palette.handle_key(KeyboardKey::Character(character.to_string()));
        if action != CommandPaletteAction::None {
            return Err(elcarax_core::ElcaraxError::Internal(
                "typing should not execute command palette action".to_string(),
            ));
        }
    }
    let mut ready_executed = false;
    if palette.handle_key(KeyboardKey::Enter) == CommandPaletteAction::Execute {
        if let Some(entry) = palette.selected_entry() {
            let id = elcarax_commands::CommandId::new(entry.id.as_str()).map_err(|error| {
                elcarax_core::ElcaraxError::Internal(format!("invalid command ID: {error}"))
            })?;
            if matches!(registry.invoke(&id), CommandResult::Invoked(_))
                && id.as_str() == "elcarax.status.show_ready"
            {
                tree.set_label_text(shell.ids.status_label, "Status: Ready")
                    .map_err(|error| {
                        elcarax_core::ElcaraxError::Internal(format!(
                            "failed to update status: {error}"
                        ))
                    })?;
                ready_executed = true;
            }
        }
        palette.close();
    }
    let status_text = tree
        .get(shell.ids.status_label)
        .and_then(|node| match &node.kind {
            elcarax_ui::WidgetKind::Label(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "Status: Unknown".to_string());
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
        status_text,
    })
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
