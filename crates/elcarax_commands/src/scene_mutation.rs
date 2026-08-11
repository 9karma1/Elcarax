use elcarax_core::{ElcaraxError, Result};
use elcarax_scene_model::{PropertyChange, ScenePatch, ScenePatchError, property_change_patches};

use crate::{CommandContext, CommandEffect, CommandHistory, EditorCommand};

/// Applies a scene mutation with an explicit inverse for undo.
///
/// Local mutations apply `forward` / `inverse` patches directly.
/// Remote property mutations confirm through [`SceneMutationSink`] on apply and revert.
pub struct ApplyScenePatchCommand {
    label: String,
    payload: SceneMutationPayload,
}

#[derive(Debug, Clone)]
enum SceneMutationPayload {
    Local {
        forward: ScenePatch,
        inverse: ScenePatch,
    },
    RemoteProperty {
        change: PropertyChange,
    },
}

impl ApplyScenePatchCommand {
    pub fn local(forward: ScenePatch, inverse: ScenePatch, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            payload: SceneMutationPayload::Local { forward, inverse },
        }
    }

    pub fn from_property_change(change: PropertyChange, label: impl Into<String>) -> Self {
        let (forward, inverse) = property_change_patches(&change);
        Self::local(forward, inverse, label)
    }

    pub fn remote_property(change: PropertyChange, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            payload: SceneMutationPayload::RemoteProperty { change },
        }
    }

    pub fn change(&self) -> Option<&PropertyChange> {
        match &self.payload {
            SceneMutationPayload::RemoteProperty { change } => Some(change),
            SceneMutationPayload::Local { .. } => None,
        }
    }
}

impl EditorCommand for ApplyScenePatchCommand {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect> {
        match &self.payload {
            SceneMutationPayload::Local { forward, .. } => {
                apply_local_patch(context.scene, forward, context.property_types)?;
            }
            SceneMutationPayload::RemoteProperty { change } => {
                apply_remote_property(context, change, true)?;
            }
        }
        Ok(CommandEffect::SceneChanged)
    }

    fn revert(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect> {
        match &self.payload {
            SceneMutationPayload::Local { inverse, .. } => {
                apply_local_patch(context.scene, inverse, context.property_types)?;
            }
            SceneMutationPayload::RemoteProperty { change } => {
                apply_remote_property(context, change, false)?;
            }
        }
        Ok(CommandEffect::SceneChanged)
    }
}

fn apply_local_patch(
    scene: &mut elcarax_scene_model::SceneSnapshot,
    patch: &ScenePatch,
    property_types: &elcarax_scene_model::PropertyTypeRegistry,
) -> Result<()> {
    patch
        .apply(scene, property_types)
        .map_err(|error| ElcaraxError::Command(error.message()))
}

fn apply_remote_property(
    context: &mut CommandContext<'_>,
    change: &PropertyChange,
    forward: bool,
) -> Result<()> {
    let sink = context.mutation_sink.as_mut().ok_or_else(|| {
        ElcaraxError::Command(
            "remote scene mutation requires an adapter writeback sink".to_string(),
        )
    })?;
    let request = if forward {
        change.clone()
    } else {
        PropertyChange {
            scene_id: change.scene_id,
            object_id: change.object_id,
            component_id: change.component_id,
            path: change.path.clone(),
            old_value: change.new_value.clone(),
            new_value: change.old_value.clone(),
        }
    };
    let patch = sink
        .confirm_property_change(&request)
        .map_err(ElcaraxError::Command)?;
    patch
        .apply(context.scene, context.property_types)
        .map_err(|error: ScenePatchError| ElcaraxError::Command(error.message()))?;
    Ok(())
}

pub struct UndoCommand;

impl UndoCommand {
    pub fn apply(
        history: &mut CommandHistory,
        context: &mut CommandContext<'_>,
    ) -> Result<Option<CommandEffect>> {
        history.undo(context)
    }
}

pub struct RedoCommand;

impl RedoCommand {
    pub fn apply(
        history: &mut CommandHistory,
        context: &mut CommandContext<'_>,
    ) -> Result<Option<CommandEffect>> {
        history.redo(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandContext, CommandHistory};
    use elcarax_core::Result;
    use elcarax_scene_model::{
        ComponentInstance, ComponentSchema, ObjectSchema, PropertyKind, PropertyPath,
        PropertySchema, PropertyTypeRegistry, PropertyValue, SceneObject, SceneObjectKind,
        SceneSnapshot, components, kinds, prepare_property_change,
    };

    #[test]
    fn property_change_can_be_undone() -> Result<()> {
        let path = PropertyPath::parse("position")?;
        let (mut scene, object_id, component_id) = scene_with_position(path.clone());
        let property_types = PropertyTypeRegistry::default();
        let mut context = CommandContext {
            scene: &mut scene,
            mutation_sink: None,
            property_types: &property_types,
        };
        let mut history = CommandHistory::new();
        let change = prepare_change(
            context.scene,
            object_id,
            component_id,
            &path,
            PropertyValue::Vec3([1.0, 2.0, 3.0]),
        )?;
        history.execute(
            Box::new(ApplyScenePatchCommand::from_property_change(
                change,
                "Set Camera Position",
            )),
            &mut context,
        )?;
        history.undo(&mut context)?;

        let object = context.scene.object(object_id)?;
        assert_eq!(
            object.property(component_id, &path),
            Some(&PropertyValue::Vec3([0.0, 0.0, 0.0]))
        );
        Ok(())
    }

    #[test]
    fn apply_scene_patch_command_apply_changes_value() -> Result<()> {
        let path = PropertyPath::parse("position")?;
        let (mut scene, object_id, component_id) = scene_with_position(path.clone());
        let property_types = PropertyTypeRegistry::default();
        let change = prepare_change(
            &scene,
            object_id,
            component_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        let mut command = ApplyScenePatchCommand::from_property_change(change, "Set Position");
        let mut context = CommandContext {
            scene: &mut scene,
            mutation_sink: None,
            property_types: &property_types,
        };
        command.apply(&mut context)?;
        assert_eq!(
            context
                .scene
                .object(object_id)?
                .property(component_id, &path),
            Some(&PropertyValue::Vec3([4.0, 5.0, 6.0]))
        );
        Ok(())
    }

    #[test]
    fn apply_scene_patch_command_revert_restores_value() -> Result<()> {
        let path = PropertyPath::parse("position")?;
        let (mut scene, object_id, component_id) = scene_with_position(path.clone());
        let change = prepare_change(
            &scene,
            object_id,
            component_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        let mut command = ApplyScenePatchCommand::from_property_change(change, "Set Position");
        let property_types = PropertyTypeRegistry::default();
        let mut context = CommandContext {
            scene: &mut scene,
            mutation_sink: None,
            property_types: &property_types,
        };
        command.apply(&mut context)?;
        command.revert(&mut context)?;
        assert_eq!(
            context
                .scene
                .object(object_id)?
                .property(component_id, &path),
            Some(&PropertyValue::Vec3([0.0, 0.0, 0.0]))
        );
        Ok(())
    }

    #[test]
    fn undo_and_redo_restore_scene_property_values() -> Result<()> {
        let path = PropertyPath::parse("position")?;
        let (mut scene, object_id, component_id) = scene_with_position(path.clone());
        let change = prepare_change(
            &scene,
            object_id,
            component_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        let mut history = CommandHistory::new();
        let property_types = PropertyTypeRegistry::default();
        let mut context = CommandContext {
            scene: &mut scene,
            mutation_sink: None,
            property_types: &property_types,
        };
        history.execute(
            Box::new(ApplyScenePatchCommand::from_property_change(
                change,
                "Set Position",
            )),
            &mut context,
        )?;
        assert_eq!(history.undo_count(), 1);
        UndoCommand::apply(&mut history, &mut context)?;
        assert_eq!(
            context
                .scene
                .object(object_id)?
                .property(component_id, &path),
            Some(&PropertyValue::Vec3([0.0, 0.0, 0.0]))
        );
        RedoCommand::apply(&mut history, &mut context)?;
        assert_eq!(
            context
                .scene
                .object(object_id)?
                .property(component_id, &path),
            Some(&PropertyValue::Vec3([4.0, 5.0, 6.0]))
        );
        Ok(())
    }

    #[test]
    fn failed_edit_does_not_push_undo_entry() -> Result<()> {
        let path = PropertyPath::parse("position")?;
        let (mut scene, object_id, component_id) = scene_with_position(path.clone());
        let mut change = prepare_change(
            &scene,
            object_id,
            component_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        change.path = PropertyPath::parse("missing")?;
        let mut history = CommandHistory::new();
        let property_types = PropertyTypeRegistry::default();
        let mut context = CommandContext {
            scene: &mut scene,
            mutation_sink: None,
            property_types: &property_types,
        };
        assert!(
            history
                .execute(
                    Box::new(ApplyScenePatchCommand::from_property_change(
                        change,
                        "Broken Edit",
                    )),
                    &mut context
                )
                .is_err()
        );
        assert_eq!(history.undo_count(), 0);
        Ok(())
    }

    #[test]
    fn command_label_is_meaningful() -> Result<()> {
        let path = PropertyPath::parse("position")?;
        let (scene, object_id, component_id) = scene_with_position(path.clone());
        let change = prepare_change(
            &scene,
            object_id,
            component_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        let command = ApplyScenePatchCommand::from_property_change(change, "Set Player Position");
        assert_eq!(command.label(), "Set Player Position");
        Ok(())
    }

    #[test]
    fn hierarchy_add_remove_round_trips_through_command() -> Result<()> {
        let mut scene = SceneSnapshot::empty();
        let schema = ObjectSchema::new("Node");
        let type_id = schema.type_id;
        scene.add_schema(schema);
        let root = SceneObject::new("Root", SceneObjectKind::new(kinds::WORLD), type_id);
        let root_id = root.id;
        let property_types = PropertyTypeRegistry::default();
        let _ = scene.add_object(None, 0, root, &property_types);

        let child = SceneObject::new("Child", SceneObjectKind::new(kinds::MESH), type_id);
        let child_id = child.id;
        let forward = scene
            .add_object(Some(root_id), 0, child, &property_types)
            .map_err(|error| ElcaraxError::Command(error.message()))?;
        let inverse = forward
            .invert()
            .map_err(|error| ElcaraxError::Command(error.message()))?;
        inverse
            .apply(&mut scene, &property_types)
            .map_err(|error| ElcaraxError::Command(error.message()))?;

        let mut history = CommandHistory::new();
        let mut context = CommandContext {
            scene: &mut scene,
            mutation_sink: None,
            property_types: &property_types,
        };
        history.execute(
            Box::new(ApplyScenePatchCommand::local(forward, inverse, "Add Child")),
            &mut context,
        )?;
        assert!(context.scene.objects().contains_key(&child_id));
        history.undo(&mut context)?;
        assert!(!context.scene.objects().contains_key(&child_id));
        history.redo(&mut context)?;
        assert!(context.scene.objects().contains_key(&child_id));
        Ok(())
    }

    fn scene_with_position(
        path: PropertyPath,
    ) -> (
        SceneSnapshot,
        elcarax_scene_model::SceneObjectId,
        elcarax_scene_model::ComponentInstanceId,
    ) {
        let schema = ObjectSchema::new("Transform").with_component(
            ComponentSchema::new(components::TRANSFORM, "Transform").with_property(
                PropertySchema::editable(path.clone(), "Position", PropertyKind::Vec3),
            ),
        );
        let component = ComponentInstance::new(components::TRANSFORM, "Transform")
            .with_property(path.clone(), PropertyValue::Vec3([0.0, 0.0, 0.0]));
        let component_id = component.id;
        let object = SceneObject::new(
            "Camera",
            SceneObjectKind::new(kinds::CAMERA),
            schema.type_id,
        )
        .with_component(component);
        let object_id = object.id;
        let mut scene = SceneSnapshot::empty();
        scene.add_schema(schema);
        let property_types = PropertyTypeRegistry::default();
        let _ = scene.add_object(None, 0, object, &property_types);
        (scene, object_id, component_id)
    }

    fn prepare_change(
        scene: &SceneSnapshot,
        object_id: elcarax_scene_model::SceneObjectId,
        component_id: elcarax_scene_model::ComponentInstanceId,
        path: &PropertyPath,
        value: PropertyValue,
    ) -> Result<PropertyChange> {
        prepare_property_change(
            scene,
            object_id,
            component_id,
            path,
            &value,
            &PropertyTypeRegistry::default(),
        )
        .map_err(|error| elcarax_core::ElcaraxError::Command(error.message()))
    }
}
