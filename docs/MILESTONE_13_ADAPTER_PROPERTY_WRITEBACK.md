# Milestone 13: Adapter Property Writeback Foundation

Milestone 13 proves the first adapter-backed edit loop:

```text
inspector edit intent
-> app command routing
-> adapter set-property request
-> mock adapter validation and mutation
-> confirmed scene patch
-> editor scene state update
-> inspector refresh
-> adapter-backed undo/redo writeback
-> diagnostic/status update
```

The implementation uses the mock adapter only. It does not connect a real game engine.

## Included

- Expanded `elcarax_adapter_api` with writeback protocol messages:
  - `SetPropertyRequest`
  - `SetPropertyResponse`
  - `AdapterEditSource`
  - `SetPropertyStatus`
- Added accepted/rejected writeback states for object-not-found, property-not-found, read-only property, type mismatch, stale expected value, and adapter error results.
- Added optional expected-old-value checking to writeback requests.
- Added a minimal `ScenePatch` model in `elcarax_scene_model` with property update operations only.
- Added scene patch application with object lookup, property lookup, type validation, and clear errors instead of panics.
- Updated the mock adapter to keep a mutable demo scene snapshot in memory.
- Updated the mock adapter to validate and apply property writes before returning confirmed values and patches.
- Added adapter-backed inspector commands:
  - `adapter.inspector.set_player_health_demo`
  - `adapter.inspector.set_player_speed_demo`
  - `adapter.inspector.rename_player_demo`
  - `adapter.edit.undo`
  - `adapter.edit.redo`
- Added scene source tracking:
  - `SceneSource::None`
  - `SceneSource::LocalDemo`
  - `SceneSource::Adapter(AdapterId)`
- Routed local demo edits through the existing local command-history path.
- Routed adapter-backed edits through adapter writeback and applied editor state only after adapter confirmation.
- Routed adapter-backed undo and redo through adapter writeback.
- Added diagnostics for disconnected adapter, non-adapter scene source, no selection, missing object, missing property, read-only property, type mismatch, stale expected value, and rejected adapter writes.
- Preserved project, asset, scene, inspector, local edit, local undo/redo, adapter host, adapter scene import, and read-only inspector flows.

## Protocol

`SetPropertyRequest` carries:

- scene ID
- object ID
- property path
- optional expected old value
- new value
- transaction ID or editor command ID
- edit source

`SetPropertyResponse` carries:

- result status
- scene ID
- object ID
- property path
- old value when available
- confirmed new value when accepted
- optional `ScenePatch`
- diagnostics

All messages remain JSON lines with one request or response per line and request IDs for correlation.

## Scene Patch

The scene patch model is intentionally narrow:

- `ScenePatch`
- `ScenePatchOperation::PropertyUpdated`
- `PropertyUpdated`

Only property updates are implemented. Patch application validates the target object, property path, and value type before replacing the property value.

## Mock Adapter

The mock adapter now advertises:

- `provides_project_info: true`
- `provides_scene_snapshot: true`
- `provides_diagnostics: true`
- `supports_property_writeback: true`
- `supports_viewport_preview: false`

The adapter mutates its internal demo scene for accepted writes, so a later scene snapshot reflects the confirmed edit.

## Console Proof Flow

The default proof flow still runs without a GPU window:

```bash
cargo run -p elcarax_app
```

It demonstrates:

1. Local path: `project.new_demo`, `scene.load_demo`, `scene.select_player`, `inspector.set_player_health_demo`, `edit.undo`, and `edit.redo`
2. Adapter path: `adapter.start_mock`, `adapter.load_demo_scene`, `scene.select_player`, and adapter-backed health edit
3. Confirmed adapter update applied to the inspector value
4. `adapter.edit.undo` sends reverse writeback and restores the old value after confirmation
5. `adapter.edit.redo` sends forward writeback and reapplies the new value after confirmation
6. `adapter.stop_mock`

## Failure Handling

Adapter write failures return diagnostics and do not mutate editor scene state locally.

Covered failures include:

- adapter not connected
- scene not adapter-backed
- no selected object
- object not found
- property not found
- read-only property
- type mismatch
- stale expected value
- adapter rejected edit
- adapter process or transport failure

## Architecture

- `elcarax_adapter_api` owns serializable writeback protocol types.
- `elcarax_scene_model` owns property patch validation and apply logic.
- `elcarax_adapter_host` owns request/response transport and correlation.
- `elcarax_game_adapter` owns mutable mock adapter demo scene state.
- `elcarax_app` routes edits based on scene source and composes adapter writeback with scene state and undo/redo history.
- `elcarax_ui` displays adapter status, diagnostics, inspector rows, and edit affordances only.
- Widgets do not call adapter transport directly.
- The adapter API remains engine-neutral.

## Explicit Exclusions

- Real game engine integration
- C++ SDK
- Binary protocol
- Shared memory
- Viewport streaming
- Hierarchy mutation
- Component add/remove
- Asset assignment editing
- Persistent save
- Conflict resolution beyond expected-old-value checks
- Multi-object editing
- Collaborative editing
- Real engine synchronization

## Validation

Expected CI/local validation:

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

Native shell remains a manual smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```
