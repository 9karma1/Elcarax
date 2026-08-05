# ADR-004: Commands and undo/redo

## Decision

Every user-visible mutation goes through a command and transaction path. Scene mutations are expressed as `ScenePatch` operations and executed by `ApplyScenePatchCommand` through a single `CommandHistory`.

## Rationale

Editor actions must be inspectable, reversible, testable, and eventually scriptable. A single mutation contract prevents local, adapter, and hierarchy edits from forking into parallel undo stacks.

## Consequences

- Panels do not mutate project or scene state directly.
- `ScenePatchOperation` covers property updates and hierarchy ops (`ObjectAdded`, `ObjectRemoved`, `Reparented`, `Renamed`).
- Commands provide `apply` and `revert` behavior against `CommandContext`, which may carry an optional `SceneMutationSink` for adapter-confirmed writeback.
- `SessionEditService` in `elcarax_app` is the sole edit authority: local project scenes apply patches in-process; adapter-backed scenes confirm through the adapter host, then apply the returned patch.
- The undo stack is part of the editor foundation (`CommandHistory`), not a per-adapter parallel history.
