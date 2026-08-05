use std::fmt;

use elcarax_core::{ElcaraxError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::PropertyKind;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropertyPath(Vec<String>);

impl PropertyPath {
    pub fn parse(input: &str) -> Result<Self> {
        let parts: Vec<String> = input
            .split('.')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        if parts.is_empty() {
            return Err(ElcaraxError::invalid_input("property path cannot be empty"));
        }

        Ok(Self(parts))
    }

    pub fn from_static_segments(segments: &[&str]) -> Result<Self> {
        if segments.is_empty() {
            return Err(ElcaraxError::invalid_input("property path cannot be empty"));
        }
        let joined = segments.join(".");
        Self::parse(&joined)
    }

    pub(crate) fn fixture_from_segments(segments: &[&str]) -> Self {
        Self(
            segments
                .iter()
                .map(|segment| (*segment).to_string())
                .collect(),
        )
    }

    pub fn parts(&self) -> &[String] {
        &self.0
    }
}

impl fmt::Display for PropertyPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.join("."))
    }
}

impl Serialize for PropertyPath {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PropertyPath {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

/// Core property values plus an open extension slot for adapter-declared types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    ColorRgba([f32; 4]),
    Enum {
        variant: String,
    },
    AssetRef(String),
    ObjectRef(u64),
    List(Vec<PropertyValue>),
    /// Opaque extension value keyed by a registered type id.
    Extension {
        type_id: String,
        data: serde_json::Value,
    },
}

impl PropertyValue {
    pub fn kind(&self) -> PropertyKind {
        match self {
            Self::Bool(_) => PropertyKind::Bool,
            Self::I64(_) => PropertyKind::I64,
            Self::F64(_) => PropertyKind::F64,
            Self::String(_) => PropertyKind::String,
            Self::Vec2(_) => PropertyKind::Vec2,
            Self::Vec3(_) => PropertyKind::Vec3,
            Self::ColorRgba(_) => PropertyKind::ColorRgba,
            Self::Enum { .. } => PropertyKind::Enum,
            Self::AssetRef(_) => PropertyKind::AssetRef,
            Self::ObjectRef(_) => PropertyKind::ObjectRef,
            Self::List(_) => PropertyKind::List,
            Self::Extension { .. } => PropertyKind::Extension,
        }
    }

    pub fn matches_kind(&self, kind: PropertyKind) -> bool {
        matches!(
            (kind, self),
            (PropertyKind::Bool, Self::Bool(_))
                | (PropertyKind::I64, Self::I64(_))
                | (PropertyKind::F64, Self::F64(_))
                | (PropertyKind::String, Self::String(_))
                | (PropertyKind::Vec2, Self::Vec2(_))
                | (PropertyKind::Vec3, Self::Vec3(_))
                | (PropertyKind::ColorRgba, Self::ColorRgba(_))
                | (PropertyKind::Enum, Self::Enum { .. })
                | (PropertyKind::AssetRef, Self::AssetRef(_))
                | (PropertyKind::ObjectRef, Self::ObjectRef(_))
                | (PropertyKind::List, Self::List(_))
                | (PropertyKind::Extension, Self::Extension { .. })
        )
    }

    pub fn format_display(&self, snapshot: &crate::snapshot::SceneSnapshot) -> String {
        crate::property_display::format_property_value(
            self,
            crate::property_display::PropertyFormatContext { snapshot },
        )
    }

    pub fn display_label(&self) -> String {
        match self {
            Self::Bool(value) => value.to_string(),
            Self::I64(value) => value.to_string(),
            Self::F64(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Vec2(value) => format!("{}, {}", value[0], value[1]),
            Self::Vec3(value) => format!("{}, {}, {}", value[0], value[1], value[2]),
            Self::ColorRgba(value) => format!(
                "rgba({}, {}, {}, {})",
                value[0], value[1], value[2], value[3]
            ),
            Self::Enum { variant } => variant.clone(),
            Self::AssetRef(value) => value.clone(),
            Self::ObjectRef(value) => value.to_string(),
            Self::List(values) => format!("{} item(s)", values.len()),
            Self::Extension { type_id, .. } => format!("<{type_id}>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_display::{PropertyFormatContext, format_property_value};
    use crate::snapshot::SceneSnapshot;

    #[test]
    fn property_path_rejects_empty_input() {
        assert!(PropertyPath::parse("...").is_err());
    }

    #[test]
    fn property_path_formats_with_dots() -> Result<()> {
        let path = PropertyPath::parse("position.x")?;
        assert_eq!(path.to_string(), "position.x");
        Ok(())
    }

    #[test]
    fn property_value_formatting_covers_string_kind() {
        let snapshot = SceneSnapshot::empty();
        let context = PropertyFormatContext {
            snapshot: &snapshot,
        };
        assert_eq!(
            format_property_value(&PropertyValue::String("demo".to_string()), context),
            "demo"
        );
    }

    #[test]
    fn property_value_kind_covers_extension() {
        let value = PropertyValue::Extension {
            type_id: "curve".to_string(),
            data: serde_json::json!({"points": []}),
        };
        assert_eq!(value.kind(), PropertyKind::Extension);
        assert!(value.matches_kind(PropertyKind::Extension));
    }
}
