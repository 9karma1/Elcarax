# Elcarax v0.1 scaffold status

Generated: 2026-07-03

## Included

- Rust workspace scaffold targeting Rust 1.96.0 and Edition 2024
- Engine-neutral scene/property/schema model with project-owned scene files (`*.elcarax.scene.toml`)
- Command and undo/redo path with history cleared on project open/close through `EditorSession`
- Platform-neutral command registry, built-in editor commands, command filtering, and invocation results
- Adapter API, SDK, host boundary, and stdio reference game adapter
- UI tree and layout foundation for the editor shell
- UI input routing, hit testing, hover/focus/pressed state, keyboard focus traversal foundation, and basic button clicks
- Command palette shell with query filtering, keyboard selection, execution, cancel behavior, and status feedback
- Project system UI with project status, recent project count, validation diagnostics, project panel metadata, and command-palette project commands
- Real asset index with project-relative paths, stable path-derived IDs, cheap metadata, diagnostics, explicit scan/refresh, watcher dirty-state foundation, asset panel rows, selection state, and command-palette asset commands
- Scene tree foundation with engine-neutral scene model, reference scene snapshot for adapter/tests, scene panel hierarchy, selection/expand state, and command-palette scene commands
- Read-only inspector foundation with property formatting, grouped inspector rows, selection-driven updates, and command-palette inspector commands
- Editable inspector undo foundation with primitive property edit metadata, model-owned validation/mutation helpers, command-driven edits, inspector refresh, diagnostics, and undo/redo
- Adapter host integration with JSON-line protocol, mock process spawning, versioned handshake, request/response correlation, diagnostics/logs, scene snapshot import, status UI, and command-palette adapter commands
- Adapter property writeback foundation with mock adapter set-property protocol, confirmed scene patches, adapter-backed inspector edits, adapter undo/redo writeback, and diagnostics for rejected writes
- Viewport preview foundation with viewport state, adapter RGBA frame protocol, image render primitive, viewport commands, and console adapter viewport proof
- Productionized normal runtime startup with no fake project, asset, scene, inspector, adapter, or viewport data loaded automatically
- Real project file format, create/open/validate/close, recent-project persistence, native folder picker, default scene on create, and project-root asset scanning
- Scene document persistence: `scene.load`, `scene.save`, dirty tracking, unsaved close guards, manifest `active_scene` sync, Ctrl+S, and unsaved UI affordances
- `EditorSession` / `EditorSessionState` coordinator for unified project-bound lifecycle across console and native shell
- GPU-backed render primitive pipeline for rectangles, borders, lines, clip metadata, batching, and render stats
- `cosmic-text` shaping, layout cache, and system-font rasterization through `elcarax_text`
- Project, asset, text, accessibility, and devtools modules
- Native shell foundation behind `native-shell`
- `winit` platform event loop contained in `elcarax_platform`
- `wgpu` context/surface/clear-frame foundation contained in `elcarax_gpu`
- ADRs and theme tokens

## Not included yet

- Icons, images, and full vector paths
- Full editor UI system beyond the interactive empty shell and project-status foundation
- Docking, hierarchy drag/drop, component add/remove, hierarchy mutation, asset assignment editing, multi-object editing, full keybinding registry, fuzzy scoring, scroll views, multi-scene switcher UI, save-on-close dialogs, continuous autosave, project migration beyond schema version checks, asset thumbnails, asset previews, asset import pipeline, drag-and-drop asset behavior, asset rename/move/delete, asset dependency graph, asset build/import cache, scene object creation/deletion, continuous viewport frame streaming, adapter hot reload, marketplace/plugin runtime loading, dynamic library loading, adapter security sandbox, or real engine synchronization
- Normal runtime automatic fake data loading as user-facing editor behavior
- Real `AccessKit` adapter integration
- Real game engine binding
- CI execution of the native window path

## Beta readiness cleanup

- Removed in-memory asset demo index and `examples/demo_project` placeholders; asset behavior now uses real scans and temp-dir tests only.
- Renamed scene `demo_scene_snapshot` to `reference_scene_snapshot` for adapter and test boundaries.
- Consolidated adapter capabilities to `AdapterCapabilities::stdio_game_adapter()`.
- Ignored local `.elcarax/` runtime state in git.
- Replaced `project_effects.rs` with `EditorSession` for project lifecycle side effects.
- Milestone markdown files removed; history lives in [CHANGELOG.md](CHANGELOG.md).

## Running

Default console proof flow:

```bash
cargo run -p elcarax_app
```

Feature-gated native shell:

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell opens an `Elcarax` window, initializes `wgpu`, builds the UI shell through `elcarax_ui`, routes platform input into the UI tree and command palette, supports resizable side panels, editable inspector fields, native folder pickers, Ctrl+S scene save, unsaved indicators, paints into a render scene, renders static labels through `elcarax_text`, handles resize/DPI/events, and exits cleanly on close.

## Validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

Release history: [CHANGELOG.md](CHANGELOG.md). Planned work: [ROADMAP.md](ROADMAP.md).
