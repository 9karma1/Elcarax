use std::collections::BTreeMap;

use elcarax_core::{ElcaraxError, Id, IdGenerator, Result};

use crate::{ObjectSchema, ObjectTypeId, PropertyPath, PropertyValue};

pub enum SceneMarker {}
pub enum SceneObjectMarker {}

pub type SceneId = Id<SceneMarker>;
pub type SceneObjectId = Id<SceneObjectMarker>;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneObject {
    pub id: SceneObjectId,
    pub parent: Option<SceneObjectId>,
    pub children: Vec<SceneObjectId>,
    pub display_name: String,
    pub type_id: ObjectTypeId,
    pub properties: BTreeMap<PropertyPath, PropertyValue>,
}

impl SceneObject {
    pub fn new(display_name: impl Into<String>, type_id: ObjectTypeId) -> Self {
        static IDS: IdGenerator<SceneObjectMarker> = IdGenerator::new();
        Self {
            id: IDS.next_id(),
            parent: None,
            children: Vec::new(),
            display_name: display_name.into(),
            type_id,
            properties: BTreeMap::new(),
        }
    }

    pub fn set_property(&mut self, path: PropertyPath, value: PropertyValue) {
        self.properties.insert(path, value);
    }

    pub fn property(&self, path: &PropertyPath) -> Option<&PropertyValue> {
        self.properties.get(path)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneSnapshot {
    pub scene_id: SceneId,
    pub root_objects: Vec<SceneObjectId>,
    pub objects: BTreeMap<SceneObjectId, SceneObject>,
    pub schemas: BTreeMap<ObjectTypeId, ObjectSchema>,
}

impl SceneSnapshot {
    pub fn empty() -> Self {
        static IDS: IdGenerator<SceneMarker> = IdGenerator::new();
        Self {
            scene_id: IDS.next_id(),
            root_objects: Vec::new(),
            objects: BTreeMap::new(),
            schemas: BTreeMap::new(),
        }
    }

    pub fn add_schema(&mut self, schema: ObjectSchema) {
        self.schemas.insert(schema.type_id, schema);
    }

    pub fn add_root_object(&mut self, object: SceneObject) {
        self.root_objects.push(object.id);
        self.objects.insert(object.id, object);
    }

    pub fn object(&self, id: SceneObjectId) -> Result<&SceneObject> {
        self.objects
            .get(&id)
            .ok_or_else(|| ElcaraxError::not_found(format!("scene object {}", id.get())))
    }

    pub fn set_property(
        &mut self,
        object_id: SceneObjectId,
        path: PropertyPath,
        value: PropertyValue,
    ) -> Result<Option<PropertyValue>> {
        let object = self
            .objects
            .get_mut(&object_id)
            .ok_or_else(|| ElcaraxError::not_found(format!("scene object {}", object_id.get())))?;
        Ok(object.properties.insert(path, value))
    }
}
