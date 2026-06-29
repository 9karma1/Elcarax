use std::path::Path;

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterEditSource, AdapterId, AdapterName,
    AdapterVersion, GetSceneSnapshotRequest, HandshakeRequest, LoadProjectRequest,
    SetPropertyRequest, SetPropertyResponse,
};
#[cfg(test)]
use elcarax_adapter_host::AdapterSession;
use elcarax_adapter_host::{AdapterHost, AdapterHostError, AdapterHostState, AdapterProcessSpec};

use crate::adapter_display::{AdapterUiSnapshot, adapter_ui_snapshot};
use elcarax_scene_model::{
    PropertyChange, PropertyPath, PropertyValue, ScenePatch, prepare_property_change,
};

use crate::inspector_state::{
    EDIT_REDO_COMMAND, EDIT_UNDO_COMMAND, INSPECTOR_RENAME_PLAYER_DEMO_COMMAND,
    INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND, INSPECTOR_SET_PLAYER_SPEED_DEMO_COMMAND,
};
use crate::scene_state::{SceneSource, SceneState};

pub(crate) const ADAPTER_START_MOCK_COMMAND: &str = "adapter.start_mock";
pub(crate) const ADAPTER_HANDSHAKE_COMMAND: &str = "adapter.handshake";
pub(crate) const ADAPTER_LOAD_DEMO_PROJECT_COMMAND: &str = "adapter.load_demo_project";
pub(crate) const ADAPTER_LOAD_DEMO_SCENE_COMMAND: &str = "adapter.load_demo_scene";
pub(crate) const ADAPTER_SHOW_STATUS_COMMAND: &str = "adapter.show_status";
pub(crate) const ADAPTER_SHOW_DIAGNOSTICS_COMMAND: &str = "adapter.show_diagnostics";
pub(crate) const ADAPTER_STOP_MOCK_COMMAND: &str = "adapter.stop_mock";
pub(crate) const ADAPTER_INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND: &str =
    "adapter.inspector.set_player_health_demo";
pub(crate) const ADAPTER_INSPECTOR_SET_PLAYER_SPEED_DEMO_COMMAND: &str =
    "adapter.inspector.set_player_speed_demo";
pub(crate) const ADAPTER_INSPECTOR_RENAME_PLAYER_DEMO_COMMAND: &str =
    "adapter.inspector.rename_player_demo";
pub(crate) const ADAPTER_EDIT_UNDO_COMMAND: &str = "adapter.edit.undo";
pub(crate) const ADAPTER_EDIT_REDO_COMMAND: &str = "adapter.edit.redo";

pub(crate) struct AdapterState {
    connection: AdapterConnection,
    status: AdapterHostState,
    id: Option<AdapterId>,
    name: Option<AdapterName>,
    version: Option<AdapterVersion>,
    capabilities: Option<AdapterCapabilities>,
    diagnostics: Vec<AdapterDiagnostic>,
    edit_history: AdapterEditHistory,
    last_result: Option<AdapterCommandResult>,
}

enum AdapterConnection {
    None,
    Process(AdapterHost),
    #[cfg(test)]
    Fake(AdapterSession<elcarax_adapter_host::FakeAdapterTransport>),
}

impl AdapterState {
    pub(crate) fn execute_command_id(
        &mut self,
        id: &str,
        scene_state: &mut SceneState,
    ) -> Option<AdapterCommandResult> {
        let command = AdapterCommand::from_id(id)?;
        let result = match command {
            AdapterCommand::StartMock => self.start_mock(),
            AdapterCommand::Handshake => self.handshake(),
            AdapterCommand::LoadDemoProject => self.load_demo_project(),
            AdapterCommand::LoadDemoScene => self.load_demo_scene(scene_state),
            AdapterCommand::ShowStatus => self.show_status(),
            AdapterCommand::ShowDiagnostics => self.show_diagnostics(),
            AdapterCommand::StopMock => self.stop_mock(),
            AdapterCommand::SetPlayerHealthDemo => self.set_property_demo(
                scene_state,
                ADAPTER_INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND,
                "gameplay.health",
                PropertyValue::I64(65),
                "Set Adapter Player Health",
            ),
            AdapterCommand::SetPlayerSpeedDemo => self.set_property_demo(
                scene_state,
                ADAPTER_INSPECTOR_SET_PLAYER_SPEED_DEMO_COMMAND,
                "gameplay.speed",
                PropertyValue::F64(9.0),
                "Set Adapter Player Speed",
            ),
            AdapterCommand::RenamePlayerDemo => self.set_property_demo(
                scene_state,
                ADAPTER_INSPECTOR_RENAME_PLAYER_DEMO_COMMAND,
                "general.name",
                PropertyValue::String("Adapter Hero".to_string()),
                "Rename Adapter Player",
            ),
            AdapterCommand::Undo => self.undo(scene_state),
            AdapterCommand::Redo => self.redo(scene_state),
        };
        self.last_result = Some(result.clone());
        Some(result)
    }

    pub(crate) fn ui_snapshot(&self) -> AdapterUiSnapshot {
        adapter_ui_snapshot(
            self.status,
            self.name.as_ref(),
            self.version.as_ref(),
            self.capabilities.as_ref(),
            &self.diagnostics,
            self.last_result.as_ref().map(AdapterCommandResult::message),
        )
    }

    fn start_mock(&mut self) -> AdapterCommandResult {
        self.status = AdapterHostState::Starting;
        let spec = AdapterProcessSpec::cargo_mock_adapter();
        match AdapterHost::spawn(spec, Some(Path::new("."))) {
            Ok(host) => {
                self.connection = AdapterConnection::Process(host);
                let result = self.handshake();
                AdapterCommandResult::new(
                    ADAPTER_START_MOCK_COMMAND,
                    format!("started mock adapter; {}", result.message()),
                )
            }
            Err(error) => self.fail(ADAPTER_START_MOCK_COMMAND, error),
        }
    }

    fn handshake(&mut self) -> AdapterCommandResult {
        match &mut self.connection {
            AdapterConnection::Process(host) => {
                let result = host.handshake(HandshakeRequest::current("elcarax-app", None));
                match result {
                    Ok(info) => {
                        self.name = Some(info.name);
                        self.version = Some(info.version);
                        self.id = Some(info.id);
                        self.capabilities = Some(info.capabilities);
                        self.status = AdapterHostState::Connected;
                        AdapterCommandResult::new(ADAPTER_HANDSHAKE_COMMAND, "handshake succeeded")
                    }
                    Err(error) => self.fail(ADAPTER_HANDSHAKE_COMMAND, error),
                }
            }
            #[cfg(test)]
            AdapterConnection::Fake(session) => {
                match session.handshake(HandshakeRequest::current("elcarax-app", None)) {
                    Ok(info) => {
                        self.name = Some(info.name);
                        self.version = Some(info.version);
                        self.id = Some(info.id);
                        self.capabilities = Some(info.capabilities);
                        self.status = AdapterHostState::Connected;
                        AdapterCommandResult::new(ADAPTER_HANDSHAKE_COMMAND, "handshake succeeded")
                    }
                    Err(error) => self.fail(ADAPTER_HANDSHAKE_COMMAND, error),
                }
            }
            AdapterConnection::None => AdapterCommandResult::new(
                ADAPTER_HANDSHAKE_COMMAND,
                "Diagnostic: adapter is not running",
            ),
        }
    }

    fn load_demo_project(&mut self) -> AdapterCommandResult {
        let request = LoadProjectRequest { project_path: None };
        let result = match &mut self.connection {
            AdapterConnection::Process(host) => host.load_project(request),
            #[cfg(test)]
            AdapterConnection::Fake(session) => session.load_project(request),
            AdapterConnection::None => {
                return AdapterCommandResult::new(
                    ADAPTER_LOAD_DEMO_PROJECT_COMMAND,
                    "Diagnostic: adapter is not running",
                );
            }
        };
        match result {
            Ok(project) => AdapterCommandResult::new(
                ADAPTER_LOAD_DEMO_PROJECT_COMMAND,
                format!("loaded adapter project {}", project.display_name),
            ),
            Err(error) => self.fail(ADAPTER_LOAD_DEMO_PROJECT_COMMAND, error),
        }
    }

    fn load_demo_scene(&mut self, scene_state: &mut SceneState) -> AdapterCommandResult {
        let request = GetSceneSnapshotRequest { scene_id: None };
        let result = match &mut self.connection {
            AdapterConnection::Process(host) => host.get_scene_snapshot(request),
            #[cfg(test)]
            AdapterConnection::Fake(session) => session.get_scene_snapshot(request),
            AdapterConnection::None => {
                return AdapterCommandResult::new(
                    ADAPTER_LOAD_DEMO_SCENE_COMMAND,
                    "Diagnostic: adapter is not running",
                );
            }
        };
        match result {
            Ok(response) => {
                let count = response.snapshot.object_count();
                let adapter_id = self
                    .id
                    .clone()
                    .unwrap_or_else(|| AdapterId::new("unknown-adapter"));
                scene_state.load_external_snapshot(
                    response.snapshot,
                    adapter_id,
                    ADAPTER_LOAD_DEMO_SCENE_COMMAND,
                    format!(
                        "Loaded adapter scene from {} with {count} objects",
                        response.source_label
                    ),
                );
                AdapterCommandResult::new(
                    ADAPTER_LOAD_DEMO_SCENE_COMMAND,
                    format!("loaded adapter scene with {count} objects"),
                )
            }
            Err(error) => self.fail(ADAPTER_LOAD_DEMO_SCENE_COMMAND, error),
        }
    }

    fn show_status(&self) -> AdapterCommandResult {
        AdapterCommandResult::new(
            ADAPTER_SHOW_STATUS_COMMAND,
            self.ui_snapshot().adapter_status,
        )
    }

    fn show_diagnostics(&mut self) -> AdapterCommandResult {
        let result = match &mut self.connection {
            AdapterConnection::Process(host) => host.get_diagnostics(),
            #[cfg(test)]
            AdapterConnection::Fake(session) => {
                session.get_diagnostics(elcarax_adapter_api::GetDiagnosticsRequest)
            }
            AdapterConnection::None => {
                return AdapterCommandResult::new(
                    ADAPTER_SHOW_DIAGNOSTICS_COMMAND,
                    "Diagnostic: adapter is not running",
                );
            }
        };
        match result {
            Ok(response) => {
                self.diagnostics = response.diagnostics;
                AdapterCommandResult::new(
                    ADAPTER_SHOW_DIAGNOSTICS_COMMAND,
                    format!("{} adapter diagnostic(s)", self.diagnostics.len()),
                )
            }
            Err(error) => self.fail(ADAPTER_SHOW_DIAGNOSTICS_COMMAND, error),
        }
    }

    fn stop_mock(&mut self) -> AdapterCommandResult {
        let result = match &mut self.connection {
            AdapterConnection::Process(host) => host.shutdown(),
            #[cfg(test)]
            AdapterConnection::Fake(session) => session
                .shutdown_request(elcarax_adapter_api::ShutdownRequest)
                .and_then(|response| {
                    session.shutdown_transport()?;
                    Ok(response)
                }),
            AdapterConnection::None => {
                self.status = AdapterHostState::Stopped;
                return AdapterCommandResult::new(ADAPTER_STOP_MOCK_COMMAND, "adapter stopped");
            }
        };
        match result {
            Ok(_) => {
                self.status = AdapterHostState::Stopped;
                self.connection = AdapterConnection::None;
                self.edit_history.clear();
                AdapterCommandResult::new(ADAPTER_STOP_MOCK_COMMAND, "adapter stopped")
            }
            Err(error) => self.fail(ADAPTER_STOP_MOCK_COMMAND, error),
        }
    }

    fn set_property_demo(
        &mut self,
        scene_state: &mut SceneState,
        command_id: &str,
        path: &str,
        new_value: PropertyValue,
        label: &str,
    ) -> AdapterCommandResult {
        let path = match PropertyPath::parse(path) {
            Ok(path) => path,
            Err(error) => {
                return AdapterCommandResult::new(
                    command_id,
                    format!("Diagnostic: invalid property path: {error}"),
                );
            }
        };
        let change = match adapter_property_change(scene_state, &path, &new_value) {
            Ok(change) => change,
            Err(message) => {
                return AdapterCommandResult::new(command_id, format!("Diagnostic: {message}"));
            }
        };
        match self.apply_adapter_change(scene_state, &change, command_id) {
            Ok(()) => {
                self.edit_history.push(change.clone(), label);
                scene_state.record_status(
                    command_id,
                    format!(
                        "Adapter Command: {label} | {} -> {}",
                        change.old_value.display_label(),
                        change.new_value.display_label()
                    ),
                );
                AdapterCommandResult::new(
                    command_id,
                    format!(
                        "{label} confirmed | {} -> {}",
                        change.old_value.display_label(),
                        change.new_value.display_label()
                    ),
                )
            }
            Err(message) => {
                scene_state.record_status(command_id, format!("Diagnostic: {message}"));
                AdapterCommandResult::new(command_id, format!("Diagnostic: {message}"))
            }
        }
    }

    fn undo(&mut self, scene_state: &mut SceneState) -> AdapterCommandResult {
        let Some(entry) = self.edit_history.undo_candidate() else {
            return AdapterCommandResult::new(
                ADAPTER_EDIT_UNDO_COMMAND,
                "Diagnostic: nothing to undo",
            );
        };
        let reverse = entry.reverse_change();
        match self.apply_adapter_change(scene_state, &reverse, ADAPTER_EDIT_UNDO_COMMAND) {
            Ok(()) => {
                self.edit_history.commit_undo();
                scene_state.record_status(
                    ADAPTER_EDIT_UNDO_COMMAND,
                    "Adapter Command: adapter.edit.undo",
                );
                AdapterCommandResult::new(ADAPTER_EDIT_UNDO_COMMAND, "adapter undo confirmed")
            }
            Err(message) => {
                scene_state
                    .record_status(ADAPTER_EDIT_UNDO_COMMAND, format!("Diagnostic: {message}"));
                AdapterCommandResult::new(
                    ADAPTER_EDIT_UNDO_COMMAND,
                    format!("Diagnostic: {message}"),
                )
            }
        }
    }

    fn redo(&mut self, scene_state: &mut SceneState) -> AdapterCommandResult {
        let Some(entry) = self.edit_history.redo_candidate() else {
            return AdapterCommandResult::new(
                ADAPTER_EDIT_REDO_COMMAND,
                "Diagnostic: nothing to redo",
            );
        };
        let change = entry.change.clone();
        match self.apply_adapter_change(scene_state, &change, ADAPTER_EDIT_REDO_COMMAND) {
            Ok(()) => {
                self.edit_history.commit_redo();
                scene_state.record_status(
                    ADAPTER_EDIT_REDO_COMMAND,
                    "Adapter Command: adapter.edit.redo",
                );
                AdapterCommandResult::new(ADAPTER_EDIT_REDO_COMMAND, "adapter redo confirmed")
            }
            Err(message) => {
                scene_state
                    .record_status(ADAPTER_EDIT_REDO_COMMAND, format!("Diagnostic: {message}"));
                AdapterCommandResult::new(
                    ADAPTER_EDIT_REDO_COMMAND,
                    format!("Diagnostic: {message}"),
                )
            }
        }
    }

    fn apply_adapter_change(
        &mut self,
        scene_state: &mut SceneState,
        change: &PropertyChange,
        command_id: &str,
    ) -> Result<(), String> {
        let request = SetPropertyRequest {
            scene_id: change.scene_id,
            object_id: change.object_id,
            path: change.path.clone(),
            expected_old_value: Some(change.old_value.clone()),
            new_value: change.new_value.clone(),
            transaction_id: command_id.to_string(),
            edit_source: AdapterEditSource::Inspector,
        };
        let response = self.send_set_property(request)?;
        if !response.status.is_accepted() {
            let message = writeback_failure_message(&response);
            self.diagnostics.extend(response.diagnostics);
            return Err(message);
        }
        let Some(snapshot) = scene_state.snapshot_mut() else {
            return Err("scene not loaded".to_string());
        };
        let patch = response.patch.unwrap_or_else(|| {
            ScenePatch::property_updated(
                response.object_id,
                response.path.clone(),
                response
                    .confirmed_new_value
                    .clone()
                    .unwrap_or_else(|| change.new_value.clone()),
            )
        });
        patch.apply(snapshot).map_err(|error| error.message())
    }

    fn send_set_property(
        &mut self,
        request: SetPropertyRequest,
    ) -> Result<SetPropertyResponse, String> {
        let result = match &mut self.connection {
            AdapterConnection::Process(host) => host.set_property(request),
            #[cfg(test)]
            AdapterConnection::Fake(session) => session.set_property(request),
            AdapterConnection::None => return Err("adapter not connected".to_string()),
        };
        result.map_err(|error| {
            self.status = AdapterHostState::Failed;
            format!("{error}")
        })
    }

    fn fail(&mut self, command_id: &str, error: AdapterHostError) -> AdapterCommandResult {
        self.status = AdapterHostState::Failed;
        AdapterCommandResult::new(command_id, format!("Diagnostic: {error}"))
    }

    #[cfg(test)]
    fn with_fake_session(
        session: AdapterSession<elcarax_adapter_host::FakeAdapterTransport>,
    ) -> Self {
        Self {
            connection: AdapterConnection::Fake(session),
            status: AdapterHostState::Starting,
            id: None,
            name: None,
            version: None,
            capabilities: None,
            diagnostics: Vec::new(),
            edit_history: AdapterEditHistory::default(),
            last_result: None,
        }
    }

    #[cfg(test)]
    fn fake_writes(&self) -> &[String] {
        match &self.connection {
            AdapterConnection::Fake(session) => session.transport().writes(),
            AdapterConnection::None | AdapterConnection::Process(_) => {
                panic!("expected fake adapter connection")
            }
        }
    }
}

impl Default for AdapterState {
    fn default() -> Self {
        Self {
            connection: AdapterConnection::None,
            status: AdapterHostState::Disconnected,
            id: None,
            name: None,
            version: None,
            capabilities: None,
            diagnostics: Vec::new(),
            edit_history: AdapterEditHistory::default(),
            last_result: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdapterCommandResult {
    command_id: String,
    message: String,
}

impl AdapterCommandResult {
    fn new(command_id: &str, message: impl Into<String>) -> Self {
        Self {
            command_id: command_id.to_string(),
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        self.message.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdapterCommand {
    StartMock,
    Handshake,
    LoadDemoProject,
    LoadDemoScene,
    ShowStatus,
    ShowDiagnostics,
    StopMock,
    SetPlayerHealthDemo,
    SetPlayerSpeedDemo,
    RenamePlayerDemo,
    Undo,
    Redo,
}

impl AdapterCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            ADAPTER_START_MOCK_COMMAND => Some(Self::StartMock),
            ADAPTER_HANDSHAKE_COMMAND => Some(Self::Handshake),
            ADAPTER_LOAD_DEMO_PROJECT_COMMAND => Some(Self::LoadDemoProject),
            ADAPTER_LOAD_DEMO_SCENE_COMMAND => Some(Self::LoadDemoScene),
            ADAPTER_SHOW_STATUS_COMMAND => Some(Self::ShowStatus),
            ADAPTER_SHOW_DIAGNOSTICS_COMMAND => Some(Self::ShowDiagnostics),
            ADAPTER_STOP_MOCK_COMMAND => Some(Self::StopMock),
            ADAPTER_INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND => Some(Self::SetPlayerHealthDemo),
            ADAPTER_INSPECTOR_SET_PLAYER_SPEED_DEMO_COMMAND => Some(Self::SetPlayerSpeedDemo),
            ADAPTER_INSPECTOR_RENAME_PLAYER_DEMO_COMMAND => Some(Self::RenamePlayerDemo),
            ADAPTER_EDIT_UNDO_COMMAND => Some(Self::Undo),
            ADAPTER_EDIT_REDO_COMMAND => Some(Self::Redo),
            _ => None,
        }
    }
}

#[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
pub(crate) fn adapter_command_for_inspector_edit(id: &str) -> Option<&'static str> {
    match id {
        INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND => {
            Some(ADAPTER_INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND)
        }
        INSPECTOR_SET_PLAYER_SPEED_DEMO_COMMAND => {
            Some(ADAPTER_INSPECTOR_SET_PLAYER_SPEED_DEMO_COMMAND)
        }
        INSPECTOR_RENAME_PLAYER_DEMO_COMMAND => Some(ADAPTER_INSPECTOR_RENAME_PLAYER_DEMO_COMMAND),
        EDIT_UNDO_COMMAND => Some(ADAPTER_EDIT_UNDO_COMMAND),
        EDIT_REDO_COMMAND => Some(ADAPTER_EDIT_REDO_COMMAND),
        _ => None,
    }
}

#[derive(Debug, Clone, Default)]
struct AdapterEditHistory {
    undo_stack: Vec<AdapterEditEntry>,
    redo_stack: Vec<AdapterEditEntry>,
}

impl AdapterEditHistory {
    fn push(&mut self, change: PropertyChange, _label: &str) {
        self.undo_stack.push(AdapterEditEntry { change });
        self.redo_stack.clear();
    }

    fn undo_candidate(&self) -> Option<&AdapterEditEntry> {
        self.undo_stack.last()
    }

    fn redo_candidate(&self) -> Option<&AdapterEditEntry> {
        self.redo_stack.last()
    }

    fn commit_undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(entry);
        }
    }

    fn commit_redo(&mut self) {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(entry);
        }
    }

    fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[derive(Debug, Clone)]
struct AdapterEditEntry {
    change: PropertyChange,
}

impl AdapterEditEntry {
    fn reverse_change(&self) -> PropertyChange {
        PropertyChange {
            scene_id: self.change.scene_id,
            object_id: self.change.object_id,
            path: self.change.path.clone(),
            old_value: self.change.new_value.clone(),
            new_value: self.change.old_value.clone(),
        }
    }
}

fn adapter_property_change(
    scene_state: &SceneState,
    path: &PropertyPath,
    new_value: &PropertyValue,
) -> Result<PropertyChange, String> {
    let SceneSource::Adapter(_) = scene_state.source() else {
        return Err("scene is not adapter-backed".to_string());
    };
    let Some(snapshot) = scene_state.snapshot() else {
        return Err("scene not loaded".to_string());
    };
    let Some(object_id) = scene_state.selection().selected() else {
        return Err("no object selected".to_string());
    };
    prepare_property_change(snapshot, object_id, path, new_value).map_err(|error| error.message())
}

fn writeback_failure_message(response: &SetPropertyResponse) -> String {
    if let Some(diagnostic) = response.diagnostics.first() {
        return format!(
            "adapter write rejected ({:?}): {}",
            response.status, diagnostic.message
        );
    }
    format!("adapter write rejected ({:?})", response.status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspector_state::{INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND, InspectorState};
    use elcarax_adapter_api::{
        AdapterEvent, AdapterId, AdapterLog, AdapterRequestId, AdapterResponseMessage,
        GetDiagnosticsResponse, GetSceneSnapshotResponse, HandshakeResponse, LoadProjectResponse,
        ProtocolVersion, SetPropertyResponse, SetPropertyStatus, ShutdownResponse,
        decode_request_line,
    };
    use elcarax_adapter_host::{FakeAdapterTransport, event_line, response_line};
    use elcarax_commands::CommandHistory;
    use elcarax_scene_model::{ScenePatch, demo_scene_snapshot};

    #[test]
    fn fake_handshake_command_changes_status() {
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            AdapterResponseMessage::Handshake(handshake_response()),
        )]);
        let mut scene = SceneState::default();
        let result = state.execute_command_id(ADAPTER_HANDSHAKE_COMMAND, &mut scene);
        assert!(result.is_some());
        assert_eq!(state.status, AdapterHostState::Connected);
    }

    #[test]
    fn adapter_load_demo_scene_updates_scene_snapshot() {
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            AdapterResponseMessage::GetSceneSnapshot(GetSceneSnapshotResponse {
                snapshot: demo_scene_snapshot(),
                source_label: "mock-adapter".to_string(),
            }),
        )]);
        let mut scene = SceneState::default();
        let result = state.execute_command_id(ADAPTER_LOAD_DEMO_SCENE_COMMAND, &mut scene);
        assert!(result.is_some());
        assert_eq!(
            scene.snapshot().map(|snapshot| snapshot.object_count()),
            Some(10)
        );
        assert_eq!(scene.ui_snapshot().scene_name, "Demo Scene".to_string());
    }

    #[test]
    fn adapter_show_diagnostics_records_diagnostics() {
        let mut state = state_with_lines(vec![
            event(AdapterEvent::Log(AdapterLog::info("ok"))),
            response(
                AdapterRequestId(1),
                AdapterResponseMessage::GetDiagnostics(GetDiagnosticsResponse {
                    diagnostics: vec![AdapterDiagnostic::info("mock", "ready")],
                }),
            ),
        ]);
        let mut scene = SceneState::default();
        let result = state.execute_command_id(ADAPTER_SHOW_DIAGNOSTICS_COMMAND, &mut scene);
        assert_eq!(
            result.map(|result| result.message().to_string()),
            Some("1 adapter diagnostic(s)".to_string())
        );
        assert_eq!(state.diagnostics.len(), 1);
    }

    #[test]
    fn adapter_stop_mock_clears_connection() {
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            AdapterResponseMessage::Shutdown(ShutdownResponse { accepted: true }),
        )]);
        let mut scene = SceneState::default();
        let result = state.execute_command_id(ADAPTER_STOP_MOCK_COMMAND, &mut scene);
        assert_eq!(
            result.map(|result| result.message().to_string()),
            Some("adapter stopped".to_string())
        );
        assert_eq!(state.status, AdapterHostState::Stopped);
    }

    #[test]
    fn adapter_backed_edit_sends_request_and_updates_scene() {
        let mut scene = adapter_player_scene();
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            accepted_health_response(PropertyValue::I64(100), PropertyValue::I64(65)),
        )]);
        let result =
            state.execute_command_id(ADAPTER_INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND, &mut scene);
        assert!(result.is_some());
        assert_eq!(player_health(&scene), PropertyValue::I64(65));
        let request = match state.fake_writes().first() {
            Some(line) => match decode_request_line(line) {
                Ok(request) => request,
                Err(error) => panic!("request should decode: {error}"),
            },
            None => panic!("adapter request should be written"),
        };
        assert!(matches!(
            request.message,
            elcarax_adapter_api::AdapterRequestMessage::SetProperty(_)
        ));
    }

    #[test]
    fn failed_adapter_edit_records_diagnostic_and_does_not_mutate_value() {
        let mut scene = adapter_player_scene();
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            rejected_health_response(),
        )]);
        let result =
            state.execute_command_id(ADAPTER_INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND, &mut scene);
        assert!(result.is_some_and(|result| result.message().contains("Diagnostic:")));
        assert_eq!(player_health(&scene), PropertyValue::I64(100));
    }

    #[test]
    fn adapter_backed_undo_and_redo_send_writebacks() {
        let mut scene = adapter_player_scene();
        let mut state = state_with_lines(vec![
            response(
                AdapterRequestId(1),
                accepted_health_response(PropertyValue::I64(100), PropertyValue::I64(65)),
            ),
            response(
                AdapterRequestId(2),
                accepted_health_response(PropertyValue::I64(65), PropertyValue::I64(100)),
            ),
            response(
                AdapterRequestId(3),
                accepted_health_response(PropertyValue::I64(100), PropertyValue::I64(65)),
            ),
        ]);
        let _ =
            state.execute_command_id(ADAPTER_INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND, &mut scene);
        let _ = state.execute_command_id(ADAPTER_EDIT_UNDO_COMMAND, &mut scene);
        assert_eq!(player_health(&scene), PropertyValue::I64(100));
        let _ = state.execute_command_id(ADAPTER_EDIT_REDO_COMMAND, &mut scene);
        assert_eq!(player_health(&scene), PropertyValue::I64(65));
        assert_eq!(state.fake_writes().len(), 3);
    }

    #[test]
    fn disconnected_adapter_edit_fails_clearly() {
        let mut scene = adapter_player_scene();
        let mut state = AdapterState::default();
        let result =
            state.execute_command_id(ADAPTER_INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND, &mut scene);
        assert!(result.is_some_and(|result| result.message().contains("adapter not connected")));
        assert_eq!(player_health(&scene), PropertyValue::I64(100));
    }

    #[test]
    fn local_demo_edit_path_still_works() {
        let mut scene = SceneState::default();
        let _ = scene.execute_command_id(crate::scene_state::SCENE_LOAD_DEMO_COMMAND);
        let _ = scene.execute_command_id(crate::scene_state::SCENE_SELECT_PLAYER_COMMAND);
        let mut inspector = InspectorState::default();
        let mut history = CommandHistory::new();
        let result = inspector.execute_edit_command_id(
            INSPECTOR_SET_PLAYER_HEALTH_DEMO_COMMAND,
            &mut scene,
            &mut history,
        );
        assert!(result.is_some());
        assert_eq!(history.undo_count(), 1);
    }

    fn state_with_lines(lines: Vec<String>) -> AdapterState {
        AdapterState::with_fake_session(AdapterSession::new(FakeAdapterTransport::new(lines)))
    }

    fn adapter_player_scene() -> SceneState {
        let snapshot = demo_scene_snapshot();
        let mut scene = SceneState::default();
        scene.load_external_snapshot(
            snapshot,
            AdapterId::new("mock"),
            "test",
            "Loaded adapter scene",
        );
        let _ = scene.execute_command_id(crate::scene_state::SCENE_SELECT_PLAYER_COMMAND);
        scene
    }

    fn handshake_response() -> HandshakeResponse {
        HandshakeResponse {
            adapter_id: AdapterId::new("mock"),
            adapter_name: AdapterName::new("Mock Adapter"),
            adapter_version: AdapterVersion::new("0.1.0"),
            protocol_version: ProtocolVersion::V0,
            capabilities: AdapterCapabilities::mock_milestone_12(),
        }
    }

    fn response(request_id: AdapterRequestId, message: AdapterResponseMessage) -> String {
        match response_line(request_id, message) {
            Ok(line) => line,
            Err(error) => panic!("response should serialize: {error}"),
        }
    }

    fn accepted_health_response(
        old_value: PropertyValue,
        new_value: PropertyValue,
    ) -> AdapterResponseMessage {
        let snapshot = demo_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        };
        let path = path("gameplay.health");
        AdapterResponseMessage::SetProperty(SetPropertyResponse {
            status: SetPropertyStatus::Accepted,
            scene_id: snapshot.scene_id(),
            object_id: player.id,
            path: path.clone(),
            old_value: Some(old_value),
            confirmed_new_value: Some(new_value.clone()),
            patch: Some(ScenePatch::property_updated(player.id, path, new_value)),
            diagnostics: Vec::new(),
        })
    }

    fn rejected_health_response() -> AdapterResponseMessage {
        let snapshot = demo_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        };
        AdapterResponseMessage::SetProperty(SetPropertyResponse {
            status: SetPropertyStatus::Rejected,
            scene_id: snapshot.scene_id(),
            object_id: player.id,
            path: path("gameplay.health"),
            old_value: None,
            confirmed_new_value: None,
            patch: None,
            diagnostics: vec![AdapterDiagnostic::info("mock", "rejected")],
        })
    }

    fn event(event: AdapterEvent) -> String {
        match event_line(event) {
            Ok(line) => line,
            Err(error) => panic!("event should serialize: {error}"),
        }
    }

    #[allow(dead_code)]
    fn project_response() -> AdapterResponseMessage {
        AdapterResponseMessage::LoadProject(LoadProjectResponse {
            display_name: "Mock Adapter Demo Project".to_string(),
            root_path: None,
        })
    }

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("test path should parse: {error}"),
        }
    }

    fn player_health(scene: &SceneState) -> PropertyValue {
        let snapshot = match scene.snapshot() {
            Some(snapshot) => snapshot,
            None => panic!("scene should be loaded"),
        };
        let player = match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        };
        match player.property(&path("gameplay.health")) {
            Some(value) => value.clone(),
            None => panic!("health should exist"),
        }
    }
}
