use std::fmt;

pub type Result<T> = std::result::Result<T, ElcaraxError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElcaraxError {
    InvalidInput(String),
    Io(String),
    NotFound(String),
    Project(String),
    Adapter(String),
    Command(String),
    Internal(String),
}

impl ElcaraxError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }
}

impl fmt::Display for ElcaraxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(formatter, "invalid input: {message}"),
            Self::Io(message) => write!(formatter, "io error: {message}"),
            Self::NotFound(message) => write!(formatter, "not found: {message}"),
            Self::Project(message) => write!(formatter, "project error: {message}"),
            Self::Adapter(message) => write!(formatter, "adapter error: {message}"),
            Self::Command(message) => write!(formatter, "command error: {message}"),
            Self::Internal(message) => write!(formatter, "internal error: {message}"),
        }
    }
}

impl std::error::Error for ElcaraxError {}
