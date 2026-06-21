use std::path::PathBuf;

use crate::AdapterCapabilities;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolVersion(pub u32);

impl ProtocolVersion {
    pub const V0: Self = Self(0);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub protocol_version: ProtocolVersion,
    pub editor_version: String,
    pub project_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeResponse {
    pub adapter_name: String,
    pub adapter_version: String,
    pub capabilities: AdapterCapabilities,
}
