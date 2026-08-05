//! Unified session edit authority for local and adapter-backed scenes.

use elcarax_adapter_api::{AdapterEditSource, SetPropertyRequest};
use elcarax_commands::{
    ApplyScenePatchCommand, CommandContext, CommandHistory, RedoCommand, SceneMutationSink,
    UndoCommand,
};
use elcarax_scene_model::{
    InspectorDiagnostic, PropertyChange, PropertyPath, PropertyValue, ScenePatch,
    prepare_property_change,
};

use crate::adapter_state::AdapterState;
use crate::inspector_state::{
    EDIT_REDO_COMMAND, EDIT_SET_PROPERTY_COMMAND, EDIT_UNDO_COMMAND, InspectorCommandResult,
};
use crate::scene_state::SceneState;

pub(crate) struct SessionEditService;

impl SessionEditService {
    pub(crate) fn commit_property(
        scene: &mut SceneState,
        history: &mut CommandHistory,
        adapter: Option<&mut AdapterState>,
        component_id: elcarax_scene_model::ComponentInstanceId,
        path: &PropertyPath,
        new_value: PropertyValue,
        label: &str,
    ) -> Result<String, String> {
        let Some(snapshot) = scene.snapshot() else {
            return Err(InspectorDiagnostic::NoSceneLoaded.message().to_string());
        };
        let Some(object_id) = scene.selection().selected() else {
            return Err(InspectorDiagnostic::NoObjectSelected.message().to_string());
        };
        let change = prepare_property_change(snapshot, object_id, component_id, path, &new_value)
            .map_err(|error| error.message())?;
        let old_label = change.old_value.display_label();
        let new_label = change.new_value.display_label();

        if scene.adapter_id().is_some() {
            let adapter = adapter.ok_or_else(|| {
                "adapter-backed scene requires a connected adapter for edits".to_string()
            })?;
            execute_remote_property(scene, history, adapter, change, label)?;
        } else {
            execute_local_property(scene, history, change, label)?;
        }

        Ok(format!("Command: {label} | {old_label} -> {new_label}"))
    }

    pub(crate) fn undo(
        scene: &mut SceneState,
        history: &mut CommandHistory,
        adapter: Option<&mut AdapterState>,
    ) -> InspectorCommandResult {
        match execute_history_op(scene, history, adapter, HistoryOp::Undo) {
            Ok(()) => edit_status(scene, EDIT_UNDO_COMMAND, "Command: edit.undo", true),
            Err(message) => edit_status(scene, EDIT_UNDO_COMMAND, message, false),
        }
    }

    pub(crate) fn redo(
        scene: &mut SceneState,
        history: &mut CommandHistory,
        adapter: Option<&mut AdapterState>,
    ) -> InspectorCommandResult {
        match execute_history_op(scene, history, adapter, HistoryOp::Redo) {
            Ok(()) => edit_status(scene, EDIT_REDO_COMMAND, "Command: edit.redo", true),
            Err(message) => edit_status(scene, EDIT_REDO_COMMAND, message, false),
        }
    }

    pub(crate) fn commit_property_result(
        scene: &mut SceneState,
        history: &mut CommandHistory,
        adapter: Option<&mut AdapterState>,
        component_id: elcarax_scene_model::ComponentInstanceId,
        path: &PropertyPath,
        new_value: PropertyValue,
        label: &str,
    ) -> InspectorCommandResult {
        match Self::commit_property(
            scene,
            history,
            adapter,
            component_id,
            path,
            new_value,
            label,
        ) {
            Ok(message) => edit_status(scene, EDIT_SET_PROPERTY_COMMAND, message, true),
            Err(message) => edit_status(scene, EDIT_SET_PROPERTY_COMMAND, message, false),
        }
    }
}

#[derive(Clone, Copy)]
enum HistoryOp {
    Undo,
    Redo,
}

fn execute_local_property(
    scene: &mut SceneState,
    history: &mut CommandHistory,
    change: PropertyChange,
    label: &str,
) -> Result<(), String> {
    let Some(snapshot) = scene.snapshot_mut() else {
        return Err(InspectorDiagnostic::NoSceneLoaded.message().to_string());
    };
    let mut context = CommandContext::local(snapshot);
    history
        .execute(
            Box::new(ApplyScenePatchCommand::from_property_change(
                change,
                label.to_string(),
            )),
            &mut context,
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn execute_remote_property(
    scene: &mut SceneState,
    history: &mut CommandHistory,
    adapter: &mut AdapterState,
    change: PropertyChange,
    label: &str,
) -> Result<(), String> {
    let Some(snapshot) = scene.snapshot_mut() else {
        return Err(InspectorDiagnostic::NoSceneLoaded.message().to_string());
    };
    let mut context = CommandContext::with_sink(snapshot, adapter);
    history
        .execute(
            Box::new(ApplyScenePatchCommand::remote_property(
                change,
                label.to_string(),
            )),
            &mut context,
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn execute_history_op(
    scene: &mut SceneState,
    history: &mut CommandHistory,
    adapter: Option<&mut AdapterState>,
    op: HistoryOp,
) -> Result<(), String> {
    let is_adapter = scene.adapter_id().is_some();
    let Some(snapshot) = scene.snapshot_mut() else {
        return Err("No scene loaded".to_string());
    };
    let effect = if is_adapter {
        let adapter = adapter
            .ok_or_else(|| "adapter-backed scene requires a connected adapter".to_string())?;
        let mut context = CommandContext::with_sink(snapshot, adapter);
        match op {
            HistoryOp::Undo => UndoCommand::apply(history, &mut context),
            HistoryOp::Redo => RedoCommand::apply(history, &mut context),
        }
    } else {
        let mut context = CommandContext::local(snapshot);
        match op {
            HistoryOp::Undo => UndoCommand::apply(history, &mut context),
            HistoryOp::Redo => RedoCommand::apply(history, &mut context),
        }
    }
    .map_err(|error| error.to_string())?;

    match effect {
        Some(_) => Ok(()),
        None => Err(match op {
            HistoryOp::Undo => "Nothing to undo".to_string(),
            HistoryOp::Redo => "Nothing to redo".to_string(),
        }),
    }
}

fn edit_status(
    scene: &mut SceneState,
    command_id: &str,
    message: impl Into<String>,
    success: bool,
) -> InspectorCommandResult {
    let message = message.into();
    if success {
        scene.mark_document_modified();
        scene.record_status(command_id, message.clone());
        InspectorCommandResult::new(command_id, message)
    } else {
        let diagnostic = if message.starts_with("Diagnostic:") {
            message
        } else {
            format!("Diagnostic: {message}")
        };
        scene.record_status(command_id, diagnostic.clone());
        InspectorCommandResult::new(command_id, diagnostic)
    }
}

impl SceneMutationSink for AdapterState {
    fn confirm_property_change(
        &mut self,
        change: &PropertyChange,
    ) -> std::result::Result<ScenePatch, String> {
        let request = SetPropertyRequest {
            scene_id: change.scene_id,
            object_id: change.object_id,
            component_id: change.component_id,
            path: change.path.clone(),
            expected_old_value: Some(change.old_value.clone()),
            new_value: change.new_value.clone(),
            transaction_id: format!("edit-{}-{}", change.object_id.get(), change.path),
            edit_source: AdapterEditSource::Inspector,
        };
        self.confirm_set_property(request)
    }
}
