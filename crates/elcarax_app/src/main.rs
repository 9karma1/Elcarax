mod asset_display;
mod asset_state;
mod asset_ui;
#[cfg(not(feature = "native-shell"))]
mod console;
#[cfg(feature = "native-shell")]
mod native_shell;
mod project_display;
mod project_state;
mod project_ui;

use elcarax_core::Result;

#[cfg(not(feature = "native-shell"))]
fn main() -> Result<()> {
    console::run_console_proof()
}

#[cfg(feature = "native-shell")]
fn main() -> Result<()> {
    native_shell::run_native_shell()
}
