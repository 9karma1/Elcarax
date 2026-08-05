use crate::component::{ComponentInstance, ComponentInstanceId, well_known as components};
use crate::kind::{SceneObjectKind, well_known as kinds};
use crate::name::SceneName;
use crate::schema::{ComponentSchema, ObjectSchema};
use crate::snapshot::{SceneId, SceneObject, SceneObjectId, SceneSnapshot};
use crate::{PropertyKind, PropertyPath, PropertySchema, PropertyValue};

pub fn reference_scene_snapshot() -> SceneSnapshot {
    let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Reference Scene"));
    snapshot.set_scene_id(stable_scene_id(100));

    let world_schema = ObjectSchema::new("World").with_component(
        ComponentSchema::new(components::GENERAL, "General")
            .with_property(read_only("name", "Name", PropertyKind::String))
            .with_property(read_only("enabled", "Enabled", PropertyKind::Bool)),
    );
    let mut world = object(1, "World", kinds::WORLD).with_component(
        component(1, 1, components::GENERAL, "General")
            .with_property(path("name"), PropertyValue::String("World".to_string()))
            .with_property(path("enabled"), PropertyValue::Bool(true)),
    );
    world.type_id = world_schema.type_id;
    snapshot.add_schema(world_schema);

    let light_schema = ObjectSchema::new("Light")
        .with_component(
            ComponentSchema::new(components::LIGHTING, "Lighting")
                .with_property(read_only("intensity", "Intensity", PropertyKind::F64))
                .with_property(read_only("color", "Color", PropertyKind::ColorRgba)),
        )
        .with_component(
            ComponentSchema::new(components::TRANSFORM, "Transform").with_property(read_only(
                "rotation",
                "Rotation",
                PropertyKind::Vec3,
            )),
        );
    let mut directional_light = object(2, "Directional Light", kinds::LIGHT)
        .with_component(
            component(2, 1, components::LIGHTING, "Lighting")
                .with_property(path("intensity"), PropertyValue::F64(3.5))
                .with_property(
                    path("color"),
                    PropertyValue::ColorRgba([1.0, 0.95, 0.8, 1.0]),
                ),
        )
        .with_component(
            component(2, 2, components::TRANSFORM, "Transform")
                .with_property(path("rotation"), PropertyValue::Vec3([-45.0, 45.0, 0.0])),
        );
    directional_light.type_id = light_schema.type_id;
    snapshot.add_schema(light_schema);

    let camera_schema = ObjectSchema::new("Camera")
        .with_component(
            ComponentSchema::new(components::TRANSFORM, "Transform")
                .with_property(read_only("position", "Position", PropertyKind::Vec3))
                .with_property(read_only("rotation", "Rotation", PropertyKind::Vec3)),
        )
        .with_component(
            ComponentSchema::new(components::CAMERA, "Camera").with_property(read_only(
                "field_of_view",
                "Field Of View",
                PropertyKind::F64,
            )),
        );
    let mut main_camera = object(3, "Main Camera", kinds::CAMERA)
        .with_component(
            component(3, 1, components::TRANSFORM, "Transform")
                .with_property(path("position"), PropertyValue::Vec3([0.0, 2.0, -8.0]))
                .with_property(path("rotation"), PropertyValue::Vec3([15.0, 0.0, 0.0])),
        )
        .with_component(
            component(3, 2, components::CAMERA, "Camera")
                .with_property(path("field_of_view"), PropertyValue::F64(60.0)),
        );
    main_camera.type_id = camera_schema.type_id;
    snapshot.add_schema(camera_schema);

    let player_schema =
        ObjectSchema::new("Character")
            .with_component(
                ComponentSchema::new(components::GENERAL, "General").with_property(editable(
                    "name",
                    "Name",
                    PropertyKind::String,
                )),
            )
            .with_component(
                ComponentSchema::new(components::TRANSFORM, "Transform")
                    .with_property(editable("position", "Position", PropertyKind::Vec3))
                    .with_property(editable("rotation", "Rotation", PropertyKind::Vec3))
                    .with_property(editable("scale", "Scale", PropertyKind::Vec3)),
            )
            .with_component(
                ComponentSchema::new(components::GAMEPLAY, "Gameplay")
                    .with_property(editable("health", "Health", PropertyKind::I64))
                    .with_property(editable("speed", "Speed", PropertyKind::F64))
                    .with_property(editable_enum("stance", "Stance", &["Idle", "Run", "Jump"])),
            )
            .with_component(
                ComponentSchema::new(components::REFERENCES, "References")
                    .with_property(read_only("mesh", "Mesh", PropertyKind::AssetRef)),
            );
    let mut player = object(4, "Player", kinds::CHARACTER)
        .with_component(
            component(4, 1, components::GENERAL, "General")
                .with_property(path("name"), PropertyValue::String("Player".to_string())),
        )
        .with_component(
            component(4, 2, components::TRANSFORM, "Transform")
                .with_property(path("position"), PropertyValue::Vec3([0.0, 1.0, 0.0]))
                .with_property(path("rotation"), PropertyValue::Vec3([0.0, 0.0, 0.0]))
                .with_property(path("scale"), PropertyValue::Vec3([1.0, 1.0, 1.0])),
        )
        .with_component(
            component(4, 3, components::GAMEPLAY, "Gameplay")
                .with_property(path("health"), PropertyValue::I64(100))
                .with_property(path("speed"), PropertyValue::F64(6.5))
                .with_property(
                    path("stance"),
                    PropertyValue::Enum {
                        variant: "Idle".to_string(),
                    },
                ),
        )
        .with_component(
            component(4, 4, components::REFERENCES, "References").with_property(
                path("mesh"),
                PropertyValue::AssetRef("assets/models/cube.glb".to_string()),
            ),
        );
    player.type_id = player_schema.type_id;
    player.property_summary = Some("Health: 100 | Speed: 6.5".to_string());
    snapshot.add_schema(player_schema);

    let cube_schema =
        ObjectSchema::new("Cube")
            .with_component(
                ComponentSchema::new(components::TRANSFORM, "Transform").with_property(read_only(
                    "position",
                    "Position",
                    PropertyKind::Vec3,
                )),
            )
            .with_component(
                ComponentSchema::new(components::REFERENCES, "References")
                    .with_property(read_only("material", "Material", PropertyKind::AssetRef)),
            );
    let mut cube = object(9, "Cube", kinds::CUBE)
        .with_component(
            component(9, 1, components::TRANSFORM, "Transform")
                .with_property(path("position"), PropertyValue::Vec3([2.0, 0.5, 1.0])),
        )
        .with_component(
            component(9, 2, components::REFERENCES, "References").with_property(
                path("material"),
                PropertyValue::AssetRef("assets/materials/default.material".to_string()),
            ),
        );
    cube.type_id = cube_schema.type_id;
    snapshot.add_schema(cube_schema);

    let player_mesh = object(5, "Player Mesh", kinds::MESH);
    let player_audio = object(6, "Player Audio", kinds::AUDIO);
    let environment = object(7, "Environment", kinds::ENVIRONMENT);
    let ground = object(8, "Ground", kinds::GROUND);
    let trigger_zone = object(10, "Trigger Zone", kinds::TRIGGER);

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

fn path(segment: &str) -> PropertyPath {
    PropertyPath::fixture_from_segments(&[segment])
}

fn read_only(segment: &str, display_name: &str, kind: PropertyKind) -> PropertySchema {
    PropertySchema::read_only(path(segment), display_name, kind)
}

fn editable(segment: &str, display_name: &str, kind: PropertyKind) -> PropertySchema {
    PropertySchema::editable(path(segment), display_name, kind)
}

fn editable_enum(segment: &str, display_name: &str, variants: &[&str]) -> PropertySchema {
    PropertySchema::editable_enum(path(segment), display_name, variants)
}

fn component(
    object_value: u64,
    slot: u64,
    type_name: &str,
    display_name: &str,
) -> ComponentInstance {
    ComponentInstance::with_stable_id(
        stable_component_id(object_value * 100 + slot),
        type_name,
        display_name,
    )
}

fn object(id: u64, name: &str, kind: &str) -> SceneObject {
    SceneObject::with_stable_id(stable_object_id(id), name, SceneObjectKind::new(kind))
}

fn stable_scene_id(value: u64) -> SceneId {
    stable_id(value)
}

fn stable_object_id(value: u64) -> SceneObjectId {
    stable_id(value)
}

fn stable_component_id(value: u64) -> ComponentInstanceId {
    stable_id(value)
}

fn stable_id<T>(value: u64) -> elcarax_core::Id<T> {
    use std::num::NonZeroU64;
    match NonZeroU64::new(value) {
        Some(value) => elcarax_core::Id::from_non_zero(value),
        None => elcarax_core::Id::from_non_zero(NonZeroU64::MIN),
    }
}
