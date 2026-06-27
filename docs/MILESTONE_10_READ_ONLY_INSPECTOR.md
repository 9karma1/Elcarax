# Milestone 10: Read-Only Inspector Foundation

Milestone 10 introduces a read-only inspector panel driven by scene selection and the engine-neutral `elcarax_scene_model` property schema/value model.

## Included

- Property model strengthening: `PropertyId`, `PropertyName`, `PropertyGroup`, `PropertyDisplay`, `PropertyFormatContext`, and `PropertyValue::Unknown`
- Inspector view types: `InspectorObject`, `InspectorSection`, `InspectorRow`, `InspectorDiagnostic`
- Centralized property formatting for bool, int, float, string, vec3, color, asset ref, object ref, and unsupported values
- Demo scene properties on World, Directional Light, Main Camera, Player, and Cube
- App-layer `InspectorState` derived from scene selection
- Command palette commands:
  - `inspector.show_selected`
  - `inspector.clear`
  - `inspector.inspect_player`
  - `inspector.inspect_root`
  - `inspector.show_property_count`
- Right inspector panel with object name, kind, grouped read-only property rows, and empty-state message
- Automatic inspector refresh when scene selection changes
- Console proof flow covering scene selection and inspector show/clear

## Behavior Notes

- Scene selection is the source of truth. Inspector content is rebuilt from the selected scene object.
- `inspector.clear` suppresses the inspector view until selection changes or `inspector.show_selected` runs.
- Clicking a scene row in the native shell updates both scene selection and inspector content.
- Property groups render in stable sorted order (`Gameplay`, `General`, `References`, `Transform`, etc.).
- Section headers and property rows use separate label/value widgets in the inspector panel.

## Explicit Exclusions

This milestone does not add:

- property editing
- undo/redo write operations
- adapter writeback
- real engine properties
- component add/remove
- custom property widgets
- validation beyond read-only diagnostics
- scrollable inspector
- viewport/gizmo integration

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
2. Run `project.new_demo`, `asset.scan_demo`, and `scene.load_demo`
3. Select `Player` via click or `scene.select_player`
4. Confirm the right inspector shows Player name, kind, and read-only properties
5. Run `inspector.clear` and confirm the empty inspector state
6. Select another object and confirm the inspector updates automatically
