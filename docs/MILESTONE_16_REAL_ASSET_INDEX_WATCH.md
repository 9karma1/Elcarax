# Milestone 16: Real Asset Index and File Watching

## Goal

Milestone 16 replaces scan-order asset records with a real project asset index backed by the opened project's filesystem asset root. The milestone keeps asset behavior editor-local and deliberately stops before importers, thumbnails, previews, drag/drop, rename/move/delete, dependency tracking, caches, or engine synchronization.

## Domain ownership

- `elcarax_assets` owns asset records, metadata, kind detection, deterministic IDs, scan requests/results, index snapshots, diagnostics, and the `notify`-backed watch service abstraction.
- `elcarax_project` owns project roots and manifest paths.
- `elcarax_app` coordinates project lifecycle, asset commands, selected asset state, dirty state, and watcher start/stop.
- `elcarax_ui` receives asset display strings only. Widgets do not read the filesystem and watcher events do not mutate UI directly.

## Scan behavior

`asset.scan` is explicit. Opening a project validates project paths and prepares asset state, but it does not scan automatically. This avoids hidden filesystem work during project open.

Asset scans:

- take a project root and an asset root
- recursively walk the asset root
- include folders below the asset root as folder records
- return project-relative paths such as `assets/models/hero.glb`
- skip hidden dot entries and `.elcarax`
- sort records deterministically by normalized path
- detect asset kinds by extension only
- read cheap metadata: extension, file size, modified time, and absolute source path
- return diagnostics for missing roots, invalid roots, unreadable entries, and metadata failures
- treat an empty asset root as valid

Missing asset roots produce diagnostics and an empty index instead of panics.

## Stable asset IDs

Scanned filesystem asset IDs are deterministic. `AssetId` is derived from the normalized project-relative path using a stable FNV-1a 64-bit hash.

Path rules:

- path separators normalize to `/`
- `.` components are removed
- case is preserved and IDs are case-sensitive
- the same normalized project-relative path maps to the same `AssetId` across scans
- no random IDs or sidecar metadata files are used for scanned filesystem assets

## Supported kinds

Kind detection is extension-only:

- Folder
- Scene: `scene`
- Image: `png`, `jpg`, `jpeg`, `webp`, `bmp`, `gif`
- Audio: `wav`, `ogg`, `mp3`, `flac`
- Model: `glb`, `gltf`, `obj`, `fbx`
- Script: `rs`, `lua`, `gd`, `js`, `ts`
- Material: `material`, `mat`
- Text: `md`, `txt`, `json`, `toml`, `yaml`, `yml`
- Unknown: anything else

## Watch behavior

`asset.start_watching` starts an `AssetWatchService` for the current asset root. The service contains `notify` internally and exposes only Elcarax asset watch types:

- `AssetWatchService`
- `AssetWatchEvent`
- `AssetWatchStatus`
- `AssetWatchError`

Watch events are drained nonblocking by the app layer. Create, modify, and remove events mark the asset index dirty. The UI shows `Asset index dirty - refresh recommended`, and `asset.refresh` performs a full rescan and clears dirty state. Watcher failures are reported as asset diagnostics.

The watcher is stopped on `asset.stop_watching`, project close, and project switch. This milestone intentionally uses full rescan on refresh instead of incremental cache invalidation or dependency tracking.

## Commands

- `asset.scan`
- `asset.refresh`
- `asset.start_watching`
- `asset.stop_watching`
- `asset.clear_selection`
- `asset.show_selected`
- `asset.reveal_root`

Without an open project, asset commands return `No project open`.

## UI behavior

The left project/assets panel now shows real asset index state:

- no project: `Assets unavailable - no project open`
- loaded but not scanned: `Assets not scanned - Run asset.scan`
- scanning: `Scanning assets...`
- ready: asset count, kind summary, watch status, and real asset rows
- dirty: `Asset index dirty - refresh recommended`
- error: diagnostic summary

Asset rows show display name, project-relative path, and kind. Clicking a row selects the real `AssetId`; selection survives refresh when the path still exists and clears when the file disappears.

## Console proof

`cargo run -p elcarax_app` creates a temporary real project, writes real files under `assets/`, opens the project, runs `asset.scan`, prints asset kinds, selects the first asset, modifies the asset folder, runs `asset.refresh`, closes the project, and verifies asset state clears.

## Explicit exclusions

- importers
- thumbnails
- asset previews
- drag/drop
- rename/move/delete
- dependency graph
- sidecar metadata
- build/import cache
- real engine asset sync
- real game engine connection for asset behavior

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

Manual native shell smoke test:

```bash
cargo run -p elcarax_app --features native-shell
```

Suggested manual flow:

1. Open the native shell.
2. Create or open a real project.
3. Create files inside the project `assets/` folder.
4. Run `asset.scan`.
5. Confirm assets appear in the left panel.
6. Select an asset row.
7. Run `asset.start_watching`.
8. Add or remove a file in the asset folder.
9. Confirm the asset index becomes dirty, then run `asset.refresh`.
10. Close the project and confirm asset state clears.
