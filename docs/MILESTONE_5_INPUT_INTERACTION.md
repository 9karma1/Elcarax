# Milestone 5: Input and Interaction Foundation

Milestone 5 makes the retained UI tree interactive for the first time while keeping platform, UI, and rendering responsibilities separate.

## Included

- `elcarax_platform` translates `winit` window/input events into Elcarax-owned platform events.
- `elcarax_ui` owns UI-facing input events, pointer positions, pointer buttons, keyboard keys, modifier state, hit-test results, focus changes, and interaction state.
- Hit testing runs against final layout rectangles and resolves the deepest/topmost eligible widget with stable traversal order.
- UI nodes track hover, focus, active/pressed, disabled, visible, focusable, and interactive state.
- Interaction state changes mark paint, hit-test, and accessibility dirty flags where appropriate.
- `Button` and `IconButton` placeholder widget kinds exist; the demo uses a `Run` button.
- Buttons emit clicks on pointer release inside the same pressed button.
- Focused buttons activate from Enter and Space.
- Tab focus traversal has a simple forward-only foundation.
- The native shell keeps a persistent UI tree, processes input events, repaints only after scene-affecting changes, and stays idle when no redraw is needed.
- The console proof simulates a Run button click and updates the status label without requiring a window or GPU.

## Demo Behavior

The toolbar contains a `Run` button. Clicking it updates the status label to:

```text
Status: Run clicked
```

This proves pointer routing, hit testing, button press/release handling, UI event emission, status text mutation, and repaint through the existing text rasterization path.

## Explicit Exclusions

- command palette
- editable text fields
- IME
- selection
- drag resizing
- docking
- asset browser actions
- inspector editing
- adapter commands
- accessibility implementation beyond placeholder dirty flags

## Validation

The non-window validation path remains:

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

Manual desktop smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

Confirm the window opens, labels render with system-font rasterization, the `Run` button changes visual state on hover/press, clicking it updates the status text, resizing does not panic, and closing exits cleanly.
