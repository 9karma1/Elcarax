# Elcarax v0.1

Elcarax is a proprietary Rust editor platform for Elcaro Digital. The first adapter targets a game engine, but the editor core is engine-independent.

This repository is the v0.1 foundation scaffold. It includes:

- engine-neutral core domain types
- command and undo/redo architecture
- adapter protocol stubs
- adapter host and mock game adapter
- UI tree, render primitive, text, accessibility, asset, and project modules
- architecture decision records
- theme tokens

## Important status

This scaffold was generated in an environment without `rustc` or `cargo`, so it has not been compiler-verified here. It is written to be clean, small, and dependency-light so the first real validation step is straightforward on a machine with Rust 1.96.0 installed.

## First local validation

```bash
rustup toolchain install 1.96.0
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p elcarax_app
```

## Current v0.1 behavior

The default executable is a console-backed editor simulation. It proves the editor's core architecture before the native GPU shell is added:

1. creates a project model
2. creates a scene snapshot
3. selects an object
4. edits a property through the command system
5. records undo history
6. undoes the edit
7. exercises the mock adapter path

## Native UI direction

The planned native shell is:

```text
winit 0.30.x -> elcarax_platform
wgpu 29.x   -> elcarax_gpu / elcarax_render
cosmic-text -> elcarax_text
AccessKit   -> elcarax_accessibility
```

Those dependencies are intentionally not required by the default scaffold yet. The next milestone is adding the `native-gpu` feature behind these stable crate boundaries.

## Core rule

The game engine may depend on Elcarax adapter SDK types. Elcarax core must never depend on the game engine.
