# Contributing to Elcarax

Thank you for your interest in Elcarax. This project is an engine-neutral Rust editor platform; contributions that keep crate boundaries clean and tests focused are especially welcome.

## Before You Start

1. Read [README.md](README.md) for the current scope and architecture overview.
2. Follow [docs/INSTALL.md](docs/INSTALL.md) to build from source.
3. Skim [AGENTS.md](AGENTS.md) for repository conventions used by maintainers and automation.

If you plan a larger change, open an issue or draft PR early so we can confirm it fits the current roadmap ([ROADMAP.md](ROADMAP.md)).

## Development Setup

Install the pinned toolchain (Rust 1.96.0):

```bash
rustup toolchain install 1.96.0
cd Elcarax
```

The workspace uses `rust-toolchain.toml`; `cargo` will select 1.96.0 automatically when run inside the repo.

Run the standard validation gates before opening a PR:

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

Native desktop smoke test (manual, requires a display):

```bash
cargo run -p elcarax_app --features native-shell
```

On Windows, if MSVC linker temp files fail because `TMP` is relative, set absolute temp paths before running Cargo (see [docs/INSTALL.md](docs/INSTALL.md)).

## Where to Put Changes

Elcarax is a workspace of narrow crates. Put behavior in the crate that owns the domain:

| Area | Crate |
|------|-------|
| IDs, diagnostics, viewport camera math | `elcarax_core` |
| Scene/property schema, inspector widgets | `elcarax_scene_model` |
| Commands, keybindings, undo registry | `elcarax_commands` |
| Project files, validation, recents | `elcarax_project` |
| Asset index and watching | `elcarax_assets` |
| Adapter protocol types | `elcarax_adapter_api` |
| Adapter process host / stdio transport | `elcarax_adapter_host` |
| Adapter author helpers | `elcarax_adapter_sdk` |
| Windowing / platform input | `elcarax_platform` |
| GPU surface and context | `elcarax_gpu` |
| Text shaping and rasterization | `elcarax_text` |
| Render primitives and batching | `elcarax_render` |
| Retained UI, layout, widgets | `elcarax_ui` |
| App composition, shell, proof flows | `elcarax_app` |

Keep UI widgets display-only: pass project/scene/adapter state into the UI layer instead of loading or mutating project data inside widgets.

## Coding Style

- Match existing naming and module layout in the file you edit.
- Prefer explicit error propagation; workspace lints forbid `unsafe`, deny `unwrap_used` and `expect_used` in crate code.
- CI also rejects `unwrap(` and `todo!(` under `crates/`.
- Keep command handlers, validation, app state, and UI painting in separate modules.
- Minimize diff scope: one feature or fix per PR.

Format with `cargo fmt --all` before committing.

## Tests

Run the full workspace suite:

```bash
cargo test --workspace
```

Add tests in the crate that owns the behavior:

- Project domain → `elcarax_project`
- Command registry → `elcarax_commands`
- UI presentation → `elcarax_ui`
- App composition / adapter flows → `elcarax_app`

Prefer focused tests over broad integration tests unless the behavior is inherently cross-crate.

## Pull Requests

1. Branch from `main`.
2. Use imperative commit subjects (e.g. `Add scroll views to asset panel`, `Fix asset path normalization on Windows`).
3. Update [CHANGELOG.md](CHANGELOG.md) under `[Unreleased]` for user-visible changes.
4. Note which validation commands you ran in the PR description.
5. Keep PRs scoped to a single feature or fix.

Apache-2.0 applies to contributions; by submitting a PR you agree your work is licensed under the same terms as the project ([LICENSE](LICENSE)).

## Adapter and Engine Integrations

External engine adapters should depend on `elcarax_adapter_api` (protocol types) and optionally `elcarax_adapter_sdk`. See [docs/INSTALL.md](docs/INSTALL.md) for git-based dependency examples until crates are published to crates.io.

## Questions

Open a [GitHub issue](https://github.com/9karma1/Elcarax/issues) for bugs, design questions, or install problems. Include OS, Rust version (`rustc --version`), and the commands you ran.
