# Milestone 2: Render primitive pipeline

Milestone 2 adds Elcarax's first GPU-backed editor render layer. The goal is intentionally narrow: convert simple render commands into batched primitives and draw them through `wgpu` from the native shell.

## Included

- Editor render primitive model in `elcarax_render`:
  - `RenderPrimitive`
  - `RenderScene`
  - `RenderLayer`
  - `RenderBatch`
  - `RenderStats`
  - `Color`
  - `Rect`
  - `CornerRadius`
  - `Border`
  - `ClipRect`
- v0.1 primitives:
  - solid rectangles
  - rounded rectangle data model
  - border rectangles
  - axis-aligned simple lines
  - clip rectangle metadata
  - optional debug labels
- Renderer API:
  - `Renderer`
  - `RendererConfig`
  - `RendererError`
  - `Renderer::new(...)`
  - `Renderer::render(...)`
  - `Renderer::stats()`
- GPU path:
  - rectangle vertex and instance buffers
  - solid rectangle WGSL shader
  - render pipeline setup
  - per-frame instance upload
  - render pass integration with clear-frame behavior
  - graceful empty scene handling
- Native-shell primitive demo scene with dark background, toolbar, sidebars, viewport, inspector, status bar, separators, and a sample border.
- CPU-side tests for primitive order, batching, empty scenes, stats-related counts, normalization, and clip metadata.

## Architecture boundary

Raw GPU dependencies remain contained:

- `elcarax_gpu` owns the `wgpu` context, surface, frame clear, and surface helpers.
- `elcarax_render` depends on `elcarax_gpu` and `wgpu` to build the editor primitive renderer.
- `elcarax_app` wires the native shell to the renderer behind the `native-shell` feature.
- `elcarax_core`, `elcarax_scene_model`, `elcarax_commands`, and `elcarax_adapter_api` remain free of `wgpu`, `winit`, engine, and game-framework dependencies.

## Intentionally not included

Milestone 2 does **not** include:

- text rendering
- icons
- images
- full vector paths
- UI widgets
- docking
- panels, inspector behavior, asset browser, or adapter integration
- GPU scissor enforcement for clip rectangles
- rounded-corner shader clipping

Rounded rectangles and clip rectangles are represented in the render model so future milestones can add richer shader and scissor behavior without changing the high-level API.

## Running

Console proof flow:

```bash
cargo run -p elcarax_app
```

Native render primitive demo:

```bash
cargo run -p elcarax_app --features native-shell
```

On a desktop session, this opens an `Elcarax` window and renders the primitive demo scene. Resizing should reconfigure the surface and redraw without panicking.
