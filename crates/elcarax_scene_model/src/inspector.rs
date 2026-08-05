use std::collections::BTreeMap;

use crate::component::{ComponentInstance, ComponentInstanceId};
use crate::kind::SceneObjectKind;
use crate::name::PropertyName;
use crate::property::{PropertyPath, PropertyValue};
use crate::property_display::{PropertyFormatContext, PropertyGroup, format_property_value};
use crate::schema::{ComponentSchema, ObjectSchema, PropertyEditKind, PropertySchema};
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
) -> Result<InspectorObject, InspectorDiagnostic> {
    let object = snapshot
        .object(object_id)
        .map_err(|_| InspectorDiagnostic::ObjectNotFound)?;
    let schema = snapshot.schema(object.type_id);
    let sections = build_sections(snapshot, object, schema);
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
) -> Result<InspectorObject, InspectorDiagnostic> {
    let Some(object_id) = selected else {
        return Err(InspectorDiagnostic::NoObjectSelected);
    };
    build_inspector_object(snapshot, object_id)
}

fn build_sections(
    snapshot: &SceneSnapshot,
    object: &SceneObject,
    schema: Option<&ObjectSchema>,
) -> Vec<InspectorSection> {
    let context = PropertyFormatContext { snapshot };
    let mut grouped: BTreeMap<String, Vec<InspectorRow>> = BTreeMap::new();

    for component in &object.components {
        let component_schema = schema.and_then(|schema| schema.component(&component.type_name));
        let rows = grouped.entry(component.display_name.clone()).or_default();
        for (path, value) in &component.properties {
            let property = component_schema.and_then(|schema| property_schema(schema, path));
            rows.push(match property {
                Some(property) => inspector_row(component.id, property, value, context),
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
) -> InspectorRow {
    InspectorRow {
        label: PropertyName::from_unvalidated(property.display_name.clone()),
        component_id,
        path: property.path.clone(),
        value: format_property_value(value, context),
        editable: property.editable,
        edit_kind: property.edit_kind,
        read_only_reason: property.read_only_reason.clone(),
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
    }
}

fn path_label(path: &PropertyPath) -> String {
    match path.parts().last() {
        Some(label) => label.clone(),
        None => path.to_string(),
    }
}
