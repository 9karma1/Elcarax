//! Helpers for implementing Elcarax adapters.

use elcarax_adapter_api::{AdapterToEditor, EditorToAdapter};
use elcarax_core::Result;

pub trait ElcaraxAdapter {
    fn handle_message(&mut self, message: EditorToAdapter) -> Result<AdapterToEditor>;
}

pub struct AdapterMetadata {
    pub name: String,
    pub version: String,
}

impl AdapterMetadata {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}
