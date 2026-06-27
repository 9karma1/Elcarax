use elcarax_core::{ElcaraxError, Result};
use elcarax_scene_model::{PropertyPath, PropertyValue, SceneObjectId};

use crate::{CommandContext, CommandEffect, EditorCommand};

pub struct PropertyChangeCommand {
    label: String,
    object_id: SceneObjectId,
    path: PropertyPath,
    next_value: PropertyValue,
    previous_value: Option<PropertyValue>,
}

impl PropertyChangeCommand {
    pub fn new(object_id: SceneObjectId, path: PropertyPath, next_value: PropertyValue) -> Self {
        Self {
            label: "Set property".to_owned(),
            object_id,
            path,
            next_value,
            previous_value: None,
        }
    }
}

impl EditorCommand for PropertyChangeCommand {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect> {
        let previous_value = context.scene.set_property(
            self.object_id,
            self.path.clone(),
            self.next_value.clone(),
        )?;
        self.previous_value = previous_value;
        Ok(CommandEffect::SceneChanged)
    }

    fn revert(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect> {
        let Some(previous_value) = self.previous_value.clone() else {
            return Err(ElcaraxError::Command(format!(
                "cannot revert '{}' because no previous value was recorded",
                self.label
            )));
        };
        context
            .scene
            .set_property(self.object_id, self.path.clone(), previous_value)?;
        Ok(CommandEffect::SceneChanged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandContext, CommandHistory};
    use elcarax_core::Result;
    use elcarax_scene_model::{
        ObjectSchema, PropertyKind, PropertySchema, SceneObject, SceneObjectKind, SceneSnapshot,
    };

    #[test]
    fn property_change_can_be_undone() -> Result<()> {
        let path = PropertyPath::parse("transform.position")?;
        let schema = ObjectSchema::new("Transform").with_property(PropertySchema::editable(
            path.clone(),
            "Position",
            PropertyKind::Vec3,
        ));
        let mut object = SceneObject::new("Camera", SceneObjectKind::Camera, schema.type_id);
        object.set_property(path.clone(), PropertyValue::Vec3([0.0, 0.0, 0.0]));
        let object_id = object.id;

        let mut scene = SceneSnapshot::empty();
        scene.add_schema(schema);
        scene.add_root_object(object);

        let mut context = CommandContext { scene: &mut scene };
        let mut history = CommandHistory::new();
        history.execute(
            Box::new(PropertyChangeCommand::new(
                object_id,
                path.clone(),
                PropertyValue::Vec3([1.0, 2.0, 3.0]),
            )),
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
}
