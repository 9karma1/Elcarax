use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    pub provides_project_info: bool,
    pub provides_scene_snapshot: bool,
    pub provides_diagnostics: bool,
    pub supports_property_writeback: bool,
    pub supports_viewport_preview: bool,
}

impl AdapterCapabilities {
    pub const fn empty() -> Self {
        Self {
            provides_project_info: false,
            provides_scene_snapshot: false,
            provides_diagnostics: false,
            supports_property_writeback: false,
            supports_viewport_preview: false,
        }
    }

    pub const fn mock_milestone_12() -> Self {
        Self {
            provides_project_info: true,
            provides_scene_snapshot: true,
            provides_diagnostics: true,
            supports_property_writeback: false,
            supports_viewport_preview: false,
        }
    }

    pub const fn game_editor_v0() -> Self {
        Self::mock_milestone_12()
    }
}
