use crate::kind::SceneObjectKind;
use crate::name::SceneName;
use crate::property_display::PropertyGroup;
use crate::schema::ObjectSchema;
use crate::snapshot::{SceneId, SceneObject, SceneObjectId, SceneSnapshot};
use crate::{PropertyKind, PropertyPath, PropertySchema, PropertyValue};

pub fn demo_scene_snapshot() -> SceneSnapshot {
    let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Demo Scene"));
    snapshot.set_scene_id(stable_scene_id(100));

    let mut world = object(1, "World", SceneObjectKind::World);
    let mut directional_light = object(2, "Directional Light", SceneObjectKind::Light);
    let mut main_camera = object(3, "Main Camera", SceneObjectKind::Camera);
    let mut player = object(4, "Player", SceneObjectKind::Character);
    let player_mesh = object(5, "Player Mesh", SceneObjectKind::Mesh);
    let player_audio = object(6, "Player Audio", SceneObjectKind::Audio);
    let environment = object(7, "Environment", SceneObjectKind::Environment);
    let ground = object(8, "Ground", SceneObjectKind::Ground);
    let mut cube = object(9, "Cube", SceneObjectKind::Cube);
    let trigger_zone = object(10, "Trigger Zone", SceneObjectKind::Trigger);

    let general = PropertyGroup::new("General");
    let transform = PropertyGroup::new("Transform");
    let lighting = PropertyGroup::new("Lighting");
    let camera = PropertyGroup::new("Camera");
    let gameplay = PropertyGroup::new("Gameplay");
    let references = PropertyGroup::new("References");

    let world_schema = ObjectSchema::new("World")
        .with_property(read_only_property(
            &["general", "name"],
            "Name",
            PropertyKind::String,
            &general,
        ))
        .with_property(read_only_property(
            &["general", "enabled"],
            "Enabled",
            PropertyKind::Bool,
            &general,
        ));
    apply_schema(
        &mut world,
        &world_schema,
        &[
            (
                &["general", "name"],
                PropertyValue::String("World".to_string()),
            ),
            (&["general", "enabled"], PropertyValue::Bool(true)),
        ],
    );
    snapshot.add_schema(world_schema);

    let light_schema = ObjectSchema::new("Light")
        .with_property(read_only_property(
            &["lighting", "intensity"],
            "Intensity",
            PropertyKind::F64,
            &lighting,
        ))
        .with_property(read_only_property(
            &["lighting", "color"],
            "Color",
            PropertyKind::ColorRgba,
            &lighting,
        ))
        .with_property(read_only_property(
            &["transform", "rotation"],
            "Rotation",
            PropertyKind::Vec3,
            &transform,
        ));
    apply_schema(
        &mut directional_light,
        &light_schema,
        &[
            (&["lighting", "intensity"], PropertyValue::F64(3.5)),
            (
                &["lighting", "color"],
                PropertyValue::ColorRgba([1.0, 0.95, 0.8, 1.0]),
            ),
            (
                &["transform", "rotation"],
                PropertyValue::Vec3([-45.0, 45.0, 0.0]),
            ),
        ],
    );
    snapshot.add_schema(light_schema);

    let camera_schema = ObjectSchema::new("Camera")
        .with_property(read_only_property(
            &["transform", "position"],
            "Position",
            PropertyKind::Vec3,
            &transform,
        ))
        .with_property(read_only_property(
            &["transform", "rotation"],
            "Rotation",
            PropertyKind::Vec3,
            &transform,
        ))
        .with_property(read_only_property(
            &["camera", "field_of_view"],
            "Field Of View",
            PropertyKind::F64,
            &camera,
        ));
    apply_schema(
        &mut main_camera,
        &camera_schema,
        &[
            (
                &["transform", "position"],
                PropertyValue::Vec3([0.0, 2.0, -8.0]),
            ),
            (
                &["transform", "rotation"],
                PropertyValue::Vec3([15.0, 0.0, 0.0]),
            ),
            (&["camera", "field_of_view"], PropertyValue::F64(60.0)),
        ],
    );
    snapshot.add_schema(camera_schema);

    let player_schema = ObjectSchema::new("Character")
        .with_property(read_only_property(
            &["general", "name"],
            "Name",
            PropertyKind::String,
            &general,
        ))
        .with_property(read_only_property(
            &["transform", "position"],
            "Position",
            PropertyKind::Vec3,
            &transform,
        ))
        .with_property(read_only_property(
            &["transform", "rotation"],
            "Rotation",
            PropertyKind::Vec3,
            &transform,
        ))
        .with_property(read_only_property(
            &["transform", "scale"],
            "Scale",
            PropertyKind::Vec3,
            &transform,
        ))
        .with_property(read_only_property(
            &["gameplay", "health"],
            "Health",
            PropertyKind::I64,
            &gameplay,
        ))
        .with_property(read_only_property(
            &["gameplay", "speed"],
            "Speed",
            PropertyKind::F64,
            &gameplay,
        ))
        .with_property(read_only_property(
            &["references", "mesh"],
            "Mesh",
            PropertyKind::AssetRef,
            &references,
        ));
    apply_schema(
        &mut player,
        &player_schema,
        &[
            (
                &["general", "name"],
                PropertyValue::String("Player".to_string()),
            ),
            (
                &["transform", "position"],
                PropertyValue::Vec3([0.0, 1.0, 0.0]),
            ),
            (
                &["transform", "rotation"],
                PropertyValue::Vec3([0.0, 0.0, 0.0]),
            ),
            (
                &["transform", "scale"],
                PropertyValue::Vec3([1.0, 1.0, 1.0]),
            ),
            (&["gameplay", "health"], PropertyValue::I64(100)),
            (&["gameplay", "speed"], PropertyValue::F64(6.5)),
            (
                &["references", "mesh"],
                PropertyValue::AssetRef("assets/models/cube.glb".to_string()),
            ),
        ],
    );
    player.property_summary = Some("Health: 100 | Speed: 6.5".to_string());
    snapshot.add_schema(player_schema);

    let cube_schema = ObjectSchema::new("Cube")
        .with_property(read_only_property(
            &["transform", "position"],
            "Position",
            PropertyKind::Vec3,
            &transform,
        ))
        .with_property(read_only_property(
            &["references", "material"],
            "Material",
            PropertyKind::AssetRef,
            &references,
        ));
    apply_schema(
        &mut cube,
        &cube_schema,
        &[
            (
                &["transform", "position"],
                PropertyValue::Vec3([2.0, 0.5, 1.0]),
            ),
            (
                &["references", "material"],
                PropertyValue::AssetRef("assets/materials/default.material".to_string()),
            ),
        ],
    );
    snapshot.add_schema(cube_schema);

    snapshot.add_root_object(world);
    let world_id = stable_object_id(1);
    let _ = snapshot.attach_child(world_id, directional_light);
    let _ = snapshot.attach_child(world_id, main_camera);
    let _ = snapshot.attach_child(world_id, player);
    let player_id = stable_object_id(4);
    let _ = snapshot.attach_child(player_id, player_mesh);
    let _ = snapshot.attach_child(player_id, player_audio);
    let _ = snapshot.attach_child(world_id, environment);
    let environment_id = stable_object_id(7);
    let _ = snapshot.attach_child(environment_id, ground);
    let _ = snapshot.attach_child(environment_id, cube);
    let _ = snapshot.attach_child(environment_id, trigger_zone);
    snapshot
}

fn read_only_property(
    segments: &[&str],
    display_name: &str,
    kind: PropertyKind,
    group: &PropertyGroup,
) -> PropertySchema {
    let path = PropertyPath::demo_from_segments(segments);
    PropertySchema::read_only(path, display_name, kind, group.clone())
}

fn apply_schema(
    object: &mut SceneObject,
    schema: &ObjectSchema,
    values: &[(&[&str], PropertyValue)],
) {
    object.type_id = schema.type_id;
    for (segments, value) in values {
        let path = PropertyPath::demo_from_segments(segments);
        object.set_property(path, value.clone());
    }
}

fn object(id: u64, name: &str, kind: SceneObjectKind) -> SceneObject {
    SceneObject::with_stable_id(stable_object_id(id), name, kind)
}

fn stable_scene_id(value: u64) -> SceneId {
    stable_id(value)
}

fn stable_object_id(value: u64) -> SceneObjectId {
    stable_id(value)
}

fn stable_id<T>(value: u64) -> elcarax_core::Id<T> {
    use std::num::NonZeroU64;
    match NonZeroU64::new(value) {
        Some(value) => elcarax_core::Id::from_non_zero(value),
        None => elcarax_core::Id::from_non_zero(NonZeroU64::MIN),
    }
}
