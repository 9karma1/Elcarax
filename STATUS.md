# Elcarax v0.1 scaffold status

Generated: 2026-06-29

## Included

- Rust workspace scaffold targeting Rust 1.96.0 and Edition 2024
- Engine-neutral scene/property/schema model
- Command and undo/redo path
- Platform-neutral command registry, built-in editor commands, command filtering, and invocation results
- Adapter API, SDK, host boundary, and mock game adapter
- UI tree and layout foundation for the editor shell
- UI input routing, hit testing, hover/focus/pressed state, keyboard focus traversal foundation, and basic button clicks
- Command palette shell with query filtering, keyboard selection, execution, cancel behavior, and status feedback
- Project system UI with project status, recent project count, validation diagnostics, project panel metadata, and command-palette project commands
- Asset browser foundation with file-based asset index, demo scan, asset panel rows, selection state, and command-palette asset commands
- Scene tree foundation with engine-neutral scene model, demo snapshot, scene panel hierarchy, selection/expand state, and command-palette scene commands
- Read-only inspector foundation with property formatting, grouped inspector rows, selection-driven updates, and command-palette inspector commands
- Editable inspector undo foundation with primitive property edit metadata, model-owned validation/mutation helpers, command-driven edits, inspector refresh, diagnostics, and undo/redo
- Adapter host integration with JSON-line protocol, mock process spawning, versioned handshake, request/response correlation, diagnostics/logs, scene snapshot import, status UI, and command-palette adapter commands
- Adapter property writeback foundation with mock adapter set-property protocol, confirmed scene patches, adapter-backed inspector edits, adapter undo/redo writeback, and diagnostics for rejected writes
- Productionized normal runtime startup with no fake project, asset, scene, inspector, adapter, or viewport data loaded automatically
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
- Docking, drag resizing, real text input fields, IME, caret/selection editing, component add/remove, hierarchy mutation, asset assignment editing, multi-object editing, full keybinding system, fuzzy scoring, scroll views, file dialogs, file watching, persistent recent-project storage, project migration, asset thumbnails, asset import pipeline, drag-and-drop asset behavior, scene object creation/deletion, viewport scene rendering, viewport frame streaming, scene save/writeback, real engine writeback, adapter hot reload, marketplace/plugin runtime loading, dynamic library loading, adapter security sandbox, or real engine synchronization
- Normal runtime fixture commands, automatic fake data loading, and mock adapter startup as user-facing editor behavior
- Real `AccessKit` adapter integration
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
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

## Milestone 14A: Productionize Runtime

- Removed normal runtime registration of demo project, asset, scene, inspector edit, and mock-adapter startup commands.
- Replaced startup behavior with honest empty states: no project open, no asset root loaded, no scene loaded, no object selected, no viewport source, and adapter disconnected.
- Replaced fake-workflow console proof with a startup validation summary that builds the empty UI shell and prints state/render/undo readiness.
- Kept property editing, adapter writeback, and scene/asset/project behavior covered through tests, fixtures, and the mock adapter boundary instead of normal user-facing commands.
- Updated current docs and build notes so manual smoke tests validate the empty editor shell and explicit diagnostics.

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

## Milestone 7: Project System UI

- Added project-domain types for project identity, name, path, status, validation diagnostics, recent project entries, and project errors in `elcarax_project`.
- Added command-palette project commands for demo project creation/loading, close, validate, and recent-project reporting.
- Added app-owned project state in `elcarax_app`, separate from native UI tree state.
- Updated the shell to show project name/path/status, recent count, validation summary, command result, and project status bar text.
- Updated the console proof to execute project commands through the command palette and print initial, loaded, validated, and closed project states.
- Documented explicit project system exclusions in `docs/MILESTONE_7_PROJECT_SYSTEM_UI.md`.

## Milestone 8: Asset Browser Foundation

- Added file-based asset domain types, extension detection, in-memory demo index, synchronous scan API, and selection helpers in `elcarax_assets`.
- Added command-palette asset commands for demo scan, first selection, clear selection, and show selected.
- Added app-owned asset state separate from UI widgets and project-panel asset rows with clickable selection in the native shell.
- Updated the console proof to load a demo project, scan demo assets, select the first asset, and clear selection.
- Documented explicit asset browser exclusions in `docs/MILESTONE_8_ASSET_BROWSER_FOUNDATION.md`.

## Milestone 9: Scene Tree Foundation

- Added engine-neutral scene domain types, hierarchy flattening, selection/expansion helpers, and deterministic demo snapshot in `elcarax_scene_model`.
- Added command-palette scene commands for demo load, root/player selection, clear selection, expand/collapse all, and show selected.
- Added app-owned scene state separate from UI widgets and project-panel scene rows with clickable selection and expand toggles in the native shell.
- Updated the console proof to load the demo scene, select Player, expand/collapse all, and clear selection.
- Documented explicit scene tree exclusions in `docs/MILESTONE_9_SCENE_TREE_FOUNDATION.md`.

## Milestone 10: Read-Only Inspector Foundation

- Added inspector view model types, property formatting, grouped read-only rows, and expanded demo scene properties in `elcarax_scene_model`.
- Added command-palette inspector commands for show selected, clear, inspect player/root, and property count reporting.
- Added app-owned inspector state derived from scene selection and right-panel inspector UI in the native shell.
- Updated the console proof to select Player, show inspector properties, report property count, and clear the inspector view.
- Documented explicit read-only inspector exclusions in `docs/MILESTONE_10_READ_ONLY_INSPECTOR.md`.

## Milestone 11: Editable Inspector Undo Foundation

- Added editable/read-only metadata and basic type/editability mutation checks in `elcarax_scene_model`.
- Added command-driven scene property edits plus undo/redo wrappers in `elcarax_commands`.
- Added command-palette inspector edit demos for Player health, speed, name, transform reset, undo, and redo.
- Added editable inspector row affordances, read-only reason labels, status/diagnostic updates, and console proof coverage for edit/undo/redo.
- Documented explicit writeback, text-input, hierarchy, component, asset-assignment, multi-object, and engine-sync exclusions in `docs/MILESTONE_11_EDITABLE_INSPECTOR_UNDO.md`.

## Milestone 12: Adapter Host Integration

- Added versioned JSON-line adapter protocol types, request IDs, response/event helpers, adapter diagnostics/logs, and mock capabilities in `elcarax_adapter_api`.
- Added process spawning, stdin/stdout JSON-line transport, request correlation, event collection, failure states, and clean shutdown in `elcarax_adapter_host`.
- Converted `elcarax_game_adapter` into a deterministic mock stdio adapter that handshakes, loads project info, returns the demo scene snapshot, returns diagnostics, and shuts down.
- Added command-palette adapter commands for start, handshake, load project, load scene, show status, show diagnostics, and stop.
- Added adapter app state and UI labels for adapter status, diagnostic count, and last adapter command result.
- Updated the console proof to start the mock adapter, handshake, print capabilities, import the adapter scene snapshot, show diagnostics, and stop the process.
- Documented explicit real-engine, writeback, binary protocol, viewport streaming, hot reload, plugin runtime, dynamic library, sandbox, and timeout exclusions in `docs/MILESTONE_12_ADAPTER_HOST_INTEGRATION.md`.

## Milestone 13: Adapter Property Writeback Foundation

- Added adapter writeback protocol messages for set-property requests and confirmed/rejected responses.
- Added minimal scene patch types and property update patch application in `elcarax_scene_model`.
- Updated the mock adapter to validate, mutate, and report confirmed property changes against its internal demo scene snapshot.
- Added scene source tracking so local demo edits keep using local command history while adapter-backed edits route through the adapter boundary.
- Added adapter-backed inspector edit, undo, and redo commands that update editor scene state only after adapter confirmation.
- Added diagnostics for disconnected adapters, non-adapter scenes, missing objects/properties, read-only properties, type mismatch, stale values, and rejected writes.
- Updated the console proof to demonstrate local edit/undo/redo plus adapter-backed edit/undo/redo.
- Documented explicit real-engine, persistent save, hierarchy, component, asset-assignment, multi-object, collaborative, and advanced conflict-resolution exclusions in `docs/MILESTONE_13_ADAPTER_PROPERTY_WRITEBACK.md`.
