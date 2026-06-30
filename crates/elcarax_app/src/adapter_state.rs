#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterEditSource, AdapterName, AdapterVersion,
    SetPropertyRequest, SetPropertyResponse, ViewportFrameResponseStatus,
};
#[cfg(any(test, feature = "native-shell"))]
use elcarax_adapter_api::{
    AdapterId, GetSceneSnapshotRequest, HandshakeRequest, LoadProjectRequest,
};
#[cfg(any(test, feature = "native-shell"))]
use elcarax_adapter_api::{AdapterViewportId, GetViewportFrameRequest};
use elcarax_adapter_host::AdapterHostState;
#[cfg(any(test, feature = "native-shell"))]
use elcarax_adapter_host::AdapterHostError;
#[cfg(feature = "native-shell")]
use elcarax_adapter_host::{AdapterHost, AdapterProcessSpec};
#[cfg(test)]
use elcarax_adapter_host::AdapterSession;

use crate::adapter_display::{AdapterUiSnapshot, adapter_ui_snapshot};
#[cfg(test)]
use crate::scene_state::SceneSource;
#[cfg(any(test, feature = "native-shell"))]
use elcarax_core::ViewportFrameFormat;
use elcarax_core::{ViewportError, ViewportFrame};
use elcarax_scene_model::{PropertyChange, ScenePatch};
#[cfg(test)]
use elcarax_scene_model::{PropertyPath, PropertyValue, prepare_property_change};

use crate::inspector_state::{EDIT_REDO_COMMAND, EDIT_UNDO_COMMAND};
use crate::scene_state::SceneState;

pub(crate) const ADAPTER_CONNECT_COMMAND: &str = "adapter.connect";
pub(crate) const ADAPTER_HANDSHAKE_COMMAND: &str = "adapter.handshake";
pub(crate) const ADAPTER_LOAD_PROJECT_COMMAND: &str = "adapter.load_project";
pub(crate) const ADAPTER_LOAD_SCENE_COMMAND: &str = "adapter.load_scene";
pub(crate) const ADAPTER_SHOW_STATUS_COMMAND: &str = "adapter.show_status";
pub(crate) const ADAPTER_SHOW_DIAGNOSTICS_COMMAND: &str = "adapter.show_diagnostics";
pub(crate) const ADAPTER_DISCONNECT_COMMAND: &str = "adapter.disconnect";
pub(crate) const ADAPTER_EDIT_UNDO_COMMAND: &str = "adapter.edit.undo";
pub(crate) const ADAPTER_EDIT_REDO_COMMAND: &str = "adapter.edit.redo";

pub(crate) struct AdapterState {
    #[cfg(test)]
    connection: AdapterConnection,
    #[cfg(feature = "native-shell")]
    host: Option<AdapterHost>,
    status: AdapterHostState,
    #[cfg(any(test, feature = "native-shell"))]
    id: Option<AdapterId>,
    name: Option<AdapterName>,
    version: Option<AdapterVersion>,
    capabilities: Option<AdapterCapabilities>,
    diagnostics: Vec<AdapterDiagnostic>,
    edit_history: AdapterEditHistory,
    last_result: Option<AdapterCommandResult>,
}

#[cfg(test)]
enum AdapterConnection {
    None,
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
            AdapterCommand::Connect => self.connect(),
            AdapterCommand::Handshake => self.handshake(),
            AdapterCommand::LoadProject => self.load_project(),
            AdapterCommand::LoadScene => self.load_scene(scene_state),
            AdapterCommand::ShowStatus => self.show_status(),
            AdapterCommand::ShowDiagnostics => self.show_diagnostics(),
            AdapterCommand::Disconnect => self.disconnect(),
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

    pub(crate) fn is_connected(&self) -> bool {
        self.status == AdapterHostState::Connected
    }

    pub(crate) fn supports_viewport_preview(&self) -> bool {
        self.capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities.supports_viewport_preview)
    }

    #[cfg(feature = "native-shell")]
    pub(crate) fn connected_viewport_info(&self) -> Option<(String, bool)> {
        if !self.is_connected() {
            return None;
        }
        let id = self.id.as_ref()?.as_str().to_string();
        Some((id, self.supports_viewport_preview()))
    }

    pub(crate) fn request_viewport_frame(
        &mut self,
        viewport: &mut elcarax_core::ViewportState,
    ) -> Result<String, String> {
        if let Err(error) = viewport.begin_frame_request() {
            return Err(error.to_string());
        }
        if !self.is_connected() {
            viewport.apply_error(ViewportError::NoAdapterConnected);
            return Err("No adapter connected".to_string());
        }
        if !self.supports_viewport_preview() {
            viewport.apply_error(ViewportError::AdapterUnsupported);
            return Err("Adapter does not support viewport preview".to_string());
        }
        #[cfg(test)]
        {
            if let AdapterConnection::Fake(session) = &mut self.connection {
                let request = GetViewportFrameRequest {
                    viewport_id: AdapterViewportId(viewport.id.get()),
                    scene_id: None,
                    width: 256,
                    height: 256,
                    format: ViewportFrameFormat::Rgba8Unorm,
                };
                return match session.get_viewport_frame(request) {
                    Ok(response) => self.apply_viewport_response(viewport, response),
                    Err(error) => {
                        viewport.apply_error(ViewportError::AdapterFailed(error.to_string()));
                        Err(error.to_string())
                    }
                };
            }
        }
        #[cfg(feature = "native-shell")]
        {
            if let Some(host) = &mut self.host {
                let request = GetViewportFrameRequest {
                    viewport_id: AdapterViewportId(viewport.id.get()),
                    scene_id: None,
                    width: 256,
                    height: 256,
                    format: ViewportFrameFormat::Rgba8Unorm,
                };
                return match host.get_viewport_frame(request) {
                    Ok(response) => self.apply_viewport_response(viewport, response),
                    Err(error) => {
                        viewport.apply_error(ViewportError::AdapterFailed(error.to_string()));
                        Err(error.to_string())
                    }
                };
            }
        }
        viewport.apply_error(ViewportError::NoAdapterConnected);
        Err("No adapter connected".to_string())
    }

    fn apply_viewport_response(
        &self,
        viewport: &mut elcarax_core::ViewportState,
        response: elcarax_adapter_api::GetViewportFrameResponse,
    ) -> Result<String, String> {
        if response.status != ViewportFrameResponseStatus::Available {
            let message = response
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "adapter viewport frame unavailable".to_string());
            viewport.apply_error(ViewportError::AdapterFailed(message.clone()));
            return Err(message);
        }
        let frame = ViewportFrame::new(
            response.width,
            response.height,
            response.format,
            response.pixels,
        )
        .map_err(|error| error.to_string())?;
        viewport
            .apply_frame(frame)
            .map_err(|error| error.to_string())?;
        Ok(format!(
            "viewport frame {}x{} received",
            response.width, response.height
        ))
    }

    #[cfg(test)]
    pub(crate) fn attach_fake_session_for_tests(
        &mut self,
        session: AdapterSession<elcarax_adapter_host::FakeAdapterTransport>,
    ) {
        self.connection = AdapterConnection::Fake(session);
        self.status = AdapterHostState::Starting;
    }

    #[cfg(test)]
    pub(crate) fn handshake_for_tests(&mut self) -> AdapterCommandResult {
        self.handshake()
    }

    fn connect(&mut self) -> AdapterCommandResult {
        #[cfg(feature = "native-shell")]
        {
            if self.host.is_some() && self.status != AdapterHostState::Stopped {
                return AdapterCommandResult::new(
                    ADAPTER_CONNECT_COMMAND,
                    "adapter process already active",
                );
            }
            match AdapterHost::spawn(AdapterProcessSpec::stdio_game_adapter(), None) {
                Ok(host) => {
                    self.host = Some(host);
                    self.status = AdapterHostState::Starting;
                    AdapterCommandResult::new(ADAPTER_CONNECT_COMMAND, "adapter process started")
                }
                Err(error) => self.fail(ADAPTER_CONNECT_COMMAND, error),
            }
        }
        #[cfg(not(feature = "native-shell"))]
        {
            AdapterCommandResult::new(ADAPTER_CONNECT_COMMAND, "No adapter configured")
        }
    }

    fn handshake(&mut self) -> AdapterCommandResult {
        #[cfg(feature = "native-shell")]
        {
            if let Some(host) = &mut self.host {
                return match host.handshake(HandshakeRequest::current("elcarax-app", None)) {
                    Ok(info) => {
                        self.name = Some(info.name);
                        self.version = Some(info.version);
                        self.id = Some(info.id);
                        self.capabilities = Some(info.capabilities);
                        self.status = AdapterHostState::Connected;
                        AdapterCommandResult::new(ADAPTER_HANDSHAKE_COMMAND, "handshake succeeded")
                    }
                    Err(error) => self.fail(ADAPTER_HANDSHAKE_COMMAND, error),
                };
            }
        }
        #[cfg(test)]
        {
            if let AdapterConnection::Fake(session) = &mut self.connection {
                return match session.handshake(HandshakeRequest::current("elcarax-app", None)) {
                    Ok(info) => {
                        self.name = Some(info.name);
                        self.version = Some(info.version);
                        self.id = Some(info.id);
                        self.capabilities = Some(info.capabilities);
                        self.status = AdapterHostState::Connected;
                        AdapterCommandResult::new(ADAPTER_HANDSHAKE_COMMAND, "handshake succeeded")
                    }
                    Err(error) => self.fail(ADAPTER_HANDSHAKE_COMMAND, error),
                };
            }
        }
        AdapterCommandResult::new(
            ADAPTER_HANDSHAKE_COMMAND,
            "Diagnostic: adapter is not running",
        )
    }

    fn load_project(&mut self) -> AdapterCommandResult {
        #[cfg(feature = "native-shell")]
        {
            if let Some(host) = &mut self.host {
                let request = LoadProjectRequest { project_path: None };
                return match host.load_project(request) {
                    Ok(project) => AdapterCommandResult::new(
                        ADAPTER_LOAD_PROJECT_COMMAND,
                        format!("loaded adapter project {}", project.display_name),
                    ),
                    Err(error) => self.fail(ADAPTER_LOAD_PROJECT_COMMAND, error),
                };
            }
        }
        #[cfg(test)]
        {
            if let AdapterConnection::Fake(session) = &mut self.connection {
                let request = LoadProjectRequest { project_path: None };
                return match session.load_project(request) {
                    Ok(project) => AdapterCommandResult::new(
                        ADAPTER_LOAD_PROJECT_COMMAND,
                        format!("loaded adapter project {}", project.display_name),
                    ),
                    Err(error) => self.fail(ADAPTER_LOAD_PROJECT_COMMAND, error),
                };
            }
        }
        AdapterCommandResult::new(
            ADAPTER_LOAD_PROJECT_COMMAND,
            "Diagnostic: adapter is not running",
        )
    }

    fn load_scene(&mut self, scene_state: &mut SceneState) -> AdapterCommandResult {
        #[cfg(feature = "native-shell")]
        {
            if let Some(host) = &mut self.host {
                let request = GetSceneSnapshotRequest { scene_id: None };
                return match host.get_scene_snapshot(request) {
                    Ok(response) => self.finish_load_scene(scene_state, response),
                    Err(error) => self.fail(ADAPTER_LOAD_SCENE_COMMAND, error),
                };
            }
        }
        #[cfg(test)]
        {
            if let AdapterConnection::Fake(session) = &mut self.connection {
                let request = GetSceneSnapshotRequest { scene_id: None };
                return match session.get_scene_snapshot(request) {
                    Ok(response) => self.finish_load_scene(scene_state, response),
                    Err(error) => self.fail(ADAPTER_LOAD_SCENE_COMMAND, error),
                };
            }
        }
        let _ = scene_state;
        AdapterCommandResult::new(
            ADAPTER_LOAD_SCENE_COMMAND,
            "Diagnostic: adapter is not running",
        )
    }

    #[cfg(any(test, feature = "native-shell"))]
    fn finish_load_scene(
        &mut self,
        scene_state: &mut SceneState,
        response: elcarax_adapter_api::GetSceneSnapshotResponse,
    ) -> AdapterCommandResult {
        let count = response.snapshot.object_count();
        let adapter_id = self
            .id
            .clone()
            .unwrap_or_else(|| AdapterId::new("unknown-adapter"));
        scene_state.load_external_snapshot(
            response.snapshot,
            adapter_id,
            ADAPTER_LOAD_SCENE_COMMAND,
            format!(
                "Loaded adapter scene from {} with {count} objects",
                response.source_label
            ),
        );
        AdapterCommandResult::new(
            ADAPTER_LOAD_SCENE_COMMAND,
            format!("loaded adapter scene with {count} objects"),
        )
    }

    fn show_status(&self) -> AdapterCommandResult {
        AdapterCommandResult::new(
            ADAPTER_SHOW_STATUS_COMMAND,
            self.ui_snapshot().adapter_status,
        )
    }

    fn show_diagnostics(&mut self) -> AdapterCommandResult {
        #[cfg(feature = "native-shell")]
        {
            if let Some(host) = &mut self.host {
                return match host.get_diagnostics() {
                    Ok(response) => {
                        self.diagnostics = response.diagnostics;
                        AdapterCommandResult::new(
                            ADAPTER_SHOW_DIAGNOSTICS_COMMAND,
                            format!("{} adapter diagnostic(s)", self.diagnostics.len()),
                        )
                    }
                    Err(error) => self.fail(ADAPTER_SHOW_DIAGNOSTICS_COMMAND, error),
                };
            }
        }
        #[cfg(test)]
        {
            if let AdapterConnection::Fake(session) = &mut self.connection {
                let result = session.get_diagnostics(elcarax_adapter_api::GetDiagnosticsRequest);
                return match result {
                    Ok(response) => {
                        self.diagnostics = response.diagnostics;
                        AdapterCommandResult::new(
                            ADAPTER_SHOW_DIAGNOSTICS_COMMAND,
                            format!("{} adapter diagnostic(s)", self.diagnostics.len()),
                        )
                    }
                    Err(error) => self.fail(ADAPTER_SHOW_DIAGNOSTICS_COMMAND, error),
                };
            }
        }
        AdapterCommandResult::new(
            ADAPTER_SHOW_DIAGNOSTICS_COMMAND,
            "Diagnostic: adapter is not running",
        )
    }

    fn disconnect(&mut self) -> AdapterCommandResult {
        #[cfg(feature = "native-shell")]
        {
            if let Some(mut host) = self.host.take() {
                let result = host.shutdown();
                self.clear_adapter_session();
                return match result {
                    Ok(_) => AdapterCommandResult::new(
                        ADAPTER_DISCONNECT_COMMAND,
                        "adapter disconnected",
                    ),
                    Err(error) => self.fail(ADAPTER_DISCONNECT_COMMAND, error),
                };
            }
        }
        #[cfg(test)]
        {
            if let AdapterConnection::Fake(session) = &mut self.connection {
                let result = session
                    .shutdown_request(elcarax_adapter_api::ShutdownRequest)
                    .and_then(|response| {
                        session.shutdown_transport()?;
                        Ok(response)
                    });
                return match result {
                    Ok(_) => {
                        self.clear_adapter_session();
                        self.connection = AdapterConnection::None;
                        AdapterCommandResult::new(
                            ADAPTER_DISCONNECT_COMMAND,
                            "adapter disconnected",
                        )
                    }
                    Err(error) => self.fail(ADAPTER_DISCONNECT_COMMAND, error),
                };
            }
        }
        self.status = AdapterHostState::Stopped;
        AdapterCommandResult::new(ADAPTER_DISCONNECT_COMMAND, "adapter disconnected")
    }

    #[cfg(any(test, feature = "native-shell"))]
    fn clear_adapter_session(&mut self) {
        self.status = AdapterHostState::Stopped;
        self.id = None;
        self.name = None;
        self.version = None;
        self.capabilities = None;
        self.edit_history.clear();
    }

    #[cfg(test)]
    fn set_fixture_property(
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
        #[cfg(test)]
        {
            if let AdapterConnection::Fake(session) = &mut self.connection {
                return session.set_property(request).map_err(|error| {
                    self.status = AdapterHostState::Failed;
                    format!("{error}")
                });
            }
        }
        let _ = request;
        Err("adapter not connected".to_string())
    }

    #[cfg(any(test, feature = "native-shell"))]
    fn fail(&mut self, command_id: &str, error: AdapterHostError) -> AdapterCommandResult {
        self.status = AdapterHostState::Failed;
        AdapterCommandResult::new(command_id, format!("Diagnostic: {error}"))
    }

    #[cfg(test)]
    fn with_fake_session(
        session: AdapterSession<elcarax_adapter_host::FakeAdapterTransport>,
    ) -> Self {
        Self {
            #[cfg(test)]
            connection: AdapterConnection::Fake(session),
            #[cfg(feature = "native-shell")]
            host: None,
            status: AdapterHostState::Starting,
            #[cfg(any(test, feature = "native-shell"))]
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
            AdapterConnection::None => {
                panic!("expected fake adapter connection")
            }
        }
    }
}

impl Default for AdapterState {
    fn default() -> Self {
        Self {
            #[cfg(test)]
            connection: AdapterConnection::None,
            #[cfg(feature = "native-shell")]
            host: None,
            status: AdapterHostState::Disconnected,
            #[cfg(any(test, feature = "native-shell"))]
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
    Connect,
    Handshake,
    LoadProject,
    LoadScene,
    ShowStatus,
    ShowDiagnostics,
    Disconnect,
    Undo,
    Redo,
}

impl AdapterCommand {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            ADAPTER_CONNECT_COMMAND => Some(Self::Connect),
            ADAPTER_HANDSHAKE_COMMAND => Some(Self::Handshake),
            ADAPTER_LOAD_PROJECT_COMMAND => Some(Self::LoadProject),
            ADAPTER_LOAD_SCENE_COMMAND => Some(Self::LoadScene),
            ADAPTER_SHOW_STATUS_COMMAND => Some(Self::ShowStatus),
            ADAPTER_SHOW_DIAGNOSTICS_COMMAND => Some(Self::ShowDiagnostics),
            ADAPTER_DISCONNECT_COMMAND => Some(Self::Disconnect),
            ADAPTER_EDIT_UNDO_COMMAND => Some(Self::Undo),
            ADAPTER_EDIT_REDO_COMMAND => Some(Self::Redo),
            _ => None,
        }
    }
}

#[cfg_attr(not(feature = "native-shell"), allow(dead_code))]
pub(crate) fn adapter_command_for_inspector_edit(id: &str) -> Option<&'static str> {
    match id {
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
    #[cfg(test)]
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

    #[cfg(any(test, feature = "native-shell"))]
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

#[cfg(test)]
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
    use elcarax_adapter_api::{
        AdapterEvent, AdapterId, AdapterLog, AdapterRequestId, AdapterResponseMessage,
        GetDiagnosticsResponse, GetSceneSnapshotResponse, HandshakeResponse, ProtocolVersion,
        SetPropertyResponse, SetPropertyStatus, ShutdownResponse, decode_request_line,
    };
    use elcarax_adapter_host::{FakeAdapterTransport, event_line, response_line};
    use elcarax_scene_model::{
        ObjectSchema, PropertyGroup, PropertyKind, PropertySchema, SceneName, SceneObject,
        SceneObjectId, SceneObjectKind, ScenePatch, SceneSnapshot,
    };

    #[test]
    fn fake_transport_handshake_command_changes_status() {
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
    fn adapter_load_scene_updates_scene_snapshot() {
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            AdapterResponseMessage::GetSceneSnapshot(GetSceneSnapshotResponse {
                snapshot: fixture_scene().0,
                source_label: "fixture-adapter".to_string(),
            }),
        )]);
        let mut scene = SceneState::default();
        let result = state.execute_command_id(ADAPTER_LOAD_SCENE_COMMAND, &mut scene);
        assert!(result.is_some());
        assert_eq!(
            scene.snapshot().map(|snapshot| snapshot.object_count()),
            Some(1)
        );
        assert_eq!(scene.ui_snapshot().scene_name, "Fixture Scene".to_string());
    }

    #[test]
    fn adapter_show_diagnostics_records_diagnostics() {
        let mut state = state_with_lines(vec![
            event(AdapterEvent::Log(AdapterLog::info("ok"))),
            response(
                AdapterRequestId(1),
                AdapterResponseMessage::GetDiagnostics(GetDiagnosticsResponse {
                    diagnostics: vec![AdapterDiagnostic::info("fixture", "ready")],
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
    fn adapter_disconnect_clears_connection() {
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            AdapterResponseMessage::Shutdown(ShutdownResponse { accepted: true }),
        )]);
        let mut scene = SceneState::default();
        let result = state.execute_command_id(ADAPTER_DISCONNECT_COMMAND, &mut scene);
        assert_eq!(
            result.map(|result| result.message().to_string()),
            Some("adapter disconnected".to_string())
        );
        assert_eq!(state.status, AdapterHostState::Stopped);
    }

    #[test]
    fn adapter_backed_edit_sends_request_and_updates_scene() {
        let mut scene = adapter_fixture_scene();
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            accepted_health_response(&scene, PropertyValue::I64(100), PropertyValue::I64(65)),
        )]);
        let result = state.set_fixture_property(
            &mut scene,
            "test.adapter.set_fixture_health",
            "gameplay.health",
            PropertyValue::I64(65),
            "Set Fixture Health",
        );
        assert!(result.message().contains("confirmed"));
        assert_eq!(fixture_health(&scene), PropertyValue::I64(65));
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
        let mut scene = adapter_fixture_scene();
        let mut state = state_with_lines(vec![response(
            AdapterRequestId(1),
            rejected_health_response(&scene),
        )]);
        let result = state.set_fixture_property(
            &mut scene,
            "test.adapter.set_fixture_health",
            "gameplay.health",
            PropertyValue::I64(65),
            "Set Fixture Health",
        );
        assert!(result.message().contains("Diagnostic:"));
        assert_eq!(fixture_health(&scene), PropertyValue::I64(100));
    }

    #[test]
    fn adapter_backed_undo_and_redo_send_writebacks() {
        let mut scene = adapter_fixture_scene();
        let mut state = state_with_lines(vec![
            response(
                AdapterRequestId(1),
                accepted_health_response(&scene, PropertyValue::I64(100), PropertyValue::I64(65)),
            ),
            response(
                AdapterRequestId(2),
                accepted_health_response(&scene, PropertyValue::I64(65), PropertyValue::I64(100)),
            ),
            response(
                AdapterRequestId(3),
                accepted_health_response(&scene, PropertyValue::I64(100), PropertyValue::I64(65)),
            ),
        ]);
        let _ = state.set_fixture_property(
            &mut scene,
            "test.adapter.set_fixture_health",
            "gameplay.health",
            PropertyValue::I64(65),
            "Set Fixture Health",
        );
        let _ = state.execute_command_id(ADAPTER_EDIT_UNDO_COMMAND, &mut scene);
        assert_eq!(fixture_health(&scene), PropertyValue::I64(100));
        let _ = state.execute_command_id(ADAPTER_EDIT_REDO_COMMAND, &mut scene);
        assert_eq!(fixture_health(&scene), PropertyValue::I64(65));
        assert_eq!(state.fake_writes().len(), 3);
    }

    #[test]
    fn disconnected_adapter_edit_fails_clearly() {
        let mut scene = adapter_fixture_scene();
        let mut state = AdapterState::default();
        let result = state.set_fixture_property(
            &mut scene,
            "test.adapter.set_fixture_health",
            "gameplay.health",
            PropertyValue::I64(65),
            "Set Fixture Health",
        );
        assert!(result.message().contains("adapter not connected"));
        assert_eq!(fixture_health(&scene), PropertyValue::I64(100));
    }

    fn state_with_lines(lines: Vec<String>) -> AdapterState {
        AdapterState::with_fake_session(AdapterSession::new(FakeAdapterTransport::new(lines)))
    }

    fn adapter_fixture_scene() -> SceneState {
        let (snapshot, object_id) = fixture_scene();
        let mut scene = SceneState::default();
        scene.load_external_snapshot(
            snapshot,
            AdapterId::new("fixture-adapter"),
            "test",
            "Loaded adapter scene",
        );
        assert!(scene.select_object(object_id));
        scene
    }

    fn handshake_response() -> HandshakeResponse {
        HandshakeResponse {
            adapter_id: AdapterId::new("fixture-adapter"),
            adapter_name: AdapterName::new("Fixture Adapter"),
            adapter_version: AdapterVersion::new("0.1.0"),
            protocol_version: ProtocolVersion::V0,
            capabilities: AdapterCapabilities {
                provides_project_info: true,
                provides_scene_snapshot: true,
                provides_diagnostics: true,
                supports_property_writeback: false,
                supports_viewport_preview: false,
            },
        }
    }

    fn response(request_id: AdapterRequestId, message: AdapterResponseMessage) -> String {
        match response_line(request_id, message) {
            Ok(line) => line,
            Err(error) => panic!("response should serialize: {error}"),
        }
    }

    fn accepted_health_response(
        scene: &SceneState,
        old_value: PropertyValue,
        new_value: PropertyValue,
    ) -> AdapterResponseMessage {
        let (scene_id, object_id) = scene_ids(scene);
        let path = path("gameplay.health");
        AdapterResponseMessage::SetProperty(SetPropertyResponse {
            status: SetPropertyStatus::Accepted,
            scene_id,
            object_id,
            path: path.clone(),
            old_value: Some(old_value),
            confirmed_new_value: Some(new_value.clone()),
            patch: Some(ScenePatch::property_updated(object_id, path, new_value)),
            diagnostics: Vec::new(),
        })
    }

    fn rejected_health_response(scene: &SceneState) -> AdapterResponseMessage {
        let (scene_id, object_id) = scene_ids(scene);
        AdapterResponseMessage::SetProperty(SetPropertyResponse {
            status: SetPropertyStatus::Rejected,
            scene_id,
            object_id,
            path: path("gameplay.health"),
            old_value: None,
            confirmed_new_value: None,
            patch: None,
            diagnostics: vec![AdapterDiagnostic::info("fixture", "rejected")],
        })
    }

    fn event(event: AdapterEvent) -> String {
        match event_line(event) {
            Ok(line) => line,
            Err(error) => panic!("event should serialize: {error}"),
        }
    }

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("test path should parse: {error}"),
        }
    }

    fn fixture_scene() -> (SceneSnapshot, SceneObjectId) {
        let health_path = path("gameplay.health");
        let schema = ObjectSchema::new("Actor").with_property(PropertySchema::editable(
            health_path.clone(),
            "Health",
            PropertyKind::I64,
            PropertyGroup::new("Gameplay"),
        ));
        let mut object =
            SceneObject::new("Fixture Actor", SceneObjectKind::Character, schema.type_id);
        object.set_property(health_path, PropertyValue::I64(100));
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Fixture Scene"));
        snapshot.add_schema(schema);
        snapshot.add_root_object(object);
        (snapshot, object_id)
    }

    fn scene_ids(scene: &SceneState) -> (elcarax_scene_model::SceneId, SceneObjectId) {
        let snapshot = match scene.snapshot() {
            Some(snapshot) => snapshot,
            None => panic!("scene should be loaded"),
        };
        let actor = match snapshot.object_by_name("Fixture Actor") {
            Some(actor) => actor,
            None => panic!("fixture actor should exist"),
        };
        (snapshot.scene_id(), actor.id)
    }

    fn fixture_health(scene: &SceneState) -> PropertyValue {
        let snapshot = match scene.snapshot() {
            Some(snapshot) => snapshot,
            None => panic!("scene should be loaded"),
        };
        let actor = match snapshot.object_by_name("Fixture Actor") {
            Some(actor) => actor,
            None => panic!("fixture actor should exist"),
        };
        match actor.property(&path("gameplay.health")) {
            Some(value) => value.clone(),
            None => panic!("health should exist"),
        }
    }
}
