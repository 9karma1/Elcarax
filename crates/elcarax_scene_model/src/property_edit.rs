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

pub fn parse_property_text(
    path: &PropertyPath,
    edit_kind: PropertyEditKind,
    text: &str,
) -> Result<PropertyValue, PropertyEditError> {
    let text = text.trim();
    match edit_kind {
        PropertyEditKind::Bool => match text.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(PropertyValue::Bool(true)),
            "false" | "0" | "no" => Ok(PropertyValue::Bool(false)),
            _ => Err(type_mismatch(path, edit_kind, text)),
        },
        PropertyEditKind::Integer => text
            .parse::<i64>()
            .map(PropertyValue::I64)
            .map_err(|_| type_mismatch(path, edit_kind, text)),
        PropertyEditKind::Float => text
            .parse::<f64>()
            .map(PropertyValue::F64)
            .map_err(|_| type_mismatch(path, edit_kind, text)),
        PropertyEditKind::String => Ok(PropertyValue::String(text.to_string())),
        PropertyEditKind::Vec2 => {
            let values = parse_vector_components(text, 2)
                .map_err(|_| type_mismatch(path, edit_kind, text))?;
            Ok(PropertyValue::Vec2([values[0], values[1]]))
        }
        PropertyEditKind::Vec3 => {
            let values = parse_vector_components(text, 3)
                .map_err(|_| type_mismatch(path, edit_kind, text))?;
            Ok(PropertyValue::Vec3([values[0], values[1], values[2]]))
        }
        PropertyEditKind::Unsupported => Err(PropertyEditError::ReadOnly {
            path: path.clone(),
            reason: "Property type does not support text entry".to_string(),
        }),
    }
}

fn type_mismatch(
    path: &PropertyPath,
    expected: PropertyEditKind,
    actual: &str,
) -> PropertyEditError {
    PropertyEditError::TypeMismatch {
        path: path.clone(),
        expected,
        actual: actual.to_string(),
    }
}

fn parse_vector_components(text: &str, size: usize) -> Result<Vec<f32>, ()> {
    let parts: Vec<f32> = text
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;
    if parts.len() == size {
        Ok(parts)
    } else {
        Err(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("fixture path should parse: {error}"),
        }
    }

    #[test]
    fn parse_property_text_covers_supported_kinds() {
        let integer =
            parse_property_text(&path("gameplay.health"), PropertyEditKind::Integer, "42");
        assert_eq!(integer.ok(), Some(PropertyValue::I64(42)));
        let float = parse_property_text(&path("gameplay.speed"), PropertyEditKind::Float, "6.5");
        assert_eq!(float.ok(), Some(PropertyValue::F64(6.5)));
        let vec3 = parse_property_text(
            &path("transform.position"),
            PropertyEditKind::Vec3,
            "0, 1, 0",
        );
        assert_eq!(vec3.ok(), Some(PropertyValue::Vec3([0.0, 1.0, 0.0])));
    }
}
