# Milestone 15: Real Project Open and Persistence

## Goal

Replace fixture-driven project usage with real editor project persistence using `elcarax.project.toml`.

## Project file format

Path inside a project folder:

```toml
elcarax.project.toml
```

Minimum content:

```toml
schema_version = 1
name = "My Elcarax Project"

[paths]
asset_root = "assets"
scene_root = "scenes"
settings_dir = ".elcarax"
```

Rules:

- only `schema_version = 1` is accepted today
- relative paths resolve from the project root
- project creation also creates `assets/`, `scenes/`, and `.elcarax/`

## Domain ownership

- `elcarax_project` owns manifest parsing, validation, create/open filesystem logic, and recent-project storage
- `elcarax_assets` scans paths provided by app project state
- `elcarax_app` coordinates commands, config paths, and dependent state clearing
- `elcarax_ui` displays project view models only

## Create and open paths

Native file picker is deferred. Project paths are accepted through:

| Mechanism | Purpose |
|-----------|---------|
| `--create-project <path>` | create a project at the given folder |
| `--project <path>` | open an existing project folder |
| `--project-name <name>` | optional name for create |
| `ELCARAX_PROJECT_CREATE_PATH` | create path for `project.create` |
| `ELCARAX_PROJECT_PATH` | open path for `project.open` |
| `ELCARAX_PROJECT_NAME` | optional create name |
| `ELCARAX_RECENT_PROJECTS_PATH` | override recent-project store location |
| `project.reopen_last` | open the most recent stored project |

Default recent-project store:

```text
.elcarax/recent-projects.toml
```

## Command palette commands

- `project.create`
- `project.open`
- `project.close`
- `project.validate`
- `project.show_recent`
- `project.reopen_last`

## Runtime behavior

### No project

- toolbar: `Elcarax — No Project`
- project panel: `No project open`
- assets section: unavailable
- status bar: `Ready - open a project or connect an adapter`

### Project loaded

- toolbar: `Elcarax — <Project Name>`
- project panel shows name, root, asset root, scene root, validation summary, and recent count
- `asset.scan` scans the real project asset root
- empty `assets/` is valid

### Project close

Clears:

- current project
- asset index/scan/selection
- scene state
- inspector state

Adapter connection and viewport preview remain available when not project-bound.

## Console proof

`cargo run -p elcarax_app` now:

1. prints empty startup state
2. creates a temporary real project
3. opens and validates it
4. scans the empty asset root
5. records it in recent projects
6. closes and reopens the last project
7. runs the adapter viewport proof

## Explicit exclusions

- full native file dialog
- asset importers
- file watching
- thumbnails
- project migrations beyond schema version check
- cloud sync
- real game adapter integration
- scene save/writeback
- project templates beyond the minimal project scaffold
