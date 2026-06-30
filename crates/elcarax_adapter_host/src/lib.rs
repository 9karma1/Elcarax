//! Adapter process supervision and JSON-line transport for Elcarax.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterError, AdapterEvent, AdapterId, AdapterLine,
    AdapterLog, AdapterName, AdapterRequest, AdapterRequestId, AdapterRequestMessage,
    AdapterResponse, AdapterResponseMessage, AdapterVersion, ErrorResponse, GetDiagnosticsRequest,
    GetDiagnosticsResponse, GetSceneSnapshotRequest, GetSceneSnapshotResponse,
    GetViewportFrameRequest, GetViewportFrameResponse, HandshakeRequest, LoadProjectRequest,
    LoadProjectResponse, SetPropertyRequest, SetPropertyResponse, ShutdownRequest,
    ShutdownResponse, decode_adapter_line, encode_request_line,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterHostState {
    Disconnected,
    Starting,
    Connected,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterHostError {
    SpawnFailed(String),
    MissingStdin,
    MissingStdout,
    TransportWrite(String),
    TransportRead(String),
    InvalidJson(String),
    AdapterExited,
    MismatchedRequestId {
        expected: AdapterRequestId,
        actual: AdapterRequestId,
    },
    UnexpectedResponse(String),
    Adapter(AdapterError),
}

impl fmt::Display for AdapterHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpawnFailed(message) => write!(formatter, "failed to spawn adapter: {message}"),
            Self::MissingStdin => write!(formatter, "adapter process stdin was not captured"),
            Self::MissingStdout => write!(formatter, "adapter process stdout was not captured"),
            Self::TransportWrite(message) => write!(formatter, "adapter write failed: {message}"),
            Self::TransportRead(message) => write!(formatter, "adapter read failed: {message}"),
            Self::InvalidJson(message) => write!(formatter, "invalid adapter JSON: {message}"),
            Self::AdapterExited => write!(formatter, "adapter exited before response"),
            Self::MismatchedRequestId { expected, actual } => write!(
                formatter,
                "adapter response request ID mismatch: expected {}, received {}",
                expected.0, actual.0
            ),
            Self::UnexpectedResponse(message) => {
                write!(formatter, "unexpected adapter response: {message}")
            }
            Self::Adapter(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for AdapterHostError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterProcessSpec {
    pub executable: PathBuf,
    pub args: Vec<String>,
}

impl AdapterProcessSpec {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
        }
    }

    pub fn stdio_game_adapter() -> Self {
        Self::cargo_mock_adapter()
    }

    pub fn cargo_mock_adapter() -> Self {
        Self {
            executable: PathBuf::from("cargo"),
            args: vec![
                "run".to_string(),
                "--quiet".to_string(),
                "-p".to_string(),
                "elcarax_game_adapter".to_string(),
            ],
        }
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }
}

pub trait AdapterTransport {
    fn write_line(&mut self, line: &str) -> Result<(), AdapterHostError>;
    fn read_line(&mut self) -> Result<Option<String>, AdapterHostError>;
    fn shutdown(&mut self) -> Result<(), AdapterHostError>;
}

pub struct AdapterProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl AdapterProcess {
    pub fn spawn(
        spec: &AdapterProcessSpec,
        current_dir: Option<&Path>,
    ) -> Result<Self, AdapterHostError> {
        let mut command = Command::new(&spec.executable);
        command.args(&spec.args);
        if let Some(current_dir) = current_dir {
            command.current_dir(current_dir);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|error| AdapterHostError::SpawnFailed(error.to_string()))?;
        let stdin = child.stdin.take().ok_or(AdapterHostError::MissingStdin)?;
        let stdout = child.stdout.take().ok_or(AdapterHostError::MissingStdout)?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }
}

impl AdapterTransport for AdapterProcess {
    fn write_line(&mut self, line: &str) -> Result<(), AdapterHostError> {
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.write_all(b"\n"))
            .and_then(|()| self.stdin.flush())
            .map_err(|error| AdapterHostError::TransportWrite(error.to_string()))
    }

    fn read_line(&mut self) -> Result<Option<String>, AdapterHostError> {
        if let Ok(Some(_)) = self.child.try_wait() {
            return Ok(None);
        }
        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .map_err(|error| AdapterHostError::TransportRead(error.to_string()))?;
        if read == 0 {
            return Ok(None);
        }
        Ok(Some(line.trim_end_matches(['\r', '\n']).to_string()))
    }

    fn shutdown(&mut self) -> Result<(), AdapterHostError> {
        match self.child.try_wait() {
            Ok(Some(_)) => Ok(()),
            Ok(None) => {
                let _ = self.child.kill();
                self.child
                    .wait()
                    .map(|_| ())
                    .map_err(|error| AdapterHostError::TransportRead(error.to_string()))
            }
            Err(error) => Err(AdapterHostError::TransportRead(error.to_string())),
        }
    }
}

pub type ProcessAdapterSession = AdapterSession<AdapterProcess>;

pub struct AdapterHost {
    session: Option<ProcessAdapterSession>,
    state: AdapterHostState,
}

impl AdapterHost {
    pub const fn disconnected() -> Self {
        Self {
            session: None,
            state: AdapterHostState::Disconnected,
        }
    }

    pub fn spawn(
        spec: AdapterProcessSpec,
        current_dir: Option<&Path>,
    ) -> Result<Self, AdapterHostError> {
        let process = AdapterProcess::spawn(&spec, current_dir)?;
        Ok(Self {
            session: Some(AdapterSession::new(process)),
            state: AdapterHostState::Starting,
        })
    }

    pub const fn state(&self) -> AdapterHostState {
        self.state
    }

    pub fn session_mut(&mut self) -> Option<&mut ProcessAdapterSession> {
        self.session.as_mut()
    }

    pub fn handshake(
        &mut self,
        request: HandshakeRequest,
    ) -> Result<AdapterSessionInfo, AdapterHostError> {
        let session = self
            .session
            .as_mut()
            .ok_or(AdapterHostError::AdapterExited)?;
        match session.handshake(request) {
            Ok(info) => {
                self.state = AdapterHostState::Connected;
                Ok(info)
            }
            Err(error) => {
                self.state = AdapterHostState::Failed;
                Err(error)
            }
        }
    }

    pub fn load_project(
        &mut self,
        request: LoadProjectRequest,
    ) -> Result<LoadProjectResponse, AdapterHostError> {
        self.session_mut_or_failed()?.load_project(request)
    }

    pub fn get_scene_snapshot(
        &mut self,
        request: GetSceneSnapshotRequest,
    ) -> Result<GetSceneSnapshotResponse, AdapterHostError> {
        self.session_mut_or_failed()?.get_scene_snapshot(request)
    }

    pub fn set_property(
        &mut self,
        request: SetPropertyRequest,
    ) -> Result<SetPropertyResponse, AdapterHostError> {
        self.session_mut_or_failed()?.set_property(request)
    }

    pub fn get_diagnostics(&mut self) -> Result<GetDiagnosticsResponse, AdapterHostError> {
        self.session_mut_or_failed()?
            .get_diagnostics(GetDiagnosticsRequest)
    }

    pub fn get_viewport_frame(
        &mut self,
        request: GetViewportFrameRequest,
    ) -> Result<GetViewportFrameResponse, AdapterHostError> {
        self.session_mut_or_failed()?.get_viewport_frame(request)
    }

    pub fn shutdown(&mut self) -> Result<ShutdownResponse, AdapterHostError> {
        let Some(session) = self.session.as_mut() else {
            self.state = AdapterHostState::Stopped;
            return Ok(ShutdownResponse { accepted: true });
        };
        let response = session.shutdown_request(ShutdownRequest)?;
        session.shutdown_transport()?;
        self.state = AdapterHostState::Stopped;
        Ok(response)
    }

    fn session_mut_or_failed(&mut self) -> Result<&mut ProcessAdapterSession, AdapterHostError> {
        if self.state == AdapterHostState::Failed {
            return Err(AdapterHostError::AdapterExited);
        }
        self.session.as_mut().ok_or(AdapterHostError::AdapterExited)
    }
}

impl Default for AdapterHost {
    fn default() -> Self {
        Self::disconnected()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSessionInfo {
    pub id: AdapterId,
    pub name: AdapterName,
    pub version: AdapterVersion,
    pub capabilities: AdapterCapabilities,
}

pub struct AdapterSession<T> {
    transport: T,
    state: AdapterHostState,
    next_request_id: u64,
    diagnostics: Vec<AdapterDiagnostic>,
    logs: Vec<AdapterLog>,
    info: Option<AdapterSessionInfo>,
}

impl<T> AdapterSession<T>
where
    T: AdapterTransport,
{
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            state: AdapterHostState::Starting,
            next_request_id: 1,
            diagnostics: Vec::new(),
            logs: Vec::new(),
            info: None,
        }
    }

    pub const fn state(&self) -> AdapterHostState {
        self.state
    }

    pub fn diagnostics(&self) -> &[AdapterDiagnostic] {
        self.diagnostics.as_slice()
    }

    pub fn logs(&self) -> &[AdapterLog] {
        self.logs.as_slice()
    }

    pub fn info(&self) -> Option<&AdapterSessionInfo> {
        self.info.as_ref()
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn handshake(
        &mut self,
        request: HandshakeRequest,
    ) -> Result<AdapterSessionInfo, AdapterHostError> {
        let response = self.send(AdapterRequestMessage::Handshake(request))?;
        let AdapterResponseMessage::Handshake(response) = response else {
            self.state = AdapterHostState::Failed;
            return Err(AdapterHostError::UnexpectedResponse(
                "handshake did not return handshake response".to_string(),
            ));
        };
        let info = AdapterSessionInfo {
            id: response.adapter_id,
            name: response.adapter_name,
            version: response.adapter_version,
            capabilities: response.capabilities,
        };
        self.info = Some(info.clone());
        self.state = AdapterHostState::Connected;
        Ok(info)
    }

    pub fn load_project(
        &mut self,
        request: LoadProjectRequest,
    ) -> Result<LoadProjectResponse, AdapterHostError> {
        let response = self.send(AdapterRequestMessage::LoadProject(request))?;
        match response {
            AdapterResponseMessage::LoadProject(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn get_scene_snapshot(
        &mut self,
        request: GetSceneSnapshotRequest,
    ) -> Result<GetSceneSnapshotResponse, AdapterHostError> {
        let response = self.send(AdapterRequestMessage::GetSceneSnapshot(request))?;
        match response {
            AdapterResponseMessage::GetSceneSnapshot(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn set_property(
        &mut self,
        request: SetPropertyRequest,
    ) -> Result<SetPropertyResponse, AdapterHostError> {
        let response = self.send(AdapterRequestMessage::SetProperty(request))?;
        match response {
            AdapterResponseMessage::SetProperty(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn get_diagnostics(
        &mut self,
        request: GetDiagnosticsRequest,
    ) -> Result<GetDiagnosticsResponse, AdapterHostError> {
        let response = self.send(AdapterRequestMessage::GetDiagnostics(request))?;
        match response {
            AdapterResponseMessage::GetDiagnostics(response) => {
                self.diagnostics = response.diagnostics.clone();
                Ok(response)
            }
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn get_viewport_frame(
        &mut self,
        request: GetViewportFrameRequest,
    ) -> Result<GetViewportFrameResponse, AdapterHostError> {
        let response = self.send(AdapterRequestMessage::GetViewportFrame(request))?;
        match response {
            AdapterResponseMessage::GetViewportFrame(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn shutdown_request(
        &mut self,
        request: ShutdownRequest,
    ) -> Result<ShutdownResponse, AdapterHostError> {
        let response = self.send(AdapterRequestMessage::Shutdown(request))?;
        match response {
            AdapterResponseMessage::Shutdown(response) => {
                self.state = AdapterHostState::Stopped;
                Ok(response)
            }
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn shutdown_transport(&mut self) -> Result<(), AdapterHostError> {
        self.transport.shutdown()
    }

    fn send(
        &mut self,
        message: AdapterRequestMessage,
    ) -> Result<AdapterResponseMessage, AdapterHostError> {
        let request_id = self.next_request_id();
        let request = AdapterRequest::new(request_id, message);
        let line = encode_request_line(&request)
            .map_err(|error| AdapterHostError::InvalidJson(error.to_string()))?;
        self.transport.write_line(&line)?;
        loop {
            let line = match self.transport.read_line()? {
                Some(line) => line,
                None => {
                    self.state = AdapterHostState::Failed;
                    return Err(AdapterHostError::AdapterExited);
                }
            };
            let adapter_line = decode_adapter_line(&line)
                .map_err(|error| AdapterHostError::InvalidJson(error.to_string()))?;
            match adapter_line {
                AdapterLine::Event(event) => self.record_event(event),
                AdapterLine::Response(response) => {
                    return self.handle_response(request_id, response);
                }
            }
        }
    }

    fn next_request_id(&mut self) -> AdapterRequestId {
        let id = AdapterRequestId(self.next_request_id);
        self.next_request_id = self.next_request_id.saturating_add(1);
        id
    }

    fn record_event(&mut self, event: AdapterEvent) {
        match event {
            AdapterEvent::Diagnostic(diagnostic) => self.diagnostics.push(diagnostic),
            AdapterEvent::Log(log) => self.logs.push(log),
        }
    }

    fn handle_response(
        &mut self,
        expected: AdapterRequestId,
        response: AdapterResponse,
    ) -> Result<AdapterResponseMessage, AdapterHostError> {
        if response.request_id != expected {
            return Err(AdapterHostError::MismatchedRequestId {
                expected,
                actual: response.request_id,
            });
        }
        match response.message {
            AdapterResponseMessage::Error(ErrorResponse { error }) => {
                self.state = AdapterHostState::Failed;
                Err(AdapterHostError::Adapter(error))
            }
            other => Ok(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FakeAdapterTransport {
    responses: VecDeque<String>,
    writes: Vec<String>,
    exit_on_read: bool,
}

impl FakeAdapterTransport {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: VecDeque::from(responses),
            writes: Vec::new(),
            exit_on_read: false,
        }
    }

    pub fn exiting() -> Self {
        Self {
            responses: VecDeque::new(),
            writes: Vec::new(),
            exit_on_read: true,
        }
    }

    pub fn writes(&self) -> &[String] {
        self.writes.as_slice()
    }
}

impl AdapterTransport for FakeAdapterTransport {
    fn write_line(&mut self, line: &str) -> Result<(), AdapterHostError> {
        self.writes.push(line.to_string());
        Ok(())
    }

    fn read_line(&mut self) -> Result<Option<String>, AdapterHostError> {
        if self.exit_on_read {
            return Ok(None);
        }
        Ok(self.responses.pop_front())
    }

    fn shutdown(&mut self) -> Result<(), AdapterHostError> {
        Ok(())
    }
}

pub fn response_line(
    request_id: AdapterRequestId,
    message: AdapterResponseMessage,
) -> Result<String, serde_json::Error> {
    elcarax_adapter_api::encode_response_line(&AdapterResponse::new(request_id, message))
}

pub fn event_line(event: AdapterEvent) -> Result<String, serde_json::Error> {
    elcarax_adapter_api::encode_event_line(&event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_adapter_api::{
        AdapterCapabilities, AdapterEditSource, AdapterId, AdapterName, ProtocolVersion,
        SetPropertyResponse, SetPropertyStatus, decode_request_line,
    };
    use elcarax_scene_model::{PropertyPath, PropertyValue, ScenePatch, demo_scene_snapshot};

    #[test]
    fn fake_transport_handshake_succeeds() {
        let response = response_line(
            AdapterRequestId(1),
            AdapterResponseMessage::Handshake(elcarax_adapter_api::HandshakeResponse {
                adapter_id: AdapterId::new("mock"),
                adapter_name: AdapterName::new("Mock Adapter"),
                adapter_version: AdapterVersion::new("0.1.0"),
                protocol_version: ProtocolVersion::V0,
                capabilities: AdapterCapabilities::mock_milestone_12(),
            }),
        );
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![must(response)]));
        let info = must(session.handshake(HandshakeRequest::current("test", None)));
        assert_eq!(info.name.as_str(), "Mock Adapter");
        assert_eq!(session.state(), AdapterHostState::Connected);
    }

    #[test]
    fn fake_transport_load_scene_succeeds() {
        let response = response_line(
            AdapterRequestId(1),
            AdapterResponseMessage::GetSceneSnapshot(GetSceneSnapshotResponse {
                snapshot: demo_scene_snapshot(),
                source_label: "mock-adapter".to_string(),
            }),
        );
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![must(response)]));
        let response = must(session.get_scene_snapshot(GetSceneSnapshotRequest { scene_id: None }));
        assert_eq!(response.snapshot.object_count(), 10);
    }

    #[test]
    fn fake_transport_set_property_succeeds() {
        let snapshot = demo_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        };
        let response = response_line(
            AdapterRequestId(1),
            AdapterResponseMessage::SetProperty(SetPropertyResponse {
                status: SetPropertyStatus::Accepted,
                scene_id: snapshot.scene_id(),
                object_id: player.id,
                path: path("gameplay.health"),
                old_value: Some(PropertyValue::I64(100)),
                confirmed_new_value: Some(PropertyValue::I64(65)),
                patch: Some(ScenePatch::property_updated(
                    player.id,
                    path("gameplay.health"),
                    PropertyValue::I64(65),
                )),
                diagnostics: Vec::new(),
            }),
        );
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![must(response)]));
        let response = must(session.set_property(SetPropertyRequest {
            scene_id: snapshot.scene_id(),
            object_id: player.id,
            path: path("gameplay.health"),
            expected_old_value: Some(PropertyValue::I64(100)),
            new_value: PropertyValue::I64(65),
            transaction_id: "test".to_string(),
            edit_source: AdapterEditSource::Inspector,
        }));
        assert_eq!(response.status, SetPropertyStatus::Accepted);
        let request = match session.transport.writes().first() {
            Some(line) => match decode_request_line(line) {
                Ok(request) => request,
                Err(error) => panic!("request should decode: {error}"),
            },
            None => panic!("request should have been written"),
        };
        assert!(matches!(
            request.message,
            AdapterRequestMessage::SetProperty(_)
        ));
    }

    #[test]
    fn invalid_json_produces_clear_error() {
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![
            "{not-valid-json".to_string(),
        ]));
        let error = match session.get_diagnostics(GetDiagnosticsRequest) {
            Ok(_) => panic!("invalid JSON should fail"),
            Err(error) => error,
        };
        assert!(matches!(error, AdapterHostError::InvalidJson(_)));
    }

    #[test]
    fn adapter_exit_produces_failed_state() {
        let mut session = AdapterSession::new(FakeAdapterTransport::exiting());
        let error = match session.get_diagnostics(GetDiagnosticsRequest) {
            Ok(_) => panic!("adapter exit should fail"),
            Err(error) => error,
        };
        assert_eq!(error, AdapterHostError::AdapterExited);
        assert_eq!(session.state(), AdapterHostState::Failed);
    }

    #[test]
    fn stopping_adapter_transitions_state() {
        let response = response_line(
            AdapterRequestId(1),
            AdapterResponseMessage::Shutdown(ShutdownResponse { accepted: true }),
        );
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![must(response)]));
        let stopped = must(session.shutdown_request(ShutdownRequest));
        assert!(stopped.accepted);
        assert_eq!(session.state(), AdapterHostState::Stopped);
    }

    #[test]
    fn missing_adapter_executable_fails_to_spawn() {
        let spec = AdapterProcessSpec::new("definitely_missing_elcarax_adapter_binary");
        let result = AdapterProcess::spawn(&spec, None);
        assert!(matches!(result, Err(AdapterHostError::SpawnFailed(_))));
    }

    #[test]
    fn event_lines_are_collected_before_response() {
        let event = event_line(AdapterEvent::Log(AdapterLog::info("ready")));
        let response = response_line(
            AdapterRequestId(1),
            AdapterResponseMessage::GetDiagnostics(GetDiagnosticsResponse {
                diagnostics: vec![AdapterDiagnostic::info("mock", "ok")],
            }),
        );
        let mut session =
            AdapterSession::new(FakeAdapterTransport::new(vec![must(event), must(response)]));
        let diagnostics = must(session.get_diagnostics(GetDiagnosticsRequest));
        assert_eq!(diagnostics.diagnostics.len(), 1);
        assert_eq!(session.logs().len(), 1);
    }

    fn must<T, E: fmt::Display>(value: Result<T, E>) -> T {
        match value {
            Ok(value) => value,
            Err(error) => panic!("operation should succeed: {error}"),
        }
    }

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("test path should parse: {error}"),
        }
    }
}
