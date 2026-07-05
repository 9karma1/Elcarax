#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use std::path::{Path, PathBuf};

use elcarax_adapter_api::AdapterId;
use elcarax_scene_model::{
    SceneDiagnostic, SceneExpansion, SceneIoError, SceneObjectId, SceneSelection, SceneSnapshot,
    load_scene_from_project, write_scene_file,
};

use crate::scene_display::{SceneUiSnapshot, scene_ui_snapshot_with_scroll};

pub(crate) const SCENE_LOAD_COMMAND: &str = "scene.load";
pub(crate) const SCENE_SAVE_COMMAND: &str = "scene.save";
pub(crate) const SCENE_CLEAR_COMMAND: &str = "scene.clear";
pub(crate) const SCENE_CLEAR_SELECTION_COMMAND: &str = "scene.clear_selection";

pub(crate) const UNSAVED_SCENE_MESSAGE: &str = "Unsaved scene changes — save with scene.save or reload with scene.load before closing the project";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SceneState {
    snapshot: Option<SceneSnapshot>,
    source: SceneSource,
    project_binding: Option<SceneProjectBinding>,
    selection: SceneSelection,
    expansion: SceneExpansion,
    diagnostics: Vec<SceneDiagnostic>,
    last_command_result: Option<SceneCommandResult>,
    document_dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneProjectBinding {
    scene_root: PathBuf,
    active_scene: Option<PathBuf>,
}

impl SceneState {
    pub(crate) fn execute_command_id(&mut self, id: &str) -> Option<SceneCommandResult> {
        let command = SceneCommand::from_id(id)?;
        let result = match command {
            SceneCommand::Load => self.load(),
            SceneCommand::Save => self.save(),
            SceneCommand::Clear => self.clear(),
            SceneCommand::ClearSelection => self.clear_selection(),
        };
        self.last_command_result = Some(result.clone());
        Some(result)
    }

    #[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
    pub(crate) fn select_object(&mut self, id: SceneObjectId) -> bool {
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        if self.selection.select_existing(snapshot, id).is_err() {
            return false;
        }
        self.last_command_result = None;
        true
    }

    pub(crate) fn toggle_expand_row_at(&mut self, row_index: usize, scroll_offset: usize) -> bool {
        let id = match self.ui_snapshot_at(scroll_offset).visible_object_ids[row_index] {
            Some(id) => id,
            None => return false,
        };
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        let Ok(object) = snapshot.object(id) else {
            return false;
        };
        if object.children.is_empty() {
            return false;
        }
        self.expansion.toggle(id);
        self.last_command_result = None;
        true
    }

    pub(crate) fn ui_snapshot(&self) -> SceneUiSnapshot {
        self.ui_snapshot_at(0)
    }

    pub(crate) fn ui_snapshot_at(&self, scroll_offset: usize) -> SceneUiSnapshot {
        scene_ui_snapshot_with_scroll(
            self.snapshot.as_ref(),
            &self.selection,
            &self.expansion,
            &self.diagnostics,
            self.last_command_result
                .as_ref()
                .map(SceneCommandResult::message),
            self.has_unsaved_changes(),
            scroll_offset,
        )
    }

    pub(crate) fn has_unsaved_changes(&self) -> bool {
        self.document_dirty && self.is_project_document()
    }

    pub(crate) fn is_project_document(&self) -> bool {
        matches!(self.source, SceneSource::Project(_))
    }

    pub(crate) fn is_adapter_backed(&self) -> bool {
        self.adapter_id().is_some()
    }

    pub(crate) fn mark_document_modified(&mut self) {
        if self.is_project_document() {
            self.document_dirty = true;
        }
    }

    pub(crate) fn active_scene_relative_path(&self) -> Option<PathBuf> {
        let SceneSource::Project(path) = &self.source else {
            return None;
        };
        let binding = self.project_binding.as_ref()?;
        path.strip_prefix(binding.scene_root.as_path())
            .ok()
            .map(|relative| relative.to_path_buf())
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn snapshot(&self) -> Option<&SceneSnapshot> {
        self.snapshot.as_ref()
    }

    pub(crate) fn snapshot_mut(&mut self) -> Option<&mut SceneSnapshot> {
        self.snapshot.as_mut()
    }

    #[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
    pub(crate) fn adapter_id(&self) -> Option<&AdapterId> {
        match &self.source {
            SceneSource::Adapter(id) => Some(id),
            #[cfg(test)]
            SceneSource::Local => None,
            SceneSource::None | SceneSource::Project(_) => None,
        }
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn selection(&self) -> &SceneSelection {
        &self.selection
    }

    #[cfg_attr(feature = "native-shell", allow(dead_code))]
    pub(crate) fn expansion(&self) -> &SceneExpansion {
        &self.expansion
    }

    pub(crate) fn record_status(&mut self, command_id: &str, message: impl Into<String>) {
        self.last_command_result = Some(SceneCommandResult::new(command_id, message));
    }

    #[allow(dead_code)]
    pub(crate) fn load_external_snapshot(
        &mut self,
        snapshot: SceneSnapshot,
        adapter_id: AdapterId,
        command_id: &str,
        message: impl Into<String>,
    ) {
        self.snapshot = Some(snapshot);
        self.source = SceneSource::Adapter(adapter_id);
        self.selection.clear();
        self.expansion.collapse_all();
        self.diagnostics.clear();
        self.document_dirty = false;
        self.last_command_result = Some(SceneCommandResult::new(command_id, message));
    }

    pub(crate) fn on_project_opened(&mut self, scene_root: &Path, active_scene: Option<&Path>) {
        self.clear_loaded_scene();
        self.project_binding = Some(SceneProjectBinding {
            scene_root: scene_root.to_path_buf(),
            active_scene: active_scene.map(Path::to_path_buf),
        });
    }

    fn load(&mut self) -> SceneCommandResult {
        let Some(binding) = self.project_binding.as_ref() else {
            return SceneCommandResult::new(SCENE_LOAD_COMMAND, "No project open");
        };
        match load_scene_from_project(
            binding.scene_root.as_path(),
            binding.active_scene.as_deref(),
        ) {
            Ok((snapshot, path)) => {
                self.apply_loaded_snapshot(snapshot, SceneSource::Project(path.clone()));
                SceneCommandResult::new(
                    SCENE_LOAD_COMMAND,
                    format!("Loaded scene from {}", path.display()),
                )
            }
            Err(SceneIoError::NoSceneFileFound) => SceneCommandResult::new(
                SCENE_LOAD_COMMAND,
                "No scene file found in project scene root",
            ),
            Err(error) => SceneCommandResult::new(SCENE_LOAD_COMMAND, error.to_string()),
        }
    }

    fn save(&mut self) -> SceneCommandResult {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return SceneCommandResult::new(SCENE_SAVE_COMMAND, "No scene loaded");
        };
        let SceneSource::Project(path) = &self.source else {
            return SceneCommandResult::new(
                SCENE_SAVE_COMMAND,
                "Only project-owned scenes can be saved in this milestone",
            );
        };
        match write_scene_file(path.as_path(), snapshot) {
            Ok(()) => {
                self.document_dirty = false;
                SceneCommandResult::new(
                    SCENE_SAVE_COMMAND,
                    format!("Saved scene to {}", path.display()),
                )
            }
            Err(error) => SceneCommandResult::new(SCENE_SAVE_COMMAND, error.to_string()),
        }
    }

    fn clear(&mut self) -> SceneCommandResult {
        self.clear_loaded_scene();
        SceneCommandResult::new(SCENE_CLEAR_COMMAND, "Cleared loaded scene")
    }

    pub(crate) fn on_project_closed(&mut self) {
        self.clear_loaded_scene();
        self.project_binding = None;
    }

    fn clear_selection(&mut self) -> SceneCommandResult {
        self.selection.clear();
        SceneCommandResult::new(SCENE_CLEAR_SELECTION_COMMAND, "Cleared scene selection")
    }

    fn clear_loaded_scene(&mut self) {
        self.snapshot = None;
        self.source = SceneSource::None;
        self.selection.clear();
        self.expansion.collapse_all();
        self.diagnostics.clear();
        self.last_command_result = None;
        self.document_dirty = false;
    }

    fn apply_loaded_snapshot(&mut self, snapshot: SceneSnapshot, source: SceneSource) {
        self.snapshot = Some(snapshot);
        self.source = source;
        self.selection.clear();
        self.expansion.collapse_all();
        self.diagnostics.clear();
        self.last_command_result = None;
        self.document_dirty = false;
    }

    #[cfg(test)]
    pub(crate) fn load_fixture_snapshot(&mut self, snapshot: SceneSnapshot) {
        self.apply_loaded_snapshot(snapshot, SceneSource::Local);
    }

    #[cfg(test)]
    fn expand_all(&mut self) -> SceneCommandResult {
        let Some(snapshot) = &self.snapshot else {
            return SceneCommandResult::new("scene.expand_all", "No scene loaded");
        };
        self.expansion.expand_all(snapshot);
        SceneCommandResult::new(
            "scene.expand_all",
            format!("Expanded {} nodes", self.expansion.len()),
        )
    }

    #[cfg(test)]
    fn collapse_all(&mut self) -> SceneCommandResult {
        self.expansion.collapse_all();
        SceneCommandResult::new("scene.collapse_all", "Collapsed scene tree")
    }
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            snapshot: None,
            source: SceneSource::None,
            project_binding: None,
            selection: SceneSelection::none(),
            expansion: SceneExpansion::new(),
            diagnostics: Vec::new(),
            last_command_result: None,
            document_dirty: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SceneSource {
    None,
    #[cfg(test)]
    Local,
    Project(PathBuf),
    #[allow(dead_code)]
    Adapter(AdapterId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneCommand {
    Load,
    Save,
    Clear,
    ClearSelection,
}

impl SceneCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            SCENE_LOAD_COMMAND => Some(Self::Load),
            SCENE_SAVE_COMMAND => Some(Self::Save),
            SCENE_CLEAR_COMMAND => Some(Self::Clear),
            SCENE_CLEAR_SELECTION_COMMAND => Some(Self::ClearSelection),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneCommandResult {
    command_id: String,
    message: String,
}

impl SceneCommandResult {
    fn new(command_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            command_id: command_id.into(),
            message: message.into(),
        }
    }

    pub(crate) fn command_id(&self) -> &str {
        self.command_id.as_str()
    }

    pub(crate) fn message(&self) -> &str {
        self.message.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_commands::{CommandId, CommandResult, RegisteredCommand, built_in_commands};
    use elcarax_project::{ProjectCreateRequest, create_project};
    use elcarax_scene_model::{
        ObjectSchema, PropertyGroup, PropertyKind, PropertyPath, PropertySchema, PropertyValue,
        SceneName, SceneObject, SceneObjectKind,
    };
    use elcarax_ui::{CommandPaletteAction, CommandPaletteEntry, CommandPaletteState, KeyboardKey};
    use std::fs;

    #[test]
    fn scene_load_reports_no_project_when_unbound() {
        let mut state = SceneState::default();
        let result = state.execute_command_id(SCENE_LOAD_COMMAND);
        assert_eq!(
            result.as_ref().map(SceneCommandResult::message),
            Some("No project open")
        );
        assert!(state.snapshot().is_none());
    }

    #[test]
    fn scene_save_clears_document_dirty_flag() {
        let temp = std::env::temp_dir().join(format!("elcarax-scene-dirty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let loaded = create_project(&ProjectCreateRequest::new(&temp, "Scene Dirty"));
        let loaded = match loaded {
            Ok(value) => value,
            Err(error) => panic!("create should succeed: {error}"),
        };
        let mut state = SceneState::default();
        state.on_project_opened(
            loaded.project.scene_root(),
            loaded.project.editor_settings().active_scene_relative(),
        );
        let _ = state.execute_command_id(SCENE_LOAD_COMMAND);
        state.mark_document_modified();
        assert!(state.has_unsaved_changes());
        let _ = state.execute_command_id(SCENE_SAVE_COMMAND);
        assert!(!state.has_unsaved_changes());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn scene_load_reads_default_scene_from_created_project() {
        let temp = std::env::temp_dir().join(format!("elcarax-scene-load-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let loaded = create_project(&ProjectCreateRequest::new(&temp, "Scene Load"));
        let loaded = match loaded {
            Ok(value) => value,
            Err(error) => panic!("create should succeed: {error}"),
        };
        let mut state = SceneState::default();
        state.on_project_opened(
            loaded.project.scene_root(),
            loaded.project.editor_settings().active_scene_relative(),
        );
        let result = state.execute_command_id(SCENE_LOAD_COMMAND);
        assert!(
            result
                .as_ref()
                .map(SceneCommandResult::message)
                .is_some_and(|message| message.starts_with("Loaded scene from")),
        );
        assert!(state.snapshot().is_some());
        assert!(matches!(state.source, SceneSource::Project(_)));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn scene_save_writes_loaded_project_scene() {
        let temp = std::env::temp_dir().join(format!("elcarax-scene-save-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let loaded = create_project(&ProjectCreateRequest::new(&temp, "Scene Save"));
        let loaded = match loaded {
            Ok(value) => value,
            Err(error) => panic!("create should succeed: {error}"),
        };
        let mut state = SceneState::default();
        state.on_project_opened(
            loaded.project.scene_root(),
            loaded.project.editor_settings().active_scene_relative(),
        );
        let _ = state.execute_command_id(SCENE_LOAD_COMMAND);
        if let Some(snapshot) = state.snapshot_mut() {
            let schema = ObjectSchema::new("Marker");
            let object = SceneObject::new("Saved Root", SceneObjectKind::World, schema.type_id);
            snapshot.add_schema(schema);
            snapshot.add_root_object(object);
        }
        let save = state.execute_command_id(SCENE_SAVE_COMMAND);
        assert!(
            save.as_ref()
                .map(SceneCommandResult::message)
                .is_some_and(|message| message.starts_with("Saved scene to"))
        );
        let reload = state.execute_command_id(SCENE_LOAD_COMMAND);
        assert!(reload.is_some());
        assert_eq!(state.snapshot().map(|value| value.object_count()), Some(1));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn scene_object_selection_updates_scene_selection() {
        let (mut state, object_id) = loaded_fixture_scene();
        assert!(state.select_object(object_id));
        assert!(state.selection.selected().is_some());
    }

    #[test]
    fn scene_clear_selection_clears_scene_selection() {
        let (mut state, object_id) = loaded_fixture_scene();
        assert!(state.select_object(object_id));
        let _ = state.execute_command_id(SCENE_CLEAR_SELECTION_COMMAND);
        assert_eq!(state.selection.selected(), None);
    }

    #[test]
    fn scene_expand_all_updates_expanded_set() {
        let (mut state, _) = loaded_fixture_scene();
        let _ = state.expand_all();
        assert_eq!(state.expansion.len(), 0);
    }

    #[test]
    fn scene_collapse_all_clears_expanded_set() {
        let (mut state, _) = loaded_fixture_scene();
        let _ = state.expand_all();
        let _ = state.collapse_all();
        assert!(state.expansion.is_empty());
    }

    #[test]
    fn scene_commands_are_discoverable_through_registry() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        for command_id in [SCENE_LOAD_COMMAND, SCENE_SAVE_COMMAND] {
            let id = match CommandId::new(command_id) {
                Ok(id) => id,
                Err(error) => panic!("scene command ID should be valid: {error}"),
            };
            assert!(matches!(registry.invoke(&id), CommandResult::Invoked(_)));
        }
    }

    #[test]
    fn command_palette_can_execute_scene_load() {
        let registry = match built_in_commands() {
            Ok(registry) => registry,
            Err(error) => panic!("built-ins should register: {error}"),
        };
        let mut palette = CommandPaletteState::new(
            registry
                .all()
                .into_iter()
                .map(palette_entry_from_command)
                .collect(),
        );
        palette.open();
        for character in SCENE_LOAD_COMMAND.chars() {
            assert_eq!(
                palette.handle_key(KeyboardKey::Character(character.to_string())),
                CommandPaletteAction::None
            );
        }
        assert_eq!(
            palette.handle_key(KeyboardKey::Enter),
            CommandPaletteAction::Execute
        );
        let selected_id = match palette.selected_entry() {
            Some(entry) => match CommandId::new(entry.id.as_str()) {
                Ok(id) => id,
                Err(error) => panic!("selected scene command ID should be valid: {error}"),
            },
            None => panic!("scene command should be selected"),
        };
        assert_eq!(selected_id.as_str(), SCENE_LOAD_COMMAND);
    }

    fn loaded_fixture_scene() -> (SceneState, SceneObjectId) {
        let path = fixture_path("general.name");
        let schema = ObjectSchema::new("Entity").with_property(PropertySchema::editable(
            path.clone(),
            "Name",
            PropertyKind::String,
            PropertyGroup::new("General"),
        ));
        let mut object =
            SceneObject::new("Fixture Object", SceneObjectKind::Character, schema.type_id);
        object.set_property(path, PropertyValue::String("Fixture Object".to_string()));
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Fixture Scene"));
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
        let mut state = SceneState::default();
        state.load_fixture_snapshot(snapshot);
        (state, object_id)
    }

    fn fixture_path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("fixture path should parse: {error}"),
        }
    }

    fn palette_entry_from_command(command: &RegisteredCommand) -> CommandPaletteEntry {
        CommandPaletteEntry::new(
            command.id().as_str(),
            command.name().as_str(),
            command.category().label(),
            command
                .description()
                .map(|description| description.as_str().to_string()),
            command.enabled(),
        )
    }
}
