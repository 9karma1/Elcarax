use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    EmptyAssetName,
    EmptyAssetPath,
    Io(String),
}

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAssetName => write!(formatter, "asset name cannot be empty"),
            Self::EmptyAssetPath => write!(formatter, "asset path cannot be empty"),
            Self::Io(message) => write!(formatter, "asset io error: {message}"),
        }
    }
}

impl Error for AssetError {}
