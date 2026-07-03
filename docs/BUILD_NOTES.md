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

## Project Paths

Real project create/open commands use configured CLI/env paths when provided. The native shell falls back to an `rfd` folder picker when no path is configured.

The native shell supports drag-resizing the project and inspector side panels. Widths persist in `.elcarax/shell-layout.toml`.

Editable inspector properties render as text fields. Click a value, type, and press Enter to commit. Escape cancels editing.

**Ctrl+S** saves the loaded project scene. Unsaved project scenes show `*` in the toolbar and scene panel. Project close/open/switch is blocked until the scene is saved or reloaded with `scene.load`.

CLI:

```bash
cargo run -p elcarax_app -- --create-project /path/to/project --project-name "My Elcarax Project"
cargo run -p elcarax_app -- --project /path/to/project
```

Environment variables:

- `ELCARAX_PROJECT_CREATE_PATH`
- `ELCARAX_PROJECT_PATH`
- `ELCARAX_PROJECT_NAME`
- `ELCARAX_RECENT_PROJECTS_PATH`

Recent projects persist to `.elcarax/recent-projects.toml` by default.

## Native Shell

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell is a manual desktop smoke test. It opens an `Elcarax` window through `winit`, initializes `wgpu`, builds the UI shell through `elcarax_ui`, routes pointer and keyboard input into the UI tree and command palette, renders primitive rectangles/lines through `elcarax_render`, and renders static labels through `elcarax_text`.

The toolbar `Open` button executes `project.open`, using a configured path when present or the native folder picker otherwise.

## Windows MSVC Linker (LNK1104)

If `link.exe` fails with `LNK1104` while building a dependency build script (for example `rfd`), MSVC cannot write linker temp files. This usually means the machine `TMP`/`TEMP` values are missing, invalid, or list multiple directories. The workspace `.cargo/config.toml` forces `TMP` and `TEMP` to the checked-in `.msvc-tmp/` directory for all Cargo builds in this repo.

If the error persists after a clean rebuild, check that no antivirus is locking `target/` or your cargo registry, and that no stale `elcarax_app` process is holding build outputs:

```powershell
cargo clean
cargo run -p elcarax_app --features native-shell
```

Ctrl+K should open the command palette. Typing `ready` and pressing Enter should execute `Show Ready Status` and update the status text to `Ready - open a project or connect an adapter`. Escape should close the palette without executing a command.

Typing `project.create`, `project.open`, `project.validate`, `project.close`, `project.show_recent`, or `project.reopen_last` in the command palette should update the status bar and project panel. `project.create` and `project.open` use configured paths when present, otherwise the native folder picker.

`asset.scan` scans the loaded project's asset root. Without a loaded project it reports `No project open`. `project.open` prepares the asset root but does not scan automatically.

`asset.refresh` rescans the loaded project's asset root and clears dirty state. `asset.start_watching` starts the asset-root watcher, `asset.stop_watching` stops it, and filesystem changes mark the asset index dirty until refresh. Watcher events are drained nonblocking by the app layer; CI tests use synthetic events instead of flaky real-time watcher timing.

`asset.clear_selection`, `asset.show_selected`, and `asset.reveal_root` operate on the current real asset index. They do not rename, move, delete, preview, import, or reveal through an OS shell yet.

`scene.load` loads the active scene from the open project's `scenes/` directory. `scene.save` writes a loaded project-owned scene back to disk and syncs `[editor].active_scene` in `elcarax.project.toml`. `scene.clear` and `scene.clear_selection` remain available on empty or loaded scenes.

Project open/create/reopen runs through `EditorSession`, which binds the asset root, auto-loads the active scene, resets inspector state, and clears undo/redo history.

`inspector.clear`, `edit.undo`, and `edit.redo` remain registered. With no loaded scene or selected object, the inspector should show `No object selected`.

Editable inspector tests still cover command-history mutation through fixtures, but normal runtime no longer registers fixture property-edit commands.

`adapter.connect`, `adapter.handshake`, `adapter.load_project`, `adapter.load_scene`, `adapter.disconnect`, `adapter.show_status`, and `adapter.show_diagnostics` are the normal runtime command names. With the `native-shell` feature, `adapter.connect` spawns the stdio game adapter process; the console build still reports `No adapter configured` for `adapter.connect`. Viewport preview uses `adapter.load_project` (when required by the adapter) followed by `viewport.request_frame`.

Adapter-backed writeback remains covered through adapter tests and reference scene fixtures. Normal UI widgets emit editor actions only and do not spawn adapter processes directly.

The console proof prints empty startup state, exercises real temporary project create/open/validate with auto-loaded default scene, writes real files under `assets/`, runs `asset.scan`, reports asset kinds, selects the first asset, modifies the asset folder, runs `asset.refresh`, proves project close clears asset state, reopens the last project, proves scene save and document round-trip, then runs the viewport proof with the stdio game adapter.

`viewport.request_frame`, `viewport.clear`, and `viewport.show_status` are registered viewport commands. Without a connected adapter, `viewport.request_frame` reports `No adapter connected`. With a connected adapter that supports viewport preview, the center viewport should display the adapter-provided RGBA frame.

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
- `elcarax_assets` owns filesystem asset scanning, stable path-derived asset IDs, metadata, diagnostics, index snapshots, and the contained `notify` watcher service abstraction.
- `elcarax_app` owns `EditorSession` / `EditorSessionState`, app-level project/asset/scene/inspector/viewport/adapter state composition, routes local edits through command history, routes adapter-backed edits through adapter writeback, then pushes display text into the UI tree

## Current Exclusions

The current shell deliberately excludes full tabbed/floating docking, IME/full caret selection editing, full keybinding registry, fuzzy scoring, command macros, scroll views, real accessibility adapter integration, async command execution, request timeouts, project migration beyond schema version checks, asset thumbnails, asset previews, asset import pipeline, asset drag/drop, asset rename/move/delete, asset dependency graph, asset sidecar metadata, asset build/import cache, hierarchy mutation, hierarchy drag/drop, component add/remove, scene object creation/deletion, multi-scene switcher UI, save-on-close confirmation dialogs, continuous autosave, asset assignment editing, multi-object editing, validation beyond basic type/editability checks, conflict resolution beyond expected-old-value checks, continuous viewport frame streaming, shared GPU texture interop, adapter hot reload, marketplace/plugin runtime loading, dynamic library loading, adapter security sandbox, real engine synchronization, real engine adapter integration, and C++ adapter SDK integration.
