use elcarax_core::Result;

use crate::{CommandContext, CommandEffect, EditorCommand};

#[derive(Default)]
pub struct CommandHistory {
    undo_stack: Vec<Box<dyn EditorCommand>>,
    redo_stack: Vec<Box<dyn EditorCommand>>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute(
        &mut self,
        mut command: Box<dyn EditorCommand>,
        context: &mut CommandContext<'_>,
    ) -> Result<CommandEffect> {
        let effect = command.apply(context)?;
        self.undo_stack.push(command);
        self.redo_stack.clear();
        Ok(effect)
    }

    pub fn undo(&mut self, context: &mut CommandContext<'_>) -> Result<Option<CommandEffect>> {
        let Some(mut command) = self.undo_stack.pop() else {
            return Ok(None);
        };
        match command.revert(context) {
            Ok(effect) => {
                self.redo_stack.push(command);
                Ok(Some(effect))
            }
            Err(error) => {
                self.undo_stack.push(command);
                Err(error)
            }
        }
    }

    pub fn redo(&mut self, context: &mut CommandContext<'_>) -> Result<Option<CommandEffect>> {
        let Some(mut command) = self.redo_stack.pop() else {
            return Ok(None);
        };
        match command.apply(context) {
            Ok(effect) => {
                self.undo_stack.push(command);
                Ok(Some(effect))
            }
            Err(error) => {
                self.redo_stack.push(command);
                Err(error)
            }
        }
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
