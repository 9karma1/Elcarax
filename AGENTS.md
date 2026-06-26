# Repository Guidelines

## Project Structure & Module Organization

Elcarax is a Rust workspace of narrow crates. `elcarax_core`, `elcarax_scene_model`, `elcarax_commands`, `elcarax_adapter_api`, and `elcarax_project` are engine-, GPU-, window-, renderer-, UI-, and text-library-neutral. `elcarax_platform` owns `winit`; `elcarax_gpu` owns `wgpu`; `elcarax_text` owns `cosmic-text`; `elcarax_render` owns primitives, batching, and draw submission; `elcarax_ui` owns retained UI state, layout, input, paint output, and command palette presentation. `elcarax_app` composes console proof and native shell behavior. Keep new feature logic in the owning crate and pass display state into UI rather than letting widgets load or mutate project data.

## Build, Test, and Development Commands

Use Rust 1.96.0 from `rust-toolchain.toml`. Standard gates are:

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

Run a single crate test with `cargo test -p elcarax_project`. Native desktop smoke testing is `cargo run -p elcarax_app --features native-shell`.

## Coding Style & Naming Conventions

Workspace lints forbid unsafe code and deny `unwrap_used` and `expect_used`. CI also rejects `unwrap(` and `todo!(` under `crates/`. Prefer explicit error propagation through existing error types. Keep command handlers, validation, app state, and UI painting in separate modules.

## Testing Guidelines

Tests are Rust unit/doc tests run by `cargo test --workspace`. Add focused tests in the crate that owns the behavior: project-domain tests in `elcarax_project`, registry tests in `elcarax_commands`, presentation tests in `elcarax_ui`, and composition tests in `elcarax_app`.

## Commit & Pull Request Guidelines

Recent history uses imperative milestone subjects such as `Milestone 6: Add command palette shell`, plus concise maintenance subjects like `Stop tracking build outputs`. Keep PRs scoped to one milestone or fix, and include the validation commands that were run.
