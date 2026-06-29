use std::fmt;

use crate::{
    PropertyEditKind, PropertyKind, PropertyPath, PropertySchema, PropertyValue, SceneId,
    SceneObjectId, SceneSnapshot,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyChange {
    pub scene_id: SceneId,
    pub object_id: SceneObjectId,
    pub path: PropertyPath,
    pub old_value: PropertyValue,
    pub new_value: PropertyValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyEditError {
    ObjectNotFound {
        object_id: SceneObjectId,
    },
    PropertyNotFound {
        path: PropertyPath,
    },
    ReadOnly {
        path: PropertyPath,
        reason: String,
    },
    TypeMismatch {
        path: PropertyPath,
        expected: PropertyEditKind,
        actual: String,
    },
}

impl PropertyEditError {
    pub fn message(&self) -> String {
        match self {
            Self::ObjectNotFound { object_id } => {
                format!("Object {} was not found", object_id.get())
            }
            Self::PropertyNotFound { path } => format!("Property '{path}' was not found"),
            Self::ReadOnly { path, reason } => {
                format!("Property '{path}' is read-only: {reason}")
            }
            Self::TypeMismatch {
                path,
                expected,
                actual,
            } => format!(
                "Property '{path}' expects {} but received {actual}",
                expected.label()
            ),
        }
    }
}

impl fmt::Display for PropertyEditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message())
    }
}

impl std::error::Error for PropertyEditError {}

pub type PropertyEditResult = Result<PropertyChange, PropertyEditError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyChangeValue {
    Old,
    New,
}

pub fn prepare_property_change(
    snapshot: &SceneSnapshot,
    object_id: SceneObjectId,
    path: &PropertyPath,
    new_value: &PropertyValue,
) -> PropertyEditResult {
    let object = snapshot
        .objects()
        .get(&object_id)
        .ok_or(PropertyEditError::ObjectNotFound { object_id })?;
    let schema = editable_schema_for(snapshot, object.type_id, path)?;
    validate_value_type(path, schema.kind, schema.edit_kind, new_value)?;
    let old_value = object
        .property(path)
        .cloned()
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() })?;
    Ok(PropertyChange {
        scene_id: snapshot.scene_id(),
        object_id,
        path: path.clone(),
        old_value,
        new_value: new_value.clone(),
    })
}

pub fn edit_scene_property(
    snapshot: &mut SceneSnapshot,
    object_id: SceneObjectId,
    path: &PropertyPath,
    new_value: PropertyValue,
) -> PropertyEditResult {
    let change = prepare_property_change(snapshot, object_id, path, &new_value)?;
    apply_property_change(snapshot, &change, PropertyChangeValue::New)?;
    Ok(change)
}

pub fn apply_property_change(
    snapshot: &mut SceneSnapshot,
    change: &PropertyChange,
    value: PropertyChangeValue,
) -> Result<(), PropertyEditError> {
    let selected_value = match value {
        PropertyChangeValue::Old => &change.old_value,
        PropertyChangeValue::New => &change.new_value,
    };
    prepare_property_change(snapshot, change.object_id, &change.path, selected_value)?;
    snapshot
        .replace_existing_property(change.object_id, &change.path, selected_value.clone())
        .map_err(|_| PropertyEditError::ObjectNotFound {
            object_id: change.object_id,
        })?;
    Ok(())
}

fn editable_schema_for<'a>(
    snapshot: &'a SceneSnapshot,
    type_id: crate::ObjectTypeId,
    path: &PropertyPath,
) -> Result<&'a PropertySchema, PropertyEditError> {
    let schema = snapshot
        .schema(type_id)
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() })?;
    let property = schema
        .properties
        .iter()
        .find(|property| property.path == *path)
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() })?;
    if !property.editable {
        return Err(PropertyEditError::ReadOnly {
            path: path.clone(),
            reason: property
                .read_only_reason
                .clone()
                .unwrap_or_else(|| "Property is not editable".to_string()),
        });
    }
    Ok(property)
}

fn validate_value_type(
    path: &PropertyPath,
    kind: PropertyKind,
    edit_kind: PropertyEditKind,
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
    );
    if matches {
        return Ok(());
    }
    Err(PropertyEditError::TypeMismatch {
        path: path.clone(),
        expected: edit_kind,
        actual: value_kind_label(value).to_string(),
    })
}

fn value_kind_label(value: &PropertyValue) -> &'static str {
    match value {
        PropertyValue::Bool(_) => "bool",
        PropertyValue::I64(_) => "integer",
        PropertyValue::F64(_) => "float",
        PropertyValue::String(_) => "string",
        PropertyValue::Vec2(_) => "vec2",
        PropertyValue::Vec3(_) => "vec3",
        PropertyValue::ColorRgba(_) => "color",
        PropertyValue::Enum { .. } => "enum",
        PropertyValue::AssetRef(_) => "asset ref",
        PropertyValue::ObjectRef(_) => "object ref",
        PropertyValue::Unknown => "unknown",
        PropertyValue::List(_) => "list",
    }
}
