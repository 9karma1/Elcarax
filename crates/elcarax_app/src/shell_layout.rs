#![cfg(feature = "native-shell")]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use elcarax_ui::EditorShellLayout;

pub(crate) const MIN_PANEL_WIDTH: f32 = 180.0;
pub(crate) const MIN_VIEWPORT_WIDTH: f32 = 320.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShellLayout {
    pub left_width: f32,
    pub right_width: f32,
    pub splitter_width: f32,
}

impl Default for ShellLayout {
    fn default() -> Self {
        let defaults = EditorShellLayout::default();
        Self {
            left_width: defaults.left_panel_width,
            right_width: defaults.right_panel_width,
            splitter_width: defaults.splitter_width,
        }
    }
}

impl ShellLayout {
    pub(crate) fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(contents) => Self::parse(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub(crate) fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        writeln!(file, "left_width = {}", self.left_width)?;
        writeln!(file, "right_width = {}", self.right_width)?;
        writeln!(file, "splitter_width = {}", self.splitter_width)?;
        Ok(())
    }

    pub(crate) fn editor_shell_layout(&self) -> EditorShellLayout {
        EditorShellLayout {
            left_panel_width: self.left_width,
            right_panel_width: self.right_width,
            splitter_width: self.splitter_width,
        }
    }

    pub(crate) fn clamp_for_body_width(&mut self, body_width: f32) {
        let reserved = self.splitter_width * 2.0 + MIN_VIEWPORT_WIDTH;
        let max_side = (body_width - reserved).max(MIN_PANEL_WIDTH * 2.0);
        self.left_width = self
            .left_width
            .clamp(MIN_PANEL_WIDTH, max_side - MIN_PANEL_WIDTH);
        let max_right =
            body_width - self.splitter_width * 2.0 - MIN_VIEWPORT_WIDTH - self.left_width;
        self.right_width = self
            .right_width
            .clamp(MIN_PANEL_WIDTH, max_right.max(MIN_PANEL_WIDTH));
    }

    fn parse(contents: &str) -> Option<Self> {
        let mut layout = Self::default();
        for line in contents.lines() {
            let line = line.split('#').next()?.trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim();
            match key.trim() {
                "left_width" => layout.left_width = value.parse().ok()?,
                "right_width" => layout.right_width = value.parse().ok()?,
                "splitter_width" => layout.splitter_width = value.parse().ok()?,
                _ => {}
            }
        }
        Some(layout)
    }
}

pub(crate) fn default_shell_layout_path() -> PathBuf {
    PathBuf::from(".elcarax/shell-layout.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn round_trips_shell_layout_file() {
        let path =
            std::env::temp_dir().join(format!("elcarax-shell-layout-{}", std::process::id()));
        let layout = ShellLayout {
            left_width: 260.0,
            right_width: 310.0,
            splitter_width: 4.0,
        };
        assert!(layout.save(&path).is_ok());
        let loaded = ShellLayout::load(&path);
        assert_eq!(loaded, layout);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn clamps_panel_widths_for_narrow_body() {
        let mut layout = ShellLayout {
            left_width: 400.0,
            right_width: 400.0,
            splitter_width: 4.0,
        };
        layout.clamp_for_body_width(900.0);
        assert!(layout.left_width >= MIN_PANEL_WIDTH);
        assert!(layout.right_width >= MIN_PANEL_WIDTH);
        assert!(
            layout.left_width
                + layout.right_width
                + layout.splitter_width * 2.0
                + MIN_VIEWPORT_WIDTH
                <= 900.0 + f32::EPSILON
        );
    }
}
