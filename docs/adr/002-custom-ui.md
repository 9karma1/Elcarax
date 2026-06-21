# ADR-002: Custom UI shell

## Decision

Elcarax uses a custom GPU-first UI architecture rather than basing the core shell on an immediate-mode debug UI library or webview stack.

## Rationale

The product goal is an extremely optimized and beautiful proprietary editor. The UI is a product surface and competitive advantage, not plumbing.

## Consequences

- The v0.1 UI crate owns widget identity, dirty flags, layout, focus, and theme tokens.
- Rendering is expressed as editor-specific primitives first: rectangles, text, icons, images, lines, shadows, and clipping.
- External graphics/text/accessibility crates are wrapped at crate boundaries.
