# Build Notes

Elcarax v0.1 is now a multi-crate Rust workspace with both a console proof flow and a feature-gated native shell.

## Rust Toolchain

The workspace targets Rust 1.96.0 and Edition 2024.

```bash
rustup toolchain install 1.96.0
```

## Standard Validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

The default app command is a console proof. It does not require a desktop session or GPU window.

## Native Shell

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell is a manual desktop smoke test. It opens an `Elcarax` window through `winit`, initializes `wgpu`, builds the UI shell through `elcarax_ui`, renders primitive rectangles/lines through `elcarax_render`, and renders static labels through `elcarax_text`.

CI should compile the native-shell feature but should not require opening a desktop window.

## Windows Temp Path Note

If local Windows builds fail with MSVC linker temp-file errors and `TMP` is relative, set absolute temp paths before running Cargo:

```powershell
New-Item -ItemType Directory -Force -Path D:\elcarax_v0_1\target\tmp | Out-Null
$env:TMP='D:\elcarax_v0_1\target\tmp'
$env:TEMP='D:\elcarax_v0_1\target\tmp'
```

## Dependency Boundaries

- `elcarax_core`, `elcarax_scene_model`, `elcarax_commands`, and `elcarax_adapter_api` remain engine-, GPU-, window-, and text-library-neutral.
- `elcarax_platform` owns `winit` integration.
- `elcarax_gpu` owns `wgpu` context and surface integration.
- `elcarax_text` owns `cosmic-text` shaping, layout cache, and system-font rasterization.
- `elcarax_render` owns editor render primitives, batching, render stats, and GPU draw submission.
- `elcarax_ui` owns retained UI tree, layout, dirty flags, theme/style resolution, and paint output.

## Current Exclusions

The current shell deliberately excludes docking, drag resizing, tree views, asset browser behavior, inspector editing, command palette, text input, scroll views, accessibility implementation, process IPC, and real engine adapter integration.
