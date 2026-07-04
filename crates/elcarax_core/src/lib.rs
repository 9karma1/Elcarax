//! Engine-neutral foundation types for Elcarax.

mod diagnostic;
mod error;
mod id;
mod viewport;
mod viewport_camera;
mod workspace;

pub use diagnostic::{Diagnostic, DiagnosticSource, Severity};
pub use error::{ElcaraxError, Result};
pub use id::{Id, IdGenerator};
pub use viewport::{
    ViewportDiagnostic, ViewportError, ViewportFrame, ViewportFrameFormat, ViewportFramePixels,
    ViewportFrameSize, ViewportId, ViewportSource, ViewportState, ViewportStatus,
};
pub use viewport_camera::{
    NormalizedViewportCoord, ViewportCamera, ViewportFramePlacement, ViewportRect, fit_frame_rect,
    layout_viewport_frame, pointer_to_frame_uv,
};
pub use workspace::{Workspace, WorkspaceId, WorkspaceMarker};
