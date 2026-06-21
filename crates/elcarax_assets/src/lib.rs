//! Asset indexing foundation for Elcarax.

use std::collections::BTreeMap;
use std::path::PathBuf;

use elcarax_core::{Id, IdGenerator};

pub enum AssetMarker {}
pub type AssetId = Id<AssetMarker>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetKind {
    Unknown,
    Folder,
    Scene,
    Texture,
    Model,
    Script,
    Material,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRecord {
    pub id: AssetId,
    pub path: PathBuf,
    pub kind: AssetKind,
}

impl AssetRecord {
    pub fn new(path: impl Into<PathBuf>, kind: AssetKind) -> Self {
        static IDS: IdGenerator<AssetMarker> = IdGenerator::new();
        Self {
            id: IDS.next_id(),
            path: path.into(),
            kind,
        }
    }
}

#[derive(Default)]
pub struct AssetIndex {
    records: BTreeMap<AssetId, AssetRecord>,
}

impl AssetIndex {
    pub fn insert(&mut self, record: AssetRecord) {
        self.records.insert(record.id, record);
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}
