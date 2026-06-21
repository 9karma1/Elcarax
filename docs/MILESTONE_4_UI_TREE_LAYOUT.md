# Milestone 4: UI tree and layout foundation

Milestone 4 introduces the first retained Elcarax UI layer on top of the existing render primitive and text pipeline.

## Included

- Stable `WidgetId` identity with generated runtime IDs and explicit deterministic IDs for tests/shell construction.
- Parent/child relationships, lookup by ID, and stable depth-first traversal order.
- `UiTree`, `UiNode`, `UiContext`, `UiStyle`, `Theme`, `LayoutNode`, `LayoutConstraints`, `LayoutResult`, `PaintContext`, `UiEvent`, and `UiError`.
- Minimal layout primitives:
  - fixed size
  - fill remaining space
  - content-sized labels
  - horizontal and vertical stacks
  - split row/column behavior through axis stacks
  - padding/insets
  - root bounds from an absolute window rect
- Dirty flags for layout, paint, text, hit-test, and accessibility placeholders.
- Non-interactive widgets:
  - root
  - panel
  - label
  - separator
  - status bar
  - toolbar
  - viewport placeholder
- Theme tokens for background, surfaces, viewport, border, text, muted text, accent, spacing, and font sizes.
- UI-generated editor shell for both console proof and native shell:
  - toolbar label
  - project panel
  - center viewport panel
  - inspector panel
  - status bar
  - static labels

## Console Proof

The default app flow remains GPU-free:

```bash
cargo run -p elcarax_app
```

It builds the UI shell, lays it out, paints it into a `RenderScene`, and prints node, layout, primitive, text primitive, and dirty flag counts.

## Native Shell

The feature-gated native path now builds the same UI shell and paints the resulting scene:

```bash
cargo run -p elcarax_app --features native-shell
```

This remains a manual desktop smoke test and should not be required in CI.

## Explicit Exclusions

- docking
- drag resizing
- tree views
- asset browser behavior
- inspector property editing
- command palette
- text input
- scroll views
- accessibility implementation
- adapter integration
- full CSS, flexbox, or grid layout
