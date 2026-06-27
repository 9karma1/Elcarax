use crate::index::AssetIndex;
use crate::record::AssetId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AssetSelection {
    selected: Option<AssetId>,
}

impl AssetSelection {
    pub const fn none() -> Self {
        Self { selected: None }
    }

    pub const fn selected(&self) -> Option<AssetId> {
        self.selected
    }

    pub fn select(&mut self, id: AssetId) {
        self.selected = Some(id);
    }

    pub fn clear(&mut self) {
        self.selected = None;
    }

    pub fn select_first(&mut self, index: &AssetIndex) -> bool {
        let Some(first) = index.first() else {
            self.clear();
            return false;
        };
        self.select(first.id);
        true
    }
}
