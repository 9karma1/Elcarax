//! Schema-driven inspector value widget descriptors.

use crate::{NumericEditMetadata, PropertyEditKind, PropertyKind, PropertySchema, PropertyValue};

pub const MAX_ENUM_VARIANTS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnumVariantList {
    pub variants: [String; MAX_ENUM_VARIANTS],
    pub len: u8,
}

impl EnumVariantList {
    pub fn from_slice(values: &[String]) -> Self {
        let mut list = Self::default();
        for (index, value) in values.iter().take(MAX_ENUM_VARIANTS).enumerate() {
            list.variants[index] = value.clone();
        }
        list.len = values.len().min(MAX_ENUM_VARIANTS) as u8;
        list
    }

    pub fn as_slice(&self) -> &[String] {
        &self.variants[..self.len as usize]
    }

    pub fn contains(&self, variant: &str) -> bool {
        self.as_slice().iter().any(|value| value == variant)
    }

    pub fn next_variant(&self, current: &str) -> Option<String> {
        let slice = self.as_slice();
        if slice.is_empty() {
            return None;
        }
        let index = slice.iter().position(|value| value == current).unwrap_or(0);
        let next = (index + 1) % slice.len();
        Some(slice[next].clone())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum InspectorValueWidget {
    #[default]
    Hidden,
    ReadOnly(String),
    Text(String),
    Toggle {
        checked: bool,
    },
    Number {
        display: String,
        step: f64,
        is_integer: bool,
    },
    Vector {
        components: [String; 3],
        count: u8,
        step: f64,
    },
    Enum {
        selected: String,
        variants: EnumVariantList,
    },
}

pub fn inspector_value_widget_for(
    schema: &PropertySchema,
    value: &PropertyValue,
) -> InspectorValueWidget {
    if !schema.editable {
        return InspectorValueWidget::ReadOnly(value.display_label());
    }
    match schema.edit_kind {
        PropertyEditKind::Bool => InspectorValueWidget::Toggle {
            checked: matches!(value, PropertyValue::Bool(true)),
        },
        PropertyEditKind::Integer => InspectorValueWidget::Number {
            display: integer_display(value),
            step: numeric_step(schema.numeric, true),
            is_integer: true,
        },
        PropertyEditKind::Float => InspectorValueWidget::Number {
            display: float_display(value),
            step: numeric_step(schema.numeric, false),
            is_integer: false,
        },
        PropertyEditKind::String => InspectorValueWidget::Text(string_display(value)),
        PropertyEditKind::Vec2 => InspectorValueWidget::Vector {
            components: vector_components(value, 2),
            count: 2,
            step: numeric_step(schema.numeric, false),
        },
        PropertyEditKind::Vec3 => InspectorValueWidget::Vector {
            components: vector_components(value, 3),
            count: 3,
            step: numeric_step(schema.numeric, false),
        },
        PropertyEditKind::Enum => InspectorValueWidget::Enum {
            selected: enum_display(value),
            variants: EnumVariantList::from_slice(&schema.enum_variants),
        },
        PropertyEditKind::Unsupported => InspectorValueWidget::ReadOnly(value.display_label()),
    }
}

pub fn inspector_value_widget_for_row(
    editable: bool,
    edit_kind: PropertyEditKind,
    value_text: &str,
    numeric: Option<NumericEditMetadata>,
    enum_variants: &[String],
    value: &PropertyValue,
) -> InspectorValueWidget {
    if !editable {
        return InspectorValueWidget::ReadOnly(value_text.to_string());
    }
    let schema = PropertySchema {
        path: crate::PropertyPath::fixture_from_segments(&["fixture"]),
        display_name: String::new(),
        kind: property_kind_for_edit(edit_kind),
        editable: true,
        edit_kind,
        numeric,
        enum_variants: enum_variants.to_vec(),
        read_only_reason: None,
        extension_type_id: None,
    };
    inspector_value_widget_for(&schema, value)
}

fn property_kind_for_edit(edit_kind: PropertyEditKind) -> PropertyKind {
    match edit_kind {
        PropertyEditKind::Bool => PropertyKind::Bool,
        PropertyEditKind::Integer => PropertyKind::I64,
        PropertyEditKind::Float => PropertyKind::F64,
        PropertyEditKind::String => PropertyKind::String,
        PropertyEditKind::Vec2 => PropertyKind::Vec2,
        PropertyEditKind::Vec3 => PropertyKind::Vec3,
        PropertyEditKind::Enum => PropertyKind::Enum,
        PropertyEditKind::Unsupported => PropertyKind::String,
    }
}

fn numeric_step(metadata: Option<NumericEditMetadata>, is_integer: bool) -> f64 {
    metadata
        .and_then(|value| value.step)
        .unwrap_or(if is_integer { 1.0 } else { 0.5 })
}

fn integer_display(value: &PropertyValue) -> String {
    match value {
        PropertyValue::I64(value) => value.to_string(),
        _ => "0".to_string(),
    }
}

fn float_display(value: &PropertyValue) -> String {
    match value {
        PropertyValue::F64(value) => format!("{value:.2}"),
        _ => "0.00".to_string(),
    }
}

fn string_display(value: &PropertyValue) -> String {
    match value {
        PropertyValue::String(value) => value.clone(),
        _ => String::new(),
    }
}

fn enum_display(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Enum { variant } => variant.clone(),
        _ => String::new(),
    }
}

fn vector_components(value: &PropertyValue, count: u8) -> [String; 3] {
    let mut components = ["0.00".to_string(), "0.00".to_string(), "0.00".to_string()];
    match (value, count) {
        (PropertyValue::Vec2(values), 2) => {
            components[0] = format!("{:.2}", values[0]);
            components[1] = format!("{:.2}", values[1]);
        }
        (PropertyValue::Vec3(values), 3) => {
            components[0] = format!("{:.2}", values[0]);
            components[1] = format!("{:.2}", values[1]);
            components[2] = format!("{:.2}", values[2]);
        }
        _ => {}
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PropertyKind, PropertyPath, PropertySchema};

    #[test]
    fn bool_schema_maps_to_toggle_widget() {
        let schema = PropertySchema::editable(path("enabled"), "Enabled", PropertyKind::Bool);
        let widget = inspector_value_widget_for(&schema, &PropertyValue::Bool(true));
        assert_eq!(widget, InspectorValueWidget::Toggle { checked: true });
    }

    #[test]
    fn enum_schema_maps_to_enum_widget() {
        let schema = PropertySchema::editable_enum(path("stance"), "Stance", &["Idle", "Run"]);
        let widget = inspector_value_widget_for(
            &schema,
            &PropertyValue::Enum {
                variant: "Idle".to_string(),
            },
        );
        assert!(matches!(widget, InspectorValueWidget::Enum { .. }));
    }

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("fixture path should parse: {error}"),
        }
    }
}
