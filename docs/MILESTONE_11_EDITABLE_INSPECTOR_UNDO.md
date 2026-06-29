# Milestone 11: Editable Inspector Undo Foundation

Milestone 11 proves the first command-driven editor mutation loop for scene properties:

```text
UI edit
-> command
-> transaction
-> scene state update
-> inspector refresh
-> undo/redo
-> status/diagnostic update
```

The implementation stays in memory. It does not connect adapter writeback, scene saving, or engine synchronization.

## Included

- Added editable property metadata in `elcarax_scene_model`:
  - `editable`
  - `PropertyEditKind`
  - optional numeric step metadata
  - read-only reason text
  - enum variant storage for future use
- Added model-owned property mutation helpers:
  - `PropertyChange`
  - `PropertyEditError`
  - `PropertyEditResult`
  - `prepare_property_change`
  - `edit_scene_property`
  - `apply_property_change`
- Supported editable value kinds:
  - bool
  - integer
  - float
  - string
  - vec2
  - vec3
- Kept these display-only in this milestone:
  - color
  - asset ref
  - object ref
  - unknown or unsupported values
- Added `SetScenePropertyCommand`, `UndoCommand`, and `RedoCommand` in `elcarax_commands`.
- Added command-palette commands:
  - `inspector.set_player_health_demo`
  - `inspector.set_player_speed_demo`
  - `inspector.rename_player_demo`
  - `inspector.reset_player_transform_demo`
  - `edit.undo`
  - `edit.redo`
- Added app-level inspector edit handling that composes scene edits with `CommandHistory`.
- Added minimal editable inspector affordances by rendering editable value rows as `[Set]` buttons.
- Kept read-only rows muted with read-only reason text.
- Updated console proof flow to show Player health edit, undo, and redo.
- Added diagnostics for no scene, no selection, missing object, missing property, read-only property, and type mismatch cases.

## Console Proof Flow

The default proof flow still runs without a GPU window:

```bash
cargo run -p elcarax_app
```

It demonstrates:

1. `project.new_demo`
2. `asset.scan_demo`
3. `scene.load_demo`
4. `scene.select_player`
5. `inspector.show_selected`
6. `inspector.set_player_health_demo`
7. inspector health changes to `75`
8. `edit.undo`
9. inspector health returns to `100`
10. `edit.redo`
11. inspector health returns to `75`

## Architecture

- `elcarax_scene_model` owns property schema, validation, and snapshot mutation rules.
- `elcarax_commands` owns the command and undo/redo abstractions.
- `elcarax_app` composes inspector edit commands with `CommandHistory`.
- `elcarax_ui` displays editable/read-only row affordances and emits actions only.
- Adapter API and adapter crates are untouched.
- Widgets do not mutate scene state directly.

## Explicit Exclusions

- Real text input fields
- IME, caret, and selection behavior
- Adapter writeback
- Scene save-to-disk
- Component add/remove
- Scene hierarchy mutation
- Asset assignment editing
- Real engine synchronization
- Multi-object editing
- Validation beyond basic type and editability checks
- Real custom property widgets
- Asset drag/drop assignment

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
