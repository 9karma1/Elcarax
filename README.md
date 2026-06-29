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
- interactive editor shell foundation with toolbar, project panel, asset browser, scene tree, viewport, inspector, status bar, and command palette
- project-domain model, recent project list, validation diagnostics, and project commands
- asset browser foundation with file-based asset indexing, scan state, selection state, and clickable asset rows
- scene tree foundation with engine-neutral scene model, hierarchy display, selection/expand state, and scene commands
- read-only inspector foundation with property formatting, grouped rows, selection-driven updates, and inspector commands
- editable inspector undo foundation with command-driven primitive property edits, inspector refresh, diagnostics, and undo/redo
- adapter host integration with JSON-line process spawning, handshake, diagnostics/logs, scene snapshot import, and adapter command-palette commands
- adapter property writeback foundation with mock-adapter-only set-property requests, confirmed scene patches, adapter-backed inspector edits, and adapter undo/redo
- productionized empty runtime startup with fixture data kept out of normal app flow
- project, asset, accessibility state, and devtools modules
- architecture decision records and milestone documentation

This is not a full editor yet. Docking, drag resizing, hierarchy drag/drop, real text input fields, IME/caret/selection editing, component add/remove, asset assignment editing, multi-object editing, scroll views, real accessibility integration, file dialogs, file watching, hot reload, plugin/marketplace runtime loading, asset import pipeline, scene save/writeback, viewport scene rendering or frame streaming, real engine synchronization, C++ integration, real engine writeback, and real engine binding are intentionally out of scope for the current milestone.

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

The console flow builds the empty editor shell without opening a GPU window and prints a startup validation summary. It does not load fake project, asset, scene, inspector, or adapter data.

Manual native shell smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell opens an `Elcarax` window, initializes `wgpu`, builds the UI shell through `elcarax_ui`, routes platform input into the UI tree and command palette, paints it into a render scene, renders static labels through the `elcarax_text` rasterizer, handles resize/DPI/events, and exits cleanly on close.

Suggested manual flow:

1. Open the native shell
2. Confirm no project, asset root, scene, viewport source, selected object, or adapter is loaded automatically
3. Confirm the left panel shows `No project open`, assets unavailable until a project/root exists, and `No scene loaded`
4. Confirm the center viewport says `No viewport source`
5. Confirm the right inspector says `No object selected`
6. Confirm the status bar says `Ready - open a project or connect an adapter`
7. Press Ctrl+K and confirm the palette exposes real editor commands such as `project.create`, `project.open`, `asset.scan`, `scene.load`, `inspector.clear`, `edit.undo`, `edit.redo`, `adapter.connect`, `adapter.load_scene`, and adapter status/diagnostic commands
8. Run unimplemented setup commands such as `project.open` or `adapter.connect` and confirm they report clear diagnostics instead of creating fake data

The command palette shows eight rows at a time and filters with query text. The toolbar `Open` button reports that project opening is not implemented yet.

## Architecture

Elcarax keeps external systems behind crate boundaries:

- `elcarax_core`: foundational IDs, errors, diagnostics, workspace types
- `elcarax_scene_model`: engine-neutral scene/property/schema model
- `elcarax_commands`: command and undo/redo behavior
- `elcarax_project`: project model, validation, status, and recent-project domain types
- `elcarax_assets`: asset index, scan, selection, and extension-based kind detection
- `elcarax_adapter_api`: stable adapter boundary
- `elcarax_adapter_host`: adapter process, JSON-line transport, request correlation, events, and failure handling
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
- Milestone 11: editable inspector undo foundation
- Milestone 12: adapter host integration
- Milestone 13: adapter property writeback foundation
- Milestone 14A: productionized empty runtime startup

See `docs/` for detailed milestone notes and ADRs. Latest milestone docs:

- `docs/MILESTONE_10_READ_ONLY_INSPECTOR.md`
- `docs/MILESTONE_11_EDITABLE_INSPECTOR_UNDO.md`
- `docs/MILESTONE_12_ADAPTER_HOST_INTEGRATION.md`
- `docs/MILESTONE_13_ADAPTER_PROPERTY_WRITEBACK.md`
- `docs/MILESTONE_14A_PRODUCTIONIZE_RUNTIME.md`
