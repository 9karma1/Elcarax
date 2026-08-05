use std::collections::BTreeMap;
use std::fmt;

use elcarax_core::{Id, IdGenerator};
use serde::{Deserialize, Serialize};

use crate::{PropertyPath, PropertyValue};

pub enum ComponentInstanceMarker {}
pub type ComponentInstanceId = Id<ComponentInstanceMarker>;

/// Open component type name. Schemas and adapters register component types by string id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComponentTypeName(String);

impl ComponentTypeName {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ComponentTypeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<&str> for ComponentTypeName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ComponentTypeName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Well-known component type names used by the reference scene.
pub mod well_known {
    pub const GENERAL: &str = "general";
    pub const TRANSFORM: &str = "transform";
    pub const LIGHTING: &str = "lighting";
    pub const CAMERA: &str = "camera";
    pub const GAMEPLAY: &str = "gameplay";
    pub const REFERENCES: &str = "references";
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentInstance {
    pub id: ComponentInstanceId,
    pub type_name: ComponentTypeName,
    pub display_name: String,
    pub properties: BTreeMap<PropertyPath, PropertyValue>,
}

impl ComponentInstance {
    pub fn new(type_name: impl Into<ComponentTypeName>, display_name: impl Into<String>) -> Self {
        static IDS: IdGenerator<ComponentInstanceMarker> = IdGenerator::new();
        Self::with_id(IDS.next_id(), type_name, display_name)
    }

    pub fn with_stable_id(
        id: ComponentInstanceId,
        type_name: impl Into<ComponentTypeName>,
        display_name: impl Into<String>,
    ) -> Self {
        Self::with_id(id, type_name, display_name)
    }

    fn with_id(
        id: ComponentInstanceId,
        type_name: impl Into<ComponentTypeName>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            type_name: type_name.into(),
            display_name: display_name.into(),
            properties: BTreeMap::new(),
        }
    }

    pub fn set_property(&mut self, path: PropertyPath, value: PropertyValue) {
        self.properties.insert(path, value);
    }

    pub fn property(&self, path: &PropertyPath) -> Option<&PropertyValue> {
        self.properties.get(path)
    }

    pub fn with_property(mut self, path: PropertyPath, value: PropertyValue) -> Self {
        self.set_property(path, value);
        self
    }
}

/// Path to the display-name property within a General component.
pub fn display_name_property_path() -> PropertyPath {
    PropertyPath::fixture_from_segments(&["name"])
}

pub fn is_display_name_property(type_name: &ComponentTypeName, path: &PropertyPath) -> bool {
    type_name.as_str() == well_known::GENERAL && path.parts() == ["name"]
}
