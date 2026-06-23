# Elcarax v0.1 scaffold status

Generated: 2026-06-21

## Included

- Rust workspace scaffold targeting Rust 1.96.0 and Edition 2024
- Engine-neutral scene/property/schema model
- Command and undo/redo path
- Platform-neutral command registry, built-in editor commands, command filtering, and invocation results
- Adapter API, SDK, host boundary, and mock game adapter
- UI tree and layout foundation for the editor shell
- UI input routing, hit testing, hover/focus/pressed state, keyboard focus traversal foundation, and basic button clicks
- Command palette shell with query filtering, keyboard selection, execution, cancel behavior, and status feedback
- GPU-backed render primitive pipeline for rectangles, borders, lines, clip metadata, batching, and render stats
- `cosmic-text` shaping, layout cache, and system-font rasterization through `elcarax_text`
- Project, asset, text, accessibility, and devtools modules
- Native shell foundation behind `native-shell`
- `winit` platform event loop contained in `elcarax_platform`
- `wgpu` context/surface/clear-frame foundation contained in `elcarax_gpu`
- ADRs and theme tokens

## Not included yet

- Icons, images, and full vector paths
- Full editor UI system beyond the interactive shell foundation
- Docking, drag resizing, tree views, inspector editing, editable text fields, IME, selection, full keybinding system, fuzzy scoring, scroll views, or asset browser behavior
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

The native shell opens an `Elcarax` window, initializes `wgpu`, builds the UI shell through `elcarax_ui`, routes platform input into the UI tree and command palette, paints it into a render scene, renders static labels through the `elcarax_text` rasterizer, handles resize/DPI/events, and exits cleanly on close.

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

## Milestone 5: Input and Interaction Foundation

- Added Elcarax-owned platform input events for pointer, keyboard, modifiers, focus, wheel, redraw, resize, and close routing.
- Added UI-facing input types, hit-test results, focus changes, and interaction state in `elcarax_ui`.
- Added hit testing against final layout rectangles with deterministic deepest/topmost selection.
- Added hover, focus, active/pressed, disabled, visible, focusable, and interactive state with dirty flag propagation.
- Added basic `Button` and `IconButton` placeholder widget kinds plus a toolbar `Run` button.
- Updated console and native shell flows so clicking `Run` updates the status label to `Status: Run clicked`.
- Documented explicit interaction exclusions in `docs/MILESTONE_5_INPUT_INTERACTION.md`.

## Milestone 6: Command Palette Shell

- Added platform-neutral command registry types, built-in command registration, filtering, lookup, and invocation results in `elcarax_commands`.
- Added command palette state, simple query buffer, filtered entries, selected row movement, and overlay painting in `elcarax_ui`.
- Added Ctrl+K command palette opening in the native shell and keyboard routing for query input, Backspace, ArrowUp, ArrowDown, Enter, and Escape.
- Added command execution for renderer stats, ready status, and the demo run action.
- Updated the console proof to execute `Show Ready Status` through the command palette and print `Status: Ready`.
- Documented explicit command palette exclusions in `docs/MILESTONE_6_COMMAND_PALETTE.md`.
