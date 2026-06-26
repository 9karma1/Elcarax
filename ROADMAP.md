# Roadmap

Elcarax v0.1 is building the editor foundation in small milestones while preserving crate boundaries and headless CI proof paths.

## Completed

- Milestone 1: native shell foundation
- Milestone 2: GPU render primitive pipeline
- Milestone 3: text rendering foundation
- Milestone 4: UI tree and layout foundation
- Milestone 5: input and interaction foundation
- Milestone 6: command palette shell
- Milestone 7: project system UI

## Next Milestones

1. Asset browser foundation: visible asset list model, placeholder rows, and command-driven refresh without directory watching.
2. Scene tree foundation: engine-neutral scene hierarchy display and selection state without adapter coupling.
3. Inspector foundation: read-only property display before editable fields, IME, or validation-heavy controls.
4. Project persistence: explicit save/load format, recent-project persistence, and migration policy after the in-memory model stabilizes.
5. Adapter process integration: IPC transport and adapter lifecycle once editor-owned project/scene state is stable.
6. Accessibility integration: wire real accessibility output after retained UI semantics settle.

## Standing Constraints

- Keep core/domain crates free of `wgpu`, `winit`, `cosmic-text`, game engines, and adapter implementations.
- Keep `cargo run -p elcarax_app` as the headless proof path used by CI.
- Keep native shell validation manual with `cargo run -p elcarax_app --features native-shell`.
- Do not add asset scanning, file watching, scene editing, inspector editing, or adapter loading until the relevant milestone explicitly owns it.
