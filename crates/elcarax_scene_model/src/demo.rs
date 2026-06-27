use crate::kind::SceneObjectKind;
use crate::name::SceneName;
use crate::schema::ObjectSchema;
use crate::snapshot::{SceneId, SceneObject, SceneObjectId, SceneSnapshot};
use crate::{PropertyKind, PropertyPath, PropertySchema, PropertyValue};

pub fn demo_scene_snapshot() -> SceneSnapshot {
    let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Demo Scene"));
    snapshot.set_scene_id(stable_scene_id(100));

    let world = object(1, "World", SceneObjectKind::World);
    let directional_light = object(2, "Directional Light", SceneObjectKind::Light);
    let main_camera = object(3, "Main Camera", SceneObjectKind::Camera);
    let mut player = object(4, "Player", SceneObjectKind::Character);
    let player_mesh = object(5, "Player Mesh", SceneObjectKind::Mesh);
    let player_audio = object(6, "Player Audio", SceneObjectKind::Audio);
    let environment = object(7, "Environment", SceneObjectKind::Environment);
    let ground = object(8, "Ground", SceneObjectKind::Ground);
    let cube = object(9, "Cube", SceneObjectKind::Cube);
    let trigger_zone = object(10, "Trigger Zone", SceneObjectKind::Trigger);

    let position_path = match PropertyPath::parse("transform.position") {
        Ok(path) => path,
        Err(_) => return SceneSnapshot::with_name(SceneName::from_unvalidated("Demo Scene")),
    };
    let player_schema = ObjectSchema::new("Character").with_property(PropertySchema::editable(
        position_path.clone(),
        "Position",
        PropertyKind::Vec3,
    ));
    let player_type_id = player_schema.type_id;
    snapshot.add_schema(player_schema);
    player.type_id = player_type_id;
    player.set_property(position_path, PropertyValue::Vec3([0.0, 1.0, 0.0]));
    player.property_summary = Some("Position: (0, 1, 0)".to_string());

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
