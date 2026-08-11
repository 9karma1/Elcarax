#![cfg_attr(not(feature = "native-shell"), allow(dead_code))]

use elcarax_adapter_api::{
    AdapterCapabilities, AdapterDiagnostic, AdapterName, AdapterVersion, SetPropertyRequest,
    SetPropertyResponse, ViewportFrameResponseStatus,
};
#[cfg(any(test, feature = "native-shell"))]
use elcarax_adapter_api::{
    AdapterId, GetSceneSnapshotRequest, HandshakeRequest, LoadProjectRequest,
};
#[cfg(any(test, feature = "native-shell"))]
use elcarax_adapter_api::{AdapterViewportId, GetViewportFrameRequest};
#[cfg(feature = "native-shell")]
use elcarax_adapter_api::{PickViewportObjectRequest, ViewportPickResponseStatus};
#[cfg(any(test, feature = "native-shell"))]
use elcarax_adapter_host::AdapterHostError;
use elcarax_adapter_host::AdapterHostState;
#[cfg(test)]
use elcarax_adapter_host::AdapterSession;
#[cfg(feature = "native-shell")]
use elcarax_adapter_host::{AdapterHost, AdapterProcessSpec};

use crate::adapter_display::{AdapterUiSnapshot, adapter_ui_snapshot};
use crate::project_config::AppProjectConfig;
use crate::viewport_state::ViewportFrameRequestSize;
use elcarax_adapter_api::{ViewportCameraInput, ViewportEditorInput};
#[cfg(any(test, feature = "native-shell"))]
use elcarax_core::ViewportFrameFormat;
use elcarax_core::{ViewportError, ViewportFrame};
#[cfg(test)]
use elcarax_scene_model::PropertyPath;
#[cfg(feature = "native-shell")]
use elcarax_scene_model::SceneObjectId;
use elcarax_scene_model::ScenePatch;

use crate::scene_state::{SceneState, UNSAVED_SCENE_MESSAGE};

pub(crate) const ADAPTER_CONNECT_COMMAND: &str = "adapter.connect";
pub(crate) const ADAPTER_HANDSHAKE_COMMAND: &str = "adapter.handshake";
pub(crate) const ADAPTER_LOAD_PROJECT_COMMAND: &str = "adapter.load_project";
pub(crate) const ADAPTER_LOAD_SCENE_COMMAND: &str = "adapter.load_scene";
pub(crate) const ADAPTER_SHOW_STATUS_COMMAND: &str = "adapter.show_status";
pub(crate) const ADAPTER_SHOW_DIAGNOSTICS_COMMAND: &str = "adapter.show_diagnostics";
pub(crate) const ADAPTER_DISCONNECT_COMMAND: &str = "adapter.disconnect";

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
    last_result: Option<AdapterCommandResult>,
    config: AdapterLaunchConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AdapterLaunchConfig {
    executable: Option<std::path::PathBuf>,
    project_path: Option<std::path::PathBuf>,
    auto_connect: bool,
}

impl AdapterLaunchConfig {
    pub(crate) fn from_project_config(config: &AppProjectConfig) -> Self {
        Self {
            executable: config.adapter_executable.clone(),
            project_path: config.adapter_project_path.clone(),
            auto_connect: config.auto_connect_adapter,
        }
    }

    pub(crate) const fn auto_connect(&self) -> bool {
        self.auto_connect
    }
}

#[cfg(test)]
enum AdapterConnection {
    None,
    Fake(AdapterSession<elcarax_adapter_host::FakeAdapterTransport>),
}

impl AdapterState {
    pub(crate) fn new(config: AdapterLaunchConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    pub(crate) fn auto_connect_enabled(&self) -> bool {
        self.config.auto_connect()
    }

    pub(crate) fn execute(
        &mut self,
        command: AdapterCommand,
        scene_state: &mut SceneState,
    ) -> AdapterCommandResult {
        let result = match command {
            AdapterCommand::Connect => self.connect(),
            AdapterCommand::Handshake => self.handshake(),
            AdapterCommand::LoadProject => self.load_project(),
            AdapterCommand::LoadScene => self.load_scene(scene_state),
            AdapterCommand::ShowStatus => self.show_status(),
            AdapterCommand::ShowDiagnostics => self.show_diagnostics(),
            AdapterCommand::Disconnect => self.disconnect(),
        };
        self.last_result = Some(result.clone());
        result
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
    pub(crate) fn supports_viewport_picking(&self) -> bool {
        self.capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities.supports_viewport_picking)
    }

    #[cfg(feature = "native-shell")]
    pub(crate) fn connected_viewport_info(&self) -> Option<(String, bool)> {
        if !self.is_connected() {
            return None;
        }
        let id = self.id.as_ref()?.as_str().to_string();
        Some((id, self.supports_viewport_preview()))
    }

    #[allow(unused_variables)]
    pub(crate) fn request_viewport_frame(
        &mut self,
        viewport: &mut elcarax_core::ViewportState,
        request_size: ViewportFrameRequestSize,
        camera_input: Option<ViewportCameraInput>,
        editor_input: Option<ViewportEditorInput>,
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
                    width: request_size.width(),
                    height: request_size.height(),
                    format: ViewportFrameFormat::Rgba8Unorm,
                    camera_input,
                    editor_input,
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
                    width: request_size.width(),
                    height: request_size.height(),
                    format: ViewportFrameFormat::Rgba8Unorm,
                    camera_input,
                    editor_input,
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

    #[cfg(feature = "native-shell")]
    pub(crate) fn pick_viewport_object(
        &mut self,
        viewport_id: u64,
        u: f32,
        v: f32,
    ) -> Result<Option<SceneObjectId>, String> {
        if !self.is_connected() {
            return Err("No adapter connected".to_string());
        }
        if !self.supports_viewport_picking() {
            return Err("Adapter does not support viewport picking".to_string());
        }
        let Some(host) = &mut self.host else {
            return Err("No adapter connected".to_string());
        };
        let request = PickViewportObjectRequest {
            viewport_id: AdapterViewportId(viewport_id),
            scene_id: None,
            u,
            v,
        };
        let response = host
            .pick_viewport_object(request)
            .map_err(|error| error.to_string())?;
        match response.status {
            ViewportPickResponseStatus::Picked => Ok(response.object_id),
            ViewportPickResponseStatus::Missed => Ok(None),
            _ => Err(response
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "adapter viewport pick failed".to_string())),
        }
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
            match AdapterHost::spawn(self.process_spec(), None) {
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

    #[cfg(feature = "native-shell")]
    fn process_spec(&self) -> AdapterProcessSpec {
        match &self.config.executable {
            Some(executable) => AdapterProcessSpec::new(executable),
            None => AdapterProcessSpec::stdio_game_adapter(),
        }
    }

    fn handshake(&mut self) -> AdapterCommandResult {
        #[cfg(feature = "native-shell")]
        {
            if let Some(host) = &mut self.host {
                return match host.handshake(HandshakeRequest::current(
                    "elcarax-app",
                    self.config.project_path.clone(),
                )) {
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
                return match session.handshake(HandshakeRequest::current(
                    "elcarax-app",
                    self.config.project_path.clone(),
                )) {
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
                let request = LoadProjectRequest {
                    project_path: self.config.project_path.clone(),
                };
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
                let request = LoadProjectRequest {
                    project_path: self.config.project_path.clone(),
                };
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
        if scene_state.has_unsaved_changes() {
            return AdapterCommandResult::new(
                ADAPTER_LOAD_SCENE_COMMAND,
                format!("Diagnostic: {UNSAVED_SCENE_MESSAGE}"),
            );
        }
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
    }

    pub(crate) fn confirm_set_property(
        &mut self,
        request: SetPropertyRequest,
    ) -> Result<ScenePatch, String> {
        let fallback_value = request.new_value.clone();
        let response = self.send_set_property(request)?;
        if !response.status.is_accepted() {
            let message = writeback_failure_message(&response);
            self.diagnostics.extend(response.diagnostics);
            return Err(message);
        }
        Ok(response.patch.unwrap_or_else(|| {
            ScenePatch::property_updated(
                response.object_id,
                response.component_id,
                response.path.clone(),
                response
                    .confirmed_new_value
                    .clone()
                    .unwrap_or(fallback_value),
            )
        }))
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
        #[cfg(feature = "native-shell")]
        {
            if let Some(host) = &mut self.host {
                return host.set_property(request).map_err(|error| {
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
            last_result: None,
            config: AdapterLaunchConfig::default(),
        }
    }

    #[cfg(test)]
    fn fake_writes(&self) -> &[elcarax_adapter_api::AdapterFrame] {
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
            last_result: None,
            config: AdapterLaunchConfig::default(),
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
pub(crate) enum AdapterCommand {
    Connect,
    Handshake,
    LoadProject,
    LoadScene,
    ShowStatus,
    ShowDiagnostics,
    Disconnect,
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
        SetPropertyResponse, SetPropertyStatus, ShutdownResponse, decode_request_frame,
    };
    use elcarax_adapter_host::{FakeAdapterTransport, event_frame, response_frame};
    use elcarax_commands::CommandHistory;
    use elcarax_scene_model::{
        ComponentInstance, ComponentSchema, ComponentTypeName, ObjectSchema, PropertyKind,
        PropertySchema, PropertyTypeRegistry, PropertyValue, SceneName, SceneObject, SceneObjectId,
        SceneObjectKind, ScenePatch, SceneSnapshot, components, kinds,
    };

    use crate::edit_service::{ScenePropertyEdit, SessionEditService};
    use crate::editor_session::EditorSessionState;
    use crate::project_config::AppProjectConfig;

    #[test]
    fn fake_transport_handshake_command_changes_status() {
        let mut state = state_with_frames(vec![response(
            AdapterRequestId(1),
            AdapterResponseMessage::Handshake(handshake_response()),
        )]);
        let mut scene = SceneState::default();
        let result = state.execute(AdapterCommand::Handshake, &mut scene);
        assert!(result.message().contains("handshake"));
        assert_eq!(state.status, AdapterHostState::Connected);
    }

    #[test]
    fn adapter_load_scene_is_blocked_while_project_scene_is_dirty() {
        let temp =
            std::env::temp_dir().join(format!("elcarax-adapter-dirty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let mut editor = EditorSessionState::new(AppProjectConfig {
            create_root: Some(temp.clone()),
            ..AppProjectConfig::default()
        });
        let _ = editor
            .session_mut()
            .execute_project_command(crate::project_state::ProjectCommand::Create, None);
        editor.scene.mark_document_modified();
        let mut adapter = AdapterState::default();
        let result = adapter.execute(AdapterCommand::LoadScene, &mut editor.scene);
        assert!(result.message().contains("Unsaved"));
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn adapter_load_scene_updates_scene_snapshot() {
        let mut state = state_with_frames(vec![response(
            AdapterRequestId(1),
            AdapterResponseMessage::GetSceneSnapshot(GetSceneSnapshotResponse {
                snapshot: fixture_scene().0,
                source_label: "fixture-adapter".to_string(),
            }),
        )]);
        let mut scene = SceneState::default();
        let result = state.execute(AdapterCommand::LoadScene, &mut scene);
        assert!(result.message().contains("scene"));
        assert_eq!(
            scene.snapshot().map(|snapshot| snapshot.object_count()),
            Some(1)
        );
        assert_eq!(scene.ui_snapshot().scene_name, "Fixture Scene".to_string());
    }

    #[test]
    fn adapter_show_diagnostics_records_diagnostics() {
        let mut state = state_with_frames(vec![
            event(AdapterEvent::Log(AdapterLog::info("ok"))),
            response(
                AdapterRequestId(1),
                AdapterResponseMessage::GetDiagnostics(GetDiagnosticsResponse {
                    diagnostics: vec![AdapterDiagnostic::info("fixture", "ready")],
                }),
            ),
        ]);
        let mut scene = SceneState::default();
        let result = state.execute(AdapterCommand::ShowDiagnostics, &mut scene);
        assert_eq!(result.message(), "1 adapter diagnostic(s)");
        assert_eq!(state.diagnostics.len(), 1);
    }

    #[test]
    fn adapter_disconnect_clears_connection() {
        let mut state = state_with_frames(vec![response(
            AdapterRequestId(1),
            AdapterResponseMessage::Shutdown(ShutdownResponse { accepted: true }),
        )]);
        let mut scene = SceneState::default();
        let result = state.execute(AdapterCommand::Disconnect, &mut scene);
        assert_eq!(result.message(), "adapter disconnected");
        assert_eq!(state.status, AdapterHostState::Stopped);
    }

    #[test]
    fn adapter_backed_edit_sends_request_and_updates_scene() {
        let mut scene = adapter_fixture_scene();
        let component_id = fixture_health_component_id(&scene);
        let mut state = state_with_frames(vec![response(
            AdapterRequestId(1),
            accepted_health_response(
                &scene,
                component_id,
                PropertyValue::I64(100),
                PropertyValue::I64(65),
            ),
        )]);
        let mut history = CommandHistory::new();
        let result = SessionEditService::commit_property(
            &mut scene,
            &mut history,
            Some(&mut state),
            ScenePropertyEdit::new(
                component_id,
                path("health"),
                PropertyValue::I64(65),
                "Set Fixture Health",
            ),
            &PropertyTypeRegistry::default(),
        );
        assert!(result.is_ok_and(|message| message.contains("65")));
        assert_eq!(fixture_health(&scene), PropertyValue::I64(65));
        let request = match state.fake_writes().first() {
            Some(frame) => match decode_request_frame(frame) {
                Ok(request) => request,
                Err(error) => panic!("request should decode: {error}"),
            },
            None => panic!("adapter request should be written"),
        };
        assert!(matches!(
            request.message,
            elcarax_adapter_api::AdapterRequestMessage::SetProperty(_)
        ));
        assert_eq!(history.undo_count(), 1);
    }

    #[test]
    fn failed_adapter_edit_records_diagnostic_and_does_not_mutate_value() {
        let mut scene = adapter_fixture_scene();
        let component_id = fixture_health_component_id(&scene);
        let mut state = state_with_frames(vec![response(
            AdapterRequestId(1),
            rejected_health_response(&scene, component_id),
        )]);
        let mut history = CommandHistory::new();
        let result = SessionEditService::commit_property_result(
            &mut scene,
            &mut history,
            Some(&mut state),
            ScenePropertyEdit::new(
                component_id,
                path("health"),
                PropertyValue::I64(65),
                "Set Fixture Health",
            ),
            &PropertyTypeRegistry::default(),
        );
        assert!(result.message().contains("Diagnostic:"));
        assert_eq!(fixture_health(&scene), PropertyValue::I64(100));
    }

    #[test]
    fn adapter_backed_undo_and_redo_send_writebacks() {
        let mut scene = adapter_fixture_scene();
        let component_id = fixture_health_component_id(&scene);
        let mut state = state_with_frames(vec![
            response(
                AdapterRequestId(1),
                accepted_health_response(
                    &scene,
                    component_id,
                    PropertyValue::I64(100),
                    PropertyValue::I64(65),
                ),
            ),
            response(
                AdapterRequestId(2),
                accepted_health_response(
                    &scene,
                    component_id,
                    PropertyValue::I64(65),
                    PropertyValue::I64(100),
                ),
            ),
            response(
                AdapterRequestId(3),
                accepted_health_response(
                    &scene,
                    component_id,
                    PropertyValue::I64(100),
                    PropertyValue::I64(65),
                ),
            ),
        ]);
        let mut history = CommandHistory::new();
        let _ = SessionEditService::commit_property(
            &mut scene,
            &mut history,
            Some(&mut state),
            ScenePropertyEdit::new(
                component_id,
                path("health"),
                PropertyValue::I64(65),
                "Set Fixture Health",
            ),
            &PropertyTypeRegistry::default(),
        );
        let property_types = PropertyTypeRegistry::default();
        let undo =
            SessionEditService::undo(&mut scene, &mut history, Some(&mut state), &property_types);
        assert!(undo.message().contains("edit.undo"));
        assert_eq!(fixture_health(&scene), PropertyValue::I64(100));
        let redo =
            SessionEditService::redo(&mut scene, &mut history, Some(&mut state), &property_types);
        assert!(redo.message().contains("edit.redo"));
        assert_eq!(fixture_health(&scene), PropertyValue::I64(65));
        assert_eq!(state.fake_writes().len(), 3);
    }

    #[test]
    fn disconnected_adapter_edit_fails_clearly() {
        let mut scene = adapter_fixture_scene();
        let component_id = fixture_health_component_id(&scene);
        let mut state = AdapterState::default();
        let mut history = CommandHistory::new();
        let result = SessionEditService::commit_property_result(
            &mut scene,
            &mut history,
            Some(&mut state),
            ScenePropertyEdit::new(
                component_id,
                path("health"),
                PropertyValue::I64(65),
                "Set Fixture Health",
            ),
            &PropertyTypeRegistry::default(),
        );
        assert!(result.message().contains("Diagnostic:"));
        assert!(result.message().contains("adapter not connected"));
        assert_eq!(fixture_health(&scene), PropertyValue::I64(100));
    }

    fn state_with_frames(frames: Vec<elcarax_adapter_api::AdapterFrame>) -> AdapterState {
        let mut state =
            AdapterState::with_fake_session(AdapterSession::new(FakeAdapterTransport::new(frames)));
        state.status = AdapterHostState::Connected;
        state
    }

    fn adapter_fixture_scene() -> SceneState {
        let (snapshot, object_id, _component_id) = fixture_scene();
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
                supports_viewport_picking: false,
            },
        }
    }

    fn response(
        request_id: AdapterRequestId,
        message: AdapterResponseMessage,
    ) -> elcarax_adapter_api::AdapterFrame {
        match response_frame(request_id, message) {
            Ok(frame) => frame,
            Err(error) => panic!("response should serialize: {error}"),
        }
    }

    fn accepted_health_response(
        scene: &SceneState,
        component_id: elcarax_scene_model::ComponentInstanceId,
        old_value: PropertyValue,
        new_value: PropertyValue,
    ) -> AdapterResponseMessage {
        let (scene_id, object_id) = scene_ids(scene);
        let health_path = path("health");
        AdapterResponseMessage::SetProperty(SetPropertyResponse {
            status: SetPropertyStatus::Accepted,
            scene_id,
            object_id,
            component_id,
            path: health_path.clone(),
            old_value: Some(old_value),
            confirmed_new_value: Some(new_value.clone()),
            patch: Some(ScenePatch::property_updated(
                object_id,
                component_id,
                health_path,
                new_value,
            )),
            diagnostics: Vec::new(),
        })
    }

    fn rejected_health_response(
        scene: &SceneState,
        component_id: elcarax_scene_model::ComponentInstanceId,
    ) -> AdapterResponseMessage {
        let (scene_id, object_id) = scene_ids(scene);
        AdapterResponseMessage::SetProperty(SetPropertyResponse {
            status: SetPropertyStatus::Rejected,
            scene_id,
            object_id,
            component_id,
            path: path("health"),
            old_value: None,
            confirmed_new_value: None,
            patch: None,
            diagnostics: vec![AdapterDiagnostic::info("fixture", "rejected")],
        })
    }

    fn event(event: AdapterEvent) -> elcarax_adapter_api::AdapterFrame {
        match event_frame(event) {
            Ok(frame) => frame,
            Err(error) => panic!("event should serialize: {error}"),
        }
    }

    fn path(value: &str) -> PropertyPath {
        match PropertyPath::parse(value) {
            Ok(path) => path,
            Err(error) => panic!("test path should parse: {error}"),
        }
    }

    fn fixture_scene() -> (
        SceneSnapshot,
        SceneObjectId,
        elcarax_scene_model::ComponentInstanceId,
    ) {
        let health_path = path("health");
        let schema = ObjectSchema::new("Actor").with_component(
            ComponentSchema::new(components::GAMEPLAY, "Gameplay").with_property(
                PropertySchema::editable(health_path.clone(), "Health", PropertyKind::I64),
            ),
        );
        let component = ComponentInstance::new(components::GAMEPLAY, "Gameplay")
            .with_property(health_path, PropertyValue::I64(100));
        let component_id = component.id;
        let object = SceneObject::new(
            "Fixture Actor",
            SceneObjectKind::new(kinds::CHARACTER),
            schema.type_id,
        )
        .with_component(component);
        let object_id = object.id;
        let mut snapshot = SceneSnapshot::with_name(SceneName::from_unvalidated("Fixture Scene"));
        snapshot.add_schema(schema);
        let _ = snapshot.add_object(
            None,
            0,
            object,
            &elcarax_scene_model::PropertyTypeRegistry::default(),
        );
        (snapshot, object_id, component_id)
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

    fn fixture_health_component_id(scene: &SceneState) -> elcarax_scene_model::ComponentInstanceId {
        let snapshot = match scene.snapshot() {
            Some(snapshot) => snapshot,
            None => panic!("scene should be loaded"),
        };
        let actor = match snapshot.object_by_name("Fixture Actor") {
            Some(actor) => actor,
            None => panic!("fixture actor should exist"),
        };
        match actor.component_by_type(&ComponentTypeName::new(components::GAMEPLAY)) {
            Some(component) => component.id,
            None => panic!("gameplay component should exist"),
        }
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
        let component = match actor.component_by_type(&ComponentTypeName::new(components::GAMEPLAY))
        {
            Some(component) => component,
            None => panic!("gameplay component should exist"),
        };
        match component.property(&path("health")) {
            Some(value) => value.clone(),
            None => panic!("health should exist"),
        }
    }
}
