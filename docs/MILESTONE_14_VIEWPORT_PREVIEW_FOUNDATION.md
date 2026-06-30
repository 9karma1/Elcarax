# Milestone 14: Viewport Preview Foundation

Milestone 14 turns the center panel into a real viewport preview surface that can display preview frames supplied through the adapter boundary.

## Runtime Behavior

Normal startup still reports an honest empty viewport:

- viewport title: `Viewport`
- viewport message: `No viewport source`
- viewport status: `NoSource`

`viewport.request_frame` without a connected adapter returns `No adapter connected`.

When an adapter supports viewport preview and returns a frame, the editor updates viewport state to `FrameAvailable` and paints the RGBA frame in the center viewport area with a small `Adapter Preview` overlay label.

`viewport.clear` clears the current frame while preserving the adapter source when one exists.

`viewport.show_status` reports viewport source and status to command output.

The console proof spawns the stdio game adapter process, loads its project, requests a viewport frame, prints frame metadata, clears the viewport, and shuts the adapter down cleanly.

## Domain Model

`elcarax_core` owns engine-neutral viewport types:

- `ViewportId`
- `ViewportState`
- `ViewportSource`
- `ViewportFrame`
- `ViewportFrameFormat` (`Rgba8Unorm`)
- `ViewportFrameSize`
- `ViewportFramePixels`
- `ViewportStatus` (`NoSource`, `WaitingForFrame`, `FrameAvailable`, `Error`)
- `ViewportError`
- `ViewportDiagnostic`

## Adapter Protocol

`elcarax_adapter_api` owns serializable viewport protocol messages:

- `GetViewportFrameRequest`
- `GetViewportFrameResponse`
- `ViewportFrameResponseStatus`

Frames travel as JSON-friendly `Vec<u8>` RGBA payloads for Milestone 14.

The mock game adapter generates a deterministic procedural checker/gradient frame when a project is loaded. Invalid sizes and missing project state return explicit adapter errors.

## Rendering

`elcarax_render` adds `RenderPrimitive::Image` with destination rect, optional source rect, opacity, clip metadata, layer, and debug label support. The renderer uploads RGBA8 frame data to a dynamic GPU texture and draws it in the viewport content area.

## App Integration

`elcarax_app` keeps viewport state separate from UI state:

- `viewport_state.rs` — command handling and adapter frame application
- `viewport_display.rs` — UI snapshot formatting
- `viewport_ui.rs` — viewport label and paint snapshot updates

Command palette commands:

- `viewport.request_frame`
- `viewport.clear`
- `viewport.show_status`

## Explicit Exclusions

This milestone does not add:

- real engine scene rendering
- camera controls
- gizmos
- viewport picking
- shared GPU texture interop
- binary frame transport
- depth/stencil buffers
- image asset pipeline
- screenshot/export
- streaming continuous frames
- real C++/game adapter integration
