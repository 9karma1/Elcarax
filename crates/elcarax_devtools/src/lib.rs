//! Internal performance and debug surfaces for Elcarax.

use elcarax_gpu::FrameStats;
use elcarax_render::RenderStats;

#[derive(Debug, Clone, PartialEq)]
pub struct DevtoolsSnapshot {
    pub frame: FrameStats,
    pub render: RenderStats,
    pub adapter_messages: u64,
}

impl DevtoolsSnapshot {
    pub fn summary(&self) -> String {
        format!(
            "cpu={:.2}ms gpu={:?} primitives={} batches={} text_primitives={} glyphs={} adapter_messages={}",
            self.frame.cpu_frame_ms,
            self.frame.gpu_frame_ms,
            self.render.primitive_count,
            self.render.batch_count,
            self.render.text_primitive_count,
            self.render.glyph_count,
            self.adapter_messages
        )
    }
}
