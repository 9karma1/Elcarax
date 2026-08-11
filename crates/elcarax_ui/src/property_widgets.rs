//! Typed inspector property widgets.

use elcarax_render::{
    Border, CornerRadius, FontFamily, FontWeight, Rect, RenderLayer, RenderPrimitive, TextStyle,
};

use crate::{
    InteractionState, PaintContext, PointerPosition, TextRole, Theme, TypeRole, UiNode, WidgetKind,
    paint_background, text_field::TextFieldState, text_field::paint_text_field,
};

pub const TOGGLE_FIELD_WIDTH: f32 = 44.0;
pub const NUMBER_STEPPER_WIDTH: f32 = 24.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ToggleFieldState {
    pub checked: bool,
}

impl ToggleFieldState {
    pub const fn new(checked: bool) -> Self {
        Self { checked }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberFieldState {
    pub text: String,
    pub step: f64,
    pub is_integer: bool,
}

impl NumberFieldState {
    pub fn new(text: impl Into<String>, step: f64, is_integer: bool) -> Self {
        Self {
            text: text.into(),
            step,
            is_integer,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorFieldState {
    pub components: [String; 3],
    pub count: u8,
    pub focused_component: Option<u8>,
}

impl VectorFieldState {
    pub fn new(components: [String; 3], count: u8) -> Self {
        Self {
            components,
            count,
            focused_component: None,
        }
    }

    pub fn merged_text(&self) -> String {
        match self.count {
            2 => format!("{}, {}", self.components[0], self.components[1]),
            3 => format!(
                "{}, {}, {}",
                self.components[0], self.components[1], self.components[2]
            ),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumFieldState {
    pub selected: String,
    pub variants: Vec<String>,
}

impl EnumFieldState {
    pub fn new(selected: impl Into<String>, variants: &[String]) -> Self {
        Self {
            selected: selected.into(),
            variants: variants.to_vec(),
        }
    }

    pub fn next_variant(&self) -> String {
        let slice = self.variants.as_slice();
        if slice.is_empty() {
            return self.selected.clone();
        }
        let index = slice
            .iter()
            .position(|value| value == &self.selected)
            .unwrap_or(0);
        slice[(index + 1) % slice.len()].clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyWidgetClick {
    None,
    Toggle,
    NumberDecrement,
    NumberIncrement,
    EnumCycle,
    VectorComponent(u8),
}

pub fn paint_toggle_field(
    state: &ToggleFieldState,
    node: &UiNode,
    context: &PaintContext,
    scene: &mut elcarax_render::RenderScene,
) {
    let theme = &context.theme;
    paint_background(node, context, scene);
    let track = Rect::new(
        node.rect.x + node.rect.width - TOGGLE_FIELD_WIDTH,
        node.rect.y + (node.rect.height - 18.0) * 0.5,
        TOGGLE_FIELD_WIDTH,
        18.0,
    );
    let fill = if state.checked {
        theme.accent
    } else {
        theme.surface_raised
    };
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::rounded_rect(track, CornerRadius::uniform(9.0), fill)
            .with_debug_label("toggle track"),
    );
    let knob_x = if state.checked {
        track.x + track.width - 16.0
    } else {
        track.x + 2.0
    };
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::rounded_rect(
            Rect::new(knob_x, track.y + 2.0, 14.0, 14.0),
            CornerRadius::uniform(7.0),
            theme.text,
        )
        .with_debug_label("toggle knob"),
    );
}

pub fn paint_number_field(
    state: &NumberFieldState,
    node: &UiNode,
    context: &PaintContext,
    scene: &mut elcarax_render::RenderScene,
) {
    let theme = &context.theme;
    paint_background(node, context, scene);
    let decrement = Rect::new(
        node.rect.x,
        node.rect.y,
        NUMBER_STEPPER_WIDTH,
        node.rect.height,
    );
    let increment = Rect::new(
        node.rect.x + node.rect.width - NUMBER_STEPPER_WIDTH,
        node.rect.y,
        NUMBER_STEPPER_WIDTH,
        node.rect.height,
    );
    paint_stepper_button("-", decrement, theme, scene);
    paint_stepper_button("+", increment, theme, scene);
    let value_rect = Rect::new(
        node.rect.x + NUMBER_STEPPER_WIDTH,
        node.rect.y,
        node.rect.width - NUMBER_STEPPER_WIDTH * 2.0,
        node.rect.height,
    );
    let text_field = TextFieldState::new(state.text.clone());
    let mut value_node = node.clone();
    value_node.rect = value_rect;
    paint_text_field(&text_field, &value_node, context, scene);
}

pub fn paint_vector_field(
    state: &VectorFieldState,
    node: &UiNode,
    context: &PaintContext,
    scene: &mut elcarax_render::RenderScene,
) {
    let theme = &context.theme;
    paint_background(node, context, scene);
    let count = state.count.max(1) as usize;
    let gap = theme.spacing.xs;
    let component_width = (node.rect.width - gap * (count.saturating_sub(1) as f32)) / count as f32;
    for index in 0..count {
        let x = node.rect.x + (component_width + gap) * index as f32;
        let rect = Rect::new(x, node.rect.y, component_width, node.rect.height);
        let label = match index {
            0 => "X",
            1 => "Y",
            _ => "Z",
        };
        scene.push(
            RenderLayer::Chrome,
            RenderPrimitive::text(
                label,
                rect.x + theme.spacing.xs,
                rect.y + rect.height * 0.5,
                theme.text_style_for(TextRole::Muted, TypeRole::Caption),
            )
            .with_debug_label("vector axis label"),
        );
        let value_rect = Rect::new(
            rect.x + 14.0,
            rect.y + 2.0,
            rect.width - 16.0,
            rect.height - 4.0,
        );
        let fill = if state.focused_component == Some(index as u8) {
            theme.surface_raised
        } else {
            theme.surface
        };
        scene.push(
            RenderLayer::Chrome,
            RenderPrimitive::rounded_rect(value_rect, CornerRadius::uniform(4.0), fill)
                .with_debug_label("vector component"),
        );
        scene.push(
            RenderLayer::Chrome,
            RenderPrimitive::border_rect(value_rect, Border::new(1.0, theme.border))
                .with_debug_label("vector component border"),
        );
        scene.push(
            RenderLayer::Chrome,
            RenderPrimitive::text(
                state.components[index].clone(),
                value_rect.x + theme.spacing.xs,
                value_rect.y + value_rect.height * 0.5,
                theme.text_style_for(TextRole::Accent, TypeRole::Body),
            )
            .with_debug_label("vector component value"),
        );
    }
}

pub fn paint_enum_field(
    state: &EnumFieldState,
    node: &UiNode,
    context: &PaintContext,
    scene: &mut elcarax_render::RenderScene,
) {
    let theme = &context.theme;
    paint_background(node, context, scene);
    let body = Rect::new(
        node.rect.x,
        node.rect.y + 2.0,
        node.rect.width - 20.0,
        node.rect.height - 4.0,
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::rounded_rect(body, CornerRadius::uniform(4.0), theme.surface_raised)
            .with_debug_label("enum field"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::border_rect(body, Border::new(1.0, theme.border))
            .with_debug_label("enum field border"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text(
            state.selected.clone(),
            body.x + theme.spacing.sm,
            body.y + body.height * 0.5,
            theme.text_style_for(TextRole::Accent, TypeRole::Body),
        )
        .with_debug_label("enum selected"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text(
            "v",
            node.rect.x + node.rect.width - 14.0,
            node.rect.y + node.rect.height * 0.5,
            theme.text_style_for(TextRole::Muted, TypeRole::Caption),
        )
        .with_debug_label("enum chevron"),
    );
}

pub fn property_widget_click(
    kind: &WidgetKind,
    rect: Rect,
    position: PointerPosition,
) -> PropertyWidgetClick {
    match kind {
        WidgetKind::Toggle(_) => PropertyWidgetClick::Toggle,
        WidgetKind::NumberField(_) => number_field_click(rect, position),
        WidgetKind::EnumField(_) => PropertyWidgetClick::EnumCycle,
        WidgetKind::VectorField(state) => vector_field_click(state, rect, position),
        _ => PropertyWidgetClick::None,
    }
}

pub fn step_number_field(state: &mut NumberFieldState, delta_steps: i32) -> bool {
    if delta_steps == 0 {
        return false;
    }
    let current = if state.is_integer {
        state.text.parse::<i64>().unwrap_or(0) as f64
    } else {
        state.text.parse::<f64>().unwrap_or(0.0)
    };
    let next = current + state.step * delta_steps as f64;
    state.text = if state.is_integer {
        next.round().to_string()
    } else {
        format!("{next:.2}")
    };
    true
}

pub fn vector_component_from_click(
    state: &VectorFieldState,
    rect: Rect,
    position: PointerPosition,
) -> Option<u8> {
    let count = state.count.max(1) as usize;
    let gap = 4.0;
    let component_width = (rect.width - gap * (count.saturating_sub(1) as f32)) / count as f32;
    for index in 0..count {
        let x = rect.x + (component_width + gap) * index as f32;
        let component_rect = Rect::new(x, rect.y, component_width, rect.height);
        if position.x >= component_rect.x
            && position.y >= component_rect.y
            && position.x < component_rect.x + component_rect.width
            && position.y < component_rect.y + component_rect.height
        {
            return Some(index as u8);
        }
    }
    None
}

pub fn interaction_for_property_widget(kind: &WidgetKind) -> InteractionState {
    match kind {
        WidgetKind::Toggle(_)
        | WidgetKind::NumberField(_)
        | WidgetKind::EnumField(_)
        | WidgetKind::VectorField(_) => InteractionState::control(),
        _ => InteractionState::passive(),
    }
}

pub fn is_property_widget(kind: &WidgetKind) -> bool {
    matches!(
        kind,
        WidgetKind::Toggle(_)
            | WidgetKind::NumberField(_)
            | WidgetKind::EnumField(_)
            | WidgetKind::VectorField(_)
    )
}

fn number_field_click(rect: Rect, position: PointerPosition) -> PropertyWidgetClick {
    if position.x < rect.x + NUMBER_STEPPER_WIDTH {
        PropertyWidgetClick::NumberDecrement
    } else if position.x >= rect.x + rect.width - NUMBER_STEPPER_WIDTH {
        PropertyWidgetClick::NumberIncrement
    } else {
        PropertyWidgetClick::None
    }
}

fn vector_field_click(
    state: &VectorFieldState,
    rect: Rect,
    position: PointerPosition,
) -> PropertyWidgetClick {
    vector_component_from_click(state, rect, position)
        .map(PropertyWidgetClick::VectorComponent)
        .unwrap_or(PropertyWidgetClick::None)
}

fn paint_stepper_button(
    label: &str,
    rect: Rect,
    theme: &Theme,
    scene: &mut elcarax_render::RenderScene,
) {
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::rounded_rect(rect, CornerRadius::uniform(4.0), theme.surface_raised)
            .with_debug_label("number stepper"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::border_rect(rect, Border::new(1.0, theme.border))
            .with_debug_label("number stepper border"),
    );
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::text(
            label,
            rect.x + rect.width * 0.5 - 4.0,
            rect.y + rect.height * 0.5,
            TextStyle::new(FontFamily::SansSerif, FontWeight::Regular, 14.0, theme.text),
        )
        .with_debug_label("number stepper label"),
    );
}
