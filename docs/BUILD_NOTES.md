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

The toolbar `Open` button should show hover/pressed/focused visual state. Clicking it should update the status text to a clear project-opening diagnostic because file dialog integration is not implemented yet.

Ctrl+K should open the command palette. Typing `ready` and pressing Enter should execute `Show Ready Status` and update the status text to `Ready - open a project or connect an adapter`. Escape should close the palette without executing a command.

Typing `project.create`, `project.open`, `project.validate`, or `project.close` in the command palette should update the status bar and project panel. Creation/opening currently report explicit not-implemented diagnostics instead of loading fixture data.

`asset.scan` should report `No project open` or `No asset root loaded` until a real project/root path exists. `asset.clear_selection` remains available and should be safe on an empty asset list.

`scene.load` should report `No scene source configured` until a project or adapter can provide a real scene. `scene.clear` and `scene.clear_selection` remain available and should be safe on an empty scene.

`inspector.clear`, `edit.undo`, and `edit.redo` remain registered. With no loaded scene or selected object, the inspector should show `No object selected`.

Editable inspector tests still cover command-history mutation through fixtures, but normal runtime no longer registers fixture property-edit commands.

`adapter.connect`, `adapter.load_scene`, `adapter.disconnect`, `adapter.show_status`, and `adapter.show_diagnostics` are the normal runtime command names. `adapter.connect` currently reports `No adapter configured`; the mock adapter remains a test/mock boundary, not a normal user-facing editor flow.

Adapter-backed writeback remains covered through adapter/mock tests and fixture commands. Normal UI widgets emit editor actions only and do not spawn adapter processes directly.

The console proof now performs startup validation only. It prints app initialization, command registry, empty project/asset/adapter/scene/inspector states, empty undo/redo stack counts, UI model stats, renderer/devtools stats, and the ready status line.

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
- `elcarax_adapter_api` owns serializable adapter protocol messages only.
- `elcarax_adapter_host` owns adapter process spawning, JSON-line transport, request correlation, events, and failure handling.
- `elcarax_app` owns app-level project, asset, scene, inspector, and adapter state composition, routes local edits through command history, routes adapter-backed edits through adapter writeback, then pushes display text into the UI tree.

## Current Exclusions

The current shell deliberately excludes docking, drag resizing, real text input fields, IME, caret/selection editing, full keybinding system, fuzzy scoring, command macros, scroll views, real accessibility adapter integration, file dialogs, file watching, async command execution, request timeouts, project migration, persistent recent-project storage, asset thumbnails, asset import pipeline, hierarchy mutation, hierarchy drag/drop, component add/remove, scene object creation/deletion, asset assignment editing, multi-object editing, validation beyond basic type/editability checks, conflict resolution beyond expected-old-value checks, viewport scene rendering, viewport frame streaming, scene save/writeback, adapter hot reload, marketplace/plugin runtime loading, dynamic library loading, adapter security sandbox, real engine synchronization, real engine adapter integration, and C++ adapter SDK integration.
