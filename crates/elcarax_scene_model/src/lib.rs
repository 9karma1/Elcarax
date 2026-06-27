//! Engine-neutral scene, object, schema, and property model.

mod demo;
mod diagnostic;
mod error;
mod hierarchy;
mod kind;
mod name;
mod property;
mod schema;
mod selection;
mod snapshot;

pub use demo::demo_scene_snapshot;
pub use diagnostic::SceneDiagnostic;
pub use error::SceneError;
pub use hierarchy::{SceneHierarchy, SceneTreeRow};
pub use kind::SceneObjectKind;
pub use name::{SceneName, SceneObjectName};
pub use property::{PropertyPath, PropertyValue};
pub use schema::{ObjectSchema, ObjectTypeId, PropertyKind, PropertySchema};
pub use selection::{SceneExpansion, SceneSelection};
pub use snapshot::{
    SceneId, SceneMarker, SceneObject, SceneObjectId, SceneObjectMarker, SceneSnapshot,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_scene_has_stable_object_order() {
        let snapshot = demo_scene_snapshot();
        let mut expansion = SceneExpansion::new();
        expansion.expand_all(&snapshot);
        let rows = SceneHierarchy::visible_rows(&snapshot, &expansion);
        let names: Vec<_> = rows.iter().map(|row| row.display_name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "World",
                "Directional Light",
                "Main Camera",
                "Player",
                "Player Mesh",
                "Player Audio",
                "Environment",
                "Ground",
                "Cube",
                "Trigger Zone"
            ]
        );
    }

    #[test]
    fn hierarchy_parent_child_relationships_are_valid() {
        let snapshot = demo_scene_snapshot();
        assert!(SceneHierarchy::validate(&snapshot).is_ok());
        let world = match snapshot.object_by_name("World") {
            Some(object) => object,
            None => panic!("world should exist in demo scene"),
        };
        assert_eq!(world.children.len(), 4);
        let player = match snapshot.object_by_name("Player") {
            Some(object) => object,
            None => panic!("player should exist in demo scene"),
        };
        assert_eq!(player.children.len(), 2);
    }

    #[test]
    fn selecting_existing_object_succeeds() {
        let snapshot = demo_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(object) => object,
            None => panic!("player should exist in demo scene"),
        };
        let mut selection = SceneSelection::none();
        assert!(selection.select_existing(&snapshot, player.id).is_ok());
        assert_eq!(selection.selected(), Some(player.id));
    }

    #[test]
    fn selecting_missing_object_returns_clear_result() {
        let snapshot = demo_scene_snapshot();
        let mut selection = SceneSelection::none();
        let missing = match std::num::NonZeroU64::new(999) {
            Some(value) => SceneObjectId::from_non_zero(value),
            None => panic!("missing test ID should be non-zero"),
        };
        assert_eq!(
            selection.select_existing(&snapshot, missing),
            Err(SceneError::ObjectNotFound)
        );
    }

    #[test]
    fn expand_all_includes_all_expandable_nodes() {
        let snapshot = demo_scene_snapshot();
        let mut expansion = SceneExpansion::new();
        expansion.expand_all(&snapshot);
        assert_eq!(expansion.len(), 3);
        assert!(
            expansion.is_expanded(match snapshot.object_by_name("World") {
                Some(object) => object.id,
                None => panic!("world should exist"),
            })
        );
    }

    #[test]
    fn collapse_all_clears_expanded_state() {
        let snapshot = demo_scene_snapshot();
        let mut expansion = SceneExpansion::new();
        expansion.expand_all(&snapshot);
        expansion.collapse_all();
        assert!(expansion.is_empty());
    }

    #[test]
    fn scene_object_lookup_by_id_works() {
        let snapshot = demo_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(object) => object,
            None => panic!("player should exist in demo scene"),
        };
        assert_eq!(
            snapshot
                .object(player.id)
                .map(|object| object.display_name.as_str()),
            Ok("Player")
        );
    }

    #[test]
    fn scene_object_lookup_by_name_works() {
        let snapshot = demo_scene_snapshot();
        assert!(snapshot.object_by_name("Trigger Zone").is_some());
    }

    #[test]
    fn collapsed_node_hides_descendants() {
        let snapshot = demo_scene_snapshot();
        let expansion = SceneExpansion::new();
        let rows = SceneHierarchy::visible_rows(&snapshot, &expansion);
        let names: Vec<_> = rows.iter().map(|row| row.display_name.as_str()).collect();
        assert_eq!(names, vec!["World"]);
    }

    #[test]
    fn expanded_scene_tree_includes_descendants() {
        let snapshot = demo_scene_snapshot();
        let mut expansion = SceneExpansion::new();
        expansion.expand_all(&snapshot);
        let rows = SceneHierarchy::visible_rows(&snapshot, &expansion);
        assert_eq!(rows.len(), 10);
        assert!(rows.iter().any(|row| row.display_name.as_str() == "Player"));
    }
}
