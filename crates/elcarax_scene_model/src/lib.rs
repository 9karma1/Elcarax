//! Engine-neutral scene, object, schema, and property model.

mod demo;
mod diagnostic;
mod error;
mod hierarchy;
mod inspector;
mod kind;
mod name;
mod property;
mod property_display;
mod property_edit;
mod schema;
mod selection;
mod snapshot;

pub use demo::demo_scene_snapshot;
pub use diagnostic::SceneDiagnostic;
pub use error::SceneError;
pub use hierarchy::{SceneHierarchy, SceneTreeRow};
pub use inspector::{
    InspectorDiagnostic, InspectorObject, InspectorRow, InspectorSection,
    build_inspector_for_selection, build_inspector_object,
};
pub use kind::SceneObjectKind;
pub use name::{PropertyName, SceneName, SceneObjectName};
pub use property::{PropertyPath, PropertyValue};
pub use property_display::{
    PropertyDisplay, PropertyFormatContext, PropertyGroup, PropertyId, format_property_value,
};
pub use property_edit::{
    PropertyChange, PropertyChangeValue, PropertyEditError, PropertyEditResult,
    apply_property_change, edit_scene_property, prepare_property_change,
};
pub use schema::{
    NumericEditMetadata, ObjectSchema, ObjectTypeId, PropertyEditKind, PropertyKind, PropertySchema,
};
pub use selection::{SceneExpansion, SceneSelection};
pub use snapshot::{
    SceneId, SceneMarker, SceneObject, SceneObjectId, SceneObjectMarker, SceneSnapshot,
};

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_core::Result;

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

    #[test]
    fn property_value_formatting_covers_core_kinds() {
        let snapshot = demo_scene_snapshot();
        let context = PropertyFormatContext {
            snapshot: &snapshot,
        };
        assert_eq!(
            format_property_value(&PropertyValue::Bool(true), context),
            "true"
        );
        assert_eq!(
            format_property_value(&PropertyValue::I64(100), context),
            "100"
        );
        assert_eq!(
            format_property_value(&PropertyValue::F64(6.5), context),
            "6.50"
        );
        assert_eq!(
            format_property_value(&PropertyValue::String("Player".to_string()), context),
            "Player"
        );
        assert_eq!(
            format_property_value(&PropertyValue::Vec3([0.0, 1.0, 0.0]), context),
            "0.00, 1.00, 0.00"
        );
        assert_eq!(
            format_property_value(&PropertyValue::ColorRgba([1.0, 0.95, 0.8, 1.0]), context),
            "rgba(1.00, 0.95, 0.80, 1.00)"
        );
        assert_eq!(
            format_property_value(&PropertyValue::Unknown, context),
            "<unsupported>"
        );
    }

    #[test]
    fn demo_player_has_expected_properties() {
        let snapshot = demo_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(object) => object,
            None => panic!("player should exist"),
        };
        let inspector = match build_inspector_object(&snapshot, player.id) {
            Ok(value) => value,
            Err(_) => panic!("player inspector should build"),
        };
        assert_eq!(inspector.name, "Player");
        assert_eq!(inspector.kind, SceneObjectKind::Character);
        assert_eq!(inspector.property_count(), 7);
        let labels: Vec<_> = inspector
            .sections
            .iter()
            .flat_map(|section| section.rows.iter().map(|row| row.label.as_str()))
            .collect();
        assert!(labels.contains(&"Health"));
        assert!(labels.contains(&"Speed"));
        assert!(labels.contains(&"Mesh"));
    }

    #[test]
    fn editing_editable_int_property_succeeds() -> Result<()> {
        let mut snapshot = demo_scene_snapshot();
        let player_id = player_id(&snapshot);
        let path = path("gameplay.health");
        let change = edit_scene_property(&mut snapshot, player_id, &path, PropertyValue::I64(75))
            .map_err(|error| elcarax_core::ElcaraxError::Command(error.message()))?;
        assert_eq!(change.old_value, PropertyValue::I64(100));
        assert_eq!(change.new_value, PropertyValue::I64(75));
        assert_eq!(
            snapshot.object(player_id)?.property(&path),
            Some(&PropertyValue::I64(75))
        );
        Ok(())
    }

    #[test]
    fn editing_editable_float_property_succeeds() -> Result<()> {
        let mut snapshot = demo_scene_snapshot();
        let player_id = player_id(&snapshot);
        let path = path("gameplay.speed");
        let change = edit_scene_property(&mut snapshot, player_id, &path, PropertyValue::F64(8.0))
            .map_err(|error| elcarax_core::ElcaraxError::Command(error.message()))?;
        assert_eq!(change.old_value, PropertyValue::F64(6.5));
        Ok(())
    }

    #[test]
    fn editing_editable_string_property_succeeds() -> Result<()> {
        let mut snapshot = demo_scene_snapshot();
        let player_id = player_id(&snapshot);
        let path = path("general.name");
        edit_scene_property(
            &mut snapshot,
            player_id,
            &path,
            PropertyValue::String("Hero".to_string()),
        )
        .map_err(|error| elcarax_core::ElcaraxError::Command(error.message()))?;
        assert_eq!(snapshot.object(player_id)?.display_name, "Hero");
        Ok(())
    }

    #[test]
    fn editing_editable_vec3_property_succeeds() -> Result<()> {
        let mut snapshot = demo_scene_snapshot();
        let player_id = player_id(&snapshot);
        let path = path("transform.position");
        let change = edit_scene_property(
            &mut snapshot,
            player_id,
            &path,
            PropertyValue::Vec3([2.0, 3.0, 4.0]),
        )
        .map_err(|error| elcarax_core::ElcaraxError::Command(error.message()))?;
        assert_eq!(change.old_value, PropertyValue::Vec3([0.0, 1.0, 0.0]));
        Ok(())
    }

    #[test]
    fn editing_read_only_asset_ref_fails_clearly() {
        let mut snapshot = demo_scene_snapshot();
        let player_id = player_id(&snapshot);
        let path = path("references.mesh");
        let result = edit_scene_property(
            &mut snapshot,
            player_id,
            &path,
            PropertyValue::AssetRef("assets/models/hero.glb".to_string()),
        );
        let error = match result {
            Ok(_) => panic!("asset refs are read-only"),
            Err(error) => error,
        };
        assert!(matches!(error, PropertyEditError::ReadOnly { .. }));
    }

    #[test]
    fn editing_missing_property_fails_clearly() {
        let mut snapshot = demo_scene_snapshot();
        let player_id = player_id(&snapshot);
        let result = edit_scene_property(
            &mut snapshot,
            player_id,
            &path("gameplay.mana"),
            PropertyValue::I64(10),
        );
        let error = match result {
            Ok(_) => panic!("missing property should fail"),
            Err(error) => error,
        };
        assert!(matches!(error, PropertyEditError::PropertyNotFound { .. }));
    }

    #[test]
    fn editing_missing_object_fails_clearly() {
        let mut snapshot = demo_scene_snapshot();
        let missing = match std::num::NonZeroU64::new(999) {
            Some(value) => SceneObjectId::from_non_zero(value),
            None => panic!("missing test ID should be non-zero"),
        };
        let result = edit_scene_property(
            &mut snapshot,
            missing,
            &path("gameplay.health"),
            PropertyValue::I64(75),
        );
        let error = match result {
            Ok(_) => panic!("missing object should fail"),
            Err(error) => error,
        };
        assert!(matches!(error, PropertyEditError::ObjectNotFound { .. }));
    }

    #[test]
    fn editing_type_mismatch_fails_clearly() {
        let mut snapshot = demo_scene_snapshot();
        let player_id = player_id(&snapshot);
        let result = edit_scene_property(
            &mut snapshot,
            player_id,
            &path("gameplay.health"),
            PropertyValue::String("high".to_string()),
        );
        let error = match result {
            Ok(_) => panic!("wrong value type should fail"),
            Err(error) => error,
        };
        assert!(matches!(error, PropertyEditError::TypeMismatch { .. }));
    }

    #[test]
    fn missing_object_returns_clear_inspector_diagnostic() {
        let snapshot = demo_scene_snapshot();
        let missing = match std::num::NonZeroU64::new(999) {
            Some(value) => SceneObjectId::from_non_zero(value),
            None => panic!("missing test ID should be non-zero"),
        };
        assert_eq!(
            build_inspector_object(&snapshot, missing),
            Err(InspectorDiagnostic::ObjectNotFound)
        );
    }

    #[test]
    fn read_only_inspector_rows_are_stable_sorted_and_grouped() {
        let snapshot = demo_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(object) => object,
            None => panic!("player should exist"),
        };
        let inspector = match build_inspector_object(&snapshot, player.id) {
            Ok(value) => value,
            Err(_) => panic!("player inspector should build"),
        };
        let section_titles: Vec<_> = inspector
            .sections
            .iter()
            .map(|section| section.title.as_str())
            .collect();
        assert_eq!(
            section_titles,
            vec!["Gameplay", "General", "References", "Transform"]
        );
        let gameplay = match inspector.sections.first() {
            Some(section) => section,
            None => panic!("gameplay section should exist"),
        };
        assert_eq!(gameplay.rows[0].label.as_str(), "Health");
        assert_eq!(gameplay.rows[1].label.as_str(), "Speed");
    }

    #[test]
    fn no_selection_returns_clear_inspector_diagnostic() {
        let snapshot = demo_scene_snapshot();
        assert_eq!(
            build_inspector_for_selection(&snapshot, None),
            Err(InspectorDiagnostic::NoObjectSelected)
        );
    }

    fn player_id(snapshot: &SceneSnapshot) -> SceneObjectId {
        match snapshot.object_by_name("Player") {
            Some(object) => object.id,
            None => panic!("player should exist"),
        }
    }

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("test path should parse: {error}"),
        }
    }
}
