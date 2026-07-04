# Roadmap

Elcarax v0.1 is building an engine-neutral editor platform in small, reviewable slices while preserving crate boundaries and a headless CI proof path.

Past work is recorded in [CHANGELOG.md](CHANGELOG.md).

## Completed (through 0.1.0)

Foundation through real project open, asset index, file watching, adapter host, viewport preview, and productionized empty startup.

## Completed ([Unreleased])

Until the next tag, these items are implemented on `main` but not yet in a release:

- Project-owned scene persistence, dirty tracking, unsaved guards, and Ctrl+S
- `EditorSession` coordinator for unified project-bound lifecycle
- Scroll views for asset, scene, and inspector panels
- Typography, elevation, and component-level UI polish
- Schema-driven inspector widgets (toggle, number stepper, vector field, enum selector)
- Viewport camera with letterboxed frames, pan/zoom, actionable empty states, and scene-object picking
- Toolbar and keybinding registry with declarative command metadata, default shortcuts, conflict diagnostics, command availability reasons, shared toolbar/palette/shortcut dispatch, and command help summaries

## Next

1. **Recent projects welcome UI** - clickable recent rows when no project is open.
2. **Multi-scene management** - list `*.elcarax.scene.toml`, switch active scene with dirty guards.
3. **Adapter viewport pick protocol** - replace orthographic scene pick with adapter-backed object selection when connected.
4. **Accessibility integration** - wire real accessibility output after retained UI semantics settle.
5. **Editor settings policy** - optional `scan_assets_on_open` and related session policies exposed through project or user config.
6. **Keybinding preferences** - user-editable shortcut storage and conflict resolution UI after the default registry settles.

## Standing constraints

- Keep core/domain crates free of `wgpu`, `winit`, `cosmic-text`, game engines, and adapter implementations.
- Keep `cargo run -p elcarax_app` as the headless proof path used by CI.
- Keep native shell validation manual with `cargo run -p elcarax_app --features native-shell`.
- Route project-bound lifecycle changes through `EditorSession` rather than ad-hoc side effects in UI handlers.
- Keep UI presentation snapshot-driven: domain crates build display state; `elcarax_ui` paints widgets without mutating project data directly.
