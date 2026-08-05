use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::component::{ComponentInstance, ComponentInstanceId};
use crate::{
    PropertyEditError, PropertyKind, PropertyPath, PropertyValue, SceneObject, SceneObjectId,
    SceneSnapshot,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenePatch {
    pub operations: Vec<ScenePatchOperation>,
}

impl ScenePatch {
    pub fn empty() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    pub fn single(operation: ScenePatchOperation) -> Self {
        Self {
            operations: vec![operation],
        }
    }

    pub fn property_updated(
        object_id: SceneObjectId,
        component_id: ComponentInstanceId,
        path: PropertyPath,
        value: PropertyValue,
    ) -> Self {
        Self::single(ScenePatchOperation::PropertyUpdated(PropertyUpdated {
            object_id,
            component_id,
            path,
            value,
        }))
    }

    pub fn invert(&self) -> Result<Self, ScenePatchError> {
        let mut operations = Vec::with_capacity(self.operations.len());
        for operation in self.operations.iter().rev() {
            operations.push(operation.invert()?);
        }
        Ok(Self { operations })
    }

    pub fn apply(&self, snapshot: &mut SceneSnapshot) -> Result<(), ScenePatchError> {
        for operation in &self.operations {
            operation.apply(snapshot)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScenePatchOperation {
    PropertyUpdated(PropertyUpdated),
    ComponentAdded(ComponentAdded),
    ComponentRemoved(ComponentRemoved),
    ObjectAdded(ObjectAdded),
    ObjectRemoved(ObjectRemoved),
    Reparented(Reparented),
    Renamed(Renamed),
}

impl ScenePatchOperation {
    fn invert(&self) -> Result<Self, ScenePatchError> {
        match self {
            Self::PropertyUpdated(_) => Err(ScenePatchError::NotInvertible {
                reason: "PropertyUpdated requires a paired inverse patch with the prior value"
                    .to_string(),
            }),
            Self::ComponentAdded(added) => Ok(Self::ComponentRemoved(ComponentRemoved {
                object_id: added.object_id,
                index: added.index,
                component: added.component.clone(),
            })),
            Self::ComponentRemoved(removed) => Ok(Self::ComponentAdded(ComponentAdded {
                object_id: removed.object_id,
                index: removed.index,
                component: removed.component.clone(),
            })),
            Self::ObjectAdded(added) => Ok(Self::ObjectRemoved(ObjectRemoved {
                subtree: added.subtree.clone(),
            })),
            Self::ObjectRemoved(removed) => Ok(Self::ObjectAdded(ObjectAdded {
                subtree: removed.subtree.clone(),
            })),
            Self::Reparented(reparented) => Ok(Self::Reparented(Reparented {
                object_id: reparented.object_id,
                old_parent: reparented.new_parent,
                old_index: reparented.new_index,
                new_parent: reparented.old_parent,
                new_index: reparented.old_index,
            })),
            Self::Renamed(renamed) => Ok(Self::Renamed(Renamed {
                object_id: renamed.object_id,
                old_name: renamed.new_name.clone(),
                new_name: renamed.old_name.clone(),
            })),
        }
    }

    fn apply(&self, snapshot: &mut SceneSnapshot) -> Result<(), ScenePatchError> {
        match self {
            Self::PropertyUpdated(update) => apply_property_update(snapshot, update),
            Self::ComponentAdded(added) => apply_component_added(snapshot, added),
            Self::ComponentRemoved(removed) => apply_component_removed(snapshot, removed),
            Self::ObjectAdded(added) => apply_object_added(snapshot, added),
            Self::ObjectRemoved(removed) => apply_object_removed(snapshot, removed),
            Self::Reparented(reparented) => apply_reparented(snapshot, reparented),
            Self::Renamed(renamed) => apply_renamed(snapshot, renamed),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyUpdated {
    pub object_id: SceneObjectId,
    pub component_id: ComponentInstanceId,
    pub path: PropertyPath,
    pub value: PropertyValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentAdded {
    pub object_id: SceneObjectId,
    pub index: usize,
    pub component: ComponentInstance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentRemoved {
    pub object_id: SceneObjectId,
    pub index: usize,
    pub component: ComponentInstance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturedSubtree {
    pub root_id: SceneObjectId,
    pub parent: Option<SceneObjectId>,
    pub index: usize,
    pub objects: BTreeMap<SceneObjectId, SceneObject>,
}

impl CapturedSubtree {
    pub fn from_object(
        snapshot: &SceneSnapshot,
        object_id: SceneObjectId,
    ) -> Result<Self, ScenePatchError> {
        let object = snapshot
            .objects()
            .get(&object_id)
            .ok_or(ScenePatchError::ObjectNotFound { object_id })?;
        let parent = object.parent;
        let index = sibling_index(snapshot, object_id, parent)?;
        let mut objects = BTreeMap::new();
        collect_subtree(snapshot, object_id, &mut objects)?;
        Ok(Self {
            root_id: object_id,
            parent,
            index,
            objects,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectAdded {
    pub subtree: CapturedSubtree,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectRemoved {
    pub subtree: CapturedSubtree,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reparented {
    pub object_id: SceneObjectId,
    pub old_parent: Option<SceneObjectId>,
    pub old_index: usize,
    pub new_parent: Option<SceneObjectId>,
    pub new_index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Renamed {
    pub object_id: SceneObjectId,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenePatchError {
    Property(PropertyEditError),
    ObjectNotFound {
        object_id: SceneObjectId,
    },
    ObjectAlreadyExists {
        object_id: SceneObjectId,
    },
    ComponentNotFound {
        object_id: SceneObjectId,
        component_id: ComponentInstanceId,
    },
    ComponentAlreadyExists {
        object_id: SceneObjectId,
        component_id: ComponentInstanceId,
    },
    InvalidHierarchy {
        reason: String,
    },
    CycleDetected {
        object_id: SceneObjectId,
    },
    NotInvertible {
        reason: String,
    },
}

impl ScenePatchError {
    pub fn message(&self) -> String {
        match self {
            Self::Property(error) => error.message(),
            Self::ObjectNotFound { object_id } => {
                format!("Object {} was not found", object_id.get())
            }
            Self::ObjectAlreadyExists { object_id } => {
                format!("Object {} already exists in the scene", object_id.get())
            }
            Self::ComponentNotFound {
                object_id,
                component_id,
            } => format!(
                "Component {} was not found on object {}",
                component_id.get(),
                object_id.get()
            ),
            Self::ComponentAlreadyExists {
                object_id,
                component_id,
            } => format!(
                "Component {} already exists on object {}",
                component_id.get(),
                object_id.get()
            ),
            Self::InvalidHierarchy { reason } => format!("Invalid hierarchy: {reason}"),
            Self::CycleDetected { object_id } => {
                format!(
                    "Reparenting object {} would create a cycle",
                    object_id.get()
                )
            }
            Self::NotInvertible { reason } => reason.clone(),
        }
    }
}

impl fmt::Display for ScenePatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message())
    }
}

impl std::error::Error for ScenePatchError {}

impl From<PropertyEditError> for ScenePatchError {
    fn from(error: PropertyEditError) -> Self {
        Self::Property(error)
    }
}

fn apply_property_update(
    snapshot: &mut SceneSnapshot,
    update: &PropertyUpdated,
) -> Result<(), ScenePatchError> {
    let object =
        snapshot
            .objects()
            .get(&update.object_id)
            .ok_or(ScenePatchError::ObjectNotFound {
                object_id: update.object_id,
            })?;
    let component =
        object
            .component(update.component_id)
            .ok_or(ScenePatchError::ComponentNotFound {
                object_id: update.object_id,
                component_id: update.component_id,
            })?;
    let kind = property_kind(snapshot, object.type_id, &component.type_name, &update.path)?;
    validate_patch_value(&update.path, kind, &update.value)?;
    if component.property(&update.path).is_none() {
        return Err(PropertyEditError::PropertyNotFound {
            path: update.path.clone(),
        }
        .into());
    }
    snapshot
        .replace_existing_property(
            update.object_id,
            update.component_id,
            &update.path,
            update.value.clone(),
        )
        .map_err(|_| ScenePatchError::ObjectNotFound {
            object_id: update.object_id,
        })
}

fn apply_component_added(
    snapshot: &mut SceneSnapshot,
    added: &ComponentAdded,
) -> Result<(), ScenePatchError> {
    let object = snapshot.objects_mut().get_mut(&added.object_id).ok_or(
        ScenePatchError::ObjectNotFound {
            object_id: added.object_id,
        },
    )?;
    if object.component(added.component.id).is_some() {
        return Err(ScenePatchError::ComponentAlreadyExists {
            object_id: added.object_id,
            component_id: added.component.id,
        });
    }
    object.insert_component(added.index, added.component.clone());
    Ok(())
}

fn apply_component_removed(
    snapshot: &mut SceneSnapshot,
    removed: &ComponentRemoved,
) -> Result<(), ScenePatchError> {
    let object = snapshot.objects_mut().get_mut(&removed.object_id).ok_or(
        ScenePatchError::ObjectNotFound {
            object_id: removed.object_id,
        },
    )?;
    object
        .remove_component(removed.component.id)
        .map(|_| ())
        .ok_or(ScenePatchError::ComponentNotFound {
            object_id: removed.object_id,
            component_id: removed.component.id,
        })
}

fn apply_object_added(
    snapshot: &mut SceneSnapshot,
    added: &ObjectAdded,
) -> Result<(), ScenePatchError> {
    let subtree = &added.subtree;
    if subtree.objects.is_empty() {
        return Err(ScenePatchError::InvalidHierarchy {
            reason: "ObjectAdded subtree is empty".to_string(),
        });
    }
    if !subtree.objects.contains_key(&subtree.root_id) {
        return Err(ScenePatchError::InvalidHierarchy {
            reason: "ObjectAdded subtree is missing its root object".to_string(),
        });
    }
    for object_id in subtree.objects.keys() {
        if snapshot.objects().contains_key(object_id) {
            return Err(ScenePatchError::ObjectAlreadyExists {
                object_id: *object_id,
            });
        }
    }
    if let Some(parent_id) = subtree.parent {
        if !snapshot.objects().contains_key(&parent_id) {
            return Err(ScenePatchError::ObjectNotFound {
                object_id: parent_id,
            });
        }
        if subtree.objects.contains_key(&parent_id) {
            return Err(ScenePatchError::InvalidHierarchy {
                reason: "ObjectAdded parent cannot be inside the added subtree".to_string(),
            });
        }
    }

    for (object_id, object) in &subtree.objects {
        let mut inserted = object.clone();
        if *object_id == subtree.root_id {
            inserted.parent = subtree.parent;
        }
        snapshot.insert_object_raw(inserted);
    }
    snapshot.insert_child_link(subtree.parent, subtree.root_id, subtree.index)?;
    Ok(())
}

fn apply_object_removed(
    snapshot: &mut SceneSnapshot,
    removed: &ObjectRemoved,
) -> Result<(), ScenePatchError> {
    let subtree = &removed.subtree;
    if !snapshot.objects().contains_key(&subtree.root_id) {
        return Err(ScenePatchError::ObjectNotFound {
            object_id: subtree.root_id,
        });
    }
    for object_id in subtree.objects.keys() {
        if !snapshot.objects().contains_key(object_id) {
            return Err(ScenePatchError::ObjectNotFound {
                object_id: *object_id,
            });
        }
    }
    snapshot.remove_child_link(subtree.parent, subtree.root_id)?;
    for object_id in subtree.objects.keys() {
        snapshot.remove_object_raw(*object_id);
    }
    Ok(())
}

fn apply_reparented(
    snapshot: &mut SceneSnapshot,
    reparented: &Reparented,
) -> Result<(), ScenePatchError> {
    if !snapshot.objects().contains_key(&reparented.object_id) {
        return Err(ScenePatchError::ObjectNotFound {
            object_id: reparented.object_id,
        });
    }
    if let Some(new_parent) = reparented.new_parent {
        if !snapshot.objects().contains_key(&new_parent) {
            return Err(ScenePatchError::ObjectNotFound {
                object_id: new_parent,
            });
        }
        if would_create_cycle(snapshot, reparented.object_id, new_parent) {
            return Err(ScenePatchError::CycleDetected {
                object_id: reparented.object_id,
            });
        }
    }
    let current_parent = snapshot
        .objects()
        .get(&reparented.object_id)
        .map(|object| object.parent)
        .ok_or(ScenePatchError::ObjectNotFound {
            object_id: reparented.object_id,
        })?;
    snapshot.remove_child_link(current_parent, reparented.object_id)?;
    if let Some(object) = snapshot.objects_mut().get_mut(&reparented.object_id) {
        object.parent = reparented.new_parent;
    }
    snapshot.insert_child_link(
        reparented.new_parent,
        reparented.object_id,
        reparented.new_index,
    )?;
    Ok(())
}

fn apply_renamed(snapshot: &mut SceneSnapshot, renamed: &Renamed) -> Result<(), ScenePatchError> {
    if renamed.new_name.trim().is_empty() {
        return Err(ScenePatchError::InvalidHierarchy {
            reason: "object name cannot be empty".to_string(),
        });
    }
    let object = snapshot.objects_mut().get_mut(&renamed.object_id).ok_or(
        ScenePatchError::ObjectNotFound {
            object_id: renamed.object_id,
        },
    )?;
    object.display_name = renamed.new_name.clone();
    snapshot.sync_display_name_property(renamed.object_id, &renamed.new_name);
    Ok(())
}

fn collect_subtree(
    snapshot: &SceneSnapshot,
    object_id: SceneObjectId,
    objects: &mut BTreeMap<SceneObjectId, SceneObject>,
) -> Result<(), ScenePatchError> {
    let object = snapshot
        .objects()
        .get(&object_id)
        .ok_or(ScenePatchError::ObjectNotFound { object_id })?
        .clone();
    let children = object.children.clone();
    objects.insert(object_id, object);
    for child_id in children {
        collect_subtree(snapshot, child_id, objects)?;
    }
    Ok(())
}

fn sibling_index(
    snapshot: &SceneSnapshot,
    object_id: SceneObjectId,
    parent: Option<SceneObjectId>,
) -> Result<usize, ScenePatchError> {
    let siblings = match parent {
        Some(parent_id) => {
            let parent =
                snapshot
                    .objects()
                    .get(&parent_id)
                    .ok_or(ScenePatchError::ObjectNotFound {
                        object_id: parent_id,
                    })?;
            parent.children.as_slice()
        }
        None => snapshot.root_object_ids(),
    };
    siblings
        .iter()
        .position(|id| *id == object_id)
        .ok_or_else(|| ScenePatchError::InvalidHierarchy {
            reason: format!(
                "object {} is not listed under its parent link",
                object_id.get()
            ),
        })
}

fn would_create_cycle(
    snapshot: &SceneSnapshot,
    object_id: SceneObjectId,
    new_parent: SceneObjectId,
) -> bool {
    let mut cursor = Some(new_parent);
    while let Some(current) = cursor {
        if current == object_id {
            return true;
        }
        cursor = snapshot
            .objects()
            .get(&current)
            .and_then(|object| object.parent);
    }
    false
}

fn property_kind(
    snapshot: &SceneSnapshot,
    type_id: crate::ObjectTypeId,
    type_name: &crate::component::ComponentTypeName,
    path: &PropertyPath,
) -> Result<PropertyKind, ScenePatchError> {
    let schema = snapshot
        .schema(type_id)
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() })?;
    schema
        .property(type_name, path)
        .map(|property| property.kind)
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() }.into())
}

fn validate_patch_value(
    path: &PropertyPath,
    kind: PropertyKind,
    value: &PropertyValue,
) -> Result<(), ScenePatchError> {
    if value.matches_kind(kind) {
        return Ok(());
    }
    Err(PropertyEditError::TypeMismatch {
        path: path.clone(),
        expected: crate::PropertyEditKind::for_property_kind(kind),
        actual: value.display_label(),
    }
    .into())
}
