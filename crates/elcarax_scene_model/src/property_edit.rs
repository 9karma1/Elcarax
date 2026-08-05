use std::fmt;

use crate::component::ComponentInstanceId;
use crate::{
    PropertyEditKind, PropertyKind, PropertyPath, PropertySchema, PropertyValue, SceneId,
    SceneObjectId, SceneSnapshot,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyChange {
    pub scene_id: SceneId,
    pub object_id: SceneObjectId,
    pub component_id: ComponentInstanceId,
    pub path: PropertyPath,
    pub old_value: PropertyValue,
    pub new_value: PropertyValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyEditError {
    ObjectNotFound {
        object_id: SceneObjectId,
    },
    ComponentNotFound {
        component_id: ComponentInstanceId,
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
            Self::ComponentNotFound { component_id } => {
                format!("Component {} was not found", component_id.get())
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
    component_id: ComponentInstanceId,
    path: &PropertyPath,
    new_value: &PropertyValue,
) -> PropertyEditResult {
    let object = snapshot
        .objects()
        .get(&object_id)
        .ok_or(PropertyEditError::ObjectNotFound { object_id })?;
    let component = object
        .component(component_id)
        .ok_or(PropertyEditError::ComponentNotFound { component_id })?;
    let schema = editable_schema_for(snapshot, object.type_id, &component.type_name, path)?;
    validate_value_type(path, schema.kind, schema.edit_kind, new_value)?;
    if schema.kind == PropertyKind::Enum {
        let PropertyValue::Enum { variant } = new_value else {
            return Err(PropertyEditError::TypeMismatch {
                path: path.clone(),
                expected: schema.edit_kind,
                actual: new_value.display_label(),
            });
        };
        if !schema.enum_variants.iter().any(|value| value == variant) {
            return Err(PropertyEditError::TypeMismatch {
                path: path.clone(),
                expected: schema.edit_kind,
                actual: format!("unknown enum variant '{variant}'"),
            });
        }
    }
    let old_value = component
        .property(path)
        .cloned()
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() })?;
    Ok(PropertyChange {
        scene_id: snapshot.scene_id(),
        object_id,
        component_id,
        path: path.clone(),
        old_value,
        new_value: new_value.clone(),
    })
}

pub fn edit_scene_property(
    snapshot: &mut SceneSnapshot,
    object_id: SceneObjectId,
    component_id: ComponentInstanceId,
    path: &PropertyPath,
    new_value: PropertyValue,
) -> PropertyEditResult {
    let change = prepare_property_change(snapshot, object_id, component_id, path, &new_value)?;
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
        PropertyEditKind::Enum => Ok(PropertyValue::Enum {
            variant: text.to_string(),
        }),
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
    crate::ScenePatch::property_updated(
        change.object_id,
        change.component_id,
        change.path.clone(),
        selected_value.clone(),
    )
    .apply(snapshot)
    .map_err(|error| match error {
        crate::ScenePatchError::Property(error) => error,
        crate::ScenePatchError::ObjectNotFound { object_id } => {
            PropertyEditError::ObjectNotFound { object_id }
        }
        crate::ScenePatchError::ComponentNotFound { component_id, .. } => {
            PropertyEditError::ComponentNotFound { component_id }
        }
        other => PropertyEditError::ReadOnly {
            path: change.path.clone(),
            reason: other.message(),
        },
    })
}

pub fn property_change_patches(change: &PropertyChange) -> (crate::ScenePatch, crate::ScenePatch) {
    (
        crate::ScenePatch::property_updated(
            change.object_id,
            change.component_id,
            change.path.clone(),
            change.new_value.clone(),
        ),
        crate::ScenePatch::property_updated(
            change.object_id,
            change.component_id,
            change.path.clone(),
            change.old_value.clone(),
        ),
    )
}

fn editable_schema_for<'a>(
    snapshot: &'a SceneSnapshot,
    type_id: crate::ObjectTypeId,
    type_name: &crate::component::ComponentTypeName,
    path: &PropertyPath,
) -> Result<&'a PropertySchema, PropertyEditError> {
    let schema = snapshot
        .schema(type_id)
        .ok_or_else(|| PropertyEditError::PropertyNotFound { path: path.clone() })?;
    let property = schema
        .property(type_name, path)
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
    if value.matches_kind(kind) {
        return Ok(());
    }
    Err(PropertyEditError::TypeMismatch {
        path: path.clone(),
        expected: edit_kind,
        actual: value.display_label(),
    })
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
