use elcarax_core::Severity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneDiagnostic {
    severity: Severity,
    field: String,
    message: String,
}

impl SceneDiagnostic {
    pub fn new(severity: Severity, field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, field, message)
    }

    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, field, message)
    }

    pub const fn severity(&self) -> Severity {
        self.severity
    }

    pub fn field(&self) -> &str {
        self.field.as_str()
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    pub fn summary(&self) -> String {
        format!("{}: {}", self.field, self.message)
    }
}
