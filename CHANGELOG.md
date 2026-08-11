# Changelog

All notable changes to Elcarax are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once tagged releases begin.

## [Unreleased]

### Added

- Open scene authoring type system: string `SceneObjectKind`, `PropertyValue::Extension`, and public `components` / `kinds` well-known constant modules
- Atomic scene transaction kernel with strict hierarchy validation, persisted-ID observation, and rollback on failed multi-operation patches
- Registry-backed property extension authoring with handler-owned parse, validate, and display behavior
- Typed `EditorCommandRouter` covering every registered executable command for console, toolbar, palette, shortcut, adapter, and viewport dispatch
- Component slots on scene objects (`ComponentInstance`, `ComponentAdded` / `ComponentRemoved` patches) with component-scoped property paths and inspector rows
- Scene file schema version 2 storing per-object components instead of a flat property bag
- `component_id` on adapter `SetPropertyRequest` / `SetPropertyResponse` and editor property commits
- Binary adapter framing (`ELCX` header + JSON + optional binary segment) with viewport pixels out of JSON
- Non-blocking `AdapterHost` worker thread with `submit` / `poll` (sync helpers wait on the worker)
- Unified scene mutation contract: `ScenePatch` operations for property updates plus hierarchy (`ObjectAdded`, `ObjectRemoved`, `Reparented`, `Renamed`) with invertible hierarchy patches
- `ApplyScenePatchCommand` and `SceneMutationSink` as the single undoable mutation path through `CommandHistory`
- `SessionEditService` as the sole edit authority for inspector commits and `edit.undo` / `edit.redo` (local and adapter-backed scenes)
- SceneSnapshot APIs `add_object`, `remove_object`, `reparent_object`, and `rename_object` that return applied patches
- Project-owned scene files (`*.elcarax.scene.toml`) with `scene.load`, `scene.save`, document dirty tracking, unsaved project close guards, manifest `[editor].active_scene` sync, toolbar `*` affordances, and native-shell **Ctrl+S**
- Default `scenes/main.elcarax.scene.toml` written on project create
- `EditorSession` / `EditorSessionState` coordinator for unified project open/close/switch, dependent state binding, and undo history reset across console and native shell
- `EditorSessionPolicy` hook (e.g. future `scan_assets_on_open`)
- Editable inspector text fields (click value, Enter to commit, Escape to cancel)
- Resize cursor affordances on resizable side panels
- Render shadow/elevation primitives for editor chrome, with hard-edge fallback shadows and inner highlights for toolbar and panels
- Explicit font family/weight text primitives, a 5-step editor type scale, shared text baseline math, layout gaps, and full-width list-item rows for asset and scene panels
- Theme-backed info rows and clipped text paint so toolbar/status/project/inspector facts render as bounded components instead of loose labels
- Component-level UI polish for buttons, property rows, and the command palette: disabled button text/fill states, aligned inspector label/value rows, palette scrim/shadow, and row-backed palette entries
- Width-constrained text primitives so clipped UI text is constrained during layout/rasterization and cannot bleed out of component rows
- Scroll-view foundation for asset, scene, and inspector row regions with wheel input, offset-backed presentation windows, and scrollbar paint metrics
- Viewport camera and letterboxed frame layout with scroll-to-zoom, Alt/middle-drag pan, actionable empty states, and scene-object picking from normalized viewport coordinates
- Schema-driven inspector value widgets: toggles, number steppers, vector fields, and enum selectors wired through `InspectorValueWidget` descriptors
- `PropertyEditKind::Enum` with `PropertySchema::editable_enum` and enum variant validation on property commits
- Declarative command metadata, platform-neutral key chords, default keybindings, conflict diagnostics, command availability reasons, help summaries, and a toolbar action snapshot generated from the command registry
- Native-shell adapter launch configuration through `ELCARAX_ADAPTER_EXE`, `ELCARAX_ADAPTER_PROJECT_PATH`, `ELCARAX_ADAPTER_AUTO_CONNECT`, `--adapter`, `--adapter-project`, and `--auto-connect-adapter`, with startup auto-connect/load for engine-provided adapters
- Adapter-backed inspector property commits routed through `EditorSession` and `AdapterState::commit_inspector_property`, with native-shell `AdapterHost::set_property` writeback and adapter undo history population
- Viewport interaction no longer reloads adapter scenes on pointer hover; local `ViewportCamera` stays aligned with pan/zoom/orbit for pick UV math; adapter pick misses no longer fall through to local geometric pick
- `adapter.load_scene` respects unsaved project scene guards; `EditorSession` releases adapter/viewport bindings on project open/close/switch via `EditorShellContext`
- Mock game adapter honors viewport `camera_input` / `editor_input`, supports frames up to 1024px, and renders at the requested resolution
- [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/INSTALL.md](docs/INSTALL.md) for contributor workflow and source/git install instructions (crates.io publishing not yet available)

### Changed

- Scene model properties live on components; inspector, patches, edits, and adapter writeback address `(object_id, component_id, path)` instead of dotted paths like `gameplay.health`
- Reference scene and all in-repo fixtures migrated to component-based schemas
- Adapter stdio transport replaced JSON-lines with binary frames; `GetViewportFrameResponse` uses `byte_len` + binary payload instead of JSON `pixels`
- ADR-003 updated for framed transport and worker-hosted adapter I/O

- Route project-bound lifecycle through `EditorSession` instead of `project_effects.rs` (removed)
- Property edits apply through `ScenePatch` instead of direct property map writes on the command path
- Native-shell adapter `SetProperty` writeback uses `AdapterHost` (no longer test-only)
- Inspector property commits on adapter-backed scenes confirm through the adapter before updating the local snapshot
- Console proof covers command binding diagnostics, toolbar snapshot construction, Ctrl+S-equivalent scene save dispatch through the typed router, and scene document save round-trip after project reopen
- Native shell shortcut handling now converts platform input into registry key chords and dispatches toolbar, palette, and shortcut commands through one command execution path
- Scene construction and inspector commits now use patch-backed request objects; superseded direct root attachment and argument-heavy edit paths were removed
- Beta cleanup: removed in-memory asset demo index and `examples/demo_project`; renamed `demo_scene_snapshot` to `reference_scene_snapshot`; consolidated adapter capabilities to `AdapterCapabilities::stdio_game_adapter()`
- Documentation: milestone markdown files replaced by this changelog; README, STATUS, and BUILD_NOTES updated for scroll views, inspector widgets, viewport camera behavior, toolbar actions, and keybindings

### Removed

- `SetScenePropertyCommand` / `SetScenePropertiesCommand` (replaced by `ApplyScenePatchCommand`)
- `AdapterEditHistory` and `adapter.edit.undo` / `adapter.edit.redo` parallel undo path

## [0.1.0] - 2026-07-03

First foundation release: engine-neutral editor platform scaffold through real project/asset workflows. Headless console proof remains the default CI path; native shell is feature-gated.

### Added

- Workspace scaffold, crate boundaries, Apache-2.0 license, and CI quality gates (`fmt`, `check`, `clippy`, `test`, console proof)
- **Native shell** (`native-shell` feature): `winit` event loop in `elcarax_platform`, `wgpu` surface/context in `elcarax_gpu`, dark clear-color frames, resize/DPI handling
- **GPU render primitives** in `elcarax_render`: rectangles, borders, lines, clip metadata, batching, render stats
- **Text rendering** in `elcarax_text`: `cosmic-text` shaping, system-font rasterization, layout cache; static labels in shell
- **Retained UI** in `elcarax_ui`: widget tree, layout (stacks, splits), theme/styles, dirty flags, paint output; toolbar, panels, status bar, viewport placeholder
- **Input and interaction**: hit testing, hover/focus/pressed state, keyboard focus traversal, basic `Button` / `IconButton`
- **Command palette**: registry in `elcarax_commands`, query filter, keyboard navigation, Ctrl+K in native shell
- **Project system UI**: project domain types in `elcarax_project`, project panel, validation diagnostics, recent-project count, palette commands
- **Asset browser foundation**: asset domain types, demo index (later replaced), selection, project-panel rows, palette commands
- **Scene tree foundation**: engine-neutral `elcarax_scene_model`, hierarchy display, selection/expand state, palette commands
- **Read-only inspector**: property formatting, grouped rows, selection-driven updates
- **Editable inspector undo**: command-driven primitive property edits, undo/redo, diagnostics
- **Adapter host**: JSON-line stdio protocol, process spawn, handshake, request correlation, mock `elcarax_game_adapter`, scene snapshot import, palette commands
- **Adapter property writeback**: set-property protocol, confirmed scene patches, adapter-backed inspector edit/undo/redo
- **Viewport preview**: viewport state, adapter RGBA frame protocol, `RenderPrimitive::Image`, viewport commands, console adapter viewport proof
- **Productionized empty runtime**: no fake project/asset/scene/adapter loaded at startup; honest empty UI states and explicit diagnostics
- **Real project persistence**: `elcarax.project.toml` (schema v1), create/open/validate/close, recent projects, CLI/env paths, native folder picker (`rfd`), default `assets/` / `scenes/` / `.elcarax/` on create
- **Real asset index and file watching**: path-derived stable `AssetId`, filesystem scan/refresh, `notify`-backed watcher, asset commands (`asset.scan`, `asset.refresh`, watch start/stop, selection), dirty/ready UI states; explicit scan on open (not automatic)

### Security

- Workspace forbids `unsafe`; lints deny `unwrap`/`expect` in crate code
