//! Engine-neutral scene, object, schema, and property model.

mod property;
mod schema;
mod snapshot;

pub use property::{PropertyPath, PropertyValue};
pub use schema::{ObjectSchema, ObjectTypeId, PropertyKind, PropertySchema};
pub use snapshot::{
    SceneId, SceneMarker, SceneObject, SceneObjectId, SceneObjectMarker, SceneSnapshot,
};
