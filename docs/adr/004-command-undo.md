# ADR-004: Commands and undo/redo

## Decision

Every user-visible mutation goes through a command and transaction path.

## Rationale

Editor actions must be inspectable, reversible, testable, and eventually scriptable.

## Consequences

- Panels do not mutate project or scene state directly.
- Commands provide `apply` and `revert` behavior.
- The undo stack is part of the editor foundation, not a plugin.
