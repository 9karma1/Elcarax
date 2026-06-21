use std::fmt;

use elcarax_core::{ElcaraxError, Result};

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

    pub fn parts(&self) -> &[String] {
        &self.0
    }
}

impl fmt::Display for PropertyPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.join("."))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    ColorRgba([f32; 4]),
    Enum { variant: String },
    AssetRef(String),
    ObjectRef(u64),
    List(Vec<PropertyValue>),
}

impl PropertyValue {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_path_rejects_empty_input() {
        assert!(PropertyPath::parse("...").is_err());
    }

    #[test]
    fn property_path_formats_with_dots() -> Result<()> {
        let path = PropertyPath::parse("transform.position.x")?;
        assert_eq!(path.to_string(), "transform.position.x");
        Ok(())
    }
}
