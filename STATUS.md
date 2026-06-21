# Elcarax v0.1 scaffold status

Generated: 2026-06-21

## Included

- Rust workspace scaffold targeting Rust 1.96.0 and Edition 2024
- Engine-neutral scene/property/schema model
- Command and undo/redo path
- Adapter API, SDK, host boundary, and mock game adapter
- UI tree and layout foundation for the editor shell
- GPU-backed render primitive pipeline for rectangles, borders, lines, clip metadata, batching, and render stats
- `cosmic-text` shaping, layout cache, and system-font rasterization through `elcarax_text`
- Project, asset, text, accessibility, and devtools modules
- Native shell foundation behind `native-shell`
- `winit` platform event loop contained in `elcarax_platform`
- `wgpu` context/surface/clear-frame foundation contained in `elcarax_gpu`
- ADRs and theme tokens

## Not included yet

- Icons, images, and full vector paths
- Full editor UI system beyond the static shell
- Docking, drag resizing, tree views, inspector editing, command palette, text input, scroll views, or asset browser behavior
- Real `AccessKit` adapter integration
- Real process IPC transport
- Real game engine binding
- CI execution of the native window path

## Running

Default console proof flow:

```bash
cargo run -p elcarax_app
```

Feature-gated native shell:

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell opens an `Elcarax` window, initializes `wgpu`, builds the UI shell through `elcarax_ui`, paints it into a render scene, renders static labels through the `elcarax_text` rasterizer, handles resize/DPI/events, and exits cleanly on close.

## Validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

## Milestone 3: Text Rendering Foundation

- Added `elcarax_text` static text layout types, cosmic-text-backed shaping, metrics, glyph placement data, and layout cache tests.
- Added text primitives, text batching metadata, glyph atlas stats, real system-font rasterization, and text/glyph render stats in `elcarax_render`.
- Updated the demo proof scene to include static labels: Elcarax, Project, Viewport, Inspector, Console, and Status: Renderer online.
- Documented the current system-font discovery/rasterization strategy and explicit text editing exclusions in `docs/MILESTONE_3_TEXT_RENDERING.md`.

## Milestone 4: UI Tree and Layout Foundation

- Added retained UI nodes, stable widget identity, tree traversal, layout constraints/results, dirty flags, theme/style resolution, and paint contexts in `elcarax_ui`.
- Added fixed/fill/content sizing, horizontal/vertical stack layout, padding/insets, and split-style row/column shell composition.
- Added non-interactive root, panel, label, separator, toolbar, status bar, and viewport placeholder widgets.
- Replaced app-owned hardcoded shell primitives with UI-generated render scenes in console and native paths.
- Documented explicit editor feature exclusions in `docs/MILESTONE_4_UI_TREE_LAYOUT.md`.
