use elcarax_render::{Border, Color, CornerRadius, RenderLayer, RenderPrimitive, RenderScene};

use crate::{KeyboardKey, PaintContext, UiNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFieldState {
    pub text: String,
    pub caret: usize,
}

impl TextFieldState {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let caret = text.chars().count();
        Self { text, caret }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFieldKeyAction {
    None,
    Changed,
    Committed,
    Cancelled,
}

pub fn handle_key(state: &mut TextFieldState, key: KeyboardKey) -> TextFieldKeyAction {
    match key {
        KeyboardKey::Character(value) => {
            let Some(character) = value.chars().next() else {
                return TextFieldKeyAction::None;
            };
            if character.is_control() {
                return TextFieldKeyAction::None;
            }
            insert_character(state, character);
            TextFieldKeyAction::Changed
        }
        KeyboardKey::Backspace => {
            delete_before_caret(state);
            TextFieldKeyAction::Changed
        }
        KeyboardKey::ArrowLeft => {
            move_caret(state, -1);
            TextFieldKeyAction::Changed
        }
        KeyboardKey::ArrowRight => {
            move_caret(state, 1);
            TextFieldKeyAction::Changed
        }
        KeyboardKey::Enter => TextFieldKeyAction::Committed,
        KeyboardKey::Escape => TextFieldKeyAction::Cancelled,
        _ => TextFieldKeyAction::None,
    }
}

pub fn paint_text_field(
    state: &TextFieldState,
    node: &UiNode,
    context: &PaintContext,
    scene: &mut RenderScene,
) {
    let background = if node.interaction.disabled {
        context.theme.control.disabled
    } else if node.interaction.focused {
        context.theme.control.active
    } else if node.interaction.hovered {
        context.theme.control.hovered
    } else {
        context.theme.control.default
    };
    scene.push(
        RenderLayer::Chrome,
        RenderPrimitive::rounded_rect(
            node.rect,
            CornerRadius::uniform(node.style.corner_radius),
            background,
        )
        .with_debug_label("text field"),
    );
    if node.interaction.focused {
        scene.push(
            RenderLayer::Chrome,
            RenderPrimitive::border_rect(node.rect, Border::new(1.0, context.theme.accent))
                .with_debug_label("text field focus"),
        );
    }
    let font_size = context.theme.fonts.small;
    let text_y = node.rect.y + font_size + 4.0;
    let text_x = node.rect.x + 8.0;
    scene.push(
        RenderLayer::Overlay,
        RenderPrimitive::text(
            state.text.clone(),
            text_x,
            text_y,
            font_size,
            context.theme.text_color_for(node.style.text_role),
        )
        .with_debug_label("text field value"),
    );
    if node.interaction.focused {
        paint_caret(
            state,
            text_x,
            text_y,
            font_size,
            context.theme.accent,
            scene,
        );
    }
}

fn paint_caret(
    state: &TextFieldState,
    text_x: f32,
    text_y: f32,
    font_size: f32,
    color: Color,
    scene: &mut RenderScene,
) {
    let prefix: String = state.text.chars().take(state.caret).collect();
    let caret_x = text_x + caret_offset(prefix.chars().count(), font_size);
    let top = text_y - font_size;
    let bottom = text_y + 2.0;
    scene.push(
        RenderLayer::Overlay,
        RenderPrimitive::line([caret_x, top], [caret_x, bottom], 1.0, color)
            .with_debug_label("text field caret"),
    );
}

fn caret_offset(char_count: usize, font_size: f32) -> f32 {
    char_count as f32 * font_size * 0.55
}

fn insert_character(state: &mut TextFieldState, character: char) {
    let mut chars: Vec<char> = state.text.chars().collect();
    let index = state.caret.min(chars.len());
    chars.insert(index, character);
    state.text = chars.into_iter().collect();
    state.caret = index.saturating_add(1);
}

fn delete_before_caret(state: &mut TextFieldState) {
    if state.caret == 0 {
        return;
    }
    let mut chars: Vec<char> = state.text.chars().collect();
    chars.remove(state.caret - 1);
    state.text = chars.into_iter().collect();
    state.caret -= 1;
}

fn move_caret(state: &mut TextFieldState, delta: isize) {
    let len = state.text.chars().count() as isize;
    let next = (state.caret as isize + delta).clamp(0, len);
    state.caret = next as usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_and_deletes_characters_at_caret() {
        let mut state = TextFieldState::new("ab");
        state.caret = 1;
        assert_eq!(
            handle_key(&mut state, KeyboardKey::Character("c".to_string())),
            TextFieldKeyAction::Changed
        );
        assert_eq!(state.text, "acb");
        assert_eq!(state.caret, 2);
        assert_eq!(
            handle_key(&mut state, KeyboardKey::Backspace),
            TextFieldKeyAction::Changed
        );
        assert_eq!(state.text, "ab");
        assert_eq!(state.caret, 1);
    }

    #[test]
    fn enter_commits_and_escape_cancels() {
        let mut state = TextFieldState::new("42");
        assert_eq!(
            handle_key(&mut state, KeyboardKey::Enter),
            TextFieldKeyAction::Committed
        );
        assert_eq!(
            handle_key(&mut state, KeyboardKey::Escape),
            TextFieldKeyAction::Cancelled
        );
    }
}
