# Milestone 9: Scene Tree Foundation

Milestone 9 introduces a neutral scene/object hierarchy display for Elcarax using `elcarax_scene_model`. The goal is to prove Elcarax can show scene structure without depending on a real game engine yet.

## Included

- `elcarax_scene_model` domain types: `SceneId`, `SceneName`, `SceneSnapshot`, `SceneObjectId`, `SceneObject`, `SceneObjectName`, `SceneObjectKind`, `SceneHierarchy`, `SceneSelection`, `SceneExpansion`, `SceneDiagnostic`, and `SceneError`
- Deterministic demo scene snapshot with stable object IDs and parent/child relationships
- App-layer `SceneState` separate from UI widgets
- Command palette commands:
  - `scene.load_demo`
  - `scene.select_root`
  - `scene.select_player`
  - `scene.clear_selection`
  - `scene.expand_all`
  - `scene.collapse_all`
  - `scene.show_selected`
- Left project panel scene section with scene name, hierarchy rows, expand/collapse markers, selected summary, and clickable rows in the native shell
- Combined status bar text including project, asset, scene, and selected object
- Console proof flow covering project load, asset scan, scene load, selection, expand/collapse, and clear selection

## Behavior Notes

- `scene.load_demo` loads an in-memory deterministic demo scene. It does not require a loaded project or adapter connection.
- `scene.select_root` selects the `World` root object.
- `scene.select_player` selects the `Player` object by name.
- Expand/collapse markers use simple ASCII: `v` expanded, `>` collapsed, `-` leaf.
- Scene row buttons and expand icon buttons are clickable in the native shell.
- Console proof runs scene commands after asset selection and before project validate/close.

## Demo Scene Hierarchy

```
Demo Scene
  World
    Directional Light
    Main Camera
    Player
      Player Mesh
      Player Audio
    Environment
      Ground
      Cube
      Trigger Zone
```

## Explicit Exclusions

This milestone does not add:

- real adapter connection
- real engine scene loading
- hierarchy drag/drop
- object creation/deletion
- property inspector editing
- component editing
- scene save/writeback
- viewport rendering
- gizmos
- keyboard tree navigation

## Validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

Manual native shell smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

Suggested manual flow:

1. Open the native shell
2. Run `project.new_demo`
3. Run `asset.scan_demo`
4. Run `scene.load_demo`
5. Confirm the scene tree appears in the left panel
6. Select `Player` using a command or click
7. Confirm the selected row highlights and the status bar reports the selected object
8. Run `scene.expand_all` and `scene.collapse_all`
9. Confirm resizing does not panic and closing exits cleanly
