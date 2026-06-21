# Third-Party Notices

Elcarax is open source under Apache-2.0. Third-party dependencies should still be reviewed before being added so license compatibility, redistribution requirements, and platform impact stay clear.

## Dependency Policy

- MIT: allowed after review
- Apache-2.0: allowed after review
- BSD-2/BSD-3: allowed after review
- MPL-2.0: requires review for file-level obligations
- GPL/AGPL/LGPL: blocked unless compatibility and distribution obligations are explicitly accepted

## Current Dependency Areas

- `winit` is isolated in `elcarax_platform`.
- `wgpu` is isolated in `elcarax_gpu` and consumed by `elcarax_render`.
- `cosmic-text` is isolated in `elcarax_text`.

Editor-neutral crates should remain free of platform, GPU, text-rasterization, engine, and game-framework dependencies unless an ADR explicitly changes that boundary.
