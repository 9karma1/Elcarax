use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    EmptySceneName,
    EmptyObjectName,
    ObjectNotFound,
    InvalidHierarchy,
}

impl fmt::Display for SceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySceneName => write!(formatter, "scene name cannot be empty"),
            Self::EmptyObjectName => write!(formatter, "scene object name cannot be empty"),
            Self::ObjectNotFound => write!(formatter, "scene object was not found"),
            Self::InvalidHierarchy => write!(formatter, "scene hierarchy is invalid"),
        }
    }
}

impl Error for SceneError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneIoError {
    Io(String),
    UnsupportedSchemaVersion(u32),
    InvalidDocument(String),
    NoSceneFileFound,
    Scene(SceneError),
}

impl fmt::Display for SceneIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "{message}"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported scene schema version {version}")
            }
            Self::InvalidDocument(message) => {
                write!(formatter, "invalid scene document: {message}")
            }
            Self::NoSceneFileFound => write!(formatter, "no scene file found in scene root"),
            Self::Scene(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for SceneIoError {}

impl From<SceneError> for SceneIoError {
    fn from(error: SceneError) -> Self {
        Self::Scene(error)
    }
}
