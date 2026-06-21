# Elcarax v0.1

Elcarax is an open source Rust editor platform for building engine-neutral creative tools. The first adapter target is a game workflow, but the editor core is kept independent from any specific engine or game framework.

The project is licensed under Apache-2.0. See [LICENSE](LICENSE).

## Current State

This repository contains the v0.1 foundation for the Elcarax editor:

- engine-neutral workspace, scene, schema, property, and command types
- command history with undo/redo proof flow
- adapter API, SDK, host boundary, and mock game adapter
- `winit` native shell behind the `native-shell` feature
- `wgpu` surface/context and rectangle primitive rendering
- `cosmic-text` shaping and system-font rasterization through `elcarax_text`
- retained UI tree, layout primitives, dirty flags, style/theme resolution, and paint output
- static editor shell with toolbar, project panel, viewport, inspector, and status bar
- project, asset, accessibility placeholder, and devtools modules
- architecture decision records and milestone documentation

This is not a full editor yet. Docking, drag resizing, tree views, asset browser behavior, inspector editing, command palette, text input, scroll views, real accessibility integration, process IPC, and real engine binding are intentionally out of scope for the current milestone.

## Requirements

- Rust 1.96.0
- Windows, macOS, or Linux for console/library validation
- A desktop session for the manual `native-shell` smoke test

Install the pinned Rust toolchain:

```bash
rustup toolchain install 1.96.0
```

## Validation

Run the core quality gates:

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

On Windows, if MSVC linker temp files fail because `TMP` is relative, use absolute temp paths for Cargo commands:

```powershell
New-Item -ItemType Directory -Force -Path D:\elcarax_v0_1\target\tmp | Out-Null
$env:TMP='D:\elcarax_v0_1\target\tmp'
$env:TEMP='D:\elcarax_v0_1\target\tmp'
```

## Running

Default console proof flow:

```bash
cargo run -p elcarax_app
```

The console flow builds the UI shell without opening a GPU window and prints project, command history, render, UI node, layout, primitive, text primitive, and dirty flag counts.

Manual native shell smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell opens an `Elcarax` window, initializes `wgpu`, builds the UI shell through `elcarax_ui`, paints it into a render scene, renders static labels through the `elcarax_text` rasterizer, handles resize/DPI/events, and exits cleanly on close.

## Architecture

Elcarax keeps external systems behind crate boundaries:

- `elcarax_core`: foundational IDs, errors, diagnostics, workspace types
- `elcarax_scene_model`: engine-neutral scene/property/schema model
- `elcarax_commands`: command and undo/redo behavior
- `elcarax_adapter_api`: stable adapter boundary
- `elcarax_platform`: platform event loop and native window integration
- `elcarax_gpu`: `wgpu` context, surface, and render-pass helpers
- `elcarax_text`: `cosmic-text` shaping, layout cache, and system-font rasterization
- `elcarax_render`: editor render primitives, batching, GPU rendering, and render stats
- `elcarax_ui`: retained UI tree, layout, dirty flags, styles, and paint output
- `elcarax_app`: composition layer for console proof and native shell

The game engine may depend on Elcarax adapter SDK types. Elcarax core crates must not depend on the game engine.

## Milestones

- Milestone 1: native shell foundation
- Milestone 2: GPU render primitive pipeline
- Milestone 3: text rendering foundation
- Milestone 4: UI tree and layout foundation

See `docs/` for detailed milestone notes and ADRs.
