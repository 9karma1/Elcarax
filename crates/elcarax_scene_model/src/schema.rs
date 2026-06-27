use elcarax_core::{Id, IdGenerator};

use crate::PropertyPath;
use crate::property_display::PropertyGroup;

pub enum ObjectTypeMarker {}
pub type ObjectTypeId = Id<ObjectTypeMarker>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    Bool,
    I64,
    F64,
    String,
    Vec2,
    Vec3,
    ColorRgba,
    Enum,
    AssetRef,
    ObjectRef,
    List,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertySchema {
    pub path: PropertyPath,
    pub display_name: String,
    pub kind: PropertyKind,
    pub group: PropertyGroup,
    pub read_only: bool,
}

impl PropertySchema {
    pub fn read_only(
        path: PropertyPath,
        display_name: impl Into<String>,
        kind: PropertyKind,
        group: PropertyGroup,
    ) -> Self {
        Self {
            path,
            display_name: display_name.into(),
            kind,
            group,
            read_only: true,
        }
    }

    pub fn editable(
        path: PropertyPath,
        display_name: impl Into<String>,
        kind: PropertyKind,
        group: PropertyGroup,
    ) -> Self {
        Self {
            path,
            display_name: display_name.into(),
            kind,
            group,
            read_only: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectSchema {
    pub type_id: ObjectTypeId,
    pub display_name: String,
    pub properties: Vec<PropertySchema>,
}

impl ObjectSchema {
    pub fn new(display_name: impl Into<String>) -> Self {
        static IDS: IdGenerator<ObjectTypeMarker> = IdGenerator::new();
        Self {
            type_id: IDS.next_id(),
            display_name: display_name.into(),
            properties: Vec::new(),
        }
    }

    pub fn with_property(mut self, property: PropertySchema) -> Self {
        self.properties.push(property);
        self
    }
}
