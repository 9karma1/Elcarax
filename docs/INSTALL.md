# Installing Elcarax

Elcarax v0.1 is distributed as source from [GitHub](https://github.com/9karma1/Elcarax). Crates are **not yet published to [crates.io](https://crates.io)**; use a git checkout or `cargo install --git` below.

## Requirements

| Requirement | Notes |
|-------------|--------|
| **Rust 1.96.0** | Pinned in `rust-toolchain.toml` (Edition 2024) |
| **OS** | Windows, macOS, or Linux for build and console proof |
| **Desktop session** | Required only for the `native-shell` GPU window |
| **GPU drivers** | Up-to-date drivers recommended for `wgpu` / Vulkan / DX12 / Metal |

Install Rust:

```bash
rustup toolchain install 1.96.0
```

## Build from Source

```bash
git clone https://github.com/9karma1/Elcarax.git
cd Elcarax
cargo build --workspace
```

Console proof (no GPU window):

```bash
cargo run -p elcarax_app
```

Native editor window:

```bash
cargo run -p elcarax_app --features native-shell
```

Release binaries:

```bash
cargo build -p elcarax_app --release
cargo build -p elcarax_app --release --features native-shell
```

The desktop executable is `target/release/Elcarax` (or `target/debug/Elcarax` for debug builds).

Open or create a project from the CLI:

```bash
cargo run -p elcarax_app --features native-shell -- --project /path/to/project
cargo run -p elcarax_app --features native-shell -- --create-project /path/to/new-project --project-name "My Project"
```

## Install with Cargo (from Git)

Install the `Elcarax` binary into `~/.cargo/bin` without cloning a persistent working tree:

```bash
cargo install --git https://github.com/9karma1/Elcarax.git --tag v0.1.0 --path crates/elcarax_app --features native-shell --locked
```

For the latest `main` branch:

```bash
cargo install --git https://github.com/9karma1/Elcarax.git --path crates/elcarax_app --features native-shell --locked
```

Run:

```bash
Elcarax
```

> **Note:** `cargo install --git` builds all workspace path dependencies from the repository. There is no separate `elcarax` crate on crates.io yet.

## Adapter / Library Dependencies (Git)

If you are building an external adapter or tool against Elcarax protocol types, depend on git revisions until crates.io publishing lands:

```toml
[dependencies]
elcarax_adapter_api = { git = "https://github.com/9karma1/Elcarax.git", tag = "v0.1.0" }
elcarax_adapter_sdk = { git = "https://github.com/9karma1/Elcarax.git", tag = "v0.1.0" }
```

For local development inside a sibling checkout:

```toml
elcarax_adapter_api = { path = "../Elcarax/crates/elcarax_adapter_api" }
```

Reference mock adapter in this repo:

```bash
cargo run -p elcarax_game_adapter
```

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `ELCARAX_PROJECT_PATH` | Open project at startup |
| `ELCARAX_PROJECT_CREATE_PATH` | Create project at startup |
| `ELCARAX_PROJECT_NAME` | Display name when creating a project |
| `ELCARAX_RECENT_PROJECTS_PATH` | Override recent-projects file location |
| `ELCARAX_ADAPTER_EXE` | Path to adapter executable (native shell) |
| `ELCARAX_ADAPTER_PROJECT_PATH` | Project path passed to adapter on connect |
| `ELCARAX_ADAPTER_AUTO_CONNECT` | Auto-connect adapter when set to `1` / `true` |

See [README.md](../README.md) and [BUILD_NOTES.md](BUILD_NOTES.md) for full runtime behavior.

## Windows: Linker Temp Directory

If MSVC fails with errors about temporary linker files and `TMP` is a relative path, set absolute temp directories before building:

```powershell
New-Item -ItemType Directory -Force -Path D:\elcarax\target\tmp | Out-Null
$env:TMP='D:\elcarax\target\tmp'
$env:TEMP='D:\elcarax\target\tmp'
cargo build -p elcarax_app --features native-shell
```

## Verify Your Install

```bash
cargo test --workspace
cargo run -p elcarax_app
```

A successful console proof prints command registry diagnostics and exercises project create/open without opening a window.

## Next Steps

- [CONTRIBUTING.md](../CONTRIBUTING.md) — development workflow and PR guidelines
- [docs/BUILD_NOTES.md](BUILD_NOTES.md) — shortcuts, panels, and shell behavior
- [CHANGELOG.md](../CHANGELOG.md) — release history
