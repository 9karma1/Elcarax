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
