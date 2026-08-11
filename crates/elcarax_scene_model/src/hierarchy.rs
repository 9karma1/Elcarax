use std::collections::BTreeSet;

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
        let mut visited = BTreeSet::new();
        for root_id in snapshot.root_object_ids() {
            Self::visit(snapshot, expansion, *root_id, 0, &mut rows, &mut visited);
        }
        rows
    }

    pub fn validate(snapshot: &SceneSnapshot) -> Result<(), SceneError> {
        Self::validate_detailed(snapshot).map_err(|_| SceneError::InvalidHierarchy)
    }

    pub(crate) fn validate_detailed(snapshot: &SceneSnapshot) -> Result<(), String> {
        let mut roots = BTreeSet::new();
        for root_id in snapshot.root_object_ids() {
            if !roots.insert(*root_id) {
                return Err(format!(
                    "root object {} is listed more than once",
                    root_id.get()
                ));
            }
            let object = snapshot
                .objects()
                .get(root_id)
                .ok_or_else(|| format!("root object {} does not exist", root_id.get()))?;
            if object.parent.is_some() {
                return Err(format!("root object {} has a parent", root_id.get()));
            }
        }

        for (map_id, object) in snapshot.objects() {
            if *map_id != object.id {
                return Err(format!(
                    "object map key {} does not match object id {}",
                    map_id.get(),
                    object.id.get()
                ));
            }
            let mut component_ids = BTreeSet::new();
            for component in &object.components {
                if !component_ids.insert(component.id) {
                    return Err(format!(
                        "object {} contains duplicate component {}",
                        object.id.get(),
                        component.id.get()
                    ));
                }
            }

            match object.parent {
                Some(parent_id) => {
                    let parent = snapshot.objects().get(&parent_id).ok_or_else(|| {
                        format!(
                            "object {} refers to missing parent {}",
                            object.id.get(),
                            parent_id.get()
                        )
                    })?;
                    if parent
                        .children
                        .iter()
                        .filter(|id| **id == object.id)
                        .count()
                        != 1
                    {
                        return Err(format!(
                            "object {} and parent {} do not have reciprocal links",
                            object.id.get(),
                            parent_id.get()
                        ));
                    }
                    if roots.contains(&object.id) {
                        return Err(format!(
                            "object {} is both rooted and parented",
                            object.id.get()
                        ));
                    }
                }
                None => {
                    if !roots.contains(&object.id) {
                        return Err(format!(
                            "object {} is not reachable from a root",
                            object.id.get()
                        ));
                    }
                }
            }

            let mut child_ids = BTreeSet::new();
            for child_id in &object.children {
                if !child_ids.insert(*child_id) {
                    return Err(format!(
                        "object {} lists child {} more than once",
                        object.id.get(),
                        child_id.get()
                    ));
                }
                let child = snapshot.objects().get(child_id).ok_or_else(|| {
                    format!(
                        "object {} refers to missing child {}",
                        object.id.get(),
                        child_id.get()
                    )
                })?;
                if child.parent != Some(object.id) {
                    return Err(format!(
                        "object {} and child {} do not have reciprocal links",
                        object.id.get(),
                        child_id.get()
                    ));
                }
            }
        }

        let mut visited = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        for root_id in &roots {
            validate_tree_links(snapshot, *root_id, &mut visited, &mut visiting)?;
        }
        if visited.len() != snapshot.objects().len() {
            return Err("scene contains an object outside the root hierarchy".to_string());
        }
        Ok(())
    }

    /*
     * Keep this check explicit even though the parent/child reciprocal checks
     * above catch most malformed graphs. It makes cycle rejection independent
     * of root ordering and protects future hierarchy mutations.
     */
    fn visit(
        snapshot: &SceneSnapshot,
        expansion: &SceneExpansion,
        object_id: SceneObjectId,
        depth: usize,
        rows: &mut Vec<SceneTreeRow>,
        visited: &mut BTreeSet<SceneObjectId>,
    ) {
        if !visited.insert(object_id) {
            return;
        }
        let Ok(object) = snapshot.object(object_id) else {
            return;
        };
        let has_children = !object.children.is_empty();
        let expanded = has_children && expansion.is_expanded(object_id);
        rows.push(SceneTreeRow {
            object_id,
            depth,
            display_name: SceneObjectName::from_unvalidated(object.display_name.clone()),
            kind: object.kind.clone(),
            has_children,
            expanded,
        });
        if !expanded {
            return;
        }
        for child_id in &object.children {
            Self::visit(snapshot, expansion, *child_id, depth + 1, rows, visited);
        }
    }
}

fn validate_tree_links(
    snapshot: &SceneSnapshot,
    object_id: SceneObjectId,
    visited: &mut BTreeSet<SceneObjectId>,
    visiting: &mut BTreeSet<SceneObjectId>,
) -> Result<(), String> {
    if visiting.contains(&object_id) {
        return Err(format!(
            "hierarchy cycle includes object {}",
            object_id.get()
        ));
    }
    if !visited.insert(object_id) {
        return Ok(());
    }
    visiting.insert(object_id);
    let object = snapshot
        .objects()
        .get(&object_id)
        .ok_or_else(|| format!("hierarchy refers to missing object {}", object_id.get()))?;
    for child_id in &object.children {
        validate_tree_links(snapshot, *child_id, visited, visiting)?;
    }
    visiting.remove(&object_id);
    Ok(())
}
