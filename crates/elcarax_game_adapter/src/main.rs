use elcarax_adapter_api::{
    AdapterCapabilities, AdapterToEditor, EditorToAdapter, HandshakeResponse,
};
use elcarax_adapter_sdk::ElcaraxAdapter;
use elcarax_core::{Diagnostic, DiagnosticSource, ElcaraxError, Result};
use elcarax_scene_model::{
    ObjectSchema, PropertyKind, PropertyPath, PropertySchema, PropertyValue, SceneObject,
    SceneSnapshot,
};

struct MockGameAdapter {
    scene: SceneSnapshot,
}

impl MockGameAdapter {
    fn new() -> Result<Self> {
        let position_path = PropertyPath::parse("transform.position")?;
        let name_path = PropertyPath::parse("identity.name")?;
        let schema = ObjectSchema::new("Entity")
            .with_property(PropertySchema::editable(
                position_path.clone(),
                "Position",
                PropertyKind::Vec3,
            ))
            .with_property(PropertySchema::editable(
                name_path.clone(),
                "Name",
                PropertyKind::String,
            ));
        let mut object = SceneObject::new("Player", schema.type_id);
        object.set_property(position_path, PropertyValue::Vec3([0.0, 1.0, 0.0]));
        object.set_property(name_path, PropertyValue::String("Player".to_owned()));

        let mut scene = SceneSnapshot::empty();
        scene.add_schema(schema);
        scene.add_root_object(object);
        Ok(Self { scene })
    }
}

impl ElcaraxAdapter for MockGameAdapter {
    fn handle_message(&mut self, message: EditorToAdapter) -> Result<AdapterToEditor> {
        match message {
            EditorToAdapter::Handshake(_) => {
                Ok(AdapterToEditor::HandshakeAccepted(HandshakeResponse {
                    adapter_name: "elcarax-mock-game-adapter".to_owned(),
                    adapter_version: env!("CARGO_PKG_VERSION").to_owned(),
                    capabilities: AdapterCapabilities::game_editor_v0(),
                }))
            }
            EditorToAdapter::LoadProject => Ok(AdapterToEditor::ProjectLoaded {
                display_name: "Mock Game Project".to_owned(),
            }),
            EditorToAdapter::ListScenes => {
                Ok(AdapterToEditor::SceneList(vec![self.scene.scene_id]))
            }
            EditorToAdapter::GetSceneSnapshot { .. } => {
                Ok(AdapterToEditor::SceneSnapshot(self.scene.clone()))
            }
            EditorToAdapter::SetProperty {
                object_id,
                path,
                value,
            } => {
                self.scene.set_property(object_id, path, value)?;
                Ok(AdapterToEditor::PropertySet)
            }
            EditorToAdapter::OpenScene { .. } => Ok(AdapterToEditor::Diagnostic(Diagnostic::info(
                DiagnosticSource::new("game-adapter"),
                "scene opened",
            ))),
            EditorToAdapter::Shutdown => Ok(AdapterToEditor::ShutdownComplete),
        }
    }
}

fn main() -> Result<()> {
    let mut adapter = MockGameAdapter::new()?;
    let response = adapter.handle_message(EditorToAdapter::LoadProject)?;
    match response {
        AdapterToEditor::ProjectLoaded { display_name } => {
            println!("{display_name}");
            Ok(())
        }
        other => Err(ElcaraxError::Adapter(format!(
            "unexpected response: {other:?}"
        ))),
    }
}
