use std::path::Path;

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterName, AdapterVersion, GetSceneSnapshotRequest,
    HandshakeRequest, LoadProjectRequest,
};
#[cfg(test)]
use elcarax_adapter_host::AdapterSession;
use elcarax_adapter_host::{AdapterHost, AdapterHostError, AdapterHostState, AdapterProcessSpec};

use crate::adapter_display::{AdapterUiSnapshot, adapter_ui_snapshot};
use crate::scene_state::SceneState;

pub(crate) const ADAPTER_START_MOCK_COMMAND: &str = "adapter.start_mock";
pub(crate) const ADAPTER_HANDSHAKE_COMMAND: &str = "adapter.handshake";
pub(crate) const ADAPTER_LOAD_DEMO_PROJECT_COMMAND: &str = "adapter.load_demo_project";
pub(crate) const ADAPTER_LOAD_DEMO_SCENE_COMMAND: &str = "adapter.load_demo_scene";
pub(crate) const ADAPTER_SHOW_STATUS_COMMAND: &str = "adapter.show_status";
pub(crate) const ADAPTER_SHOW_DIAGNOSTICS_COMMAND: &str = "adapter.show_diagnostics";
pub(crate) const ADAPTER_STOP_MOCK_COMMAND: &str = "adapter.stop_mock";

pub(crate) struct AdapterState {
    connection: AdapterConnection,
    status: AdapterHostState,
    name: Option<AdapterName>,
    version: Option<AdapterVersion>,
    capabilities: Option<AdapterCapabilities>,
    diagnostics: Vec<AdapterDiagnostic>,
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
                scene_state.load_external_snapshot(
                    response.snapshot,
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
                AdapterCommandResult::new(ADAPTER_STOP_MOCK_COMMAND, "adapter stopped")
            }
            Err(error) => self.fail(ADAPTER_STOP_MOCK_COMMAND, error),
        }
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
            name: None,
            version: None,
            capabilities: None,
            diagnostics: Vec::new(),
            last_result: None,
        }
    }
}

impl Default for AdapterState {
    fn default() -> Self {
        Self {
            connection: AdapterConnection::None,
            status: AdapterHostState::Disconnected,
            name: None,
            version: None,
            capabilities: None,
            diagnostics: Vec::new(),
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
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_adapter_api::{
        AdapterEvent, AdapterId, AdapterLog, AdapterRequestId, AdapterResponseMessage,
        GetDiagnosticsResponse, GetSceneSnapshotResponse, HandshakeResponse, LoadProjectResponse,
        ProtocolVersion, ShutdownResponse,
    };
    use elcarax_adapter_host::{FakeAdapterTransport, event_line, response_line};
    use elcarax_scene_model::demo_scene_snapshot;

    #[test]
    fn fake_handshake_command_changes_status() {
        let mut state = state_with_lines(vec![response(AdapterResponseMessage::Handshake(
            handshake_response(),
        ))]);
        let mut scene = SceneState::default();
        let result = state.execute_command_id(ADAPTER_HANDSHAKE_COMMAND, &mut scene);
        assert!(result.is_some());
        assert_eq!(state.status, AdapterHostState::Connected);
    }

    #[test]
    fn adapter_load_demo_scene_updates_scene_snapshot() {
        let mut state = state_with_lines(vec![response(AdapterResponseMessage::GetSceneSnapshot(
            GetSceneSnapshotResponse {
                snapshot: demo_scene_snapshot(),
                source_label: "mock-adapter".to_string(),
            },
        ))]);
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
            response(AdapterResponseMessage::GetDiagnostics(
                GetDiagnosticsResponse {
                    diagnostics: vec![AdapterDiagnostic::info("mock", "ready")],
                },
            )),
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
        let mut state = state_with_lines(vec![response(AdapterResponseMessage::Shutdown(
            ShutdownResponse { accepted: true },
        ))]);
        let mut scene = SceneState::default();
        let result = state.execute_command_id(ADAPTER_STOP_MOCK_COMMAND, &mut scene);
        assert_eq!(
            result.map(|result| result.message().to_string()),
            Some("adapter stopped".to_string())
        );
        assert_eq!(state.status, AdapterHostState::Stopped);
    }

    fn state_with_lines(lines: Vec<String>) -> AdapterState {
        AdapterState::with_fake_session(AdapterSession::new(FakeAdapterTransport::new(lines)))
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

    fn response(message: AdapterResponseMessage) -> String {
        match response_line(AdapterRequestId(1), message) {
            Ok(line) => line,
            Err(error) => panic!("response should serialize: {error}"),
        }
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
}
