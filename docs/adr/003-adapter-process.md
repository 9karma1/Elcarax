# ADR-003: Engine adapter boundary

## Decision

Elcarax integrates the first game engine through an adapter boundary. The editor core remains engine-neutral.

## Rationale

The editor must later fit other engines and software domains. The game engine should be the first adapter, not the foundation of the whole editor.

## Consequences

- Game-specific names stay out of `elcarax_core`, `elcarax_ui`, and `elcarax_render`.
- Adapter protocol types live in `elcarax_adapter_api`.
- Adapter author helpers live in `elcarax_adapter_sdk`.
- Process spawning, JSON-line transport, request correlation, adapter events, diagnostics, and process failure handling live in `elcarax_adapter_host`.
- The initial stdio adapter uses one JSON message per line, with mock property writeback through request/response messages and confirmed scene patches.
- Binary protocol, shared memory, continuous viewport frame streaming, dynamic loading, hot reload, and real engine synchronization remain future work. Project-owned scene persistence is handled in-editor via `*.elcarax.scene.toml` (see [CHANGELOG.md](../../CHANGELOG.md)).
