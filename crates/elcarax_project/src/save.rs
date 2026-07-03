use std::fs;
use std::path::Path;

use crate::error::ProjectError;
use crate::manifest::{ProjectEditorSettings, ProjectFile, manifest_path_for_root};

pub fn save_project_editor_settings(
    project_root: &Path,
    editor: &ProjectEditorSettings,
) -> Result<(), ProjectError> {
    let manifest_path = manifest_path_for_root(project_root);
    let content = fs::read_to_string(&manifest_path).map_err(|error| {
        ProjectError::Io(format!(
            "failed to read {}: {error}",
            manifest_path.display()
        ))
    })?;
    let mut file = ProjectFile::from_toml_str(&content)?;
    file.manifest.editor = editor.clone();
    let toml = file.to_toml_string()?;
    fs::write(&manifest_path, toml).map_err(|error| {
        ProjectError::Io(format!(
            "failed to write {}: {error}",
            manifest_path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::{ProjectCreateRequest, create_project};
    use crate::manifest::ProjectEditorSettings;
    use crate::open::{ProjectOpenRequest, open_project};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn save_project_editor_settings_persists_active_scene() {
        let temp = std::env::temp_dir().join(format!("elcarax-save-editor-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = create_project(&ProjectCreateRequest::new(&temp, "Save Editor"));
        let mut editor = ProjectEditorSettings::with_default_active_scene();
        editor.active_scene = Some(PathBuf::from("custom.elcarax.scene.toml"));
        if let Err(error) = save_project_editor_settings(&temp, &editor) {
            panic!("save editor settings should succeed: {error}");
        }
        let loaded = match open_project(&ProjectOpenRequest::new(&temp)) {
            Ok(value) => value,
            Err(error) => panic!("reopen should succeed: {error}"),
        };
        assert_eq!(
            loaded
                .project
                .editor_settings()
                .active_scene_relative()
                .map(|path| path.display().to_string()),
            Some("custom.elcarax.scene.toml".to_string())
        );
        let _ = fs::remove_dir_all(&temp);
    }
}
