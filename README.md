# Elcarax v0.1

Elcarax is an open source Rust editor platform for building engine-neutral creative tools. The first adapter target is a game workflow, but the editor core is kept independent from any specific engine or game framework.

The project is licensed under Apache-2.0. See [LICENSE](LICENSE).

## Current State

This repository contains the v0.1 foundation for the Elcarax editor:

- engine-neutral workspace, scene, schema, property, and command types
- command metadata registry, default keybindings, conflict diagnostics, and command history with undo/redo proof flow
- adapter API, SDK, host boundary, and stdio reference game adapter
- `winit` native shell behind the `native-shell` feature
- `wgpu` surface/context and rectangle primitive rendering
- `cosmic-text` shaping and system-font rasterization through `elcarax_text`
- retained UI tree, layout primitives, hit testing, interaction state, dirty flags, style/theme resolution, scroll views, and paint output
- interactive editor shell with registry-driven toolbar actions, resizable project panel, scrollable asset browser and scene tree, viewport, inspector, status bar, and command palette with shortcut hints
- project-domain model, recent project list, validation diagnostics, and project commands
- real project asset index with project-relative paths, stable path-derived IDs, metadata, diagnostics, refresh, watcher dirty-state, and clickable asset rows
- scene tree with engine-neutral scene model, reference scene snapshot for adapter/tests, hierarchy display, selection/expand state, and scene commands
- inspector with schema-driven property rows, grouped sections, selection-driven updates, and typed edit widgets (text, toggle, number stepper, vector field, enum selector)
- editable inspector undo through command-driven property edits, validation, diagnostics, and a unified undo/redo history
- adapter host integration with JSON-line process spawning, handshake, diagnostics/logs, scene snapshot import, and adapter command-palette commands
- adapter property writeback with set-property requests, confirmed scene patches, adapter-backed inspector edits, and the same undo/redo path as local edits
- scene mutation patches for property updates and hierarchy ops (`ObjectAdded` / `ObjectRemoved` / `Reparented` / `Renamed`)
- viewport preview with adapter RGBA frames, letterboxed layout, camera pan/zoom, actionable empty states, and scene-object picking from normalized viewport coordinates
- productionized empty runtime startup with no fake project, asset, scene, inspector, adapter, or viewport data loaded automatically
- real project file format, create/open/validate/close, recent-project persistence, native folder picker, and explicit project asset scanning
- project-owned scene files (`*.elcarax.scene.toml`), auto-load on project open, `scene.save`, document dirty tracking, unsaved guards, manifest `active_scene` sync, and **Ctrl+S** in the native shell through the keybinding registry
- `EditorSession` coordinator in `elcarax_app` for unified project open/close/switch, dependent state binding, and undo history reset
- project, asset, accessibility state, and devtools modules
- architecture decision records; see [CHANGELOG.md](CHANGELOG.md) for release history

This is not a full editor yet. Docking, hierarchy drag/drop, component add/remove, asset assignment editing, multi-object editing, user-editable keybinding preferences, command macros, fuzzy command scoring, menu bars, full settings UI, IME/full caret selection editing, real accessibility integration, hot reload, plugin/marketplace runtime loading, asset import pipeline, asset thumbnails/previews, asset drag/drop, asset rename/move/delete, scene hierarchy mutation, multi-scene switcher UI, save-on-close dialogs, continuous autosave, continuous viewport frame streaming, adapter viewport pick protocol, real engine synchronization, C++ integration, real engine writeback, and real engine binding are intentionally out of scope for the current milestone.

## Requirements

- Rust 1.96.0
- Windows, macOS, or Linux for console/library validation
- A desktop session for the manual `native-shell` smoke test

See [docs/INSTALL.md](docs/INSTALL.md) for clone, build, and `cargo install --git` instructions. Crates are not on crates.io yet.

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

Create and open a real temporary project from the console proof:

```bash
cargo run -p elcarax_app -- --create-project /path/to/new-project --project-name "My Elcarax Project"
cargo run -p elcarax_app -- --project /path/to/new-project
```

Environment variables:

- `ELCARAX_PROJECT_CREATE_PATH`
- `ELCARAX_PROJECT_PATH`
- `ELCARAX_PROJECT_NAME`
- `ELCARAX_RECENT_PROJECTS_PATH`

The console flow builds the empty editor shell without opening a GPU window, reports command/keybinding registry diagnostics and toolbar snapshot construction, then exercises real project create/open/validate, auto-loads the default project scene, writes real files under the temporary project `assets/`, runs `asset.scan`, selects an asset, modifies the asset folder, runs `asset.refresh`, saves through the Ctrl+S-equivalent registry binding, proves scene document save round-trip, closes/reopens the project, and runs the adapter viewport proof.

Manual native shell smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell opens an `Elcarax` window, initializes `wgpu`, builds the UI shell through `elcarax_ui`, routes platform input into the command registry and UI tree, supports registry-driven toolbar actions, command palette shortcut hints, disabled command reasons, resizable side panels with persisted widths, scrollable asset/scene/inspector panels, schema-driven inspector widgets, viewport pan/zoom and click-to-select, native folder pickers for project open/create, paints into a render scene, renders labels through `elcarax_text`, handles resize/DPI/events, and exits cleanly on close.

Suggested manual flow:

1. Open the native shell
2. Confirm no project, asset root, scene, viewport source, selected object, or adapter is loaded automatically
3. Confirm the left panel shows `No project open`, `Assets unavailable - no project open`, and `No scene loaded`
4. Confirm the center viewport shows `No viewport source` with an actionable hint
5. Confirm the right inspector says `No object selected`
6. Confirm the status bar says `Ready - open a project or connect an adapter`
7. Press **Ctrl+K** and confirm the palette exposes editor commands such as `project.create`, `project.open`, `project.close`, `asset.scan`, `asset.refresh`, `scene.load`, `scene.save`, `edit.undo`, `edit.redo`, `viewport.request_frame`, and adapter commands with shortcut hints and muted disabled reasons
8. Confirm toolbar actions such as **New**, **Open**, **Save**, **Undo**, **Redo**, **Scan**, **Refresh**, and **Connect** reflect enabled/disabled state; clicking them dispatches the same command path as shortcuts and palette rows
9. Click the toolbar **Open** button or run `project.create` / `project.open` - uses CLI/env paths when configured, otherwise the native folder picker
10. After opening a project, confirm the default scene loads, select a scene object, and edit inspector values (number steppers, vector fields, enum cycle, text fields as applicable)
11. Confirm the `*` unsaved indicator after an edit, press **Ctrl+S** to save, then close/reopen the project
12. Optional: connect an adapter, run `viewport.request_frame`, scroll to zoom the preview, Alt+drag or middle-drag to pan, and click the viewport to select scene objects

## Architecture

Elcarax keeps external systems behind crate boundaries:

- `elcarax_core` owns foundational IDs, errors, diagnostics, workspace types, viewport domain state, and viewport camera/layout math
- `elcarax_scene_model`: engine-neutral scene/property/schema model, `InspectorValueWidget` descriptors, and viewport pick helpers
- `elcarax_commands`: command metadata, platform-neutral key chords, default keybindings, conflict diagnostics, and undo/redo behavior
- `elcarax_project`: project model, validation, status, and recent-project domain types
- `elcarax_assets`: asset index, project-relative scanning, stable path-derived IDs, metadata, diagnostics, selection, extension-based kind detection, and watch service abstraction
- `elcarax_adapter_api`: stable adapter boundary
- `elcarax_adapter_host`: adapter process, JSON-line transport, request correlation, events, and failure handling
- `elcarax_platform`: platform event loop and native window integration
- `elcarax_gpu`: `wgpu` context, surface, and render-pass helpers
- `elcarax_text`: `cosmic-text` shaping, layout cache, and system-font rasterization
- `elcarax_render`: editor render primitives, batching, GPU rendering, and render stats
- `elcarax_ui`: retained UI tree, layout, scroll views, typed property widgets, hit testing, interaction state, command palette/toolbar presentation, dirty flags, styles, and paint output
- `elcarax_app`: composition layer - `EditorSession`, command availability/dispatch, toolbar snapshots, console proof, and native shell

The game engine may depend on Elcarax adapter SDK types. Elcarax core crates must not depend on the game engine.

## Documentation

- [docs/INSTALL.md](docs/INSTALL.md) — build from source and install the `Elcarax` binary
- [CONTRIBUTING.md](CONTRIBUTING.md) — development workflow and pull request guidelines
- [CHANGELOG.md](CHANGELOG.md) — release history
- [ROADMAP.md](ROADMAP.md) — planned work
- [STATUS.md](STATUS.md) — current capability snapshot
- [docs/BUILD_NOTES.md](docs/BUILD_NOTES.md) — console and native-shell behavior
- [docs/adr/](docs/adr/) — architecture decision records
