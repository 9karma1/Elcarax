//! Adapter process supervision and binary-framed transport for Elcarax.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterError, AdapterEvent, AdapterFrame, AdapterId,
    AdapterLine, AdapterLog, AdapterName, AdapterRequest, AdapterRequestId, AdapterRequestMessage,
    AdapterResponse, AdapterResponseMessage, AdapterVersion, ErrorResponse, FrameError,
    GetDiagnosticsRequest, GetDiagnosticsResponse, GetSceneSnapshotRequest,
    GetSceneSnapshotResponse, GetViewportFrameRequest, GetViewportFrameResponse, HandshakeRequest,
    LoadProjectRequest, LoadProjectResponse, PickViewportObjectRequest, PickViewportObjectResponse,
    SetPropertyRequest, SetPropertyResponse, ShutdownRequest, ShutdownResponse,
    decode_adapter_frame, encode_request_frame, read_frame, write_frame,
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
    InvalidFrame(String),
    AdapterExited,
    MismatchedRequestId {
        expected: AdapterRequestId,
        actual: AdapterRequestId,
    },
    UnexpectedResponse(String),
    Adapter(AdapterError),
    WorkerUnavailable,
    TimedOut,
}

impl fmt::Display for AdapterHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpawnFailed(message) => write!(formatter, "failed to spawn adapter: {message}"),
            Self::MissingStdin => write!(formatter, "adapter process stdin was not captured"),
            Self::MissingStdout => write!(formatter, "adapter process stdout was not captured"),
            Self::TransportWrite(message) => write!(formatter, "adapter write failed: {message}"),
            Self::TransportRead(message) => write!(formatter, "adapter read failed: {message}"),
            Self::InvalidFrame(message) => write!(formatter, "invalid adapter frame: {message}"),
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
            Self::WorkerUnavailable => write!(formatter, "adapter worker is unavailable"),
            Self::TimedOut => write!(formatter, "timed out waiting for adapter response"),
        }
    }
}

impl Error for AdapterHostError {}

impl From<FrameError> for AdapterHostError {
    fn from(error: FrameError) -> Self {
        match error {
            FrameError::Io(message) => Self::TransportRead(message),
            FrameError::UnexpectedEof => Self::AdapterExited,
            other => Self::InvalidFrame(other.to_string()),
        }
    }
}

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
    fn send_frame(&mut self, frame: &AdapterFrame) -> Result<(), AdapterHostError>;
    fn recv_frame(&mut self) -> Result<Option<AdapterFrame>, AdapterHostError>;
    fn shutdown(&mut self) -> Result<(), AdapterHostError>;
}

pub struct AdapterProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
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
            stdout,
        })
    }
}

impl AdapterTransport for AdapterProcess {
    fn send_frame(&mut self, frame: &AdapterFrame) -> Result<(), AdapterHostError> {
        write_frame(&mut self.stdin, frame).map_err(|error| match error {
            FrameError::Io(message) => AdapterHostError::TransportWrite(message),
            other => AdapterHostError::InvalidFrame(other.to_string()),
        })
    }

    fn recv_frame(&mut self) -> Result<Option<AdapterFrame>, AdapterHostError> {
        if let Ok(Some(_)) = self.child.try_wait() {
            return Ok(None);
        }
        read_frame(&mut self.stdout).map_err(AdapterHostError::from)
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

#[derive(Debug, Clone, PartialEq)]
pub enum AdapterHostPollItem {
    Event(AdapterEvent),
    Response {
        request_id: AdapterRequestId,
        message: AdapterResponseMessage,
    },
    Failed(AdapterHostError),
    Stopped,
}

enum WorkerCommand {
    Submit(AdapterRequest),
    Shutdown,
}

enum WorkerEvent {
    Event(AdapterEvent),
    Response(AdapterResponse),
    Failed(AdapterHostError),
    Stopped,
}

struct HostWorker {
    command_tx: Sender<WorkerCommand>,
    event_rx: Receiver<WorkerEvent>,
    join: Option<JoinHandle<()>>,
}

impl HostWorker {
    fn spawn(process: AdapterProcess) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let join = thread::spawn(move || {
            let mut session = AdapterSession::new(process);
            while let Ok(command) = command_rx.recv() {
                match command {
                    WorkerCommand::Submit(request) => match session.send_request(request) {
                        Ok((events, response)) => {
                            for event in events {
                                if event_tx.send(WorkerEvent::Event(event)).is_err() {
                                    return;
                                }
                            }
                            if event_tx.send(WorkerEvent::Response(response)).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = event_tx.send(WorkerEvent::Failed(error));
                            break;
                        }
                    },
                    WorkerCommand::Shutdown => {
                        let _ = session.shutdown_request(ShutdownRequest);
                        let _ = session.shutdown_transport();
                        let _ = event_tx.send(WorkerEvent::Stopped);
                        break;
                    }
                }
            }
        });
        Self {
            command_tx,
            event_rx,
            join: Some(join),
        }
    }

    fn submit(&self, request: AdapterRequest) -> Result<(), AdapterHostError> {
        self.command_tx
            .send(WorkerCommand::Submit(request))
            .map_err(|_| AdapterHostError::WorkerUnavailable)
    }

    fn request_shutdown(&self) -> Result<(), AdapterHostError> {
        self.command_tx
            .send(WorkerCommand::Shutdown)
            .map_err(|_| AdapterHostError::WorkerUnavailable)
    }

    fn try_recv(&self) -> Result<Option<WorkerEvent>, AdapterHostError> {
        match self.event_rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(AdapterHostError::WorkerUnavailable),
        }
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<WorkerEvent, AdapterHostError> {
        match self.event_rx.recv_timeout(timeout) {
            Ok(event) => Ok(event),
            Err(RecvTimeoutError::Timeout) => Err(AdapterHostError::TimedOut),
            Err(RecvTimeoutError::Disconnected) => Err(AdapterHostError::WorkerUnavailable),
        }
    }
}

impl Drop for HostWorker {
    fn drop(&mut self) {
        let _ = self.command_tx.send(WorkerCommand::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub struct AdapterHost {
    worker: Option<HostWorker>,
    state: AdapterHostState,
    next_request_id: u64,
    pending_events: Vec<AdapterEvent>,
    info: Option<AdapterSessionInfo>,
}

impl AdapterHost {
    pub const fn disconnected() -> Self {
        Self {
            worker: None,
            state: AdapterHostState::Disconnected,
            next_request_id: 1,
            pending_events: Vec::new(),
            info: None,
        }
    }

    pub fn spawn(
        spec: AdapterProcessSpec,
        current_dir: Option<&Path>,
    ) -> Result<Self, AdapterHostError> {
        let process = AdapterProcess::spawn(&spec, current_dir)?;
        Ok(Self {
            worker: Some(HostWorker::spawn(process)),
            state: AdapterHostState::Starting,
            next_request_id: 1,
            pending_events: Vec::new(),
            info: None,
        })
    }

    pub const fn state(&self) -> AdapterHostState {
        self.state
    }

    pub fn info(&self) -> Option<&AdapterSessionInfo> {
        self.info.as_ref()
    }

    pub fn submit(
        &mut self,
        message: AdapterRequestMessage,
    ) -> Result<AdapterRequestId, AdapterHostError> {
        if self.worker.is_none() {
            return Err(AdapterHostError::AdapterExited);
        }
        if self.state == AdapterHostState::Failed {
            return Err(AdapterHostError::AdapterExited);
        }
        let request_id = self.allocate_request_id();
        let worker = self
            .worker
            .as_ref()
            .ok_or(AdapterHostError::AdapterExited)?;
        worker.submit(AdapterRequest::new(request_id, message))?;
        Ok(request_id)
    }

    pub fn poll(&mut self) -> Vec<AdapterHostPollItem> {
        let mut items = Vec::new();
        loop {
            let event = {
                let Some(worker) = self.worker.as_ref() else {
                    break;
                };
                match worker.try_recv() {
                    Ok(Some(event)) => event,
                    Ok(None) => break,
                    Err(error) => {
                        self.state = AdapterHostState::Failed;
                        items.push(AdapterHostPollItem::Failed(error));
                        break;
                    }
                }
            };
            items.push(self.ingest_worker_event(event));
        }
        items
    }

    pub fn handshake(
        &mut self,
        request: HandshakeRequest,
    ) -> Result<AdapterSessionInfo, AdapterHostError> {
        let response = self.request(AdapterRequestMessage::Handshake(request))?;
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
        match self.request(AdapterRequestMessage::LoadProject(request))? {
            AdapterResponseMessage::LoadProject(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn get_scene_snapshot(
        &mut self,
        request: GetSceneSnapshotRequest,
    ) -> Result<GetSceneSnapshotResponse, AdapterHostError> {
        match self.request(AdapterRequestMessage::GetSceneSnapshot(request))? {
            AdapterResponseMessage::GetSceneSnapshot(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn set_property(
        &mut self,
        request: SetPropertyRequest,
    ) -> Result<SetPropertyResponse, AdapterHostError> {
        match self.request(AdapterRequestMessage::SetProperty(request))? {
            AdapterResponseMessage::SetProperty(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn get_diagnostics(&mut self) -> Result<GetDiagnosticsResponse, AdapterHostError> {
        match self.request(AdapterRequestMessage::GetDiagnostics(GetDiagnosticsRequest))? {
            AdapterResponseMessage::GetDiagnostics(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn get_viewport_frame(
        &mut self,
        request: GetViewportFrameRequest,
    ) -> Result<GetViewportFrameResponse, AdapterHostError> {
        match self.request(AdapterRequestMessage::GetViewportFrame(request))? {
            AdapterResponseMessage::GetViewportFrame(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn pick_viewport_object(
        &mut self,
        request: PickViewportObjectRequest,
    ) -> Result<PickViewportObjectResponse, AdapterHostError> {
        match self.request(AdapterRequestMessage::PickViewportObject(request))? {
            AdapterResponseMessage::PickViewportObject(response) => Ok(response),
            other => Err(AdapterHostError::UnexpectedResponse(format!("{other:?}"))),
        }
    }

    pub fn shutdown(&mut self) -> Result<ShutdownResponse, AdapterHostError> {
        let Some(worker) = self.worker.take() else {
            self.state = AdapterHostState::Stopped;
            return Ok(ShutdownResponse { accepted: true });
        };
        worker.request_shutdown()?;
        let mut accepted = true;
        loop {
            match worker.recv_timeout(Duration::from_secs(5)) {
                Ok(WorkerEvent::Stopped) => break,
                Ok(WorkerEvent::Response(response)) => {
                    if let AdapterResponseMessage::Shutdown(value) = response.message {
                        accepted = value.accepted;
                    }
                }
                Ok(WorkerEvent::Event(_)) => {}
                Ok(WorkerEvent::Failed(error)) => {
                    self.state = AdapterHostState::Failed;
                    return Err(error);
                }
                Err(AdapterHostError::TimedOut) => break,
                Err(error) => {
                    self.state = AdapterHostState::Failed;
                    return Err(error);
                }
            }
        }
        self.state = AdapterHostState::Stopped;
        Ok(ShutdownResponse { accepted })
    }

    fn request(
        &mut self,
        message: AdapterRequestMessage,
    ) -> Result<AdapterResponseMessage, AdapterHostError> {
        let request_id = self.submit(message)?;
        self.wait_for_response(request_id)
    }

    fn wait_for_response(
        &mut self,
        expected: AdapterRequestId,
    ) -> Result<AdapterResponseMessage, AdapterHostError> {
        let deadline = std::time::Instant::now() + Duration::from_secs(120);
        while std::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let timeout = remaining.min(Duration::from_millis(50));
            let event = {
                let Some(worker) = self.worker.as_ref() else {
                    return Err(AdapterHostError::AdapterExited);
                };
                match worker.recv_timeout(timeout) {
                    Ok(event) => event,
                    Err(AdapterHostError::TimedOut) => continue,
                    Err(error) => {
                        self.state = AdapterHostState::Failed;
                        return Err(error);
                    }
                }
            };
            match self.ingest_worker_event(event) {
                AdapterHostPollItem::Response {
                    request_id,
                    message,
                } if request_id == expected => return Ok(message),
                AdapterHostPollItem::Failed(error) => return Err(error),
                AdapterHostPollItem::Event(event) => self.pending_events.push(event),
                AdapterHostPollItem::Stopped => {
                    self.state = AdapterHostState::Stopped;
                    return Err(AdapterHostError::AdapterExited);
                }
                AdapterHostPollItem::Response { .. } => {}
            }
        }
        Err(AdapterHostError::TimedOut)
    }

    fn ingest_worker_event(&mut self, event: WorkerEvent) -> AdapterHostPollItem {
        match event {
            WorkerEvent::Event(event) => AdapterHostPollItem::Event(event),
            WorkerEvent::Response(response) => match response.message {
                AdapterResponseMessage::Error(ErrorResponse { error }) => {
                    self.state = AdapterHostState::Failed;
                    AdapterHostPollItem::Failed(AdapterHostError::Adapter(error))
                }
                message => AdapterHostPollItem::Response {
                    request_id: response.request_id,
                    message,
                },
            },
            WorkerEvent::Failed(error) => {
                self.state = AdapterHostState::Failed;
                AdapterHostPollItem::Failed(error)
            }
            WorkerEvent::Stopped => {
                self.state = AdapterHostState::Stopped;
                AdapterHostPollItem::Stopped
            }
        }
    }

    fn allocate_request_id(&mut self) -> AdapterRequestId {
        let id = AdapterRequestId(self.next_request_id);
        self.next_request_id = self.next_request_id.saturating_add(1);
        id
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

    pub fn pick_viewport_object(
        &mut self,
        request: PickViewportObjectRequest,
    ) -> Result<PickViewportObjectResponse, AdapterHostError> {
        let response = self.send(AdapterRequestMessage::PickViewportObject(request))?;
        match response {
            AdapterResponseMessage::PickViewportObject(response) => Ok(response),
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
        self.send_request(request)
            .map(|(_events, response)| response.message)
    }

    fn send_request(
        &mut self,
        request: AdapterRequest,
    ) -> Result<(Vec<AdapterEvent>, AdapterResponse), AdapterHostError> {
        let expected = request.request_id;
        let frame = encode_request_frame(&request)
            .map_err(|error| AdapterHostError::InvalidFrame(error.to_string()))?;
        self.transport.send_frame(&frame)?;
        let mut events = Vec::new();
        loop {
            let frame = match self.transport.recv_frame()? {
                Some(frame) => frame,
                None => {
                    self.state = AdapterHostState::Failed;
                    return Err(AdapterHostError::AdapterExited);
                }
            };
            let adapter_line = decode_adapter_frame(&frame)
                .map_err(|error| AdapterHostError::InvalidFrame(error.to_string()))?;
            match adapter_line {
                AdapterLine::Event(event) => {
                    self.record_event(event.clone());
                    events.push(event);
                }
                AdapterLine::Response(response) => {
                    let response = self.handle_response(expected, response)?;
                    return Ok((events, response));
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
    ) -> Result<AdapterResponse, AdapterHostError> {
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
            _ => Ok(response),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FakeAdapterTransport {
    responses: VecDeque<AdapterFrame>,
    writes: Vec<AdapterFrame>,
    exit_on_read: bool,
}

impl FakeAdapterTransport {
    pub fn new(responses: Vec<AdapterFrame>) -> Self {
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

    pub fn writes(&self) -> &[AdapterFrame] {
        self.writes.as_slice()
    }
}

impl AdapterTransport for FakeAdapterTransport {
    fn send_frame(&mut self, frame: &AdapterFrame) -> Result<(), AdapterHostError> {
        self.writes.push(frame.clone());
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<Option<AdapterFrame>, AdapterHostError> {
        if self.exit_on_read {
            return Ok(None);
        }
        Ok(self.responses.pop_front())
    }

    fn shutdown(&mut self) -> Result<(), AdapterHostError> {
        Ok(())
    }
}

pub fn response_frame(
    request_id: AdapterRequestId,
    message: AdapterResponseMessage,
) -> Result<AdapterFrame, FrameError> {
    elcarax_adapter_api::encode_response_frame(&AdapterResponse::new(request_id, message))
}

pub fn event_frame(event: AdapterEvent) -> Result<AdapterFrame, FrameError> {
    elcarax_adapter_api::encode_event_frame(&event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use elcarax_adapter_api::{
        AdapterCapabilities, AdapterEditSource, AdapterId, AdapterName, ProtocolVersion,
        SetPropertyResponse, SetPropertyStatus, decode_request_frame,
    };
    use elcarax_scene_model::{
        ComponentTypeName, PropertyPath, PropertyValue, ScenePatch, components,
        reference_scene_snapshot,
    };

    #[test]
    fn fake_transport_handshake_succeeds() {
        let response = response_frame(
            AdapterRequestId(1),
            AdapterResponseMessage::Handshake(elcarax_adapter_api::HandshakeResponse {
                adapter_id: AdapterId::new("mock"),
                adapter_name: AdapterName::new("Mock Adapter"),
                adapter_version: AdapterVersion::new("0.1.0"),
                protocol_version: ProtocolVersion::V0,
                capabilities: AdapterCapabilities::stdio_game_adapter(),
            }),
        );
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![must(response)]));
        let info = must(session.handshake(HandshakeRequest::current("test", None)));
        assert_eq!(info.name.as_str(), "Mock Adapter");
        assert_eq!(session.state(), AdapterHostState::Connected);
    }

    #[test]
    fn fake_transport_load_scene_succeeds() {
        let response = response_frame(
            AdapterRequestId(1),
            AdapterResponseMessage::GetSceneSnapshot(GetSceneSnapshotResponse {
                snapshot: reference_scene_snapshot(),
                source_label: "mock-adapter".to_string(),
            }),
        );
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![must(response)]));
        let response = must(session.get_scene_snapshot(GetSceneSnapshotRequest { scene_id: None }));
        assert_eq!(response.snapshot.object_count(), 10);
    }

    #[test]
    fn fake_transport_set_property_succeeds() {
        let snapshot = reference_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        };
        let gameplay = match player.component_by_type(&ComponentTypeName::new(components::GAMEPLAY))
        {
            Some(component) => component,
            None => panic!("gameplay component should exist"),
        };
        let health_path = path("health");
        let response = response_frame(
            AdapterRequestId(1),
            AdapterResponseMessage::SetProperty(SetPropertyResponse {
                status: SetPropertyStatus::Accepted,
                scene_id: snapshot.scene_id(),
                object_id: player.id,
                component_id: gameplay.id,
                path: health_path.clone(),
                old_value: Some(PropertyValue::I64(100)),
                confirmed_new_value: Some(PropertyValue::I64(65)),
                patch: Some(ScenePatch::property_updated(
                    player.id,
                    gameplay.id,
                    health_path.clone(),
                    PropertyValue::I64(65),
                )),
                diagnostics: Vec::new(),
            }),
        );
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![must(response)]));
        let response = must(session.set_property(SetPropertyRequest {
            scene_id: snapshot.scene_id(),
            object_id: player.id,
            component_id: gameplay.id,
            path: health_path,
            expected_old_value: Some(PropertyValue::I64(100)),
            new_value: PropertyValue::I64(65),
            transaction_id: "test".to_string(),
            edit_source: AdapterEditSource::Inspector,
        }));
        assert_eq!(response.status, SetPropertyStatus::Accepted);
        let request = match session.transport.writes().first() {
            Some(frame) => match decode_request_frame(frame) {
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
    fn invalid_frame_json_produces_clear_error() {
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![AdapterFrame {
            kind: elcarax_adapter_api::FrameKind::Response,
            id: 1,
            json: b"{not-valid-json".to_vec(),
            binary: Vec::new(),
        }]));
        let error = match session.get_diagnostics(GetDiagnosticsRequest) {
            Ok(_) => panic!("invalid JSON should fail"),
            Err(error) => error,
        };
        assert!(matches!(error, AdapterHostError::InvalidFrame(_)));
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
        let response = response_frame(
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
    fn event_frames_are_collected_before_response() {
        let event = event_frame(AdapterEvent::Log(AdapterLog::info("ready")));
        let response = response_frame(
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

    #[test]
    fn fake_transport_pick_viewport_object_succeeds() {
        use elcarax_adapter_api::{
            AdapterViewportId, PickViewportObjectRequest, PickViewportObjectResponse,
            ViewportPickResponseStatus,
        };

        let snapshot = reference_scene_snapshot();
        let player = match snapshot.object_by_name("Player") {
            Some(player) => player,
            None => panic!("player should exist"),
        };
        let response = response_frame(
            AdapterRequestId(1),
            AdapterResponseMessage::PickViewportObject(PickViewportObjectResponse {
                viewport_id: AdapterViewportId(1),
                object_id: Some(player.id),
                diagnostics: Vec::new(),
                status: ViewportPickResponseStatus::Picked,
            }),
        );
        let mut session = AdapterSession::new(FakeAdapterTransport::new(vec![must(response)]));
        let response = must(session.pick_viewport_object(PickViewportObjectRequest {
            viewport_id: AdapterViewportId(1),
            scene_id: None,
            u: 0.5,
            v: 0.5,
        }));
        assert_eq!(response.status, ViewportPickResponseStatus::Picked);
        assert_eq!(response.object_id, Some(player.id));
    }

    fn must<T, E: fmt::Display>(value: Result<T, E>) -> T {
        match value {
            Ok(value) => value,
            Err(error) => panic!("expected success: {error}"),
        }
    }

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("test path should parse: {error}"),
        }
    }
}
