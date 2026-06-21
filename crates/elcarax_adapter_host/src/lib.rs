//! Adapter supervision boundary for Elcarax.

use elcarax_adapter_api::{AdapterToEditor, EditorToAdapter};
use elcarax_adapter_sdk::ElcaraxAdapter;
use elcarax_core::Result;

pub struct InProcessAdapterHost<A> {
    adapter: A,
    message_count: u64,
}

impl<A> InProcessAdapterHost<A>
where
    A: ElcaraxAdapter,
{
    pub fn new(adapter: A) -> Self {
        Self {
            adapter,
            message_count: 0,
        }
    }

    pub fn send(&mut self, message: EditorToAdapter) -> Result<AdapterToEditor> {
        self.message_count += 1;
        self.adapter.handle_message(message)
    }

    pub fn message_count(&self) -> u64 {
        self.message_count
    }
}

pub enum AdapterHealth {
    Starting,
    Ready,
    Failed(String),
    Stopped,
}
