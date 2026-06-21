# Elcarax v0.1 scaffold status

Generated: 2026-06-01

## Included

- Rust workspace scaffold targeting Rust 1.96.0 and Edition 2024
- Engine-neutral scene/property/schema model
- Command and undo/redo path
- Adapter API, SDK, host boundary, and mock game adapter
- UI tree and render primitive scaffolds
- Project, asset, GPU, platform, text, accessibility, and devtools modules
- ADRs and theme tokens

## Not included yet

- Real `winit` event loop
- Real `wgpu` surface/device/render passes
- Real `cosmic-text` shaping/rasterization
- Real `AccessKit` adapter integration
- Real process IPC transport
- Real game engine binding

## Validation caveat

This environment did not include `rustc` or `cargo`, so the scaffold could not be compiled here. The first validation step on a development machine should be:

```bash
rustup toolchain install 1.96.0
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p elcarax_app
```
