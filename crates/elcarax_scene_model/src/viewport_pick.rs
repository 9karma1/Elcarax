//! Orthographic viewport picking for engine-neutral scene snapshots.

use crate::{PropertyPath, PropertyValue, SceneObjectId, SceneSnapshot};

const PICK_RADIUS: f32 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPickCoord {
    pub u: f32,
    pub v: f32,
}

pub fn pick_object_at(snapshot: &SceneSnapshot, coord: ViewportPickCoord) -> Option<SceneObjectId> {
    let world_x = coord.u * 2.0 - 1.0;
    let world_z = 1.0 - coord.v * 2.0;
    let mut best: Option<(SceneObjectId, f32)> = None;
    for object in snapshot.objects().values() {
        let Some(position) = object_position(object) else {
            continue;
        };
        let dx = position[0] - world_x;
        let dz = position[2] - world_z;
        let distance_sq = dx * dx + dz * dz;
        if distance_sq > PICK_RADIUS * PICK_RADIUS {
            continue;
        }
        if best.is_none_or(|(_, best_distance)| distance_sq < best_distance) {
            best = Some((object.id, distance_sq));
        }
    }
    best.map(|(object_id, _)| object_id)
}

fn object_position(object: &crate::SceneObject) -> Option<[f32; 3]> {
    let path = match PropertyPath::from_static_segments(&["transform", "position"]) {
        Ok(path) => path,
        Err(_) => return None,
    };
    match object.property(&path)? {
        PropertyValue::Vec3(position) => Some(*position),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference_scene_snapshot;

    #[test]
    fn pick_selects_nearest_object_by_position() {
        let snapshot = reference_scene_snapshot();
        let player = match snapshot
            .objects()
            .values()
            .find(|object| object.display_name.as_str() == "Player")
            .map(|object| object.id)
        {
            Some(object_id) => object_id,
            None => panic!("player should exist"),
        };
        let picked = pick_object_at(&snapshot, ViewportPickCoord { u: 0.5, v: 0.5 });
        assert_eq!(picked, Some(player));
    }
}
