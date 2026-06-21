use elcarax_core::Diagnostic;
use elcarax_scene_model::{PropertyPath, PropertyValue, SceneId, SceneObjectId, SceneSnapshot};

use crate::{HandshakeRequest, HandshakeResponse};

#[derive(Debug, Clone, PartialEq)]
pub enum EditorToAdapter {
    Handshake(HandshakeRequest),
    LoadProject,
    ListScenes,
    OpenScene { scene_id: SceneId },
    GetSceneSnapshot { scene_id: SceneId },
    SetProperty {
        object_id: SceneObjectId,
        path: PropertyPath,
        value: PropertyValue,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AdapterToEditor {
    HandshakeAccepted(HandshakeResponse),
    ProjectLoaded { display_name: String },
    SceneList(Vec<SceneId>),
    SceneSnapshot(SceneSnapshot),
    PropertySet,
    Diagnostic(Diagnostic),
    Log(String),
    AdapterError(String),
    ShutdownComplete,
}
