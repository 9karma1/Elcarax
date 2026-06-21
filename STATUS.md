# Elcarax v0.1 scaffold status

Generated: 2026-06-21

## Included

- Rust workspace scaffold targeting Rust 1.96.0 and Edition 2024
- Engine-neutral scene/property/schema model
- Command and undo/redo path
- Adapter API, SDK, host boundary, and mock game adapter
- UI tree scaffold
- GPU-backed render primitive pipeline for rectangles, borders, lines, clip metadata, batching, and render stats
- Project, asset, text, accessibility, and devtools modules
- Native shell foundation behind `native-shell`
- `winit` platform event loop contained in `elcarax_platform`
- `wgpu` context/surface/clear-frame foundation contained in `elcarax_gpu`
- ADRs and theme tokens

## Not included yet

- Text rendering, icons, images, and full vector paths
- Full editor UI system
- Panels, docking, widgets, inspector, viewport, or asset browser
- Real `cosmic-text` shaping/rasterization
- Real `AccessKit` adapter integration
- Real process IPC transport
- Real game engine binding
- CI execution of the native window path

## Running

Default console proof flow:

```bash
cargo run -p elcarax_app
```

Feature-gated native shell:

```bash
cargo run -p elcarax_app --features native-shell
```

The native shell opens an `Elcarax` window, initializes `wgpu`, renders the Milestone 2 primitive demo (dark background, toolbar, sidebars, viewport, inspector, status bar, separators, and sample border), handles resize/DPI/events, and exits cleanly on close.

## Validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```
