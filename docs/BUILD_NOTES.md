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

## CI quality gates

GitHub Actions runs the Milestone 0 validation gates for every pull request and every push to `main`. The workflow uses Rust 1.96.0, caches Cargo state, checks formatting, builds the full workspace, runs Clippy with warnings denied, runs all tests, executes the app scaffold, and enforces ripgrep architecture guards for engine/GPU dependency tokens and unfinished shortcuts.

## Milestone 3 text rendering

Useful commands remain unchanged:

```bash
cargo run -p elcarax_app
cargo run -p elcarax_app --features native-shell
```

The native-shell command is a manual desktop smoke test; CI should not require opening a window.
