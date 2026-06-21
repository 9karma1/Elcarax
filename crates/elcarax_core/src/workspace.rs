use std::path::PathBuf;

use crate::{Id, IdGenerator};

pub enum WorkspaceMarker {}
pub type WorkspaceId = Id<WorkspaceMarker>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    id: WorkspaceId,
    name: String,
    root_path: PathBuf,
}

impl Workspace {
    pub fn new(name: impl Into<String>, root_path: impl Into<PathBuf>) -> Self {
        static IDS: IdGenerator<WorkspaceMarker> = IdGenerator::new();
        Self {
            id: IDS.next_id(),
            name: name.into(),
            root_path: root_path.into(),
        }
    }

    pub fn id(&self) -> WorkspaceId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }
}
