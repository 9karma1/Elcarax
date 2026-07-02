use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    EmptyProjectName,
    EmptyProjectPath,
    ManifestMissing,
    ManifestInvalid(String),
    UnsupportedSchemaVersion(u32),
    AlreadyExists,
    PathInvalid(String),
    Io(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProjectName => write!(formatter, "project name cannot be empty"),
            Self::EmptyProjectPath => write!(formatter, "project path cannot be empty"),
            Self::ManifestMissing => write!(formatter, "project manifest is missing"),
            Self::ManifestInvalid(message) => {
                write!(formatter, "project manifest is invalid: {message}")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported project schema version: {version}")
            }
            Self::AlreadyExists => write!(formatter, "project already exists at the target path"),
            Self::PathInvalid(message) => write!(formatter, "project path is invalid: {message}"),
            Self::Io(message) => write!(formatter, "project filesystem error: {message}"),
        }
    }
}

impl Error for ProjectError {}
