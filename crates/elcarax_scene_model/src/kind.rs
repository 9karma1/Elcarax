use std::fmt;

use serde::{Deserialize, Serialize};

/// Open object kind identifier. Adapters and schemas register kinds by string id;
/// the editor does not close over a fixed taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneObjectKind(String);

impl SceneObjectKind {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn label(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SceneObjectKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<&str> for SceneObjectKind {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SceneObjectKind {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Well-known kind ids used by the in-repo reference scene and fixtures.
pub mod well_known {
    pub const AUDIO: &str = "audio";
    pub const CAMERA: &str = "camera";
    pub const CHARACTER: &str = "character";
    pub const CUBE: &str = "cube";
    pub const ENVIRONMENT: &str = "environment";
    pub const GROUND: &str = "ground";
    pub const LIGHT: &str = "light";
    pub const MESH: &str = "mesh";
    pub const TRIGGER: &str = "trigger";
    pub const WORLD: &str = "world";
}
