# Milestone 8: Asset Browser Foundation

Milestone 8 introduces the first file-based asset browser foundation for Elcarax. The goal is a clean asset index model, simple scanning, project-panel display, selection state, and command palette integration without importers, thumbnails, or adapter wiring.

## Included

- `elcarax_assets` domain types: `AssetId`, `AssetRecord`, `AssetName`, `AssetPath`, `AssetKind`, `AssetIndex`, `AssetScan`, `AssetSelection`, `AssetDiagnostic`, and `AssetError`
- Extension-only `AssetKind` detection for scene, image, audio, model, script, material, text, folder, and unknown paths
- In-memory demo asset index with stable sorted order for CI and console proof
- Optional `examples/demo_project/` text placeholders for manual filesystem scans
- Synchronous filesystem scan API with diagnostics for missing or invalid roots
- App-layer `AssetState` separate from UI widgets
- Command palette commands:
  - `asset.scan_demo`
  - `asset.select_first`
  - `asset.clear_selection`
  - `asset.show_selected`
- Left project panel asset section with count, row labels, selected summary, and clickable asset rows in the native shell
- Console proof flow covering project load, asset scan, selection, and clear selection

## Behavior Notes

- `asset.scan_demo` requires a loaded project. If no project is loaded, it returns the diagnostic message `No project loaded` and does not populate the asset index.
- Console proof runs `project.new_demo` before asset commands.
- Demo assets are built in memory for deterministic tests and CI. Filesystem scanning is available through `AssetScan::scan_root` but is not required for the default proof flow.
- Asset rows in the project panel are simple buttons. The selected row receives keyboard focus highlighting.

## Demo Assets

The in-memory demo index contains seven stable records:

- `README.md`
- `assets/audio/click.wav`
- `assets/materials/default.material`
- `assets/models/cube.glb`
- `assets/scenes/demo.scene`
- `assets/textures/checker.png`
- `scripts/player.rs`

## Explicit Exclusions

This milestone does not add:

- real import pipeline
- thumbnails or asset previews
- async or background scanning workers
- file watching
- drag and drop
- rename, move, or delete
- scene tree
- inspector editing for assets
- game adapter integration

## Validation

```bash
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p elcarax_app
```

Manual native shell smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

Suggested manual flow:

1. Open the native shell
2. Run `project.new_demo`
3. Run `asset.scan_demo`
4. Confirm the project panel lists demo assets and the asset count
5. Click or command-select the first asset
6. Confirm the selected row highlights and the status bar reports the selected asset
7. Run `asset.clear_selection`
