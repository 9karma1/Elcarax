# Milestone 3: Text Rendering Foundation

Milestone 3 introduces Elcarax's static text rendering boundary and render primitive support.

## Scope

Included:

- `elcarax_text` owns font system setup, static shaping, metrics, glyph placement data, and layout caching.
- `elcarax_render` owns text render primitives, glyph atlas bookkeeping, text batching metadata, and text-related render stats.
- The native demo scene now includes static labels for the toolbar, project panel, viewport, inspector, console, and status bar.
- The console proof flow remains GPU/window-free and reports text primitive and glyph stats.

## Font strategy

No bundled font file is added in this milestone. Elcarax uses `cosmic-text` system font discovery and fallback through `FontSystem`. This avoids adding font assets without a license review. A future asset milestone may add a bundled default font once license and redistribution requirements are reviewed.

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
