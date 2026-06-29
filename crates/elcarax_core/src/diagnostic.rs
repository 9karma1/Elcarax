use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticSource {
    pub subsystem: String,
    pub location: Option<String>,
}

impl DiagnosticSource {
    pub fn new(subsystem: impl Into<String>) -> Self {
        Self {
            subsystem: subsystem.into(),
            location: None,
        }
    }

    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub source: DiagnosticSource,
    pub message: String,
}

impl Diagnostic {
    pub fn info(source: DiagnosticSource, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            source,
            message: message.into(),
        }
    }

    pub fn warning(source: DiagnosticSource, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            source,
            message: message.into(),
        }
    }

    pub fn error(source: DiagnosticSource, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            source,
            message: message.into(),
        }
    }
}
