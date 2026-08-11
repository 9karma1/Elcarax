use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::{PropertyEditError, PropertyEditKind, PropertyPath, PropertyValue};

/// Runtime behavior for an adapter- or plugin-defined property type.
///
/// The scene model owns the lifecycle of a property edit. Handlers only
/// provide the type-specific parse, validation, and display behavior; they do
/// not mutate scene state or bypass patch application.
pub trait PropertyTypeHandler: Send + Sync {
    fn type_id(&self) -> &str;

    fn parse_text(
        &self,
        path: &PropertyPath,
        text: &str,
    ) -> std::result::Result<PropertyValue, String>;

    fn validate(
        &self,
        path: &PropertyPath,
        value: &PropertyValue,
    ) -> std::result::Result<(), String>;

    fn display(&self, value: &PropertyValue) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyTypeRegistryError {
    EmptyTypeId,
    DuplicateTypeId(String),
}

impl fmt::Display for PropertyTypeRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTypeId => write!(formatter, "property type id cannot be empty"),
            Self::DuplicateTypeId(type_id) => {
                write!(formatter, "property type '{type_id}' is already registered")
            }
        }
    }
}

impl std::error::Error for PropertyTypeRegistryError {}

#[derive(Clone, Default)]
pub struct PropertyTypeRegistry {
    handlers: BTreeMap<String, Arc<dyn PropertyTypeHandler>>,
}

impl fmt::Debug for PropertyTypeRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PropertyTypeRegistry")
            .field("type_ids", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl PropertyTypeRegistry {
    pub fn register<H: PropertyTypeHandler + 'static>(
        &mut self,
        handler: H,
    ) -> std::result::Result<(), PropertyTypeRegistryError> {
        self.register_arc(Arc::new(handler))
    }

    pub fn register_arc(
        &mut self,
        handler: Arc<dyn PropertyTypeHandler>,
    ) -> std::result::Result<(), PropertyTypeRegistryError> {
        let type_id = handler.type_id().to_string();
        if type_id.trim().is_empty() {
            return Err(PropertyTypeRegistryError::EmptyTypeId);
        }
        if self.handlers.contains_key(&type_id) {
            return Err(PropertyTypeRegistryError::DuplicateTypeId(type_id));
        }
        self.handlers.insert(type_id, handler);
        Ok(())
    }

    pub fn contains(&self, type_id: &str) -> bool {
        self.handlers.contains_key(type_id)
    }

    pub fn handler(&self, type_id: &str) -> Option<&dyn PropertyTypeHandler> {
        self.handlers.get(type_id).map(Arc::as_ref)
    }

    pub fn parse_text(
        &self,
        path: &PropertyPath,
        type_id: &str,
        text: &str,
    ) -> std::result::Result<PropertyValue, PropertyEditError> {
        let handler = self
            .handler(type_id)
            .ok_or_else(|| PropertyEditError::ReadOnly {
                path: path.clone(),
                reason: format!("No property type handler is registered for '{type_id}'"),
            })?;
        let value = handler
            .parse_text(path, text)
            .map_err(|actual| type_mismatch(path, actual))?;
        if !matches!(
            &value,
            PropertyValue::Extension {
                type_id: value_type_id,
                ..
            } if value_type_id == type_id
        ) {
            return Err(type_mismatch(
                path,
                format!(
                    "handler returned a value for the wrong extension type (expected '{type_id}')"
                ),
            ));
        }
        self.validate(path, type_id, &value)?;
        Ok(value)
    }

    pub fn validate(
        &self,
        path: &PropertyPath,
        type_id: &str,
        value: &PropertyValue,
    ) -> std::result::Result<(), PropertyEditError> {
        let handler = self
            .handler(type_id)
            .ok_or_else(|| PropertyEditError::ReadOnly {
                path: path.clone(),
                reason: format!("No property type handler is registered for '{type_id}'"),
            })?;
        if !matches!(
            value,
            PropertyValue::Extension {
                type_id: value_type_id,
                ..
            } if value_type_id == type_id
        ) {
            return Err(type_mismatch(
                path,
                format!("expected extension value of type '{type_id}'"),
            ));
        }
        handler
            .validate(path, value)
            .map_err(|actual| type_mismatch(path, actual))
    }

    pub fn display(&self, type_id: &str, value: &PropertyValue) -> Option<String> {
        self.handler(type_id).map(|handler| handler.display(value))
    }
}

fn type_mismatch(path: &PropertyPath, actual: String) -> PropertyEditError {
    PropertyEditError::TypeMismatch {
        path: path.clone(),
        expected: PropertyEditKind::Extension,
        actual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TextExtension;

    impl PropertyTypeHandler for TextExtension {
        fn type_id(&self) -> &str {
            "test.text"
        }

        fn parse_text(
            &self,
            _path: &PropertyPath,
            text: &str,
        ) -> std::result::Result<PropertyValue, String> {
            Ok(PropertyValue::Extension {
                type_id: self.type_id().to_string(),
                data: serde_json::Value::String(text.to_string()),
            })
        }

        fn validate(
            &self,
            _path: &PropertyPath,
            value: &PropertyValue,
        ) -> std::result::Result<(), String> {
            if matches!(
                value,
                PropertyValue::Extension {
                    data: serde_json::Value::String(_),
                    ..
                }
            ) {
                Ok(())
            } else {
                Err("expected string extension data".to_string())
            }
        }

        fn display(&self, value: &PropertyValue) -> String {
            match value {
                PropertyValue::Extension {
                    data: serde_json::Value::String(text),
                    ..
                } => format!("text:{text}"),
                _ => value.display_label(),
            }
        }
    }

    #[test]
    fn registered_handler_owns_parse_validate_and_display() {
        let mut registry = PropertyTypeRegistry::default();
        assert!(registry.register(TextExtension).is_ok());
        let path = PropertyPath::fixture_from_segments(&["custom"]);
        let value = match registry.parse_text(&path, "test.text", "hello") {
            Ok(value) => value,
            Err(error) => panic!("registered extension should parse: {error}"),
        };
        assert_eq!(
            registry.display("test.text", &value),
            Some("text:hello".to_string())
        );
    }

    #[test]
    fn registered_extension_is_editable_through_scene_kernel() {
        let path = PropertyPath::fixture_from_segments(&["custom"]);
        let schema = crate::ObjectSchema::new("Custom").with_component(
            crate::ComponentSchema::new("custom", "Custom").with_property(
                crate::PropertySchema::extension(path.clone(), "Custom", "test.text", true),
            ),
        );
        let component = crate::ComponentInstance::new("custom", "Custom").with_property(
            path.clone(),
            PropertyValue::Extension {
                type_id: "test.text".to_string(),
                data: serde_json::Value::String("before".to_string()),
            },
        );
        let object = crate::SceneObject::new(
            "Custom Object",
            crate::SceneObjectKind::new(crate::kinds::WORLD),
            schema.type_id,
        )
        .with_component(component);
        let object_id = object.id;
        let component_id = object.components[0].id;
        let mut snapshot = crate::SceneSnapshot::empty();
        snapshot.add_schema(schema);
        let mut registry = PropertyTypeRegistry::default();
        assert!(registry.register(TextExtension).is_ok());
        assert!(snapshot.add_object(None, 0, object, &registry).is_ok());

        let value = match registry.parse_text(&path, "test.text", "after") {
            Ok(value) => value,
            Err(error) => panic!("extension text should parse: {error}"),
        };
        assert!(
            crate::edit_scene_property(
                &mut snapshot,
                object_id,
                component_id,
                &path,
                value,
                &registry,
            )
            .is_ok()
        );
        let inspector = match crate::build_inspector_object(&snapshot, object_id, &registry) {
            Ok(inspector) => inspector,
            Err(error) => panic!("custom inspector should build: {error:?}"),
        };
        let row = &inspector.sections[0].rows[0];
        assert!(row.editable);
        assert_eq!(row.value, "text:after");
    }
}
