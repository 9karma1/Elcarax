# ADR-002: Custom UI shell

## Decision

Elcarax uses a custom GPU-first UI architecture rather than basing the core shell on an immediate-mode debug UI library or webview stack.

## Rationale

The product goal is an extremely optimized and beautiful editor shell. The UI is a core product surface, not plumbing, and should stay tailored to Elcarax rather than inherit another toolkit's interaction model.

## Consequences

- The v0.1 UI crate owns widget identity, dirty flags, layout, focus, and theme tokens.
- Rendering is expressed as editor-specific primitives first: rectangles, text, icons, images, lines, shadows, and clipping.
- External graphics/text/accessibility crates are wrapped at crate boundaries.
