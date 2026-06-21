//! Engine-neutral foundation types for Elcarax.

mod diagnostic;
mod error;
mod id;
mod workspace;

pub use diagnostic::{Diagnostic, DiagnosticSource, Severity};
pub use error::{ElcaraxError, Result};
pub use id::{Id, IdGenerator};
pub use workspace::{Workspace, WorkspaceId, WorkspaceMarker};
