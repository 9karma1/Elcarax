use elcarax_core::Id;
use serde::{Deserialize, Serialize};

use crate::property::{PropertyPath, PropertyValue};
use crate::snapshot::{SceneObjectId, SceneSnapshot};

pub enum PropertyMarker {}
pub type PropertyId = Id<PropertyMarker>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PropertyGroup(String);

impl PropertyGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyDisplay {
    pub path: PropertyPath,
    pub name: crate::name::PropertyName,
    pub group: PropertyGroup,
    pub formatted_value: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyFormatContext<'a> {
    pub snapshot: &'a SceneSnapshot,
}

pub fn format_property_value(value: &PropertyValue, context: PropertyFormatContext<'_>) -> String {
    match value {
        PropertyValue::Bool(value) => value.to_string(),
        PropertyValue::I64(value) => value.to_string(),
        PropertyValue::F64(value) => format!("{value:.2}"),
        PropertyValue::String(value) => value.clone(),
        PropertyValue::Vec2(value) => format!("{:.2}, {:.2}", value[0], value[1]),
        PropertyValue::Vec3(value) => format!("{:.2}, {:.2}, {:.2}", value[0], value[1], value[2]),
        PropertyValue::ColorRgba(value) => {
            format!(
                "rgba({:.2}, {:.2}, {:.2}, {:.2})",
                value[0], value[1], value[2], value[3]
            )
        }
        PropertyValue::Enum { variant } => variant.clone(),
        PropertyValue::AssetRef(value) => asset_display_name(value),
        PropertyValue::ObjectRef(value) => object_display_name(context.snapshot, *value),
        PropertyValue::List(values) => format!("{} item(s)", values.len()),
        PropertyValue::Unknown => "<unsupported>".to_string(),
    }
}

fn asset_display_name(path: &str) -> String {
    match path.rsplit(['/', '\\']).next() {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => path.to_string(),
    }
}

fn object_display_name(snapshot: &SceneSnapshot, object_ref: u64) -> String {
    use std::num::NonZeroU64;
    let Some(raw_id) = NonZeroU64::new(object_ref) else {
        return object_ref.to_string();
    };
    let object_id = SceneObjectId::from_non_zero(raw_id);
    match snapshot.object(object_id) {
        Ok(object) => object.display_name.clone(),
        Err(_) => object_ref.to_string(),
    }
}
