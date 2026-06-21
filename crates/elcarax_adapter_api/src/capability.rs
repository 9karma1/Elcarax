#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterCapabilities {
    pub scene_tree: bool,
    pub property_editing: bool,
    pub viewport_preview: bool,
    pub diagnostics: bool,
    pub play_controls: bool,
}

impl AdapterCapabilities {
    pub const fn empty() -> Self {
        Self {
            scene_tree: false,
            property_editing: false,
            viewport_preview: false,
            diagnostics: false,
            play_controls: false,
        }
    }

    pub const fn game_editor_v0() -> Self {
        Self {
            scene_tree: true,
            property_editing: true,
            viewport_preview: true,
            diagnostics: true,
            play_controls: false,
        }
    }
}
