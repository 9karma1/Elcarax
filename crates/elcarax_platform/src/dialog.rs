//! Native folder picker for project open/create flows.

use std::path::PathBuf;

/// Opens a native folder picker when the `native` feature is enabled.
pub fn pick_folder(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}
