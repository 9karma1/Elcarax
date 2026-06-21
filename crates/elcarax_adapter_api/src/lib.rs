//! Versioned protocol between Elcarax and domain adapters.

mod capability;
mod message;
mod protocol;

pub use capability::AdapterCapabilities;
pub use message::{AdapterToEditor, EditorToAdapter};
pub use protocol::{HandshakeRequest, HandshakeResponse, ProtocolVersion};
