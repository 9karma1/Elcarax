//! Hybrid retained UI foundation for Elcarax.

use std::collections::BTreeMap;

use elcarax_core::{Id, IdGenerator};
use elcarax_render::{Color, CornerRadius, Rect, RenderLayer, RenderPrimitive, RenderScene};

pub enum WidgetMarker {}
pub type WidgetId = Id<WidgetMarker>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyFlags(u32);

impl DirtyFlags {
    pub const CLEAN: Self = Self(0);
    pub const LAYOUT: Self = Self(1 << 0);
    pub const PAINT: Self = Self(1 << 1);
    pub const TEXT: Self = Self(1 << 2);
    pub const ACCESSIBILITY: Self = Self(1 << 3);
    pub const HIT_TEST: Self = Self(1 << 4);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Root,
    Panel,
    Label(String),
    Button(String),
    TreeItem(String),
    InspectorRow(String),
    Viewport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetNode {
    pub id: WidgetId,
    pub kind: WidgetKind,
    pub rect: Rect,
    pub children: Vec<WidgetId>,
    pub dirty: DirtyFlags,
}

impl WidgetNode {
    pub fn new(kind: WidgetKind, rect: Rect) -> Self {
        static IDS: IdGenerator<WidgetMarker> = IdGenerator::new();
        Self {
            id: IDS.next_id(),
            kind,
            rect,
            children: Vec::new(),
            dirty: DirtyFlags::LAYOUT,
        }
    }
}

#[derive(Default)]
pub struct UiTree {
    nodes: BTreeMap<WidgetId, WidgetNode>,
    root_id: Option<WidgetId>,
}

impl UiTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root(&mut self, node: WidgetNode) -> WidgetId {
        let id = node.id;
        self.root_id = Some(id);
        self.nodes.insert(id, node);
        id
    }

    pub fn insert_child(&mut self, parent_id: WidgetId, node: WidgetNode) -> Option<WidgetId> {
        let id = node.id;
        self.nodes.insert(id, node);
        let parent = self.nodes.get_mut(&parent_id)?;
        parent.children.push(id);
        parent.dirty.insert(DirtyFlags::LAYOUT);
        Some(id)
    }

    pub fn paint(&self) -> RenderScene {
        let mut scene = RenderScene::new();
        for node in self.nodes.values() {
            match &node.kind {
                WidgetKind::Root | WidgetKind::Panel | WidgetKind::Viewport => {
                    scene.push(
                        RenderLayer::Chrome,
                        RenderPrimitive::rounded_rect(
                            node.rect,
                            CornerRadius::uniform(8.0),
                            Color::srgb(0.08, 0.10, 0.14, 1.0),
                        ),
                    );
                }
                WidgetKind::Label(text)
                | WidgetKind::Button(text)
                | WidgetKind::TreeItem(text)
                | WidgetKind::InspectorRow(text) => {
                    scene.push(
                        RenderLayer::Chrome,
                        RenderPrimitive::text(
                            text.clone(),
                            node.rect.x,
                            node.rect.y + 14.0,
                            14.0,
                            Color::srgb(0.91, 0.93, 0.97, 1.0),
                        ),
                    );
                }
            }
        }
        scene
    }

    pub fn root_id(&self) -> Option<WidgetId> {
        self.root_id
    }
}
