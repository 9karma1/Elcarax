# Milestone 12: Adapter Host Integration

Milestone 12 proves the first external adapter loop:

```text
command palette
-> adapter host
-> spawned mock adapter process
-> versioned JSON-line handshake
-> request/response correlation
-> scene snapshot import
-> diagnostics/log status
-> clean shutdown or failure diagnostic
```

The implementation uses a deterministic mock adapter. It does not connect a real game engine.

## Included

- Expanded `elcarax_adapter_api` with stable serializable protocol types:
  - `ProtocolVersion`
  - `AdapterId`
  - `AdapterName`
  - `AdapterVersion`
  - `AdapterCapabilities`
  - `AdapterRequestId`
  - `AdapterRequest`
  - `AdapterResponse`
  - `AdapterEvent`
  - `AdapterDiagnostic`
  - `AdapterLog`
  - `AdapterError`
- Added JSON-line protocol helpers for one request, response, or event per line.
- Added request IDs on every request and response.
- Added Milestone 12 capabilities:
  - project info
  - scene snapshot
  - diagnostics
  - property writeback flag
  - viewport preview flag
- Implemented `elcarax_adapter_host` process and session concepts:
  - `AdapterHost`
  - `AdapterProcess`
  - `AdapterSession`
  - `AdapterTransport`
  - `AdapterHostState`
  - `AdapterHostError`
- Contained process spawning, stdin writes, stdout reads, JSON parsing, event collection, request correlation, failure detection, and shutdown in `elcarax_adapter_host`.
- Converted `elcarax_game_adapter` into a mock JSON-line stdio adapter.
- Added adapter app state for status, name/version, capabilities, diagnostics, last result, and scene import.
- Added command-palette commands:
  - `adapter.start_mock`
  - `adapter.handshake`
  - `adapter.load_demo_project`
  - `adapter.load_demo_scene`
  - `adapter.show_status`
  - `adapter.show_diagnostics`
  - `adapter.stop_mock`
- Added adapter status, diagnostic count, and last adapter result to the editor shell.
- Added console proof coverage for starting the mock adapter, handshaking, loading project info, importing a scene snapshot, showing diagnostics, and stopping the adapter.
- Preserved local project, asset, scene, inspector edit, undo, and redo flows.

## Mock Adapter

`elcarax_game_adapter` is a mock process for this milestone. It reads adapter requests from stdin and writes responses/events to stdout as JSON lines.

Mock capabilities:

- `provides_project_info: true`
- `provides_scene_snapshot: true`
- `provides_diagnostics: true`
- `supports_property_writeback: false`
- `supports_viewport_preview: false`

The app starts the mock adapter through `cargo run --quiet -p elcarax_game_adapter` during the console proof and native command flow.

## Console Proof Flow

The default proof flow still runs without a GPU window:

```bash
cargo run -p elcarax_app
```

It demonstrates:

1. Existing local flow: `project.new_demo`, `asset.scan_demo`, `scene.load_demo`, `scene.select_player`, `inspector.show_selected`, `inspector.set_player_health_demo`, `edit.undo`, and `edit.redo`
2. `adapter.start_mock`
3. handshake success and mock capabilities
4. `adapter.load_demo_project`
5. `adapter.load_demo_scene`
6. scene tree state updated from the adapter snapshot
7. `adapter.show_diagnostics`
8. `adapter.stop_mock`

## Failure Handling

The host and app return diagnostics instead of panicking for:

- missing adapter executable
- adapter exit before response
- invalid JSON response
- unsupported protocol version
- mismatched request ID
- adapter error responses

Request timeout is still excluded because no timing infrastructure exists yet.

## Architecture

- `elcarax_adapter_api` owns serializable protocol types only.
- `elcarax_adapter_host` owns process, transport, session state, request correlation, events, and failure handling.
- `elcarax_game_adapter` is a mock adapter executable that depends on adapter API and scene model demo data.
- `elcarax_app` composes adapter host commands with scene state import and UI snapshots.
- `elcarax_ui` displays adapter labels only.
- Widgets do not spawn adapter processes or parse adapter messages.
- Adapter writeback is not connected.

## Explicit Exclusions

- Real game engine adapter
- C++ adapter SDK
- Binary protocol
- Shared memory
- Viewport frame streaming
- Property writeback
- Adapter hot reload
- Marketplace or plugin system
- Dynamic library loading
- Adapter security sandbox
- Request timeout
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
