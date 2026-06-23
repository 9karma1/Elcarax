# Milestone 6: Command Palette Shell

Milestone 6 adds Elcarax's first command palette shell on top of the existing UI tree, input routing, focus state, static text rendering, and command registry foundation.

## Included

- `elcarax_commands` now exposes platform-neutral command IDs, names, descriptions, categories, registry lookup, filtering, and invocation results.
- Built-in editor commands are registered for opening/closing the palette, showing renderer stats, showing ready status, and running the demo action.
- `elcarax_ui` owns command palette UI state, query buffering, filtered command entries, selected row movement, and overlay painting.
- The palette supports a simple query buffer, Backspace, ArrowUp, ArrowDown, Enter, and Escape.
- The native shell opens the palette with Ctrl+K, routes keyboard input to the palette while it is open, executes selected commands, and repaints only when state changes.
- The console proof opens the palette, types `ready`, executes `Show Ready Status`, closes the palette, and prints the resulting status text.

## Built-In Commands

- `elcarax.palette.open`: Open Command Palette
- `elcarax.palette.close`: Close Command Palette
- `elcarax.status.show_renderer_stats`: Show Renderer Stats
- `elcarax.status.show_ready`: Show Ready Status
- `elcarax.demo.run`: Run Demo Action

## Explicit Exclusions

- full text input widget
- IME
- editable inspector fields
- full keybinding system
- fuzzy scoring algorithm
- command macros
- adapter commands
- project commands
- async command execution

## Validation

The non-window validation path remains:

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

Confirm the toolbar `Run` button still works, Ctrl+K opens the command palette, typing `ready` filters to `Show Ready Status`, Enter updates the status text to `Status: Ready`, Escape closes without executing, resizing does not panic, and closing exits cleanly.
