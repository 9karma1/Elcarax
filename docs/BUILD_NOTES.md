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

`scene.load_demo`, `scene.select_player`, `scene.expand_all`, `scene.collapse_all`, `scene.clear_selection`, and `scene.show_selected` should update the scene tree, selected summary, expand markers, and status bar. Scene row and expand buttons in the project panel should be clickable in the native shell.

`inspector.show_selected`, `inspector.inspect_player`, `inspector.clear`, and `inspector.show_property_count` should update the right inspector panel with grouped property rows derived from scene selection.

`inspector.set_player_health_demo`, `inspector.set_player_speed_demo`, `inspector.rename_player_demo`, `inspector.reset_player_transform_demo`, `edit.undo`, and `edit.redo` should route edits through command history, refresh inspector rows, and update status/diagnostic text. Editable primitive rows render as simple `[Set]` controls; read-only rows remain muted.

The console proof runs `project.new_demo`, asset scan/select commands, scene load/select commands, inspector show/edit/undo/redo/clear commands, and prints asset, scene, and inspector summary lines.

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
- `elcarax_app` owns app-level project, asset, scene, inspector state composition, and command-history composition for inspector edits, then pushes display text into the UI tree.

## Current Exclusions

The current shell deliberately excludes docking, drag resizing, real text input fields, IME, caret/selection editing, full keybinding system, fuzzy scoring, command macros, scroll views, accessibility implementation beyond placeholder dirty flags, file dialogs, file watching, process IPC, adapter commands, adapter writeback, async command execution, project migration, persistent recent-project storage, asset thumbnails, asset import pipeline, hierarchy mutation, hierarchy drag/drop, component add/remove, scene object creation/deletion, asset assignment editing, multi-object editing, validation beyond basic type/editability checks, viewport scene rendering, scene save/writeback, real engine synchronization, and real engine adapter integration.
