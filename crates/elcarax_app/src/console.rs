use elcarax_commands::{CommandContext, CommandHistory, PropertyChangeCommand};
use elcarax_core::Result;
use elcarax_devtools::DevtoolsSnapshot;
use elcarax_gpu::FrameStats;
use elcarax_platform::NativeShellSpec;
use elcarax_project::ProjectFile;
use elcarax_render::{
    Color, Rect, RenderLayer, RenderPrimitive, RenderStats, batch_scene, text_stats,
};
use elcarax_scene_model::{
    ObjectSchema, PropertyKind, PropertyPath, PropertySchema, PropertyValue, SceneObject,
    SceneSnapshot,
};
use elcarax_ui::{UiTree, WidgetKind, WidgetNode};

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
    let primitives = build_placeholder_ui(&shell);
    let text_stats = text_stats(&primitives);
    let snapshot = DevtoolsSnapshot {
        frame: FrameStats::empty(),
        render: RenderStats {
            primitive_count: primitives.primitives().len(),
            batch_count: batch_scene(&primitives).len(),
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
    Ok(())
}

fn build_placeholder_ui(shell: &NativeShellSpec) -> elcarax_render::RenderScene {
    let mut ui = UiTree::new();
    let root_id = ui.set_root(WidgetNode::new(
        WidgetKind::Root,
        Rect {
            x: 0.0,
            y: 0.0,
            width: shell.width as f32,
            height: shell.height as f32,
        },
    ));
    let _panel_id = ui.insert_child(
        root_id,
        WidgetNode::new(
            WidgetKind::Panel,
            Rect {
                x: 16.0,
                y: 16.0,
                width: 320.0,
                height: 860.0,
            },
        ),
    );
    let mut scene = ui.paint();
    let color = Color::srgb(0.91, 0.93, 0.97, 1.0);
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text("Elcarax", 24.0, 38.0, 18.0, color),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text("Project", 32.0, 96.0, 14.0, color),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text("Viewport", 380.0, 96.0, 14.0, color),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text("Inspector", 1180.0, 96.0, 14.0, color),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text("Console", 380.0, shell.height as f32 - 120.0, 14.0, color),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text(
            "Status: Renderer online",
            24.0,
            shell.height as f32 - 24.0,
            13.0,
            color,
        ),
    );
    scene
}
