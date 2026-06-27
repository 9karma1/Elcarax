use std::collections::BTreeMap;

use crate::kind::SceneObjectKind;
use crate::name::PropertyName;
use crate::property::{PropertyPath, PropertyValue};
use crate::property_display::{PropertyFormatContext, PropertyGroup, format_property_value};
use crate::schema::{ObjectSchema, PropertySchema};
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
    pub value: String,
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
        kind: object.kind,
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

    if let Some(schema) = schema {
        for property in &schema.properties {
            let Some(value) = object.property(&property.path) else {
                continue;
            };
            let row = inspector_row(property, value, context);
            grouped
                .entry(property.group.as_str().to_string())
                .or_default()
                .push(row);
        }
    }

    for (path, value) in &object.properties {
        if schema.is_some_and(|schema| schema_has_path(schema, path)) {
            continue;
        }
        let group = infer_group(path);
        let label = path_label(path);
        let row = InspectorRow {
            label: PropertyName::from_unvalidated(label),
            value: format_property_value(value, context),
        };
        grouped
            .entry(group.as_str().to_string())
            .or_default()
            .push(row);
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

fn schema_has_path(schema: &ObjectSchema, path: &PropertyPath) -> bool {
    schema
        .properties
        .iter()
        .any(|property| property.path == *path)
}

fn inspector_row(
    property: &PropertySchema,
    value: &PropertyValue,
    context: PropertyFormatContext<'_>,
) -> InspectorRow {
    InspectorRow {
        label: PropertyName::from_unvalidated(property.display_name.clone()),
        value: format_property_value(value, context),
    }
}

fn infer_group(path: &PropertyPath) -> PropertyGroup {
    let head = path.parts().first().map_or("General", |part| part.as_str());
    match head {
        "transform" => PropertyGroup::new("Transform"),
        "gameplay" => PropertyGroup::new("Gameplay"),
        "references" | "refs" => PropertyGroup::new("References"),
        "lighting" => PropertyGroup::new("Lighting"),
        "camera" => PropertyGroup::new("Camera"),
        _ => PropertyGroup::new("General"),
    }
}

fn path_label(path: &PropertyPath) -> String {
    match path.parts().last() {
        Some(label) => label.clone(),
        None => path.to_string(),
    }
}
