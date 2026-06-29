# Milestone 14A: Productionize Runtime

Milestone 14A removes demo-first runtime behavior from the normal Elcarax app path. The editor now starts as an empty product shell instead of pretending that a project, asset index, scene, selected object, viewport source, or adapter process already exists.

## Runtime Behavior

Normal startup now reports:

- app initialized
- command registry initialized
- project state: no project open
- asset state: no asset root loaded
- adapter state: disconnected and no adapter configured
- scene state: no scene loaded
- inspector state: no object selected
- undo and redo stacks empty
- UI model can build the empty shell
- status: `Ready - open a project or connect an adapter`

`cargo run -p elcarax_app` performs this startup validation only. It does not run a fake editor session, load fixture data, spawn the mock adapter, mutate properties, or exercise undo/redo as if a user had opened real content.

## Command Cleanup

Normal runtime command registration now uses product command names:

- `project.create`
- `project.open`
- `project.close`
- `project.validate`
- `asset.scan`
- `asset.clear_selection`
- `scene.load`
- `scene.clear`
- `scene.clear_selection`
- `inspector.clear`
- `edit.undo`
- `edit.redo`
- `adapter.connect`
- `adapter.disconnect`
- `adapter.load_scene`
- `adapter.show_status`
- `adapter.show_diagnostics`

Commands that cannot do real work yet return explicit diagnostics such as `Not implemented yet`, `No project open`, `No asset root loaded`, `No scene source configured`, or `No adapter configured`. Removed fixture-style command names are not exposed through the normal command palette.

## Fixture Boundaries

Useful fixture coverage was preserved instead of deleted:

- project, asset, scene, inspector, and adapter writeback tests use fixture constructors or in-test fixture data
- the mock adapter remains a mock boundary for adapter protocol and writeback tests
- older milestone documents remain historical records of the sequence that built the foundation

The normal app composition path no longer imports fixture data to make the UI look busy.

## Mock Adapter Status

`elcarax_game_adapter` remains the deterministic mock stdio adapter used by adapter protocol tests and future developer-only flows. It is not the real game engine adapter, and normal runtime commands no longer start it as user-facing editor behavior.

## UI Empty States

The native shell should open with honest empty states:

- project panel: no project open
- assets: unavailable until a project/root exists
- scene tree: no scene loaded
- viewport: no viewport source
- inspector: no object selected
- adapter: disconnected/no adapter configured
- status: ready to open a project or connect an adapter

## Guards

CI now includes a practical runtime naming guard that fails if fixture-oriented terms leak into the normal app, command registry, or public UI shell source paths. Tests, mock adapter crates, old milestone notes, and domain fixture helpers may still use explicit fixture/mock naming.

## Explicit Exclusions

This milestone does not add:

- viewport streaming
- real game engine integration
- C++ adapter support
- new editor features
- file dialogs
- persistent scene save
- hierarchy mutation
- component add/remove
- asset assignment editing
- mock adapter startup as normal runtime behavior
- automatic fake runtime data loading
