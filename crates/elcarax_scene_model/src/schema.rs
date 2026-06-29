use elcarax_core::{Id, IdGenerator};
use serde::{Deserialize, Serialize};

use crate::PropertyPath;
use crate::property_display::PropertyGroup;

pub enum ObjectTypeMarker {}
pub type ObjectTypeId = Id<ObjectTypeMarker>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyEditKind {
    Bool,
    Integer,
    Float,
    String,
    Vec2,
    Vec3,
    Unsupported,
}

impl PropertyEditKind {
    pub const fn for_property_kind(kind: PropertyKind) -> Self {
        match kind {
            PropertyKind::Bool => Self::Bool,
            PropertyKind::I64 => Self::Integer,
            PropertyKind::F64 => Self::Float,
            PropertyKind::String => Self::String,
            PropertyKind::Vec2 => Self::Vec2,
            PropertyKind::Vec3 => Self::Vec3,
            PropertyKind::ColorRgba
            | PropertyKind::Enum
            | PropertyKind::AssetRef
            | PropertyKind::ObjectRef
            | PropertyKind::List => Self::Unsupported,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::String => "string",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Unsupported => "unsupported",
        }
    }

    pub const fn is_supported(self) -> bool {
        !matches!(self, Self::Unsupported)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NumericEditMetadata {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
}

impl NumericEditMetadata {
    pub const fn step(step: f64) -> Self {
        Self {
            min: None,
            max: None,
            step: Some(step),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertySchema {
    pub path: PropertyPath,
    pub display_name: String,
    pub kind: PropertyKind,
    pub group: PropertyGroup,
    pub editable: bool,
    pub edit_kind: PropertyEditKind,
    pub numeric: Option<NumericEditMetadata>,
    pub enum_variants: Vec<String>,
    pub read_only_reason: Option<String>,
}

impl PropertySchema {
    pub fn read_only(
        path: PropertyPath,
        display_name: impl Into<String>,
        kind: PropertyKind,
        group: PropertyGroup,
    ) -> Self {
        let edit_kind = PropertyEditKind::for_property_kind(kind);
        Self {
            path,
            display_name: display_name.into(),
            kind,
            group,
            editable: false,
            edit_kind,
            numeric: None,
            enum_variants: Vec::new(),
            read_only_reason: Some(read_only_reason(kind)),
        }
    }

    pub fn editable(
        path: PropertyPath,
        display_name: impl Into<String>,
        kind: PropertyKind,
        group: PropertyGroup,
    ) -> Self {
        let edit_kind = PropertyEditKind::for_property_kind(kind);
        Self {
            path,
            display_name: display_name.into(),
            kind,
            group,
            editable: edit_kind.is_supported(),
            edit_kind,
            numeric: numeric_metadata(kind),
            enum_variants: Vec::new(),
            read_only_reason: if edit_kind.is_supported() {
                None
            } else {
                Some(read_only_reason(kind))
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

fn numeric_metadata(kind: PropertyKind) -> Option<NumericEditMetadata> {
    match kind {
        PropertyKind::I64 => Some(NumericEditMetadata::step(1.0)),
        PropertyKind::F64 => Some(NumericEditMetadata::step(0.5)),
        _ => None,
    }
}

fn read_only_reason(kind: PropertyKind) -> String {
    match kind {
        PropertyKind::ColorRgba => "Color editing is not enabled in this milestone".to_string(),
        PropertyKind::AssetRef => {
            "Asset assignment editing is not enabled in this milestone".to_string()
        }
        PropertyKind::ObjectRef => {
            "Object reference editing is not enabled in this milestone".to_string()
        }
        PropertyKind::Enum => "Enum editing is not enabled in this milestone".to_string(),
        PropertyKind::List => "List editing is not enabled in this milestone".to_string(),
        PropertyKind::Bool
        | PropertyKind::I64
        | PropertyKind::F64
        | PropertyKind::String
        | PropertyKind::Vec2
        | PropertyKind::Vec3 => "Property is read-only in this scene snapshot".to_string(),
    }
}
