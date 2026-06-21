# Milestone 3: Text Rendering Foundation

Milestone 3 introduces Elcarax's static text rendering boundary and render primitive support.

## Scope

Included:

- `elcarax_text` owns font system setup, static shaping, metrics, glyph placement data, layout caching, and system-font rasterization.
- `elcarax_render` owns text render primitives, glyph atlas bookkeeping, text batching metadata, text-related render stats, and GPU submission of rasterized text pixels.
- The native demo scene now includes static labels for the toolbar, project panel, viewport, inspector, console, and status bar.
- The console proof flow remains GPU/window-free and reports text primitive and glyph stats.

## Font Strategy

No bundled font file is added in this milestone. Elcarax uses `cosmic-text` system font discovery through `FontSystem` and rasterizes glyph pixels through `SwashCache`. This keeps font selection and rasterization in `elcarax_text` while `elcarax_render` remains focused on render primitives and GPU submission.

A future asset milestone may add a bundled default font once redistribution requirements are reviewed.

## Explicit exclusions

This milestone does not implement:

- editable text fields
- caret movement or rendering
- selection
- IME
- rich text
- syntax highlighting
- emoji or color glyph support
- command palette
- widgets

## Validation

CI and local checks should continue to use non-windowed commands. Native-shell execution is a manual desktop smoke test and is not required in CI.
