use std::collections::BTreeMap;

use elcarax_core::{ElcaraxError, Id, IdGenerator, Result};
use serde::{Deserialize, Serialize};

use crate::kind::SceneObjectKind;
use crate::name::{SceneName, SceneObjectName};
use crate::schema::ObjectTypeMarker;
use crate::{ObjectSchema, ObjectTypeId, PropertyPath, PropertyValue};

pub enum SceneMarker {}
pub enum SceneObjectMarker {}

pub type SceneId = Id<SceneMarker>;
pub type SceneObjectId = Id<SceneObjectMarker>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneObject {
    pub id: SceneObjectId,
    pub parent: Option<SceneObjectId>,
    pub children: Vec<SceneObjectId>,
    pub display_name: String,
    pub kind: SceneObjectKind,
    pub type_id: ObjectTypeId,
    pub property_summary: Option<String>,
    pub properties: BTreeMap<PropertyPath, PropertyValue>,
}

impl SceneObject {
    pub fn new(
        display_name: impl Into<String>,
        kind: SceneObjectKind,
        type_id: ObjectTypeId,
    ) -> Self {
        static IDS: IdGenerator<SceneObjectMarker> = IdGenerator::new();
        Self::with_id(IDS.next_id(), display_name, kind, type_id)
    }

    pub fn with_stable_id(
        id: SceneObjectId,
        display_name: impl Into<String>,
        kind: SceneObjectKind,
    ) -> Self {
        static TYPE_IDS: IdGenerator<ObjectTypeMarker> = IdGenerator::new();
        Self::with_id(id, display_name, kind, TYPE_IDS.next_id())
    }

    fn with_id(
        id: SceneObjectId,
        display_name: impl Into<String>,
        kind: SceneObjectKind,
        type_id: ObjectTypeId,
    ) -> Self {
        Self {
            id,
            parent: None,
            children: Vec::new(),
            display_name: display_name.into(),
            kind,
            type_id,
            property_summary: None,
            properties: BTreeMap::new(),
        }
    }

    pub fn object_name(&self) -> SceneObjectName {
        SceneObjectName::from_unvalidated(self.display_name.clone())
    }

    pub fn set_property(&mut self, path: PropertyPath, value: PropertyValue) {
        self.properties.insert(path, value);
    }

    pub fn property(&self, path: &PropertyPath) -> Option<&PropertyValue> {
        self.properties.get(path)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneSnapshot {
    scene_id: SceneId,
    name: SceneName,
    root_objects: Vec<SceneObjectId>,
    objects: BTreeMap<SceneObjectId, SceneObject>,
    schemas: BTreeMap<ObjectTypeId, ObjectSchema>,
}

impl SceneSnapshot {
    pub fn empty() -> Self {
        static IDS: IdGenerator<SceneMarker> = IdGenerator::new();
        Self::with_id_and_name(IDS.next_id(), SceneName::from_unvalidated("Untitled Scene"))
    }

    pub fn with_name(name: SceneName) -> Self {
        static IDS: IdGenerator<SceneMarker> = IdGenerator::new();
        Self::with_id_and_name(IDS.next_id(), name)
    }

    fn with_id_and_name(scene_id: SceneId, name: SceneName) -> Self {
        Self {
            scene_id,
            name,
            root_objects: Vec::new(),
            objects: BTreeMap::new(),
            schemas: BTreeMap::new(),
        }
    }

    pub(crate) fn set_scene_id(&mut self, scene_id: SceneId) {
        self.scene_id = scene_id;
    }

    pub fn scene_id(&self) -> SceneId {
        self.scene_id
    }

    pub fn name(&self) -> &SceneName {
        &self.name
    }

    pub fn root_object_ids(&self) -> &[SceneObjectId] {
        self.root_objects.as_slice()
    }

    pub fn objects(&self) -> &BTreeMap<SceneObjectId, SceneObject> {
        &self.objects
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn object_schemas(&self) -> &BTreeMap<ObjectTypeId, ObjectSchema> {
        &self.schemas
    }

    pub fn schema(&self, type_id: ObjectTypeId) -> Option<&ObjectSchema> {
        self.schemas.get(&type_id)
    }

    pub fn add_schema(&mut self, schema: ObjectSchema) {
        self.schemas.insert(schema.type_id, schema);
    }

    pub fn add_root_object(&mut self, object: SceneObject) {
        self.root_objects.push(object.id);
        self.objects.insert(object.id, object);
    }

    pub fn attach_child(&mut self, parent_id: SceneObjectId, mut child: SceneObject) -> Result<()> {
        let parent = self
            .objects
            .get_mut(&parent_id)
            .ok_or_else(|| ElcaraxError::not_found(format!("scene object {}", parent_id.get())))?;
        child.parent = Some(parent_id);
        let child_id = child.id;
        parent.children.push(child_id);
        self.objects.insert(child_id, child);
        Ok(())
    }

    pub fn object(&self, id: SceneObjectId) -> Result<&SceneObject> {
        self.objects
            .get(&id)
            .ok_or_else(|| ElcaraxError::not_found(format!("scene object {}", id.get())))
    }

    pub fn object_by_name(&self, name: &str) -> Option<&SceneObject> {
        self.objects
            .values()
            .find(|object| object.display_name == name)
    }

    pub fn root_object_id(&self) -> Option<SceneObjectId> {
        self.root_objects.first().copied()
    }

    pub fn expandable_object_ids(&self) -> Vec<SceneObjectId> {
        let mut ids: Vec<_> = self
            .objects
            .values()
            .filter(|object| !object.children.is_empty())
            .map(|object| object.id)
            .collect();
        ids.sort_by_key(|id| id.get());
        ids
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

    pub(crate) fn replace_existing_property(
        &mut self,
        object_id: SceneObjectId,
        path: &PropertyPath,
        value: PropertyValue,
    ) -> Result<()> {
        let object = self
            .objects
            .get_mut(&object_id)
            .ok_or_else(|| ElcaraxError::not_found(format!("scene object {}", object_id.get())))?;
        if !object.properties.contains_key(path) {
            return Err(ElcaraxError::not_found(format!("property {path}")));
        }
        if is_display_name_path(path)
            && let PropertyValue::String(name) = &value
        {
            object.display_name = name.clone();
        }
        object.properties.insert(path.clone(), value);
        Ok(())
    }
}

fn is_display_name_path(path: &PropertyPath) -> bool {
    path.parts() == ["general".to_string(), "name".to_string()]
}
