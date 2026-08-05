//! Command and undo/redo system for Elcarax.

mod command;
mod history;
mod scene_mutation;

pub use command::{
    CommandAvailability, CommandBindingDiagnostic, CommandBindingRegistry, CommandCategory,
    CommandContext, CommandDescription, CommandEffect, CommandId, CommandInvocation, CommandName,
    CommandPresentation, CommandRegistry, CommandRegistryError, CommandResult, CommandScope,
    CommandShortcut, CommandTitle, CommandToolbarPlacement, EditorCommand, KeyBinding, KeyChord,
    KeyModifier, RegisteredCommand, SceneMutationSink, built_in_commands,
};
pub use history::CommandHistory;
pub use scene_mutation::{ApplyScenePatchCommand, RedoCommand, UndoCommand};
