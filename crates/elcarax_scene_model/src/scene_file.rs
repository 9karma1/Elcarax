use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use elcarax_core::Id;
use serde::{Deserialize, Serialize};

use crate::SceneIoError;
use crate::component::{ComponentInstance, ComponentTypeName};
use crate::kind::SceneObjectKind;
use crate::name::SceneName;
use crate::schema::{ComponentSchema, ObjectSchema, ObjectTypeId};
use crate::snapshot::{SceneObject, SceneObjectId, SceneSnapshot};
use crate::{PropertyPath, PropertyValue};

pub const SCENE_FILE_SUFFIX: &str = ".elcarax.scene.toml";
pub const DEFAULT_SCENE_FILENAME: &str = "main.elcarax.scene.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneFileVersion(pub u32);

impl SceneFileVersion {
    pub const CURRENT: Self = Self(2);

    pub const fn value(self) -> u32 {
        self.0
    }

    pub fn validate(self) -> Result<(), SceneIoError> {
        if self.0 == Self::CURRENT.0 {
            Ok(())
        } else {
            Err(SceneIoError::UnsupportedSchemaVersion(self.0))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneFile {
    version: SceneFileVersion,
    snapshot: SceneSnapshot,
}

impl SceneFile {
    pub fn from_snapshot(snapshot: &SceneSnapshot) -> Self {
        Self {
            version: SceneFileVersion::CURRENT,
            snapshot: snapshot.clone(),
        }
    }

    pub fn into_snapshot(self) -> SceneSnapshot {
        self.snapshot
    }

    pub fn snapshot(&self) -> &SceneSnapshot {
        &self.snapshot
    }

    pub fn to_toml_string(&self) -> Result<String, SceneIoError> {
        let document = SceneFileDocument::from_snapshot(&self.snapshot, self.version);
        toml::to_string_pretty(&document)
            .map_err(|error| SceneIoError::InvalidDocument(error.to_string()))
    }

    pub fn from_toml_str(content: &str) -> Result<Self, SceneIoError> {
        let document: SceneFileDocument = toml::from_str(content)
            .map_err(|error| SceneIoError::InvalidDocument(error.to_string()))?;
        let version = SceneFileVersion(document.schema_version);
        version.validate()?;
        let snapshot = document.into_snapshot()?;
        Ok(Self { version, snapshot })
    }
}

pub fn is_scene_file_name(file_name: &str) -> bool {
    file_name.ends_with(SCENE_FILE_SUFFIX)
}

pub fn scene_file_path_in_root(scene_root: &Path, file_name: &str) -> PathBuf {
    scene_root.join(file_name)
}

#[derive(Debug, Serialize, Deserialize)]
struct SceneFileDocument {
    schema_version: u32,
    scene_id: u64,
    name: String,
    root_object_ids: Vec<u64>,
    objects: Vec<SceneObjectDocument>,
    schemas: Vec<ObjectSchemaDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SceneObjectDocument {
    id: u64,
    parent: Option<u64>,
    children: Vec<u64>,
    display_name: String,
    kind: SceneObjectKind,
    type_id: u64,
    property_summary: Option<String>,
    components: Vec<ComponentInstanceDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComponentInstanceDocument {
    id: u64,
    type_name: ComponentTypeName,
    display_name: String,
    properties: BTreeMap<String, PropertyValue>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ObjectSchemaDocument {
    type_id: u64,
    display_name: String,
    components: Vec<ComponentSchema>,
}

impl SceneFileDocument {
    fn from_snapshot(snapshot: &SceneSnapshot, version: SceneFileVersion) -> Self {
        Self {
            schema_version: version.value(),
            scene_id: snapshot.scene_id().get(),
            name: snapshot.name().as_str().to_string(),
            root_object_ids: snapshot
                .root_object_ids()
                .iter()
                .map(|id| id.get())
                .collect(),
            objects: snapshot
                .objects()
                .values()
                .map(SceneObjectDocument::from_object)
                .collect(),
            schemas: snapshot
                .object_schemas()
                .values()
                .map(ObjectSchemaDocument::from_schema)
                .collect(),
        }
    }

    fn into_snapshot(self) -> Result<SceneSnapshot, SceneIoError> {
        let scene_id = required_id::<crate::snapshot::SceneMarker>(self.scene_id, "scene_id")?;
        let name = SceneName::parse(&self.name)
            .map_err(|error| SceneIoError::InvalidDocument(error.to_string()))?;
        let root_objects = ids_from_values(self.root_object_ids, "root_object_id")?;
        let objects = objects_from_documents(self.objects)?;
        let schemas = schemas_from_documents(self.schemas)?;
        let snapshot = SceneSnapshot::from_storage(scene_id, name, root_objects, objects, schemas);
        if let Err(reason) = crate::SceneHierarchy::validate_detailed(&snapshot) {
            return Err(SceneIoError::InvalidDocument(reason));
        }
        Ok(snapshot)
    }
}

impl SceneObjectDocument {
    fn from_object(object: &SceneObject) -> Self {
        Self {
            id: object.id.get(),
            parent: object.parent.map(|id| id.get()),
            children: object.children.iter().map(|id| id.get()).collect(),
            display_name: object.display_name.clone(),
            kind: object.kind.clone(),
            type_id: object.type_id.get(),
            property_summary: object.property_summary.clone(),
            components: object
                .components
                .iter()
                .map(ComponentInstanceDocument::from_component)
                .collect(),
        }
    }
}

impl ComponentInstanceDocument {
    fn from_component(component: &ComponentInstance) -> Self {
        Self {
            id: component.id.get(),
            type_name: component.type_name.clone(),
            display_name: component.display_name.clone(),
            properties: component
                .properties
                .iter()
                .map(|(path, value)| (path.to_string(), value.clone()))
                .collect(),
        }
    }

    fn into_component(self) -> Result<ComponentInstance, SceneIoError> {
        let id = required_id::<crate::component::ComponentInstanceMarker>(self.id, "component_id")?;
        let mut component =
            ComponentInstance::with_stable_id(id, self.type_name, self.display_name);
        component.properties = properties_from_document(self.properties)?;
        Ok(component)
    }
}

impl ObjectSchemaDocument {
    fn from_schema(schema: &ObjectSchema) -> Self {
        Self {
            type_id: schema.type_id.get(),
            display_name: schema.display_name.clone(),
            components: schema.components.clone(),
        }
    }
}

fn objects_from_documents(
    documents: Vec<SceneObjectDocument>,
) -> Result<BTreeMap<SceneObjectId, SceneObject>, SceneIoError> {
    let mut objects = BTreeMap::new();
    for document in documents {
        let id = required_id::<crate::snapshot::SceneObjectMarker>(document.id, "object_id")?;
        let parent = optional_id::<crate::snapshot::SceneObjectMarker>(document.parent)?;
        let children = ids_from_values(document.children, "child_id")?;
        let type_id = required_id::<crate::schema::ObjectTypeMarker>(document.type_id, "type_id")?;
        let components = document
            .components
            .into_iter()
            .map(ComponentInstanceDocument::into_component)
            .collect::<Result<Vec<_>, _>>()?;
        let object = SceneObject {
            id,
            parent,
            children,
            display_name: document.display_name,
            kind: document.kind,
            type_id,
            property_summary: document.property_summary,
            components,
        };
        if objects.insert(id, object).is_some() {
            return Err(SceneIoError::InvalidDocument(format!(
                "duplicate object id {}",
                id.get()
            )));
        }
    }
    Ok(objects)
}

fn schemas_from_documents(
    documents: Vec<ObjectSchemaDocument>,
) -> Result<BTreeMap<ObjectTypeId, ObjectSchema>, SceneIoError> {
    let mut schemas = BTreeMap::new();
    for document in documents {
        let type_id =
            required_id::<crate::schema::ObjectTypeMarker>(document.type_id, "schema_type_id")?;
        let schema = ObjectSchema {
            type_id,
            display_name: document.display_name,
            components: document.components,
        };
        if schemas.insert(type_id, schema).is_some() {
            return Err(SceneIoError::InvalidDocument(format!(
                "duplicate schema type id {}",
                type_id.get()
            )));
        }
    }
    Ok(schemas)
}

fn properties_from_document(
    properties: BTreeMap<String, PropertyValue>,
) -> Result<BTreeMap<PropertyPath, PropertyValue>, SceneIoError> {
    let mut parsed = BTreeMap::new();
    for (path, value) in properties {
        let path = PropertyPath::parse(&path)
            .map_err(|error| SceneIoError::InvalidDocument(error.to_string()))?;
        parsed.insert(path, value);
    }
    Ok(parsed)
}

fn required_id<T>(value: u64, field: &str) -> Result<Id<T>, SceneIoError> {
    Id::new(value)
        .ok_or_else(|| SceneIoError::InvalidDocument(format!("{field} must be a non-zero id")))
}

fn optional_id<T>(value: Option<u64>) -> Result<Option<Id<T>>, SceneIoError> {
    match value {
        Some(value) => required_id::<T>(value, "parent_id").map(Some),
        None => Ok(None),
    }
}

fn ids_from_values<T>(values: Vec<u64>, field: &str) -> Result<Vec<Id<T>>, SceneIoError> {
    values
        .into_iter()
        .map(|value| required_id::<T>(value, field))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObjectSchema;
    use crate::kind::SceneObjectKind;

    #[test]
    fn default_scene_file_serializes_schema_version() {
        let snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Main Scene"));
        let content = match SceneFile::from_snapshot(&snapshot).to_toml_string() {
            Ok(value) => value,
            Err(error) => panic!("serialize should succeed: {error}"),
        };
        assert!(content.contains("schema_version = 2"));
        assert!(content.contains("name = \"Main Scene\""));
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let content = "schema_version = 99\nscene_id = 1\nname = \"Bad\"\nroot_object_ids = []\nobjects = []\nschemas = []";
        let result = SceneFile::from_toml_str(content);
        assert!(matches!(
            result,
            Err(SceneIoError::UnsupportedSchemaVersion(99))
        ));
    }

    #[test]
    fn scene_file_name_suffix_is_recognized() {
        assert!(is_scene_file_name("main.elcarax.scene.toml"));
        assert!(!is_scene_file_name("main.toml"));
    }

    #[test]
    fn object_roundtrip_preserves_hierarchy_fields() {
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Hierarchy"));
        let schema = ObjectSchema::new("World");
        let type_id = schema.type_id;
        snapshot.add_schema(schema);
        let root = SceneObject::new(
            "World",
            SceneObjectKind::new(crate::kind::well_known::WORLD),
            type_id,
        );
        let root_id = root.id;
        let child = SceneObject::new(
            "Child",
            SceneObjectKind::new(crate::kind::well_known::MESH),
            type_id,
        );
        let child_id = child.id;
        let property_types = crate::PropertyTypeRegistry::default();
        if let Err(error) = snapshot.add_object(None, 0, root, &property_types) {
            panic!("add root should succeed: {error}");
        }
        if let Err(error) = snapshot.add_object(Some(root_id), 0, child, &property_types) {
            panic!("add child should succeed: {error}");
        }
        let serialized = match SceneFile::from_snapshot(&snapshot).to_toml_string() {
            Ok(value) => value,
            Err(error) => panic!("serialize should succeed: {error}"),
        };
        let restored = match SceneFile::from_toml_str(&serialized) {
            Ok(value) => value.into_snapshot(),
            Err(error) => panic!("deserialize should succeed: {error}"),
        };
        let parent = match restored.object(child_id) {
            Ok(value) => value,
            Err(error) => panic!("child should exist: {error}"),
        };
        assert_eq!(parent.parent, Some(root_id));
        let root = match restored.object(root_id) {
            Ok(value) => value,
            Err(error) => panic!("root should exist: {error}"),
        };
        assert_eq!(root.children, vec![child_id]);
    }
}
