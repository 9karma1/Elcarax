use crate::error::SceneError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SceneName(String);

impl SceneName {
    pub fn new(value: impl Into<String>) -> Result<Self, SceneError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(SceneError::EmptySceneName);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn from_unvalidated(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SceneObjectName(String);

impl SceneObjectName {
    pub fn new(value: impl Into<String>) -> Result<Self, SceneError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(SceneError::EmptyObjectName);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn from_unvalidated(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
