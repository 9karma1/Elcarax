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
pub enum RenderPrimitive {
    SolidRect { rect: Rect, color: ColorRgba },
    RoundedRect { rect: Rect, radius: f32, color: ColorRgba },
    Border { rect: Rect, width: f32, color: ColorRgba },
    TextRun { origin_x: f32, origin_y: f32, text: String, color: ColorRgba },
    Line { from: [f32; 2], to: [f32; 2], width: f32, color: ColorRgba },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderStats {
    pub primitive_count: usize,
    pub batch_count: usize,
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
        RenderStats {
            primitive_count: self.primitives.len(),
            batch_count: self.primitives.len().min(1),
        }
    }

    pub fn primitives(&self) -> &[RenderPrimitive] {
        &self.primitives
    }
}
