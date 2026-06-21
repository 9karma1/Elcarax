use elcarax_core::Result;
use elcarax_scene_model::SceneSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEffect {
    NoChange,
    SceneChanged,
    ProjectChanged,
    UiChanged,
}

pub struct CommandContext<'a> {
    pub scene: &'a mut SceneSnapshot,
}

pub trait EditorCommand {
    fn label(&self) -> &str;
    fn apply(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect>;
    fn revert(&mut self, context: &mut CommandContext<'_>) -> Result<CommandEffect>;
}
