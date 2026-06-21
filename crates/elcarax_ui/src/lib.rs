//! Retained UI tree, layout, style, and paint foundation for Elcarax.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use elcarax_core::{Id, IdGenerator};
use elcarax_render::{
    Border, Color, CornerRadius, Rect, RenderLayer, RenderPrimitive, RenderScene,
};

pub enum WidgetMarker {}
pub type WidgetId = Id<WidgetMarker>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyFlags(u32);

impl DirtyFlags {
    pub const CLEAN: Self = Self(0);
    pub const LAYOUT: Self = Self(1 << 0);
    pub const PAINT: Self = Self(1 << 1);
    pub const TEXT: Self = Self(1 << 2);
    pub const HIT_TEST: Self = Self(1 << 3);
    pub const ACCESSIBILITY: Self = Self(1 << 4);
    pub const ALL: Self = Self(
        Self::LAYOUT.0 | Self::PAINT.0 | Self::TEXT.0 | Self::HIT_TEST.0 | Self::ACCESSIBILITY.0,
    );

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn is_clean(self) -> bool {
        self.0 == 0
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
    IconButton(String),
    Separator(Axis),
    StatusBar,
    Toolbar,
    ViewportPlaceholder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizePolicy {
    Fixed(f32),
    Fill,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const ZERO: Self = Self::uniform(0.0);

    pub const fn uniform(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    fn shrink(self, rect: Rect) -> Rect {
        Rect::new(
            rect.x + self.left,
            rect.y + self.top,
            (rect.width - self.left - self.right).max(0.0),
            (rect.height - self.top - self.bottom).max(0.0),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutMode {
    Leaf,
    Stack(Axis),
    Split(Axis),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutNode {
    pub width: SizePolicy,
    pub height: SizePolicy,
    pub mode: LayoutMode,
    pub padding: Insets,
    pub alignment: Alignment,
}

impl LayoutNode {
    pub const fn leaf() -> Self {
        Self {
            width: SizePolicy::Content,
            height: SizePolicy::Content,
            mode: LayoutMode::Leaf,
            padding: Insets::ZERO,
            alignment: Alignment::Start,
        }
    }

    pub const fn fill(mode: LayoutMode) -> Self {
        Self {
            width: SizePolicy::Fill,
            height: SizePolicy::Fill,
            mode,
            padding: Insets::ZERO,
            alignment: Alignment::Stretch,
        }
    }

    pub const fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: SizePolicy::Fixed(width),
            height: SizePolicy::Fixed(height),
            mode: LayoutMode::Leaf,
            padding: Insets::ZERO,
            alignment: Alignment::Stretch,
        }
    }

    pub const fn with_padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    pub const fn with_width(mut self, width: SizePolicy) -> Self {
        self.width = width;
        self
    }

    pub const fn with_height(mut self, height: SizePolicy) -> Self {
        self.height = height;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraints {
    pub bounds: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutResult {
    pub node_count: usize,
    pub bounds: BTreeMap<WidgetId, Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiStyle {
    pub role: StyleRole,
    pub text_role: TextRole,
    pub corner_radius: f32,
}

impl UiStyle {
    pub const ROOT: Self = Self::new(StyleRole::Background);
    pub const PANEL: Self = Self::new(StyleRole::Surface);
    pub const TOOLBAR: Self = Self::new(StyleRole::RaisedSurface);
    pub const STATUS_BAR: Self = Self::new(StyleRole::RaisedSurface);
    pub const VIEWPORT: Self = Self::new(StyleRole::Viewport);
    pub const LABEL: Self = Self::new(StyleRole::Transparent);
    pub const BUTTON: Self = Self::new(StyleRole::Control).rounded(4.0);
    pub const SEPARATOR: Self = Self::new(StyleRole::Border);

    pub const fn new(role: StyleRole) -> Self {
        Self {
            role,
            text_role: TextRole::Default,
            corner_radius: 0.0,
        }
    }

    pub const fn muted_text(mut self) -> Self {
        self.text_role = TextRole::Muted;
        self
    }

    pub const fn rounded(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleRole {
    Background,
    Surface,
    RaisedSurface,
    Control,
    Viewport,
    Border,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRole {
    Default,
    Muted,
    Accent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub background: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub viewport: Color,
    pub control: Color,
    pub control_hovered: Color,
    pub control_active: Color,
    pub focus_ring: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub spacing: SpacingScale,
    pub fonts: FontScale,
}

impl Theme {
    pub const fn editor_dark() -> Self {
        Self {
            background: Color::ELCARAX_DARK,
            surface: Color::srgb(0.095, 0.105, 0.14, 1.0),
            surface_raised: Color::srgb(0.075, 0.082, 0.11, 1.0),
            viewport: Color::srgb(0.045, 0.05, 0.07, 1.0),
            control: Color::srgb(0.13, 0.15, 0.20, 1.0),
            control_hovered: Color::srgb(0.18, 0.21, 0.29, 1.0),
            control_active: Color::srgb(0.23, 0.29, 0.44, 1.0),
            focus_ring: Color::srgb(0.58, 0.68, 0.95, 1.0),
            border: Color::srgb(0.18, 0.20, 0.26, 1.0),
            text: Color::srgb(0.91, 0.93, 0.97, 1.0),
            text_muted: Color::srgb(0.62, 0.66, 0.74, 1.0),
            accent: Color::srgb(0.26, 0.34, 0.55, 1.0),
            spacing: SpacingScale {
                xs: 4.0,
                sm: 8.0,
                md: 16.0,
                lg: 24.0,
            },
            fonts: FontScale {
                small: 13.0,
                body: 14.0,
                title: 18.0,
            },
        }
    }

    pub fn color_for(self, style: UiStyle) -> Option<Color> {
        match style.role {
            StyleRole::Background => Some(self.background),
            StyleRole::Surface => Some(self.surface),
            StyleRole::RaisedSurface => Some(self.surface_raised),
            StyleRole::Control => Some(self.control),
            StyleRole::Viewport => Some(self.viewport),
            StyleRole::Border => Some(self.border),
            StyleRole::Transparent => None,
        }
    }

    pub const fn text_color_for(self, role: TextRole) -> Color {
        match role {
            TextRole::Default => self.text,
            TextRole::Muted => self.text_muted,
            TextRole::Accent => self.accent,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::editor_dark()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingScale {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontScale {
    pub small: f32,
    pub body: f32,
    pub title: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerPosition {
    pub x: f32,
    pub y: f32,
}

impl PointerPosition {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyboardKey {
    Enter,
    Space,
    Tab,
    Escape,
    Character(String),
    Other(String),
}

impl KeyboardKey {
    pub fn from_platform_key(key: impl Into<String>) -> Self {
        let key = key.into();
        match key.as_str() {
            "Enter" | "Named(Enter)" => Self::Enter,
            " " | "Space" | "Named(Space)" => Self::Space,
            "Tab" | "Named(Tab)" => Self::Tab,
            "Escape" | "Named(Escape)" => Self::Escape,
            _ if key.chars().count() == 1 => Self::Character(key),
            _ => Self::Other(key),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModifierState {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl ModifierState {
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        alt: false,
        super_key: false,
    };
}

impl Default for ModifierState {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiInputEvent {
    PointerMoved(PointerPosition),
    PointerEntered,
    PointerLeft,
    PointerButtonPressed(PointerButton),
    PointerButtonReleased(PointerButton),
    MouseWheel { delta_x: f32, delta_y: f32 },
    KeyPressed(KeyboardKey),
    KeyReleased(KeyboardKey),
    ModifiersChanged(ModifierState),
    WindowFocused,
    WindowUnfocused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitTestResult {
    pub id: WidgetId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusChange {
    pub previous: Option<WidgetId>,
    pub next: Option<WidgetId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionState {
    pub hovered: bool,
    pub focused: bool,
    pub active: bool,
    pub disabled: bool,
    pub visible: bool,
    pub focusable: bool,
    pub interactive: bool,
}

impl InteractionState {
    pub const fn container() -> Self {
        Self {
            hovered: false,
            focused: false,
            active: false,
            disabled: false,
            visible: true,
            focusable: false,
            interactive: true,
        }
    }

    pub const fn passive() -> Self {
        Self {
            hovered: false,
            focused: false,
            active: false,
            disabled: false,
            visible: true,
            focusable: false,
            interactive: false,
        }
    }

    pub const fn control() -> Self {
        Self {
            hovered: false,
            focused: false,
            active: false,
            disabled: false,
            visible: true,
            focusable: true,
            interactive: true,
        }
    }

    pub const fn can_hit_test(self) -> bool {
        self.visible && !self.disabled && self.interactive
    }

    pub const fn can_focus(self) -> bool {
        self.visible && !self.disabled && self.focusable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEvent {
    HoverChanged { id: WidgetId, hovered: bool },
    FocusChanged(FocusChange),
    ActiveChanged { id: WidgetId, active: bool },
    Clicked { id: WidgetId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiError {
    MissingRoot,
    MissingNode(WidgetId),
    DuplicateNode(WidgetId),
}

impl fmt::Display for UiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRoot => write!(formatter, "UI tree has no root node"),
            Self::MissingNode(id) => write!(formatter, "UI node {id:?} does not exist"),
            Self::DuplicateNode(id) => write!(formatter, "UI node {id:?} already exists"),
        }
    }
}

impl Error for UiError {}

#[derive(Debug, Clone, PartialEq)]
pub struct UiNode {
    pub id: WidgetId,
    pub parent: Option<WidgetId>,
    pub children: Vec<WidgetId>,
    pub kind: WidgetKind,
    pub style: UiStyle,
    pub layout: LayoutNode,
    pub rect: Rect,
    pub dirty: DirtyFlags,
    pub interaction: InteractionState,
}

impl UiNode {
    pub fn new(id: WidgetId, kind: WidgetKind, style: UiStyle, layout: LayoutNode) -> Self {
        Self {
            id,
            parent: None,
            children: Vec::new(),
            interaction: default_interaction_for(&kind),
            kind,
            style,
            layout,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            dirty: DirtyFlags::ALL,
        }
    }

    pub fn with_interaction(mut self, interaction: InteractionState) -> Self {
        self.interaction = interaction;
        self
    }
}

#[derive(Debug, Clone)]
pub struct UiContext {
    pub theme: Theme,
    pub root_bounds: Rect,
}

impl UiContext {
    pub const fn new(theme: Theme, root_bounds: Rect) -> Self {
        Self { theme, root_bounds }
    }
}

#[derive(Debug, Clone)]
pub struct PaintContext {
    pub theme: Theme,
}

impl PaintContext {
    pub const fn new(theme: Theme) -> Self {
        Self { theme }
    }
}

#[derive(Default)]
pub struct UiTree {
    nodes: BTreeMap<WidgetId, UiNode>,
    root_id: Option<WidgetId>,
    ids: IdGenerator<WidgetMarker>,
    hovered_id: Option<WidgetId>,
    focused_id: Option<WidgetId>,
    active_id: Option<WidgetId>,
    pointer_position: Option<PointerPosition>,
    modifiers: ModifierState,
}

impl UiTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn root_id(&self) -> Option<WidgetId> {
        self.root_id
    }

    pub fn get(&self, id: WidgetId) -> Option<&UiNode> {
        self.nodes.get(&id)
    }

    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut UiNode> {
        self.nodes.get_mut(&id)
    }

    pub fn create_node(
        &mut self,
        kind: WidgetKind,
        style: UiStyle,
        layout: LayoutNode,
    ) -> WidgetId {
        let id = self.ids.next_id();
        self.nodes.insert(id, UiNode::new(id, kind, style, layout));
        id
    }

    pub fn set_root(&mut self, node: UiNode) -> Result<WidgetId, UiError> {
        let id = node.id;
        self.insert_node(node)?;
        self.root_id = Some(id);
        Ok(id)
    }

    pub fn insert_child(
        &mut self,
        parent_id: WidgetId,
        mut node: UiNode,
    ) -> Result<WidgetId, UiError> {
        let id = node.id;
        if !self.nodes.contains_key(&parent_id) {
            return Err(UiError::MissingNode(parent_id));
        }
        node.parent = Some(parent_id);
        self.insert_node(node)?;
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(id);
            parent.dirty.insert(DirtyFlags::LAYOUT);
            parent.dirty.insert(DirtyFlags::PAINT);
        }
        Ok(id)
    }

    pub fn attach_existing_child(
        &mut self,
        parent_id: WidgetId,
        child_id: WidgetId,
    ) -> Result<(), UiError> {
        if !self.nodes.contains_key(&parent_id) {
            return Err(UiError::MissingNode(parent_id));
        }
        let Some(child) = self.nodes.get_mut(&child_id) else {
            return Err(UiError::MissingNode(child_id));
        };
        child.parent = Some(parent_id);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(child_id);
            parent.dirty.insert(DirtyFlags::LAYOUT);
            parent.dirty.insert(DirtyFlags::PAINT);
        }
        Ok(())
    }

    pub fn layout(&mut self, constraints: LayoutConstraints) -> Result<LayoutResult, UiError> {
        let root_id = self.root_id.ok_or(UiError::MissingRoot)?;
        self.layout_subtree(root_id, constraints.bounds)?;
        self.clear_layout_dirty();
        Ok(LayoutResult {
            node_count: self.nodes.len(),
            bounds: self
                .nodes
                .iter()
                .map(|(id, node)| (*id, node.rect))
                .collect(),
        })
    }

    pub fn paint(&self, context: &PaintContext) -> Result<RenderScene, UiError> {
        let root_id = self.root_id.ok_or(UiError::MissingRoot)?;
        let mut scene = RenderScene::new();
        for id in self.traversal_from(root_id)? {
            let Some(node) = self.nodes.get(&id) else {
                return Err(UiError::MissingNode(id));
            };
            paint_node(node, context, &mut scene);
        }
        Ok(scene)
    }

    pub fn process_input(&mut self, input: UiInputEvent) -> Result<Vec<UiEvent>, UiError> {
        match input {
            UiInputEvent::PointerMoved(position) => {
                self.pointer_position = Some(position);
                let hit = self.hit_test(position).map(|result| result.id);
                self.update_hover(hit)
            }
            UiInputEvent::PointerEntered => Ok(Vec::new()),
            UiInputEvent::PointerLeft => {
                self.pointer_position = None;
                let mut events = self.update_hover(None)?;
                events.extend(self.set_active(None)?);
                Ok(events)
            }
            UiInputEvent::PointerButtonPressed(PointerButton::Primary) => {
                self.press_primary_button()
            }
            UiInputEvent::PointerButtonReleased(PointerButton::Primary) => {
                self.release_primary_button()
            }
            UiInputEvent::KeyPressed(key) => self.press_key(key),
            UiInputEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
                Ok(Vec::new())
            }
            UiInputEvent::WindowUnfocused => self.clear_focus_and_pointer_state(),
            UiInputEvent::PointerButtonPressed(_)
            | UiInputEvent::PointerButtonReleased(_)
            | UiInputEvent::KeyReleased(_)
            | UiInputEvent::MouseWheel { .. }
            | UiInputEvent::WindowFocused => Ok(Vec::new()),
        }
    }

    pub fn hit_test(&self, position: PointerPosition) -> Option<HitTestResult> {
        let root_id = self.root_id?;
        let traversal = self.traversal_from(root_id).ok()?;
        traversal.into_iter().rev().find_map(|id| {
            let node = self.nodes.get(&id)?;
            if node_can_receive_hit(node, position) {
                Some(HitTestResult { id })
            } else {
                None
            }
        })
    }

    pub fn traversal(&self) -> Result<Vec<WidgetId>, UiError> {
        let root_id = self.root_id.ok_or(UiError::MissingRoot)?;
        self.traversal_from(root_id)
    }

    pub fn set_label_text(&mut self, id: WidgetId, text: impl Into<String>) -> Result<(), UiError> {
        let Some(node) = self.nodes.get_mut(&id) else {
            return Err(UiError::MissingNode(id));
        };
        node.kind = WidgetKind::Label(text.into());
        node.dirty.insert(DirtyFlags::TEXT);
        node.dirty.insert(DirtyFlags::LAYOUT);
        node.dirty.insert(DirtyFlags::PAINT);
        self.mark_ancestors(id, DirtyFlags::LAYOUT);
        Ok(())
    }

    pub fn set_hovered(&mut self, id: WidgetId, hovered: bool) -> Result<(), UiError> {
        let Some(node) = self.nodes.get_mut(&id) else {
            return Err(UiError::MissingNode(id));
        };
        if node.interaction.hovered == hovered {
            return Ok(());
        }
        node.interaction.hovered = hovered;
        node.dirty.insert(DirtyFlags::PAINT);
        Ok(())
    }

    pub fn set_focused(&mut self, next: Option<WidgetId>) -> Result<Vec<UiEvent>, UiError> {
        if next == self.focused_id {
            return Ok(Vec::new());
        }
        if let Some(id) = next
            && !self
                .nodes
                .get(&id)
                .is_some_and(|node| node.interaction.can_focus())
        {
            return Ok(Vec::new());
        }
        let previous = self.focused_id;
        if let Some(id) = previous {
            self.set_node_focus(id, false)?;
        }
        if let Some(id) = next {
            self.set_node_focus(id, true)?;
        }
        self.focused_id = next;
        Ok(vec![UiEvent::FocusChanged(FocusChange { previous, next })])
    }

    pub fn set_disabled(&mut self, id: WidgetId, disabled: bool) -> Result<(), UiError> {
        let Some(node) = self.nodes.get_mut(&id) else {
            return Err(UiError::MissingNode(id));
        };
        if node.interaction.disabled == disabled {
            return Ok(());
        }
        node.interaction.disabled = disabled;
        node.dirty.insert(DirtyFlags::PAINT);
        node.dirty.insert(DirtyFlags::HIT_TEST);
        node.dirty.insert(DirtyFlags::ACCESSIBILITY);
        if disabled {
            if self.hovered_id == Some(id) {
                self.hovered_id = None;
                node.interaction.hovered = false;
            }
            if self.active_id == Some(id) {
                self.active_id = None;
                node.interaction.active = false;
            }
        }
        Ok(())
    }

    pub fn resize_root(&mut self, bounds: Rect) -> Result<(), UiError> {
        let root_id = self.root_id.ok_or(UiError::MissingRoot)?;
        let Some(root) = self.nodes.get_mut(&root_id) else {
            return Err(UiError::MissingNode(root_id));
        };
        root.rect = bounds;
        root.dirty.insert(DirtyFlags::LAYOUT);
        root.dirty.insert(DirtyFlags::PAINT);
        Ok(())
    }

    pub fn dirty_summary(&self) -> DirtySummary {
        let mut summary = DirtySummary::default();
        for node in self.nodes.values() {
            summary.record(node.dirty);
        }
        summary
    }

    fn insert_node(&mut self, node: UiNode) -> Result<(), UiError> {
        if self.nodes.contains_key(&node.id) {
            return Err(UiError::DuplicateNode(node.id));
        }
        self.nodes.insert(node.id, node);
        Ok(())
    }

    fn press_primary_button(&mut self) -> Result<Vec<UiEvent>, UiError> {
        let hit = self
            .pointer_position
            .and_then(|position| self.hit_test(position))
            .map(|result| result.id);
        let mut events = Vec::new();
        if let Some(id) = hit {
            events.extend(self.set_focused(Some(id))?);
            if is_clickable(self.nodes.get(&id)) {
                events.extend(self.set_active(Some(id))?);
            }
        } else {
            events.extend(self.set_focused(None)?);
        }
        Ok(events)
    }

    fn release_primary_button(&mut self) -> Result<Vec<UiEvent>, UiError> {
        let pressed = self.active_id;
        let release_hit = self
            .pointer_position
            .and_then(|position| self.hit_test(position))
            .map(|result| result.id);
        let mut events = self.set_active(None)?;
        if let Some(id) = pressed
            && release_hit == Some(id)
            && is_clickable(self.nodes.get(&id))
        {
            events.push(UiEvent::Clicked { id });
        }
        Ok(events)
    }

    fn press_key(&mut self, key: KeyboardKey) -> Result<Vec<UiEvent>, UiError> {
        match key {
            KeyboardKey::Enter | KeyboardKey::Space => {
                if let Some(id) = self.focused_id
                    && is_clickable(self.nodes.get(&id))
                {
                    return Ok(vec![UiEvent::Clicked { id }]);
                }
                Ok(Vec::new())
            }
            KeyboardKey::Tab => self.focus_next(),
            _ => Ok(Vec::new()),
        }
    }

    fn focus_next(&mut self) -> Result<Vec<UiEvent>, UiError> {
        let focusable = self.focusable_widgets()?;
        if focusable.is_empty() {
            return self.set_focused(None);
        }
        let current_index = self
            .focused_id
            .and_then(|id| focusable.iter().position(|candidate| *candidate == id));
        let next_index = current_index.map_or(0, |index| (index + 1) % focusable.len());
        self.set_focused(Some(focusable[next_index]))
    }

    fn focusable_widgets(&self) -> Result<Vec<WidgetId>, UiError> {
        let root_id = self.root_id.ok_or(UiError::MissingRoot)?;
        Ok(self
            .traversal_from(root_id)?
            .into_iter()
            .filter(|id| {
                self.nodes
                    .get(id)
                    .is_some_and(|node| node.interaction.can_focus())
            })
            .collect())
    }

    fn clear_focus_and_pointer_state(&mut self) -> Result<Vec<UiEvent>, UiError> {
        let mut events = self.update_hover(None)?;
        events.extend(self.set_active(None)?);
        events.extend(self.set_focused(None)?);
        Ok(events)
    }

    fn update_hover(&mut self, next: Option<WidgetId>) -> Result<Vec<UiEvent>, UiError> {
        if next == self.hovered_id {
            return Ok(Vec::new());
        }
        let previous = self.hovered_id;
        if let Some(id) = previous {
            self.set_hovered(id, false)?;
        }
        if let Some(id) = next {
            self.set_hovered(id, true)?;
        }
        self.hovered_id = next;
        let mut events = Vec::new();
        if let Some(id) = previous {
            events.push(UiEvent::HoverChanged { id, hovered: false });
        }
        if let Some(id) = next {
            events.push(UiEvent::HoverChanged { id, hovered: true });
        }
        Ok(events)
    }

    fn set_active(&mut self, next: Option<WidgetId>) -> Result<Vec<UiEvent>, UiError> {
        if next == self.active_id {
            return Ok(Vec::new());
        }
        let previous = self.active_id;
        if let Some(id) = previous {
            self.set_node_active(id, false)?;
        }
        if let Some(id) = next {
            self.set_node_active(id, true)?;
        }
        self.active_id = next;
        let mut events = Vec::new();
        if let Some(id) = previous {
            events.push(UiEvent::ActiveChanged { id, active: false });
        }
        if let Some(id) = next {
            events.push(UiEvent::ActiveChanged { id, active: true });
        }
        Ok(events)
    }

    fn set_node_focus(&mut self, id: WidgetId, focused: bool) -> Result<(), UiError> {
        let Some(node) = self.nodes.get_mut(&id) else {
            return Err(UiError::MissingNode(id));
        };
        node.interaction.focused = focused;
        node.dirty.insert(DirtyFlags::PAINT);
        node.dirty.insert(DirtyFlags::ACCESSIBILITY);
        Ok(())
    }

    fn set_node_active(&mut self, id: WidgetId, active: bool) -> Result<(), UiError> {
        let Some(node) = self.nodes.get_mut(&id) else {
            return Err(UiError::MissingNode(id));
        };
        node.interaction.active = active;
        node.dirty.insert(DirtyFlags::PAINT);
        Ok(())
    }

    fn layout_subtree(&mut self, id: WidgetId, rect: Rect) -> Result<(), UiError> {
        let (children, layout) = {
            let Some(node) = self.nodes.get_mut(&id) else {
                return Err(UiError::MissingNode(id));
            };
            node.rect = rect.normalized();
            (node.children.clone(), node.layout)
        };
        let child_rects = layout_children(layout, rect, &children, &self.nodes)?;
        for (child_id, child_rect) in child_rects {
            self.layout_subtree(child_id, child_rect)?;
        }
        Ok(())
    }

    fn clear_layout_dirty(&mut self) {
        for node in self.nodes.values_mut() {
            if node.dirty.contains(DirtyFlags::LAYOUT) {
                node.dirty.clear();
            }
        }
    }

    fn traversal_from(&self, root_id: WidgetId) -> Result<Vec<WidgetId>, UiError> {
        let mut ids = Vec::new();
        self.collect_traversal(root_id, &mut ids)?;
        Ok(ids)
    }

    fn collect_traversal(&self, id: WidgetId, ids: &mut Vec<WidgetId>) -> Result<(), UiError> {
        let Some(node) = self.nodes.get(&id) else {
            return Err(UiError::MissingNode(id));
        };
        ids.push(id);
        for child in &node.children {
            self.collect_traversal(*child, ids)?;
        }
        Ok(())
    }

    fn mark_ancestors(&mut self, id: WidgetId, flags: DirtyFlags) {
        let mut next = self.nodes.get(&id).and_then(|node| node.parent);
        while let Some(parent_id) = next {
            next = self.nodes.get(&parent_id).and_then(|node| node.parent);
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                parent.dirty.insert(flags);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtySummary {
    pub layout: usize,
    pub paint: usize,
    pub text: usize,
    pub hit_test: usize,
    pub accessibility: usize,
}

impl DirtySummary {
    fn record(&mut self, dirty: DirtyFlags) {
        if dirty.contains(DirtyFlags::LAYOUT) {
            self.layout += 1;
        }
        if dirty.contains(DirtyFlags::PAINT) {
            self.paint += 1;
        }
        if dirty.contains(DirtyFlags::TEXT) {
            self.text += 1;
        }
        if dirty.contains(DirtyFlags::HIT_TEST) {
            self.hit_test += 1;
        }
        if dirty.contains(DirtyFlags::ACCESSIBILITY) {
            self.accessibility += 1;
        }
    }
}

pub struct EditorShell {
    pub tree: UiTree,
    pub ids: EditorShellIds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorShellIds {
    pub run_button: WidgetId,
    pub status_label: WidgetId,
}

pub fn build_editor_shell(context: &UiContext) -> Result<UiTree, UiError> {
    build_editor_shell_with_ids(context).map(|shell| shell.tree)
}

pub fn build_editor_shell_with_ids(context: &UiContext) -> Result<EditorShell, UiError> {
    let mut tree = UiTree::new();
    let root = WidgetId::new(1).ok_or(UiError::MissingRoot)?;
    let toolbar = WidgetId::new(2).ok_or(UiError::MissingRoot)?;
    let top_separator = WidgetId::new(3).ok_or(UiError::MissingRoot)?;
    let body = WidgetId::new(4).ok_or(UiError::MissingRoot)?;
    let bottom_separator = WidgetId::new(5).ok_or(UiError::MissingRoot)?;
    let status = WidgetId::new(6).ok_or(UiError::MissingRoot)?;
    let project = WidgetId::new(7).ok_or(UiError::MissingRoot)?;
    let left_separator = WidgetId::new(8).ok_or(UiError::MissingRoot)?;
    let viewport = WidgetId::new(9).ok_or(UiError::MissingRoot)?;
    let right_separator = WidgetId::new(10).ok_or(UiError::MissingRoot)?;
    let inspector = WidgetId::new(11).ok_or(UiError::MissingRoot)?;
    let title = WidgetId::new(12).ok_or(UiError::MissingRoot)?;
    let project_label = WidgetId::new(13).ok_or(UiError::MissingRoot)?;
    let viewport_label = WidgetId::new(14).ok_or(UiError::MissingRoot)?;
    let inspector_label = WidgetId::new(15).ok_or(UiError::MissingRoot)?;
    let status_label = WidgetId::new(16).ok_or(UiError::MissingRoot)?;
    let run_button = WidgetId::new(17).ok_or(UiError::MissingRoot)?;

    tree.set_root(UiNode::new(
        root,
        WidgetKind::Root,
        UiStyle::ROOT,
        LayoutNode::fill(LayoutMode::Stack(Axis::Vertical)),
    ))?;
    tree.insert_child(
        root,
        UiNode::new(
            toolbar,
            WidgetKind::Toolbar,
            UiStyle::TOOLBAR,
            LayoutNode::fill(LayoutMode::Stack(Axis::Horizontal))
                .with_height(SizePolicy::Fixed(56.0))
                .with_padding(Insets::symmetric(context.theme.spacing.lg, 0.0)),
        ),
    )?;
    tree.insert_child(
        root,
        UiNode::new(
            top_separator,
            WidgetKind::Separator(Axis::Horizontal),
            UiStyle::SEPARATOR,
            LayoutNode::fixed(0.0, 1.0),
        ),
    )?;
    tree.insert_child(
        root,
        UiNode::new(
            body,
            WidgetKind::Panel,
            UiStyle::new(StyleRole::Transparent),
            LayoutNode::fill(LayoutMode::Split(Axis::Horizontal)),
        ),
    )?;
    tree.insert_child(
        root,
        UiNode::new(
            bottom_separator,
            WidgetKind::Separator(Axis::Horizontal),
            UiStyle::SEPARATOR,
            LayoutNode::fixed(0.0, 1.0),
        ),
    )?;
    tree.insert_child(
        root,
        UiNode::new(
            status,
            WidgetKind::StatusBar,
            UiStyle::STATUS_BAR,
            LayoutNode::fill(LayoutMode::Stack(Axis::Horizontal))
                .with_height(SizePolicy::Fixed(28.0))
                .with_padding(Insets::symmetric(context.theme.spacing.lg, 0.0)),
        ),
    )?;
    tree.insert_child(
        toolbar,
        UiNode::new(
            title,
            WidgetKind::Label("Elcarax".to_string()),
            UiStyle::LABEL,
            LayoutNode::leaf().with_width(SizePolicy::Content),
        ),
    )?;
    tree.insert_child(
        toolbar,
        UiNode::new(
            run_button,
            WidgetKind::Button("Run".to_string()),
            UiStyle::BUTTON,
            LayoutNode::leaf().with_width(SizePolicy::Content),
        ),
    )?;
    tree.insert_child(
        body,
        UiNode::new(
            project,
            WidgetKind::Panel,
            UiStyle::PANEL,
            LayoutNode::fill(LayoutMode::Stack(Axis::Vertical))
                .with_width(SizePolicy::Fixed(248.0))
                .with_padding(Insets::uniform(context.theme.spacing.md)),
        ),
    )?;
    tree.insert_child(
        body,
        UiNode::new(
            left_separator,
            WidgetKind::Separator(Axis::Vertical),
            UiStyle::SEPARATOR,
            LayoutNode::fixed(1.0, 0.0),
        ),
    )?;
    tree.insert_child(
        body,
        UiNode::new(
            viewport,
            WidgetKind::ViewportPlaceholder,
            UiStyle::VIEWPORT,
            LayoutNode::fill(LayoutMode::Stack(Axis::Vertical))
                .with_width(SizePolicy::Fill)
                .with_padding(Insets::uniform(context.theme.spacing.lg)),
        ),
    )?;
    tree.insert_child(
        body,
        UiNode::new(
            right_separator,
            WidgetKind::Separator(Axis::Vertical),
            UiStyle::SEPARATOR,
            LayoutNode::fixed(1.0, 0.0),
        ),
    )?;
    tree.insert_child(
        body,
        UiNode::new(
            inspector,
            WidgetKind::Panel,
            UiStyle::PANEL,
            LayoutNode::fill(LayoutMode::Stack(Axis::Vertical))
                .with_width(SizePolicy::Fixed(292.0))
                .with_padding(Insets::uniform(context.theme.spacing.md)),
        ),
    )?;
    tree.insert_child(
        project,
        UiNode::new(
            project_label,
            WidgetKind::Label("Project".to_string()),
            UiStyle::LABEL,
            LayoutNode::leaf(),
        ),
    )?;
    tree.insert_child(
        viewport,
        UiNode::new(
            viewport_label,
            WidgetKind::Label("Viewport".to_string()),
            UiStyle::LABEL,
            LayoutNode::leaf(),
        ),
    )?;
    tree.insert_child(
        inspector,
        UiNode::new(
            inspector_label,
            WidgetKind::Label("Inspector".to_string()),
            UiStyle::LABEL,
            LayoutNode::leaf(),
        ),
    )?;
    tree.insert_child(
        status,
        UiNode::new(
            status_label,
            WidgetKind::Label("Status: Renderer online".to_string()),
            UiStyle::LABEL.muted_text(),
            LayoutNode::leaf(),
        ),
    )?;
    tree.layout(LayoutConstraints {
        bounds: context.root_bounds,
    })?;
    Ok(EditorShell {
        tree,
        ids: EditorShellIds {
            run_button,
            status_label,
        },
    })
}

fn layout_children(
    layout: LayoutNode,
    rect: Rect,
    children: &[WidgetId],
    nodes: &BTreeMap<WidgetId, UiNode>,
) -> Result<Vec<(WidgetId, Rect)>, UiError> {
    if children.is_empty() {
        return Ok(Vec::new());
    }
    let content = layout.padding.shrink(rect);
    match layout.mode {
        LayoutMode::Leaf => Ok(children
            .iter()
            .map(|id| (*id, child_rect(*id, content, nodes)))
            .collect()),
        LayoutMode::Stack(axis) | LayoutMode::Split(axis) => {
            stack_children(axis, content, children, nodes)
        }
    }
}

fn stack_children(
    axis: Axis,
    rect: Rect,
    children: &[WidgetId],
    nodes: &BTreeMap<WidgetId, UiNode>,
) -> Result<Vec<(WidgetId, Rect)>, UiError> {
    let fixed = fixed_span(axis, children, nodes)?;
    let fill_count = children
        .iter()
        .filter(|id| {
            primary_policy(axis, nodes.get(id)).is_some_and(|policy| policy == SizePolicy::Fill)
        })
        .count();
    let available = primary_size(axis, rect);
    let fill_span = if fill_count == 0 {
        0.0
    } else {
        (available - fixed).max(0.0) / fill_count as f32
    };
    Ok(place_children(axis, rect, children, nodes, fill_span))
}

fn fixed_span(
    axis: Axis,
    children: &[WidgetId],
    nodes: &BTreeMap<WidgetId, UiNode>,
) -> Result<f32, UiError> {
    let mut span = 0.0;
    for id in children {
        let Some(node) = nodes.get(id) else {
            return Err(UiError::MissingNode(*id));
        };
        match main_policy(axis, node.layout) {
            SizePolicy::Fixed(value) => span += value,
            SizePolicy::Content => span += content_span(axis, &node.kind),
            SizePolicy::Fill => {}
        }
    }
    Ok(span)
}

fn place_children(
    axis: Axis,
    rect: Rect,
    children: &[WidgetId],
    nodes: &BTreeMap<WidgetId, UiNode>,
    fill_span: f32,
) -> Vec<(WidgetId, Rect)> {
    let mut cursor = match axis {
        Axis::Horizontal => rect.x,
        Axis::Vertical => rect.y,
    };
    children
        .iter()
        .filter_map(|id| {
            let node = nodes.get(id)?;
            let span = resolved_span(axis, node, fill_span);
            let child = match axis {
                Axis::Horizontal => Rect::new(cursor, rect.y, span, cross_size(axis, rect)),
                Axis::Vertical => Rect::new(rect.x, cursor, cross_size(axis, rect), span),
            };
            cursor += span;
            Some((*id, child))
        })
        .collect()
}

fn child_rect(id: WidgetId, rect: Rect, nodes: &BTreeMap<WidgetId, UiNode>) -> Rect {
    let Some(node) = nodes.get(&id) else {
        return rect;
    };
    let width = match node.layout.width {
        SizePolicy::Fixed(value) => value,
        SizePolicy::Content => content_span(Axis::Horizontal, &node.kind),
        SizePolicy::Fill => rect.width,
    };
    let height = match node.layout.height {
        SizePolicy::Fixed(value) => value,
        SizePolicy::Content => content_span(Axis::Vertical, &node.kind),
        SizePolicy::Fill => rect.height,
    };
    Rect::new(
        rect.x,
        rect.y,
        width.min(rect.width),
        height.min(rect.height),
    )
}

fn resolved_span(axis: Axis, node: &UiNode, fill_span: f32) -> f32 {
    match main_policy(axis, node.layout) {
        SizePolicy::Fixed(value) => value,
        SizePolicy::Fill => fill_span,
        SizePolicy::Content => content_span(axis, &node.kind),
    }
}

fn primary_policy(axis: Axis, node: Option<&UiNode>) -> Option<SizePolicy> {
    node.map(|node| main_policy(axis, node.layout))
}

fn main_policy(axis: Axis, layout: LayoutNode) -> SizePolicy {
    match axis {
        Axis::Horizontal => layout.width,
        Axis::Vertical => layout.height,
    }
}

fn primary_size(axis: Axis, rect: Rect) -> f32 {
    match axis {
        Axis::Horizontal => rect.width,
        Axis::Vertical => rect.height,
    }
}

fn cross_size(axis: Axis, rect: Rect) -> f32 {
    match axis {
        Axis::Horizontal => rect.height,
        Axis::Vertical => rect.width,
    }
}

fn content_span(axis: Axis, kind: &WidgetKind) -> f32 {
    match axis {
        Axis::Horizontal => {
            let text_width =
                text_content(kind).map_or(1.0, |text| text.chars().count() as f32 * 8.0);
            match kind {
                WidgetKind::Button(_) => text_width + 28.0,
                WidgetKind::IconButton(_) => 32.0,
                _ => text_width,
            }
        }
        Axis::Vertical => match kind {
            WidgetKind::Button(_) | WidgetKind::IconButton(_) => 32.0,
            _ => 22.0,
        },
    }
}

fn text_content(kind: &WidgetKind) -> Option<&str> {
    match kind {
        WidgetKind::Label(text) => Some(text.as_str()),
        WidgetKind::Button(text) => Some(text.as_str()),
        WidgetKind::IconButton(text) => Some(text.as_str()),
        _ => None,
    }
}

fn default_interaction_for(kind: &WidgetKind) -> InteractionState {
    match kind {
        WidgetKind::Button(_) | WidgetKind::IconButton(_) => InteractionState::control(),
        WidgetKind::Label(_) | WidgetKind::Separator(_) => InteractionState::passive(),
        WidgetKind::Root
        | WidgetKind::Panel
        | WidgetKind::StatusBar
        | WidgetKind::Toolbar
        | WidgetKind::ViewportPlaceholder => InteractionState::container(),
    }
}

fn node_can_receive_hit(node: &UiNode, position: PointerPosition) -> bool {
    node.interaction.can_hit_test() && rect_contains(node.rect, position)
}

fn is_clickable(node: Option<&UiNode>) -> bool {
    node.is_some_and(|node| {
        matches!(node.kind, WidgetKind::Button(_) | WidgetKind::IconButton(_))
            && node.interaction.can_hit_test()
    })
}

fn rect_contains(rect: Rect, position: PointerPosition) -> bool {
    let rect = rect.normalized();
    position.x >= rect.x
        && position.y >= rect.y
        && position.x < rect.x + rect.width
        && position.y < rect.y + rect.height
}

fn paint_node(node: &UiNode, context: &PaintContext, scene: &mut RenderScene) {
    match &node.kind {
        WidgetKind::Root | WidgetKind::Panel | WidgetKind::Toolbar | WidgetKind::StatusBar => {
            paint_background(node, context, scene);
        }
        WidgetKind::ViewportPlaceholder => {
            paint_background(node, context, scene);
            scene.push(
                RenderLayer::Overlay,
                RenderPrimitive::border_rect(node.rect, Border::new(2.0, context.theme.accent))
                    .with_debug_label("viewport border"),
            );
        }
        WidgetKind::Separator(axis) => paint_separator(*axis, node, context, scene),
        WidgetKind::Label(text) => paint_label(text, node, context, scene),
        WidgetKind::Button(text) | WidgetKind::IconButton(text) => {
            paint_button(text, node, context, scene);
        }
    }
}

fn paint_background(node: &UiNode, context: &PaintContext, scene: &mut RenderScene) {
    let Some(color) = context.theme.color_for(node.style) else {
        return;
    };
    let primitive = if node.style.corner_radius > 0.0 {
        RenderPrimitive::rounded_rect(
            node.rect,
            CornerRadius::uniform(node.style.corner_radius),
            color,
        )
    } else {
        RenderPrimitive::solid_rect(node.rect, color)
    };
    scene.push(
        RenderLayer::Chrome,
        primitive.with_debug_label(format!("{:?}", node.kind)),
    );
}

fn paint_separator(axis: Axis, node: &UiNode, context: &PaintContext, scene: &mut RenderScene) {
    let (from, to) = match axis {
        Axis::Horizontal => (
            [node.rect.x, node.rect.y],
            [node.rect.x + node.rect.width, node.rect.y],
        ),
        Axis::Vertical => (
            [node.rect.x, node.rect.y],
            [node.rect.x, node.rect.y + node.rect.height],
        ),
    };
    scene.push(
        RenderLayer::Overlay,
        RenderPrimitive::line(from, to, 1.0, context.theme.border).with_debug_label("separator"),
    );
}

fn paint_label(text: &str, node: &UiNode, context: &PaintContext, scene: &mut RenderScene) {
    let font_size = if node.rect.height > context.theme.fonts.body + 6.0 {
        context.theme.fonts.body
    } else {
        context.theme.fonts.small
    };
    scene.push(
        RenderLayer::Overlay,
        RenderPrimitive::text(
            text,
            node.rect.x,
            node.rect.y + font_size,
            font_size,
            context.theme.text_color_for(node.style.text_role),
        )
        .with_debug_label("label"),
    );
}

fn paint_button(text: &str, node: &UiNode, context: &PaintContext, scene: &mut RenderScene) {
    let color = if node.interaction.disabled {
        context.theme.surface
    } else if node.interaction.active {
        context.theme.control_active
    } else if node.interaction.hovered {
        context.theme.control_hovered
    } else {
        context.theme.control
    };
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::rounded_rect(
            node.rect,
            CornerRadius::uniform(node.style.corner_radius),
            color,
        )
        .with_debug_label("button"),
    );
    if node.interaction.focused {
        scene.push(
            RenderLayer::Overlay,
            RenderPrimitive::border_rect(node.rect, Border::new(1.0, context.theme.focus_ring))
                .with_debug_label("button focus"),
        );
    }
    let font_size = context.theme.fonts.body;
    scene.push(
        RenderLayer::Overlay,
        RenderPrimitive::text(
            text,
            node.rect.x + 14.0,
            node.rect.y + 20.0,
            font_size,
            context.theme.text,
        )
        .with_debug_label("button label"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_render::RenderPrimitiveKind;

    fn id(value: u64) -> WidgetId {
        match WidgetId::new(value) {
            Some(id) => id,
            None => panic!("test widget ids must be non-zero"),
        }
    }

    fn must<T, E: std::fmt::Debug>(result: std::result::Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("expected Ok result, got {error:?}"),
        }
    }

    fn must_some<T>(value: Option<T>) -> T {
        match value {
            Some(value) => value,
            None => panic!("expected Some value"),
        }
    }

    fn root_tree(layout: LayoutNode) -> UiTree {
        let mut tree = UiTree::new();
        let result = tree.set_root(UiNode::new(id(1), WidgetKind::Root, UiStyle::ROOT, layout));
        assert_eq!(result, Ok(id(1)));
        tree
    }

    fn button_tree() -> UiTree {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Leaf));
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(2),
                    WidgetKind::Button("Run".to_string()),
                    UiStyle::BUTTON,
                    LayoutNode::fixed(80.0, 32.0)
                )
            ),
            Ok(id(2))
        );
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 200.0, 100.0),
            })
            .is_ok()
        );
        tree
    }

    #[test]
    fn empty_ui_tree_is_valid() {
        let tree = UiTree::new();
        assert_eq!(tree.node_count(), 0);
        assert_eq!(tree.root_id(), None);
    }

    #[test]
    fn root_layout_fills_given_bounds() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Leaf));
        let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        let result = tree.layout(LayoutConstraints { bounds });
        assert!(result.is_ok());
        assert_eq!(tree.get(id(1)).map(|node| node.rect), Some(bounds));
    }

    #[test]
    fn horizontal_stack_distributes_fixed_and_fill_children() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Stack(Axis::Horizontal)));
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(2),
                    WidgetKind::Panel,
                    UiStyle::PANEL,
                    LayoutNode::fixed(20.0, 0.0)
                )
            ),
            Ok(id(2))
        );
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(3),
                    WidgetKind::Panel,
                    UiStyle::PANEL,
                    LayoutNode::fill(LayoutMode::Leaf)
                )
            ),
            Ok(id(3))
        );
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 100.0, 40.0),
            })
            .is_ok()
        );
        assert_eq!(tree.get(id(2)).map(|node| node.rect.width), Some(20.0));
        assert_eq!(tree.get(id(3)).map(|node| node.rect.width), Some(80.0));
    }

    #[test]
    fn vertical_stack_distributes_fixed_and_fill_children() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Stack(Axis::Vertical)));
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(2),
                    WidgetKind::Toolbar,
                    UiStyle::TOOLBAR,
                    LayoutNode::fill(LayoutMode::Leaf).with_height(SizePolicy::Fixed(12.0))
                )
            ),
            Ok(id(2))
        );
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(3),
                    WidgetKind::Panel,
                    UiStyle::PANEL,
                    LayoutNode::fill(LayoutMode::Leaf)
                )
            ),
            Ok(id(3))
        );
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 80.0, 50.0),
            })
            .is_ok()
        );
        assert_eq!(tree.get(id(2)).map(|node| node.rect.height), Some(12.0));
        assert_eq!(tree.get(id(3)).map(|node| node.rect.height), Some(38.0));
    }

    #[test]
    fn panel_paints_background_primitive() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Leaf));
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 10.0, 10.0),
            })
            .is_ok()
        );
        let scene = tree.paint(&PaintContext::new(Theme::default()));
        assert!(matches!(
            scene.map(|scene| scene.primitives()[0].1.kind.clone()),
            Ok(RenderPrimitiveKind::SolidRect { .. })
        ));
    }

    #[test]
    fn label_paints_text_primitive() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Stack(Axis::Vertical)));
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(2),
                    WidgetKind::Label("Project".to_string()),
                    UiStyle::LABEL,
                    LayoutNode::leaf()
                )
            ),
            Ok(id(2))
        );
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 80.0, 40.0),
            })
            .is_ok()
        );
        let scene = tree.paint(&PaintContext::new(Theme::default()));
        assert!(matches!(
            scene.map(|scene| scene.primitives()[1].1.kind.clone()),
            Ok(RenderPrimitiveKind::Text(_))
        ));
    }

    #[test]
    fn separator_paints_line_primitive() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Stack(Axis::Vertical)));
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(2),
                    WidgetKind::Separator(Axis::Horizontal),
                    UiStyle::SEPARATOR,
                    LayoutNode::leaf()
                )
            ),
            Ok(id(2))
        );
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 80.0, 40.0),
            })
            .is_ok()
        );
        let scene = tree.paint(&PaintContext::new(Theme::default()));
        assert!(matches!(
            scene.map(|scene| scene.primitives()[1].1.kind.clone()),
            Ok(RenderPrimitiveKind::Line { .. })
        ));
    }

    #[test]
    fn dirty_flags_propagate_for_text_hover_and_resize() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Stack(Axis::Vertical)));
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(2),
                    WidgetKind::Label("Old".to_string()),
                    UiStyle::LABEL,
                    LayoutNode::leaf()
                )
            ),
            Ok(id(2))
        );
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 80.0, 40.0),
            })
            .is_ok()
        );
        assert!(tree.set_label_text(id(2), "New").is_ok());
        assert!(
            tree.get(id(2))
                .is_some_and(|node| node.dirty.contains(DirtyFlags::TEXT))
        );
        assert!(
            tree.get(id(1))
                .is_some_and(|node| node.dirty.contains(DirtyFlags::LAYOUT))
        );
        assert!(tree.set_hovered(id(2), true).is_ok());
        assert!(
            tree.get(id(2))
                .is_some_and(|node| node.dirty.contains(DirtyFlags::PAINT))
        );
        assert!(tree.resize_root(Rect::new(0.0, 0.0, 100.0, 50.0)).is_ok());
        assert!(
            tree.get(id(1))
                .is_some_and(|node| node.dirty.contains(DirtyFlags::PAINT))
        );
    }

    #[test]
    fn theme_tokens_resolve_correctly() {
        let theme = Theme::editor_dark();
        assert_eq!(theme.color_for(UiStyle::PANEL), Some(theme.surface));
        assert_eq!(theme.text_color_for(TextRole::Muted), theme.text_muted);
    }

    #[test]
    fn traversal_order_is_stable() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Stack(Axis::Vertical)));
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(2),
                    WidgetKind::Toolbar,
                    UiStyle::TOOLBAR,
                    LayoutNode::leaf()
                )
            ),
            Ok(id(2))
        );
        assert_eq!(
            tree.insert_child(
                id(1),
                UiNode::new(
                    id(3),
                    WidgetKind::StatusBar,
                    UiStyle::STATUS_BAR,
                    LayoutNode::leaf()
                )
            ),
            Ok(id(3))
        );
        assert_eq!(tree.traversal(), Ok(vec![id(1), id(2), id(3)]));
    }

    #[test]
    fn hit_testing_empty_tree_returns_none() {
        let tree = UiTree::new();
        assert_eq!(tree.hit_test(PointerPosition::new(1.0, 1.0)), None);
    }

    #[test]
    fn hit_testing_root_can_return_container() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Leaf));
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 100.0, 50.0),
            })
            .is_ok()
        );
        assert_eq!(
            tree.hit_test(PointerPosition::new(10.0, 10.0)),
            Some(HitTestResult { id: id(1) })
        );
    }

    #[test]
    fn hit_testing_child_beats_parent() {
        let tree = button_tree();
        assert_eq!(
            tree.hit_test(PointerPosition::new(10.0, 10.0)),
            Some(HitTestResult { id: id(2) })
        );
    }

    #[test]
    fn hit_testing_outside_bounds_returns_none() {
        let tree = button_tree();
        assert_eq!(tree.hit_test(PointerPosition::new(250.0, 10.0)), None);
    }

    #[test]
    fn hit_testing_overlapping_children_prefers_later_child() {
        let mut tree = root_tree(LayoutNode::fill(LayoutMode::Leaf));
        for value in [2, 3] {
            assert_eq!(
                tree.insert_child(
                    id(1),
                    UiNode::new(
                        id(value),
                        WidgetKind::Button(format!("Button {value}")),
                        UiStyle::BUTTON,
                        LayoutNode::fixed(80.0, 32.0)
                    )
                ),
                Ok(id(value))
            );
        }
        assert!(
            tree.layout(LayoutConstraints {
                bounds: Rect::new(0.0, 0.0, 200.0, 100.0),
            })
            .is_ok()
        );
        assert_eq!(
            tree.hit_test(PointerPosition::new(10.0, 10.0)),
            Some(HitTestResult { id: id(3) })
        );
    }

    #[test]
    fn disabled_and_non_interactive_widgets_do_not_receive_hits() {
        let mut tree = button_tree();
        assert!(tree.set_disabled(id(2), true).is_ok());
        assert_eq!(
            tree.hit_test(PointerPosition::new(10.0, 10.0)),
            Some(HitTestResult { id: id(1) })
        );

        let mut label_tree = root_tree(LayoutNode::fill(LayoutMode::Leaf));
        assert_eq!(
            label_tree.insert_child(
                id(1),
                UiNode::new(
                    id(2),
                    WidgetKind::Label("Passive".to_string()),
                    UiStyle::LABEL,
                    LayoutNode::fixed(80.0, 32.0)
                )
            ),
            Ok(id(2))
        );
        assert!(
            label_tree
                .layout(LayoutConstraints {
                    bounds: Rect::new(0.0, 0.0, 200.0, 100.0),
                })
                .is_ok()
        );
        assert_eq!(
            label_tree.hit_test(PointerPosition::new(10.0, 10.0)),
            Some(HitTestResult { id: id(1) })
        );
    }

    #[test]
    fn hover_enter_and_exit_update_dirty_flags() {
        let mut tree = button_tree();
        let events =
            must(tree.process_input(UiInputEvent::PointerMoved(PointerPosition::new(10.0, 10.0))));
        assert_eq!(
            events,
            vec![UiEvent::HoverChanged {
                id: id(2),
                hovered: true
            }]
        );
        assert!(
            tree.get(id(2)).is_some_and(
                |node| node.interaction.hovered && node.dirty.contains(DirtyFlags::PAINT)
            )
        );
        let events = must(tree.process_input(UiInputEvent::PointerLeft));
        assert!(events.contains(&UiEvent::HoverChanged {
            id: id(2),
            hovered: false
        }));
    }

    #[test]
    fn focus_change_updates_dirty_flags() {
        let mut tree = button_tree();
        let events = must(tree.set_focused(Some(id(2))));
        assert_eq!(
            events,
            vec![UiEvent::FocusChanged(FocusChange {
                previous: None,
                next: Some(id(2))
            })]
        );
        assert!(tree.get(id(2)).is_some_and(|node| {
            node.interaction.focused
                && node.dirty.contains(DirtyFlags::PAINT)
                && node.dirty.contains(DirtyFlags::ACCESSIBILITY)
        }));
    }

    #[test]
    fn button_click_emits_event() {
        let mut tree = button_tree();
        assert!(
            tree.process_input(UiInputEvent::PointerMoved(PointerPosition::new(10.0, 10.0)))
                .is_ok()
        );
        assert!(
            tree.process_input(UiInputEvent::PointerButtonPressed(PointerButton::Primary))
                .is_ok()
        );
        let events =
            must(tree.process_input(UiInputEvent::PointerButtonReleased(PointerButton::Primary)));
        assert!(events.contains(&UiEvent::Clicked { id: id(2) }));
    }

    #[test]
    fn press_outside_release_inside_does_not_click() {
        let mut tree = button_tree();
        assert!(
            tree.process_input(UiInputEvent::PointerMoved(PointerPosition::new(
                180.0, 80.0
            )))
            .is_ok()
        );
        assert!(
            tree.process_input(UiInputEvent::PointerButtonPressed(PointerButton::Primary))
                .is_ok()
        );
        assert!(
            tree.process_input(UiInputEvent::PointerMoved(PointerPosition::new(10.0, 10.0)))
                .is_ok()
        );
        let events =
            must(tree.process_input(UiInputEvent::PointerButtonReleased(PointerButton::Primary)));
        assert!(!events.contains(&UiEvent::Clicked { id: id(2) }));
    }

    #[test]
    fn press_inside_release_outside_cancels_click() {
        let mut tree = button_tree();
        assert!(
            tree.process_input(UiInputEvent::PointerMoved(PointerPosition::new(10.0, 10.0)))
                .is_ok()
        );
        assert!(
            tree.process_input(UiInputEvent::PointerButtonPressed(PointerButton::Primary))
                .is_ok()
        );
        assert!(
            tree.process_input(UiInputEvent::PointerMoved(PointerPosition::new(
                250.0, 10.0
            )))
            .is_ok()
        );
        let events =
            must(tree.process_input(UiInputEvent::PointerButtonReleased(PointerButton::Primary)));
        assert!(!events.contains(&UiEvent::Clicked { id: id(2) }));
    }

    #[test]
    fn focused_button_activates_on_enter_and_space() {
        let mut tree = button_tree();
        assert!(tree.set_focused(Some(id(2))).is_ok());
        let enter_events = must(tree.process_input(UiInputEvent::KeyPressed(KeyboardKey::Enter)));
        let space_events = must(tree.process_input(UiInputEvent::KeyPressed(KeyboardKey::Space)));
        assert!(enter_events.contains(&UiEvent::Clicked { id: id(2) }));
        assert!(space_events.contains(&UiEvent::Clicked { id: id(2) }));
    }

    #[test]
    fn disabled_button_cannot_be_clicked() {
        let mut tree = button_tree();
        assert!(tree.set_disabled(id(2), true).is_ok());
        assert!(
            tree.process_input(UiInputEvent::PointerMoved(PointerPosition::new(10.0, 10.0)))
                .is_ok()
        );
        assert!(
            tree.process_input(UiInputEvent::PointerButtonPressed(PointerButton::Primary))
                .is_ok()
        );
        let events =
            must(tree.process_input(UiInputEvent::PointerButtonReleased(PointerButton::Primary)));
        assert!(!events.contains(&UiEvent::Clicked { id: id(2) }));
    }

    #[test]
    fn status_label_can_update_after_button_event() {
        let theme = Theme::editor_dark();
        let shell = must(build_editor_shell_with_ids(&UiContext::new(
            theme,
            Rect::new(0.0, 0.0, 1440.0, 900.0),
        )));
        let mut tree = shell.tree;
        let button = must_some(tree.get(shell.ids.run_button).map(|node| node.rect));
        for input in [
            UiInputEvent::PointerMoved(PointerPosition::new(button.x + 4.0, button.y + 4.0)),
            UiInputEvent::PointerButtonPressed(PointerButton::Primary),
            UiInputEvent::PointerButtonReleased(PointerButton::Primary),
        ] {
            let events = must(tree.process_input(input));
            if events.contains(&UiEvent::Clicked {
                id: shell.ids.run_button,
            }) {
                assert!(
                    tree.set_label_text(shell.ids.status_label, "Status: Run clicked")
                        .is_ok()
                );
            }
        }
        assert!(tree.get(shell.ids.status_label).is_some_and(|node| {
            matches!(&node.kind, WidgetKind::Label(text) if text == "Status: Run clicked")
        }));
    }
}
