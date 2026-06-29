use elcarax_core::{ElcaraxError, Result};
use elcarax_scene_model::{PropertyChange, PropertyChangeValue, apply_property_change};

use crate::{CommandContext, CommandEffect, CommandHistory, EditorCommand};

pub struct SetScenePropertyCommand {
    label: String,
    change: PropertyChange,
}

impl SetScenePropertyCommand {
    pub fn new(change: PropertyChange, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            change,
        }
    }

    pub fn change(&self) -> &PropertyChange {
        &self.change
    }
}

impl EditorCommand for SetScenePropertyCommand {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect> {
        apply_property_change(context.scene, &self.change, PropertyChangeValue::New)
            .map_err(|error| ElcaraxError::Command(error.message()))?;
        Ok(CommandEffect::SceneChanged)
    }

    fn revert(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect> {
        apply_property_change(context.scene, &self.change, PropertyChangeValue::Old)
            .map_err(|error| ElcaraxError::Command(error.message()))?;
        Ok(CommandEffect::SceneChanged)
    }
}

pub struct SetScenePropertiesCommand {
    label: String,
    changes: Vec<PropertyChange>,
}

impl SetScenePropertiesCommand {
    pub fn new(changes: Vec<PropertyChange>, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            changes,
        }
    }
}

impl EditorCommand for SetScenePropertiesCommand {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect> {
        for change in &self.changes {
            apply_property_change(context.scene, change, PropertyChangeValue::New)
                .map_err(|error| ElcaraxError::Command(error.message()))?;
        }
        Ok(CommandEffect::SceneChanged)
    }

    fn revert(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect> {
        for change in self.changes.iter().rev() {
            apply_property_change(context.scene, change, PropertyChangeValue::Old)
                .map_err(|error| ElcaraxError::Command(error.message()))?;
        }
        Ok(CommandEffect::SceneChanged)
    }
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
        ObjectSchema, PropertyChange, PropertyGroup, PropertyKind, PropertyPath, PropertySchema,
        PropertyValue, SceneObject, SceneObjectKind, SceneSnapshot, prepare_property_change,
    };

    #[test]
    fn property_change_can_be_undone() -> Result<()> {
        let path = PropertyPath::parse("transform.position")?;
        let schema = ObjectSchema::new("Transform").with_property(PropertySchema::editable(
            path.clone(),
            "Position",
            PropertyKind::Vec3,
            PropertyGroup::new("Transform"),
        ));
        let mut object = SceneObject::new("Camera", SceneObjectKind::Camera, schema.type_id);
        object.set_property(path.clone(), PropertyValue::Vec3([0.0, 0.0, 0.0]));
        let object_id = object.id;

        let mut scene = SceneSnapshot::empty();
        scene.add_schema(schema);
        scene.add_root_object(object);

        let mut context = CommandContext { scene: &mut scene };
        let mut history = CommandHistory::new();
        let change = prepare_change(
            context.scene,
            object_id,
            &path,
            PropertyValue::Vec3([1.0, 2.0, 3.0]),
        )?;
        history.execute(
            Box::new(SetScenePropertyCommand::new(change, "Set Camera Position")),
            &mut context,
        )?;
        history.undo(&mut context)?;

        let object = context.scene.object(object_id)?;
        assert_eq!(
            object.property(&path),
            Some(&PropertyValue::Vec3([0.0, 0.0, 0.0]))
        );
        Ok(())
    }

    #[test]
    fn set_scene_property_command_apply_changes_value() -> Result<()> {
        let path = PropertyPath::parse("transform.position")?;
        let (mut scene, object_id) = scene_with_position(path.clone());
        let change = prepare_change(
            &scene,
            object_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        let mut command = SetScenePropertyCommand::new(change, "Set Position");
        let mut context = CommandContext { scene: &mut scene };
        command.apply(&mut context)?;
        assert_eq!(
            context.scene.object(object_id)?.property(&path),
            Some(&PropertyValue::Vec3([4.0, 5.0, 6.0]))
        );
        Ok(())
    }

    #[test]
    fn set_scene_property_command_revert_restores_value() -> Result<()> {
        let path = PropertyPath::parse("transform.position")?;
        let (mut scene, object_id) = scene_with_position(path.clone());
        let change = prepare_change(
            &scene,
            object_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        let mut command = SetScenePropertyCommand::new(change, "Set Position");
        let mut context = CommandContext { scene: &mut scene };
        command.apply(&mut context)?;
        command.revert(&mut context)?;
        assert_eq!(
            context.scene.object(object_id)?.property(&path),
            Some(&PropertyValue::Vec3([0.0, 0.0, 0.0]))
        );
        Ok(())
    }

    #[test]
    fn undo_and_redo_restore_scene_property_values() -> Result<()> {
        let path = PropertyPath::parse("transform.position")?;
        let (mut scene, object_id) = scene_with_position(path.clone());
        let change = prepare_change(
            &scene,
            object_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        let mut history = CommandHistory::new();
        let mut context = CommandContext { scene: &mut scene };
        history.execute(
            Box::new(SetScenePropertyCommand::new(change, "Set Position")),
            &mut context,
        )?;
        assert_eq!(history.undo_count(), 1);
        UndoCommand::apply(&mut history, &mut context)?;
        assert_eq!(
            context.scene.object(object_id)?.property(&path),
            Some(&PropertyValue::Vec3([0.0, 0.0, 0.0]))
        );
        RedoCommand::apply(&mut history, &mut context)?;
        assert_eq!(
            context.scene.object(object_id)?.property(&path),
            Some(&PropertyValue::Vec3([4.0, 5.0, 6.0]))
        );
        Ok(())
    }

    #[test]
    fn failed_edit_does_not_push_undo_entry() -> Result<()> {
        let path = PropertyPath::parse("transform.position")?;
        let (mut scene, object_id) = scene_with_position(path.clone());
        let mut change = prepare_change(
            &scene,
            object_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        change.path = PropertyPath::parse("transform.missing")?;
        let mut history = CommandHistory::new();
        let mut context = CommandContext { scene: &mut scene };
        assert!(
            history
                .execute(
                    Box::new(SetScenePropertyCommand::new(change, "Broken Edit")),
                    &mut context
                )
                .is_err()
        );
        assert_eq!(history.undo_count(), 0);
        Ok(())
    }

    #[test]
    fn command_label_is_meaningful() -> Result<()> {
        let path = PropertyPath::parse("transform.position")?;
        let (scene, object_id) = scene_with_position(path.clone());
        let change = prepare_change(
            &scene,
            object_id,
            &path,
            PropertyValue::Vec3([4.0, 5.0, 6.0]),
        )?;
        let command = SetScenePropertyCommand::new(change, "Set Player Position");
        assert_eq!(command.label(), "Set Player Position");
        Ok(())
    }

    fn scene_with_position(
        path: PropertyPath,
    ) -> (SceneSnapshot, elcarax_scene_model::SceneObjectId) {
        let schema = ObjectSchema::new("Transform").with_property(PropertySchema::editable(
            path.clone(),
            "Position",
            PropertyKind::Vec3,
            PropertyGroup::new("Transform"),
        ));
        let mut object = SceneObject::new("Camera", SceneObjectKind::Camera, schema.type_id);
        object.set_property(path, PropertyValue::Vec3([0.0, 0.0, 0.0]));
        let object_id = object.id;
        let mut scene = SceneSnapshot::empty();
        scene.add_schema(schema);
        scene.add_root_object(object);
        (scene, object_id)
    }

    fn prepare_change(
        scene: &SceneSnapshot,
        object_id: elcarax_scene_model::SceneObjectId,
        path: &PropertyPath,
        value: PropertyValue,
    ) -> Result<PropertyChange> {
        prepare_property_change(scene, object_id, path, &value)
            .map_err(|error| elcarax_core::ElcaraxError::Command(error.message()))
    }
}
