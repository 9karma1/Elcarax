use std::collections::BTreeSet;

use crate::SceneError;
use crate::snapshot::{SceneObjectId, SceneSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneSelection {
    selected: Option<SceneObjectId>,
}

impl SceneSelection {
    pub const fn none() -> Self {
        Self { selected: None }
    }

    pub const fn selected(&self) -> Option<SceneObjectId> {
        self.selected
    }

    pub fn select(&mut self, id: SceneObjectId) -> Result<(), SceneError> {
        self.selected = Some(id);
        Ok(())
    }

    pub fn select_existing(
        &mut self,
        snapshot: &SceneSnapshot,
        id: SceneObjectId,
    ) -> Result<(), SceneError> {
        if snapshot.object(id).is_err() {
            return Err(SceneError::ObjectNotFound);
        }
        self.select(id)
    }

    pub fn clear(&mut self) {
        self.selected = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SceneExpansion {
    expanded: BTreeSet<SceneObjectId>,
}

impl SceneExpansion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_expanded(&self, id: SceneObjectId) -> bool {
        self.expanded.contains(&id)
    }

    pub fn expand(&mut self, id: SceneObjectId) {
        self.expanded.insert(id);
    }

    pub fn collapse(&mut self, id: SceneObjectId) {
        self.expanded.remove(&id);
    }

    pub fn toggle(&mut self, id: SceneObjectId) {
        if self.is_expanded(id) {
            self.collapse(id);
        } else {
            self.expand(id);
        }
    }

    pub fn expand_all(&mut self, snapshot: &SceneSnapshot) {
        self.expanded = snapshot.expandable_object_ids().into_iter().collect();
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    pub fn expanded_set(&self) -> &BTreeSet<SceneObjectId> {
        &self.expanded
    }

    pub fn len(&self) -> usize {
        self.expanded.len()
    }

    pub fn is_empty(&self) -> bool {
        self.expanded.is_empty()
    }
}
