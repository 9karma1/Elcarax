# Build Notes

Elcarax v0.1 is now a multi-crate Rust workspace with both a console proof flow and a feature-gated native shell.

## Rust Toolchain

The workspace targets Rust 1.96.0 and Edition 2024.

```bash
rustup toolchain install 1.96.0
```

## Standard Validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

The default app command is a console proof. It does not require a desktop session or GPU window.

## Native Shell

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell is a manual desktop smoke test. It opens an `Elcarax` window through `winit`, initializes `wgpu`, builds the UI shell through `elcarax_ui`, routes pointer and keyboard input into the UI tree and command palette, renders primitive rectangles/lines through `elcarax_render`, and renders static labels through `elcarax_text`.

The toolbar `Run` button should show hover/pressed/focused visual state. Clicking it should update the status text to `Status: Run clicked`.

Ctrl+K should open the command palette. Typing `ready` and pressing Enter should execute `Show Ready Status` and update the status text to `Status: Ready`. Escape should close the palette without executing a command.

Typing `project.new_demo`, `project.validate`, or `project.close` in the command palette should update the toolbar, status bar, and project panel.

After loading a demo project, `asset.scan_demo`, `asset.select_first`, `asset.clear_selection`, and `asset.show_selected` should update the asset count, asset rows, selected summary, and status bar. Asset row buttons in the project panel should be clickable in the native shell.

The console proof runs `project.new_demo`, then asset scan/select/clear commands, and prints asset count and kind summary lines.

CI should compile the native-shell feature but should not require opening a desktop window.

## Windows Temp Path Note

If local Windows builds fail with MSVC linker temp-file errors and `TMP` is relative, set absolute temp paths before running Cargo:

```powershell
New-Item -ItemType Directory -Force -Path D:\elcarax_v0_1\target\tmp | Out-Null
$env:TMP='D:\elcarax_v0_1\target\tmp'
$env:TEMP='D:\elcarax_v0_1\target\tmp'
```

## Dependency Boundaries

- `elcarax_core`, `elcarax_scene_model`, `elcarax_commands`, `elcarax_adapter_api`, `elcarax_project`, and `elcarax_assets` remain engine-, GPU-, window-, renderer-, UI-, and text-library-neutral.
- `elcarax_platform` owns `winit` integration.
- `elcarax_gpu` owns `wgpu` context and surface integration.
- `elcarax_text` owns `cosmic-text` shaping, layout cache, and system-font rasterization.
- `elcarax_render` owns editor render primitives, batching, render stats, and GPU draw submission.
- `elcarax_ui` owns retained UI tree, layout, hit testing, interaction state, command palette state/painting, dirty flags, theme/style resolution, and paint output.
- `elcarax_app` owns app-level project and asset state composition and pushes display text into the UI tree.

## Current Exclusions

The current shell deliberately excludes docking, drag resizing, tree views, inspector editing, editable text fields, IME, selection, full keybinding system, fuzzy scoring, command macros, scroll views, accessibility implementation beyond placeholder dirty flags, file dialogs, file watching, process IPC, adapter commands, async command execution, project migration, persistent recent-project storage, asset thumbnails, asset import pipeline, and real engine adapter integration.
