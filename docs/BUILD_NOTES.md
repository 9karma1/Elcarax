# Build notes

## Why the first scaffold is std-only

The editor should eventually use `winit`, `wgpu`, `cosmic-text`, and `AccessKit`. The first scaffold keeps the default build dependency-free so the core architecture can be validated independently of graphics APIs and platform setup.

## Next build step

Add a `native-gpu` feature that wires:

- `elcarax_platform` to `winit`
- `elcarax_gpu` to `wgpu`
- `elcarax_text` to `cosmic-text`
- `elcarax_accessibility` to `accesskit`

The feature should keep all external types behind Elcarax-owned wrapper types.
