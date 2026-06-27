# Elcarax v0.1

Elcarax is an open source Rust editor platform for building engine-neutral creative tools. The first adapter target is a game workflow, but the editor core is kept independent from any specific engine or game framework.

The project is licensed under Apache-2.0. See [LICENSE](LICENSE).

## Current State

This repository contains the v0.1 foundation for the Elcarax editor:

- engine-neutral workspace, scene, schema, property, and command types
- command history with undo/redo proof flow
- adapter API, SDK, host boundary, and mock game adapter
- `winit` native shell behind the `native-shell` feature
- `wgpu` surface/context and rectangle primitive rendering
- `cosmic-text` shaping and system-font rasterization through `elcarax_text`
- retained UI tree, layout primitives, hit testing, interaction state, dirty flags, style/theme resolution, and paint output
- interactive editor shell foundation with toolbar, Run button, project panel, asset browser, scene tree, viewport, inspector, status bar, and command palette
- project-domain model, recent project list, validation diagnostics, and project commands
- asset browser foundation with demo asset index, scan/selection commands, and clickable asset rows
- scene tree foundation with engine-neutral scene model, demo snapshot, hierarchy display, selection/expand state, and scene commands
- read-only inspector foundation with property formatting, grouped rows, selection-driven updates, and inspector commands
- project, asset, accessibility placeholder, and devtools modules
- architecture decision records and milestone documentation

This is not a full editor yet. Docking, drag resizing, hierarchy drag/drop, inspector editing, editable text fields, IME, selection, scroll views, real accessibility integration, file dialogs, file watching, process IPC, adapter loading, asset import pipeline, scene save/writeback, viewport scene rendering, and real engine binding are intentionally out of scope for the current milestone.

## Requirements

- Rust 1.96.0
- Windows, macOS, or Linux for console/library validation
- A desktop session for the manual `native-shell` smoke test

Install the pinned Rust toolchain:

```bash
rustup toolchain install 1.96.0
```

## Validation

Run the core quality gates:

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

On Windows, if MSVC linker temp files fail because `TMP` is relative, use absolute temp paths for Cargo commands:

```powershell
New-Item -ItemType Directory -Force -Path D:\elcarax_v0_1\target\tmp | Out-Null
$env:TMP='D:\elcarax_v0_1\target\tmp'
$env:TEMP='D:\elcarax_v0_1\target\tmp'
```

## Running

Default console proof flow:

```bash
cargo run -p elcarax_app
```

The console flow builds the UI shell without opening a GPU window, simulates a Run button click, executes command-palette actions, runs project/asset/scene/inspector commands, updates editor UI state, and prints project, asset, scene, inspector, command history, render, UI node, layout, primitive, text primitive, dirty flag, interaction, and command palette proof output.

Manual native shell smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell opens an `Elcarax` window, initializes `wgpu`, builds the UI shell through `elcarax_ui`, routes platform input into the UI tree and command palette, paints it into a render scene, renders static labels through the `elcarax_text` rasterizer, handles resize/DPI/events, and exits cleanly on close.

Suggested manual flow:

1. Open the native shell
2. Press Ctrl+K and run `project.new_demo` (type `new demo`)
3. Run `asset.scan_demo` (type `scan`)
4. Run `scene.load_demo` (type `scene` or `load demo`)
5. Confirm the left panel shows demo assets and the demo scene hierarchy
6. Select `Player` via `scene.select_player` or by clicking the row
7. Confirm the selected row highlights and the status bar reports the selected object
8. Confirm the right inspector shows Player read-only properties
9. Run `inspector.clear` and confirm the inspector returns to the empty state
10. Run `scene.expand_all` and `scene.collapse_all`

The command palette shows eight rows at a time; filter with query text to reach scene commands below the asset section. Clicking the toolbar `Run` button updates the status text to `Status: Run clicked`.

## Architecture

Elcarax keeps external systems behind crate boundaries:

- `elcarax_core`: foundational IDs, errors, diagnostics, workspace types
- `elcarax_scene_model`: engine-neutral scene/property/schema model
- `elcarax_commands`: command and undo/redo behavior
- `elcarax_project`: project model, validation, status, and recent-project domain types
- `elcarax_assets`: asset index, scan, selection, and extension-based kind detection
- `elcarax_adapter_api`: stable adapter boundary
- `elcarax_platform`: platform event loop and native window integration
- `elcarax_gpu`: `wgpu` context, surface, and render-pass helpers
- `elcarax_text`: `cosmic-text` shaping, layout cache, and system-font rasterization
- `elcarax_render`: editor render primitives, batching, GPU rendering, and render stats
- `elcarax_ui`: retained UI tree, layout, hit testing, interaction state, dirty flags, styles, and paint output
- `elcarax_app`: composition layer for console proof and native shell

The game engine may depend on Elcarax adapter SDK types. Elcarax core crates must not depend on the game engine.

## Milestones

- Milestone 1: native shell foundation
- Milestone 2: GPU render primitive pipeline
- Milestone 3: text rendering foundation
- Milestone 4: UI tree and layout foundation
- Milestone 5: input and interaction foundation
- Milestone 6: command palette shell
- Milestone 7: project system UI
- Milestone 8: asset browser foundation
- Milestone 9: scene tree foundation
- Milestone 10: read-only inspector foundation

See `docs/` for detailed milestone notes and ADRs. Latest milestone docs:

- `docs/MILESTONE_9_SCENE_TREE_FOUNDATION.md`
- `docs/MILESTONE_10_READ_ONLY_INSPECTOR.md`
