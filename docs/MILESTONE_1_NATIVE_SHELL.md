# Milestone 1: Native shell foundation

Milestone 1 adds the first native Elcarax desktop shell while preserving the existing console proof flow as the default application behavior.

## Scope

Included:

- A `winit`-backed native window path in `elcarax_platform`.
- Platform-neutral Elcarax event types for close, resize, redraw, keyboard, pointer, mouse, and DPI events.
- A `wgpu` context and surface foundation in `elcarax_gpu`.
- Safe surface resize handling, including zero-sized windows.
- A minimal clear-color frame using the Elcarax dark background.
- A feature-gated native entry path in `elcarax_app`.

Intentionally not included:

- Panels, docking, widgets, inspectors, viewports, or asset browser UI.
- Game adapter connection.
- Fragile headless GPU/window integration tests in CI.

## Running

Console proof flow remains the default:

```bash
cargo run -p elcarax_app
```

Native shell path is feature-gated:

```bash
cargo run -p elcarax_app --features native-shell
```

The native path opens a window titled `Elcarax`, initializes GPU resources, clears frames to the dark background, handles resize/DPI/events, and exits on close request.

## CI expectations

CI checks the workspace, all features, clippy, tests, and the default console app path. CI does not run the native window because GitHub-hosted Linux runners do not reliably provide an interactive display/GPU session.

## Architecture guard

The editor-neutral crates remain free of GPU and window dependencies:

- `elcarax_core`
- `elcarax_scene_model`
- `elcarax_commands`
- `elcarax_adapter_api`
