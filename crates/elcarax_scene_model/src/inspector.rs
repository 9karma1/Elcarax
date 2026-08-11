use std::collections::BTreeMap;

use crate::component::{ComponentInstance, ComponentInstanceId};
use crate::kind::SceneObjectKind;
use crate::name::PropertyName;
use crate::property::{PropertyPath, PropertyValue};
use crate::property_display::{PropertyFormatContext, PropertyGroup, format_property_value};
use crate::property_registry::PropertyTypeRegistry;
use crate::schema::{
    ComponentSchema, ObjectSchema, PropertyEditKind, PropertyKind, PropertySchema,
};
use crate::snapshot::{SceneObject, SceneObjectId, SceneSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorDiagnostic {
    NoSceneLoaded,
    NoObjectSelected,
    ObjectNotFound,
}

impl InspectorDiagnostic {
    pub fn message(&self) -> &'static str {
        match self {
            Self::NoSceneLoaded => "No scene loaded",
            Self::NoObjectSelected => "No object selected",
            Self::ObjectNotFound => "Object not found",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorRow {
    pub label: PropertyName,
    pub component_id: ComponentInstanceId,
    pub path: PropertyPath,
    pub value: String,
    pub editable: bool,
    pub edit_kind: PropertyEditKind,
    pub read_only_reason: Option<String>,
    pub extension_type_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorSection {
    pub title: PropertyGroup,
    pub rows: Vec<InspectorRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorObject {
    pub object_id: SceneObjectId,
    pub name: String,
    pub kind: SceneObjectKind,
    pub sections: Vec<InspectorSection>,
}

impl InspectorObject {
    pub fn property_count(&self) -> usize {
        self.sections.iter().map(|section| section.rows.len()).sum()
    }
}

pub fn build_inspector_object(
    snapshot: &SceneSnapshot,
    object_id: SceneObjectId,
    property_types: &PropertyTypeRegistry,
) -> Result<InspectorObject, InspectorDiagnostic> {
    let object = snapshot
        .object(object_id)
        .map_err(|_| InspectorDiagnostic::ObjectNotFound)?;
    let schema = snapshot.schema(object.type_id);
    let sections = build_sections(snapshot, object, schema, property_types);
    Ok(InspectorObject {
        object_id,
        name: object.display_name.clone(),
        kind: object.kind.clone(),
        sections,
    })
}

pub fn build_inspector_for_selection(
    snapshot: &SceneSnapshot,
    selected: Option<SceneObjectId>,
    property_types: &PropertyTypeRegistry,
) -> Result<InspectorObject, InspectorDiagnostic> {
    let Some(object_id) = selected else {
        return Err(InspectorDiagnostic::NoObjectSelected);
    };
    build_inspector_object(snapshot, object_id, property_types)
}

fn build_sections(
    snapshot: &SceneSnapshot,
    object: &SceneObject,
    schema: Option<&ObjectSchema>,
    property_types: &PropertyTypeRegistry,
) -> Vec<InspectorSection> {
    let context = PropertyFormatContext { snapshot };
    let mut grouped: BTreeMap<String, Vec<InspectorRow>> = BTreeMap::new();

    for component in &object.components {
        let component_schema = schema.and_then(|schema| schema.component(&component.type_name));
        let rows = grouped.entry(component.display_name.clone()).or_default();
        for (path, value) in &component.properties {
            let property = component_schema.and_then(|schema| property_schema(schema, path));
            rows.push(match property {
                Some(property) => {
                    inspector_row(component.id, property, value, context, property_types)
                }
                None => unschematized_row(component, path, value, context),
            });
        }
    }

    grouped
        .into_iter()
        .map(|(title, mut rows)| {
            rows.sort_by(|left, right| left.label.as_str().cmp(right.label.as_str()));
            InspectorSection {
                title: PropertyGroup::new(title),
                rows,
            }
        })
        .collect()
}

fn property_schema<'a>(
    schema: &'a ComponentSchema,
    path: &PropertyPath,
) -> Option<&'a PropertySchema> {
    schema
        .properties
        .iter()
        .find(|property| property.path == *path)
}

fn inspector_row(
    component_id: ComponentInstanceId,
    property: &PropertySchema,
    value: &PropertyValue,
    context: PropertyFormatContext<'_>,
    property_types: &PropertyTypeRegistry,
) -> InspectorRow {
    let extension_ready = property
        .extension_type_id
        .as_deref()
        .is_some_and(|type_id| property_types.contains(type_id));
    let editable =
        property.editable && (property.kind != PropertyKind::Extension || extension_ready);
    let read_only_reason = if editable {
        None
    } else if property.kind == PropertyKind::Extension && property.editable && !extension_ready {
        Some(
            property
                .extension_type_id
                .as_deref()
                .map(|type_id| format!("No handler registered for extension type '{type_id}'"))
                .unwrap_or_else(|| "Extension property has no registered type id".to_string()),
        )
    } else {
        property.read_only_reason.clone()
    };
    InspectorRow {
        label: PropertyName::from_unvalidated(property.display_name.clone()),
        component_id,
        path: property.path.clone(),
        value: property
            .extension_type_id
            .as_deref()
            .and_then(|type_id| property_types.display(type_id, value))
            .unwrap_or_else(|| format_property_value(value, context)),
        editable,
        edit_kind: property.edit_kind,
        read_only_reason,
        extension_type_id: property.extension_type_id.clone(),
    }
}

fn unschematized_row(
    component: &ComponentInstance,
    path: &PropertyPath,
    value: &PropertyValue,
    context: PropertyFormatContext<'_>,
) -> InspectorRow {
    InspectorRow {
        label: PropertyName::from_unvalidated(path_label(path)),
        component_id: component.id,
        path: path.clone(),
        value: format_property_value(value, context),
        editable: false,
        edit_kind: PropertyEditKind::Unsupported,
        read_only_reason: Some("No editable property schema is available".to_string()),
        extension_type_id: None,
    }
}

fn path_label(path: &PropertyPath) -> String {
    match path.parts().last() {
        Some(label) => label.clone(),
        None => path.to_string(),
    }
}
