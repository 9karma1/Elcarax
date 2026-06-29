use serde::{Deserialize, Serialize};

use crate::{
    PropertyEditError, PropertyKind, PropertyPath, PropertyValue, SceneObjectId, SceneSnapshot,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenePatch {
    pub operations: Vec<ScenePatchOperation>,
}

impl ScenePatch {
    pub fn property_updated(
        object_id: SceneObjectId,
        path: PropertyPath,
        value: PropertyValue,
    ) -> Self {
        Self {
            operations: vec![ScenePatchOperation::PropertyUpdated(PropertyUpdated {
                object_id,
                path,
                value,
            })],
        }
    }

    pub fn apply(&self, snapshot: &mut SceneSnapshot) -> Result<(), PropertyEditError> {
        for operation in &self.operations {
            match operation {
                ScenePatchOperation::PropertyUpdated(update) => {
                    apply_property_update(snapshot, update)?
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScenePatchOperation {
    PropertyUpdated(PropertyUpdated),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyUpdated {
    pub object_id: SceneObjectId,
    pub path: PropertyPath,
    pub value: PropertyValue,
}

fn apply_property_update(
    snapshot: &mut SceneSnapshot,
    update: &PropertyUpdated,
) -> Result<(), PropertyEditError> {
    let object =
        snapshot
            .objects()
            .get(&update.object_id)
            .ok_or(PropertyEditError::ObjectNotFound {
                object_id: update.object_id,
            })?;
    let kind = property_kind(snapshot, object.type_id, &update.path)?;
    validate_patch_value(&update.path, kind, &update.value)?;
    if object.property(&update.path).is_none() {
        return Err(PropertyEditError::PropertyNotFound {
            path: update.path.clone(),
        });
    }
    snapshot
        .replace_existing_property(update.object_id, &update.path, update.value.clone())
        .map_err(|_| PropertyEditError::ObjectNotFound {
            object_id: update.object_id,
        })
}

fn property_kind(
    snapshot: &SceneSnapshot,
    type_id: crate::ObjectTypeId,
    path: &PropertyPath,
) -> Result<PropertyKind, PropertyEditError> {
    let schema = snapshot
        .schema(type_id)
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() })?;
    schema
        .properties
        .iter()
        .find(|property| property.path == *path)
        .map(|property| property.kind)
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() })
}

fn validate_patch_value(
    path: &PropertyPath,
    kind: PropertyKind,
    value: &PropertyValue,
) -> Result<(), PropertyEditError> {
    let matches = matches!(
        (kind, value),
        (PropertyKind::Bool, PropertyValue::Bool(_))
            | (PropertyKind::I64, PropertyValue::I64(_))
            | (PropertyKind::F64, PropertyValue::F64(_))
            | (PropertyKind::String, PropertyValue::String(_))
            | (PropertyKind::Vec2, PropertyValue::Vec2(_))
            | (PropertyKind::Vec3, PropertyValue::Vec3(_))
            | (PropertyKind::ColorRgba, PropertyValue::ColorRgba(_))
            | (PropertyKind::Enum, PropertyValue::Enum { .. })
            | (PropertyKind::AssetRef, PropertyValue::AssetRef(_))
            | (PropertyKind::ObjectRef, PropertyValue::ObjectRef(_))
            | (PropertyKind::List, PropertyValue::List(_))
    );
    if matches {
        return Ok(());
    }
    Err(PropertyEditError::TypeMismatch {
        path: path.clone(),
        expected: crate::PropertyEditKind::for_property_kind(kind),
        actual: value.display_label(),
    })
}
