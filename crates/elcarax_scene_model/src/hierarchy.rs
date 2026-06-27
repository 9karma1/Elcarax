use crate::SceneError;
use crate::kind::SceneObjectKind;
use crate::name::SceneObjectName;
use crate::selection::SceneExpansion;
use crate::snapshot::{SceneObjectId, SceneSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneTreeRow {
    pub object_id: SceneObjectId,
    pub depth: usize,
    pub display_name: SceneObjectName,
    pub kind: SceneObjectKind,
    pub has_children: bool,
    pub expanded: bool,
}

impl SceneTreeRow {
    pub fn row_label(&self) -> String {
        let indent = "  ".repeat(self.depth);
        let marker = if !self.has_children {
            "-"
        } else if self.expanded {
            "v"
        } else {
            ">"
        };
        format!(
            "{indent}{marker} {} ({})",
            self.display_name.as_str(),
            self.kind.label()
        )
    }

    pub fn expand_marker(&self) -> String {
        if !self.has_children {
            String::new()
        } else if self.expanded {
            "v".to_string()
        } else {
            ">".to_string()
        }
    }

    pub fn name_label(&self) -> String {
        let indent = "  ".repeat(self.depth);
        format!(
            "{indent}{} ({})",
            self.display_name.as_str(),
            self.kind.label()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneHierarchy;

impl SceneHierarchy {
    pub fn visible_rows(snapshot: &SceneSnapshot, expansion: &SceneExpansion) -> Vec<SceneTreeRow> {
        let mut rows = Vec::new();
        for root_id in snapshot.root_object_ids() {
            Self::visit(snapshot, expansion, *root_id, 0, &mut rows);
        }
        rows
    }

    pub fn validate(snapshot: &SceneSnapshot) -> Result<(), SceneError> {
        for object in snapshot.objects().values() {
            if let Some(parent) = object.parent
                && snapshot.object(parent).is_err()
            {
                return Err(SceneError::InvalidHierarchy);
            }
            for child in &object.children {
                if snapshot.object(*child).is_err() {
                    return Err(SceneError::InvalidHierarchy);
                }
            }
        }
        Ok(())
    }

    fn visit(
        snapshot: &SceneSnapshot,
        expansion: &SceneExpansion,
        object_id: SceneObjectId,
        depth: usize,
        rows: &mut Vec<SceneTreeRow>,
    ) {
        let Ok(object) = snapshot.object(object_id) else {
            return;
        };
        let has_children = !object.children.is_empty();
        let expanded = has_children && expansion.is_expanded(object_id);
        rows.push(SceneTreeRow {
            object_id,
            depth,
            display_name: SceneObjectName::from_unvalidated(object.display_name.clone()),
            kind: object.kind,
            has_children,
            expanded,
        });
        if !expanded {
            return;
        }
        for child_id in &object.children {
            Self::visit(snapshot, expansion, *child_id, depth + 1, rows);
        }
    }
}
