use std::collections::BTreeMap;

use elcarax_core::{ElcaraxError, Id, IdGenerator, Result};
use serde::{Deserialize, Serialize};

use crate::component::{
    ComponentInstance, ComponentInstanceId, ComponentTypeName, is_display_name_property,
};
use crate::kind::SceneObjectKind;
use crate::name::{SceneName, SceneObjectName};
use crate::schema::ObjectTypeMarker;
use crate::{ObjectSchema, ObjectTypeId, PropertyPath, PropertyValue, SceneError};

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
    pub components: Vec<ComponentInstance>,
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
            components: Vec::new(),
        }
    }

    pub fn object_name(&self) -> SceneObjectName {
        SceneObjectName::from_unvalidated(self.display_name.clone())
    }

    pub fn component(&self, component_id: ComponentInstanceId) -> Option<&ComponentInstance> {
        self.components
            .iter()
            .find(|component| component.id == component_id)
    }

    pub fn component_mut(
        &mut self,
        component_id: ComponentInstanceId,
    ) -> Option<&mut ComponentInstance> {
        self.components
            .iter_mut()
            .find(|component| component.id == component_id)
    }

    pub fn component_by_type(&self, type_name: &ComponentTypeName) -> Option<&ComponentInstance> {
        self.components
            .iter()
            .find(|component| component.type_name == *type_name)
    }

    pub fn component_index(&self, component_id: ComponentInstanceId) -> Option<usize> {
        self.components
            .iter()
            .position(|component| component.id == component_id)
    }

    pub fn add_component(&mut self, component: ComponentInstance) {
        self.components.push(component);
    }

    pub fn insert_component(&mut self, index: usize, component: ComponentInstance) {
        let index = index.min(self.components.len());
        self.components.insert(index, component);
    }

    pub fn with_component(mut self, component: ComponentInstance) -> Self {
        self.add_component(component);
        self
    }

    pub fn remove_component(
        &mut self,
        component_id: ComponentInstanceId,
    ) -> Option<(usize, ComponentInstance)> {
        let index = self.component_index(component_id)?;
        Some((index, self.components.remove(index)))
    }

    pub fn set_property(
        &mut self,
        component_id: ComponentInstanceId,
        path: PropertyPath,
        value: PropertyValue,
    ) -> std::result::Result<Option<PropertyValue>, SceneError> {
        let component = self
            .component_mut(component_id)
            .ok_or(SceneError::ComponentNotFound)?;
        Ok(component.properties.insert(path, value))
    }

    pub fn property(
        &self,
        component_id: ComponentInstanceId,
        path: &PropertyPath,
    ) -> Option<&PropertyValue> {
        self.component(component_id)?.property(path)
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
        Self::from_storage(scene_id, name, Vec::new(), BTreeMap::new(), BTreeMap::new())
    }

    pub(crate) fn from_storage(
        scene_id: SceneId,
        name: SceneName,
        root_objects: Vec<SceneObjectId>,
        objects: BTreeMap<SceneObjectId, SceneObject>,
        schemas: BTreeMap<ObjectTypeId, ObjectSchema>,
    ) -> Self {
        Self {
            scene_id,
            name,
            root_objects,
            objects,
            schemas,
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

    pub fn add_object(
        &mut self,
        parent: Option<SceneObjectId>,
        index: usize,
        mut object: SceneObject,
    ) -> std::result::Result<crate::ScenePatch, crate::ScenePatchError> {
        object.parent = parent;
        object.children.clear();
        let root_id = object.id;
        let mut objects = BTreeMap::new();
        objects.insert(root_id, object);
        let patch = crate::ScenePatch::single(crate::ScenePatchOperation::ObjectAdded(
            crate::ObjectAdded {
                subtree: crate::CapturedSubtree {
                    root_id,
                    parent,
                    index,
                    objects,
                },
            },
        ));
        patch.apply(self)?;
        Ok(patch)
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
        component_id: ComponentInstanceId,
        path: PropertyPath,
        value: PropertyValue,
    ) -> Result<Option<PropertyValue>> {
        let object = self
            .objects
            .get_mut(&object_id)
            .ok_or_else(|| ElcaraxError::not_found(format!("scene object {}", object_id.get())))?;
        object
            .set_property(component_id, path, value)
            .map_err(|_| ElcaraxError::not_found(format!("scene component {}", component_id.get())))
    }

    pub fn property(
        &self,
        object_id: SceneObjectId,
        component_id: ComponentInstanceId,
        path: &PropertyPath,
    ) -> Option<&PropertyValue> {
        self.objects.get(&object_id)?.property(component_id, path)
    }

    pub(crate) fn objects_mut(&mut self) -> &mut BTreeMap<SceneObjectId, SceneObject> {
        &mut self.objects
    }

    pub(crate) fn insert_object_raw(&mut self, object: SceneObject) {
        self.objects.insert(object.id, object);
    }

    pub(crate) fn remove_object_raw(&mut self, object_id: SceneObjectId) {
        self.objects.remove(&object_id);
    }

    pub(crate) fn insert_child_link(
        &mut self,
        parent: Option<SceneObjectId>,
        child_id: SceneObjectId,
        index: usize,
    ) -> std::result::Result<(), crate::ScenePatchError> {
        match parent {
            Some(parent_id) => {
                let parent = self.objects.get_mut(&parent_id).ok_or(
                    crate::ScenePatchError::ObjectNotFound {
                        object_id: parent_id,
                    },
                )?;
                let index = index.min(parent.children.len());
                parent.children.insert(index, child_id);
            }
            None => {
                let index = index.min(self.root_objects.len());
                self.root_objects.insert(index, child_id);
            }
        }
        Ok(())
    }

    pub(crate) fn remove_child_link(
        &mut self,
        parent: Option<SceneObjectId>,
        child_id: SceneObjectId,
    ) -> std::result::Result<(), crate::ScenePatchError> {
        match parent {
            Some(parent_id) => {
                let parent = self.objects.get_mut(&parent_id).ok_or(
                    crate::ScenePatchError::ObjectNotFound {
                        object_id: parent_id,
                    },
                )?;
                let Some(position) = parent.children.iter().position(|id| *id == child_id) else {
                    return Err(crate::ScenePatchError::InvalidHierarchy {
                        reason: format!(
                            "child {} is not linked under parent {}",
                            child_id.get(),
                            parent_id.get()
                        ),
                    });
                };
                parent.children.remove(position);
            }
            None => {
                let Some(position) = self.root_objects.iter().position(|id| *id == child_id) else {
                    return Err(crate::ScenePatchError::InvalidHierarchy {
                        reason: format!("object {} is not a root object", child_id.get()),
                    });
                };
                self.root_objects.remove(position);
            }
        }
        Ok(())
    }

    pub fn remove_object(
        &mut self,
        object_id: SceneObjectId,
    ) -> std::result::Result<crate::ScenePatch, crate::ScenePatchError> {
        let subtree = crate::CapturedSubtree::from_object(self, object_id)?;
        let patch = crate::ScenePatch::single(crate::ScenePatchOperation::ObjectRemoved(
            crate::ObjectRemoved { subtree },
        ));
        patch.apply(self)?;
        Ok(patch)
    }

    pub fn reparent_object(
        &mut self,
        object_id: SceneObjectId,
        new_parent: Option<SceneObjectId>,
        new_index: usize,
    ) -> std::result::Result<crate::ScenePatch, crate::ScenePatchError> {
        let object = self
            .objects
            .get(&object_id)
            .ok_or(crate::ScenePatchError::ObjectNotFound { object_id })?;
        let old_parent = object.parent;
        let old_index = match old_parent {
            Some(parent_id) => self
                .objects
                .get(&parent_id)
                .ok_or(crate::ScenePatchError::ObjectNotFound {
                    object_id: parent_id,
                })?
                .children
                .iter()
                .position(|id| *id == object_id)
                .ok_or_else(|| crate::ScenePatchError::InvalidHierarchy {
                    reason: format!(
                        "object {} is not listed under parent {}",
                        object_id.get(),
                        parent_id.get()
                    ),
                })?,
            None => self
                .root_objects
                .iter()
                .position(|id| *id == object_id)
                .ok_or_else(|| crate::ScenePatchError::InvalidHierarchy {
                    reason: format!("object {} is not a root object", object_id.get()),
                })?,
        };
        let patch =
            crate::ScenePatch::single(crate::ScenePatchOperation::Reparented(crate::Reparented {
                object_id,
                old_parent,
                old_index,
                new_parent,
                new_index,
            }));
        patch.apply(self)?;
        Ok(patch)
    }

    pub fn rename_object(
        &mut self,
        object_id: SceneObjectId,
        new_name: impl Into<String>,
    ) -> std::result::Result<crate::ScenePatch, crate::ScenePatchError> {
        let new_name = new_name.into();
        let object = self
            .objects
            .get(&object_id)
            .ok_or(crate::ScenePatchError::ObjectNotFound { object_id })?;
        let old_name = object.display_name.clone();
        let patch =
            crate::ScenePatch::single(crate::ScenePatchOperation::Renamed(crate::Renamed {
                object_id,
                old_name,
                new_name,
            }));
        patch.apply(self)?;
        Ok(patch)
    }

    pub fn add_component(
        &mut self,
        object_id: SceneObjectId,
        index: usize,
        component: ComponentInstance,
    ) -> std::result::Result<crate::ScenePatch, crate::ScenePatchError> {
        let patch = crate::ScenePatch::single(crate::ScenePatchOperation::ComponentAdded(
            crate::ComponentAdded {
                object_id,
                index,
                component,
            },
        ));
        patch.apply(self)?;
        Ok(patch)
    }

    pub fn remove_component(
        &mut self,
        object_id: SceneObjectId,
        component_id: ComponentInstanceId,
    ) -> std::result::Result<crate::ScenePatch, crate::ScenePatchError> {
        let object = self
            .objects
            .get(&object_id)
            .ok_or(crate::ScenePatchError::ObjectNotFound { object_id })?;
        let index = object.component_index(component_id).ok_or(
            crate::ScenePatchError::ComponentNotFound {
                object_id,
                component_id,
            },
        )?;
        let component = match object.components.get(index) {
            Some(component) => component.clone(),
            None => {
                return Err(crate::ScenePatchError::ComponentNotFound {
                    object_id,
                    component_id,
                });
            }
        };
        let patch = crate::ScenePatch::single(crate::ScenePatchOperation::ComponentRemoved(
            crate::ComponentRemoved {
                object_id,
                index,
                component,
            },
        ));
        patch.apply(self)?;
        Ok(patch)
    }

    pub(crate) fn replace_existing_property(
        &mut self,
        object_id: SceneObjectId,
        component_id: ComponentInstanceId,
        path: &PropertyPath,
        value: PropertyValue,
    ) -> Result<()> {
        let object = self
            .objects
            .get_mut(&object_id)
            .ok_or_else(|| ElcaraxError::not_found(format!("scene object {}", object_id.get())))?;
        let component = object.component_mut(component_id).ok_or_else(|| {
            ElcaraxError::not_found(format!("scene component {}", component_id.get()))
        })?;
        if !component.properties.contains_key(path) {
            return Err(ElcaraxError::not_found(format!("property {path}")));
        }
        let syncs_display_name = is_display_name_property(&component.type_name, path);
        component.properties.insert(path.clone(), value.clone());
        if syncs_display_name && let PropertyValue::String(name) = value {
            object.display_name = name;
        }
        Ok(())
    }

    /// Writes a renamed object's display name back into its General name property, when present.
    pub(crate) fn sync_display_name_property(&mut self, object_id: SceneObjectId, name: &str) {
        let Some(object) = self.objects.get_mut(&object_id) else {
            return;
        };
        let path = crate::component::display_name_property_path();
        for component in &mut object.components {
            if is_display_name_property(&component.type_name, &path)
                && component.properties.contains_key(&path)
            {
                component
                    .properties
                    .insert(path.clone(), PropertyValue::String(name.to_string()));
            }
        }
    }
}
