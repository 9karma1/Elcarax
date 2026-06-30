//! Engine-neutral foundation types for Elcarax.

mod diagnostic;
mod error;
mod id;
mod viewport;
mod workspace;

pub use diagnostic::{Diagnostic, DiagnosticSource, Severity};
pub use error::{ElcaraxError, Result};
pub use id::{Id, IdGenerator};
pub use viewport::{
    ViewportDiagnostic, ViewportError, ViewportFrame, ViewportFrameFormat, ViewportFramePixels,
    ViewportFrameSize, ViewportId, ViewportSource, ViewportState, ViewportStatus,
};
pub use workspace::{Workspace, WorkspaceId, WorkspaceMarker};
