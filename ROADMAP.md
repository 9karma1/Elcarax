# Roadmap

Elcarax v0.1 is building an engine-neutral editor platform in small, reviewable slices while preserving crate boundaries and a headless CI proof path.

Past work is recorded in [CHANGELOG.md](CHANGELOG.md).

## Completed (through 0.1.0)

Foundation through real project open, asset index, file watching, adapter host, viewport preview, and productionized empty startup. Scene persistence and editor session coordination are in `[Unreleased]` until tagged.

## Next

1. **Scroll views** — asset, scene, and inspector panels exceed fixed row caps today; add minimal `ScrollView` in `elcarax_ui`.
2. **Toolbar and keybinding registry** — declarative bindings for common commands (Ctrl+O, Ctrl+Z/Y) beyond palette and Ctrl+S.
3. **Recent projects welcome UI** — clickable recent rows when no project is open.
4. **Multi-scene management** — list `*.elcarax.scene.toml`, switch active scene with dirty guards.
5. **Accessibility integration** — wire real accessibility output after retained UI semantics settle.
6. **Editor settings policy** — optional `scan_assets_on_open` and related session policies exposed through project or user config.

## Standing constraints

- Keep core/domain crates free of `wgpu`, `winit`, `cosmic-text`, game engines, and adapter implementations.
- Keep `cargo run -p elcarax_app` as the headless proof path used by CI.
- Keep native shell validation manual with `cargo run -p elcarax_app --features native-shell`.
- Route project-bound lifecycle changes through `EditorSession` rather than ad-hoc side effects in UI handlers.
