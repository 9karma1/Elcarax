# Milestone 7: Project System UI

Milestone 7 makes the project system visible and controllable through the editor shell without adding filesystem browsing, asset scanning, adapter loading, or scene editing.

## Included

- `elcarax_project` now owns project-domain types for projects, project IDs, validated names/paths, project status, validation diagnostics, recent project entries, and project errors.
- `elcarax_commands` registers project commands:
  - `project.new_demo`
  - `project.open_demo`
  - `project.close`
  - `project.validate`
  - `project.show_recent`
- `elcarax_app` owns project app state separately from UI tree state.
- The command palette can execute project commands and update the shell.
- The toolbar shows the current project name or no-project state.
- The status bar shows project status, diagnostic count, and last project command.
- The project panel shows project name, path, status, recent project count, diagnostics, and command result.
- Diagnostics use semantic UI text roles for neutral, success, warning, and danger states.
- The console proof executes project commands through the command palette and prints initial, loaded, validated, and closed project states.

## Explicit Exclusions

- real file picker
- asset scanning
- asset browser behavior
- file watching
- scene tree
- inspector editing
- adapter loading
- project migration
- persistent recent-project storage
- async project loading

## Validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

Manual desktop smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

Confirm the window opens, Ctrl+K opens the command palette, `project.new_demo` updates the toolbar/status/project panel, `project.validate` updates diagnostics, `project.close` returns to no-project state, resizing does not panic, and closing exits cleanly.
