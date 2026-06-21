//! Editor-specific render primitive model.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub const fn srgb(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextPrimitive {
    pub content: String,
    pub position: [f32; 2],
    pub size: f32,
    pub color: ColorRgba,
    pub clip: Option<Rect>,
    pub layer: i32,
    pub debug_label: Option<String>,
}

impl TextPrimitive {
    #[must_use]
    pub fn new(content: impl Into<String>, x: f32, y: f32, size: f32, color: ColorRgba) -> Self {
        Self {
            content: content.into(),
            position: [x, y],
            size,
            color,
            clip: None,
            layer: 0,
            debug_label: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderPrimitive {
    SolidRect {
        rect: Rect,
        color: ColorRgba,
    },
    RoundedRect {
        rect: Rect,
        radius: f32,
        color: ColorRgba,
    },
    Border {
        rect: Rect,
        width: f32,
        color: ColorRgba,
    },
    Text(TextPrimitive),
    Line {
        from: [f32; 2],
        to: [f32; 2],
        width: f32,
        color: ColorRgba,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderStats {
    pub primitive_count: usize,
    pub batch_count: usize,
    pub text_primitive_count: usize,
    pub glyph_count: usize,
    pub glyph_atlas_upload_bytes: u64,
    pub glyph_cache_hits: u64,
    pub glyph_cache_misses: u64,
    pub shaped_text_cache_hits: u64,
    pub shaped_text_cache_misses: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PrimitiveList {
    primitives: Vec<RenderPrimitive>,
}

impl PrimitiveList {
    pub fn push(&mut self, primitive: RenderPrimitive) {
        self.primitives.push(primitive);
    }

    pub fn stats(&self) -> RenderStats {
        let text_primitive_count = self
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::Text(_)))
            .count();
        let glyph_count = self
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Text(text) => Some(text.content.chars().count()),
                _ => None,
            })
            .sum();
        RenderStats {
            primitive_count: self.primitives.len(),
            batch_count: usize::from(!self.primitives.is_empty()),
            text_primitive_count,
            glyph_count,
            glyph_atlas_upload_bytes: 0,
            glyph_cache_hits: 0,
            glyph_cache_misses: glyph_count as u64,
            shaped_text_cache_hits: 0,
            shaped_text_cache_misses: text_primitive_count as u64,
        }
    }

    pub fn primitives(&self) -> &[RenderPrimitive] {
        &self.primitives
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlyphAtlasStats {
    pub glyphs: usize,
    pub upload_bytes: u64,
    pub hits: u64,
    pub misses: u64,
}

#[derive(Debug, Default)]
pub struct GlyphAtlas {
    stats: GlyphAtlasStats,
}
impl GlyphAtlas {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn reserve_glyphs(&mut self, glyph_count: usize) {
        self.stats.misses += glyph_count as u64;
        self.stats.glyphs += glyph_count;
        self.stats.upload_bytes += (glyph_count as u64) * 64;
    }
    #[must_use]
    pub fn stats(&self) -> GlyphAtlasStats {
        self.stats
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBatch {
    pub glyph_count: usize,
    pub layer: i32,
    pub clip: Option<Rect>,
}

#[must_use]
pub fn build_text_batches(primitives: &[RenderPrimitive]) -> Vec<TextBatch> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text(text) if !text.content.is_empty() => Some(TextBatch {
                glyph_count: text.content.chars().count(),
                layer: text.layer,
                clip: text.clip,
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn color() -> ColorRgba {
        ColorRgba::srgb(1.0, 1.0, 1.0, 1.0)
    }
    #[test]
    fn text_primitives_keep_stable_order() {
        let mut list = PrimitiveList::default();
        list.push(RenderPrimitive::Text(TextPrimitive::new(
            "A",
            0.0,
            0.0,
            14.0,
            color(),
        )));
        list.push(RenderPrimitive::SolidRect {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            color: color(),
        });
        assert!(
            matches!(&list.primitives()[0], RenderPrimitive::Text(text) if text.content == "A")
        );
    }
    #[test]
    fn text_contributes_to_stats() {
        let mut list = PrimitiveList::default();
        list.push(RenderPrimitive::Text(TextPrimitive::new(
            "abc",
            0.0,
            0.0,
            14.0,
            color(),
        )));
        let stats = list.stats();
        assert_eq!(stats.text_primitive_count, 1);
        assert_eq!(stats.glyph_count, 3);
    }
    #[test]
    fn empty_text_has_no_batch() {
        let mut list = PrimitiveList::default();
        list.push(RenderPrimitive::Text(TextPrimitive::new(
            "",
            0.0,
            0.0,
            14.0,
            color(),
        )));
        assert!(build_text_batches(list.primitives()).is_empty());
    }
    #[test]
    fn clipping_metadata_is_preserved() {
        let mut text = TextPrimitive::new("clip", 0.0, 0.0, 14.0, color());
        text.clip = Some(Rect {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        });
        let batches = build_text_batches(&[RenderPrimitive::Text(text)]);
        assert_eq!(
            batches.first().and_then(|b| b.clip),
            Some(Rect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0
            })
        );
    }
}
