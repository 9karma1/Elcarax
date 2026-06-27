use std::collections::BTreeMap;

use crate::kind::AssetKind;
use crate::record::{AssetId, AssetRecord};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssetIndex {
    records: Vec<AssetRecord>,
}

impl AssetIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_records(mut records: Vec<AssetRecord>) -> Self {
        records.sort_by(|left, right| left.path.cmp(&right.path));
        Self { records }
    }

    pub fn insert(&mut self, record: AssetRecord) {
        self.records.push(record);
        self.records
            .sort_by(|left, right| left.path.cmp(&right.path));
    }

    pub fn records(&self) -> &[AssetRecord] {
        self.records.as_slice()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn first(&self) -> Option<&AssetRecord> {
        self.records.first()
    }

    pub fn find(&self, id: AssetId) -> Option<&AssetRecord> {
        self.records.iter().find(|record| record.id == id)
    }

    pub fn kind_counts(&self) -> BTreeMap<AssetKind, usize> {
        let mut counts = BTreeMap::new();
        for record in &self.records {
            *counts.entry(record.kind).or_insert(0) += 1;
        }
        counts
    }

    pub fn kind_summary(&self) -> String {
        let counts = self.kind_counts();
        if counts.is_empty() {
            return "none".to_string();
        }
        counts
            .into_iter()
            .map(|(kind, count)| format!("{}={count}", kind.label()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
