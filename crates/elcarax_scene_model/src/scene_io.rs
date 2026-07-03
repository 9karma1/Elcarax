use std::fs;
use std::path::{Path, PathBuf};

use crate::SceneHierarchy;
use crate::SceneIoError;
use crate::scene_file::{
    DEFAULT_SCENE_FILENAME, SceneFile, is_scene_file_name, scene_file_path_in_root,
};
use crate::snapshot::SceneSnapshot;

pub fn read_scene_file(path: &Path) -> Result<SceneFile, SceneIoError> {
    let content = fs::read_to_string(path)
        .map_err(|error| SceneIoError::Io(format!("failed to read {}: {error}", path.display())))?;
    let file = SceneFile::from_toml_str(&content)?;
    validate_loaded_snapshot(file.snapshot())?;
    Ok(file)
}

pub fn write_scene_file(path: &Path, snapshot: &SceneSnapshot) -> Result<(), SceneIoError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            SceneIoError::Io(format!(
                "failed to create scene directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    let file = SceneFile::from_snapshot(snapshot);
    let content = file.to_toml_string()?;
    fs::write(path, content)
        .map_err(|error| SceneIoError::Io(format!("failed to write {}: {error}", path.display())))
}

pub fn create_default_scene_file(path: &Path, scene_name: &str) -> Result<(), SceneIoError> {
    let snapshot = SceneSnapshot::with_name(crate::SceneName::from_unvalidated(scene_name));
    write_scene_file(path, &snapshot)
}

pub fn discover_scene_files(scene_root: &Path) -> Result<Vec<PathBuf>, SceneIoError> {
    if !scene_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let entries = fs::read_dir(scene_root).map_err(|error| {
        SceneIoError::Io(format!(
            "failed to read scene root {}: {error}",
            scene_root.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            SceneIoError::Io(format!(
                "failed to read scene root entry in {}: {error}",
                scene_root.display()
            ))
        })?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .is_some_and(|name| is_scene_file_name(name.to_string_lossy().as_ref()))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub fn resolve_scene_load_path(
    scene_root: &Path,
    active_scene: Option<&Path>,
) -> Result<PathBuf, SceneIoError> {
    if let Some(active_scene) = active_scene {
        let candidate = scene_root.join(active_scene);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    let default_path = scene_file_path_in_root(scene_root, DEFAULT_SCENE_FILENAME);
    if default_path.is_file() {
        return Ok(default_path);
    }
    let discovered = discover_scene_files(scene_root)?;
    discovered
        .into_iter()
        .next()
        .ok_or(SceneIoError::NoSceneFileFound)
}

pub fn load_scene_from_project(
    scene_root: &Path,
    active_scene: Option<&Path>,
) -> Result<(SceneSnapshot, PathBuf), SceneIoError> {
    let path = resolve_scene_load_path(scene_root, active_scene)?;
    let file = read_scene_file(&path)?;
    Ok((file.into_snapshot(), path))
}

fn validate_loaded_snapshot(snapshot: &SceneSnapshot) -> Result<(), SceneIoError> {
    if snapshot.name().as_str().trim().is_empty() {
        return Err(SceneIoError::InvalidDocument(
            "scene name cannot be empty".to_string(),
        ));
    }
    SceneHierarchy::validate(snapshot)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SceneName;
    use crate::kind::SceneObjectKind;
    use crate::snapshot::{SceneObject, SceneSnapshot};
    use std::fs;

    #[test]
    fn scene_file_roundtrip_preserves_snapshot() {
        let temp = std::env::temp_dir().join(format!("elcarax-scene-io-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(&temp);
        let path = temp.join("roundtrip.elcarax.scene.toml");
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Roundtrip"));
        let object = SceneObject::new("Root", SceneObjectKind::World, {
            use crate::ObjectSchema;
            ObjectSchema::new("World").type_id
        });
        snapshot.add_root_object(object);
        if let Err(error) = write_scene_file(&path, &snapshot) {
            panic!("write scene should succeed: {error}");
        }
        let loaded = match read_scene_file(&path) {
            Ok(value) => value,
            Err(error) => panic!("read scene should succeed: {error}"),
        };
        assert_eq!(loaded.snapshot(), &snapshot);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn resolve_scene_load_path_prefers_active_scene() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-scene-resolve-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(&temp);
        let other = temp.join("other.elcarax.scene.toml");
        let active = temp.join("active.elcarax.scene.toml");
        if let Err(error) = create_default_scene_file(&other, "Other") {
            panic!("write other should succeed: {error}");
        }
        if let Err(error) = create_default_scene_file(&active, "Active") {
            panic!("write active should succeed: {error}");
        }
        let resolved =
            match resolve_scene_load_path(&temp, Some(Path::new("active.elcarax.scene.toml"))) {
                Ok(value) => value,
                Err(error) => panic!("resolve active should succeed: {error}"),
            };
        assert_eq!(resolved, active);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn discover_scene_files_sorts_paths() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-scene-discover-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(&temp);
        if let Err(error) = create_default_scene_file(&temp.join("b.elcarax.scene.toml"), "B") {
            panic!("write b should succeed: {error}");
        }
        if let Err(error) = create_default_scene_file(&temp.join("a.elcarax.scene.toml"), "A") {
            panic!("write a should succeed: {error}");
        }
        let discovered = match discover_scene_files(&temp) {
            Ok(value) => value,
            Err(error) => panic!("discover should succeed: {error}"),
        };
        assert_eq!(discovered.len(), 2);
        assert!(discovered[0].ends_with("a.elcarax.scene.toml"));
        let _ = fs::remove_dir_all(&temp);
    }
}
