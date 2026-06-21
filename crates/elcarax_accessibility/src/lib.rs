//! Accessibility tree boundary for Elcarax.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessibleRole {
    Window,
    Panel,
    Button,
    Text,
    Tree,
    TreeItem,
    TextInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleNode {
    pub stable_id: u64,
    pub role: AccessibleRole,
    pub label: String,
    pub children: Vec<u64>,
}

impl AccessibleNode {
    pub fn new(stable_id: u64, role: AccessibleRole, label: impl Into<String>) -> Self {
        Self {
            stable_id,
            role,
            label: label.into(),
            children: Vec::new(),
        }
    }
}
