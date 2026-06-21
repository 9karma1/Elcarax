//! Editor-specific GPU-backed render primitive pipeline.

use std::{
    error::Error,
    fmt,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use elcarax_gpu::{ClearColor, GpuContext, GpuSurface, RenderError, SurfaceSize};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub const ELCARAX_DARK: Self = Self::srgb(0.035, 0.039, 0.055, 1.0);
    pub const fn srgb(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub fn normalized(self) -> Self {
        Self {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn normalized(self) -> Self {
        let (x, width) = if self.width < 0.0 {
            (self.x + self.width, -self.width)
        } else {
            (self.x, self.width)
        };
        let (y, height) = if self.height < 0.0 {
            (self.y + self.height, -self.height)
        } else {
            (self.y, self.height)
        };
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn is_visible(self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}
impl CornerRadius {
    pub const ZERO: Self = Self::uniform(0.0);
    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
    pub fn normalized(self) -> Self {
        Self {
            top_left: self.top_left.max(0.0),
            top_right: self.top_right.max(0.0),
            bottom_right: self.bottom_right.max(0.0),
            bottom_left: self.bottom_left.max(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f32,
    pub color: Color,
    pub radius: CornerRadius,
}
impl Border {
    pub fn new(width: f32, color: Color) -> Self {
        Self {
            width: width.max(0.0),
            color: color.normalized(),
            radius: CornerRadius::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipRect {
    pub rect: Rect,
}
impl ClipRect {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect: rect.normalized(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextPrimitive {
    pub content: String,
    pub position: [f32; 2],
    pub size: f32,
    pub color: Color,
}
impl TextPrimitive {
    #[must_use]
    pub fn new(content: impl Into<String>, x: f32, y: f32, size: f32, color: Color) -> Self {
        Self {
            content: content.into(),
            position: [x, y],
            size,
            color: color.normalized(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderPrimitiveKind {
    SolidRect {
        rect: Rect,
        color: Color,
    },
    RoundedRect {
        rect: Rect,
        radius: CornerRadius,
        color: Color,
    },
    BorderRect {
        rect: Rect,
        border: Border,
    },
    Text(TextPrimitive),
    Line {
        from: [f32; 2],
        to: [f32; 2],
        width: f32,
        color: Color,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPrimitive {
    pub kind: RenderPrimitiveKind,
    pub clip: Option<ClipRect>,
    pub debug_label: Option<String>,
}
impl RenderPrimitive {
    pub fn solid_rect(rect: Rect, color: Color) -> Self {
        Self::new(RenderPrimitiveKind::SolidRect {
            rect: rect.normalized(),
            color: color.normalized(),
        })
    }
    pub fn rounded_rect(rect: Rect, radius: CornerRadius, color: Color) -> Self {
        Self::new(RenderPrimitiveKind::RoundedRect {
            rect: rect.normalized(),
            radius: radius.normalized(),
            color: color.normalized(),
        })
    }
    pub fn border_rect(rect: Rect, border: Border) -> Self {
        Self::new(RenderPrimitiveKind::BorderRect {
            rect: rect.normalized(),
            border,
        })
    }
    pub fn text(content: impl Into<String>, x: f32, y: f32, size: f32, color: Color) -> Self {
        Self::new(RenderPrimitiveKind::Text(TextPrimitive::new(
            content, x, y, size, color,
        )))
    }
    pub fn line(from: [f32; 2], to: [f32; 2], width: f32, color: Color) -> Self {
        Self::new(RenderPrimitiveKind::Line {
            from,
            to,
            width: width.max(0.0),
            color: color.normalized(),
        })
    }
    pub fn with_clip(mut self, clip: ClipRect) -> Self {
        self.clip = Some(clip);
        self
    }
    pub fn with_debug_label(mut self, label: impl Into<String>) -> Self {
        self.debug_label = Some(label.into());
        self
    }
    fn new(kind: RenderPrimitiveKind) -> Self {
        Self {
            kind,
            clip: None,
            debug_label: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderLayer {
    Background,
    Chrome,
    Overlay,
    Debug,
}

#[derive(Debug, Clone, Default)]
pub struct RenderScene {
    primitives: Vec<(RenderLayer, RenderPrimitive)>,
}
impl RenderScene {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, layer: RenderLayer, primitive: RenderPrimitive) {
        self.primitives.push((layer, primitive));
    }
    pub fn primitives(&self) -> &[(RenderLayer, RenderPrimitive)] {
        &self.primitives
    }
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderBatch {
    pub layer: RenderLayer,
    pub clip: Option<ClipRect>,
    pub primitive_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
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
    pub uploaded_bytes: u64,
    pub frame_count: u64,
    pub last_frame_duration: Option<Duration>,
}

#[derive(Debug, Clone, Copy)]
pub struct RendererConfig {
    pub clear_color: Color,
}
impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            clear_color: Color::ELCARAX_DARK,
        }
    }
}

#[derive(Debug)]
pub enum RendererError {
    Gpu(RenderError),
}
impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gpu(e) => write!(f, "renderer GPU error: {e}"),
        }
    }
}
impl Error for RendererError {}
impl From<RenderError> for RendererError {
    fn from(value: RenderError) -> Self {
        Self::Gpu(value)
    }
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    config: RendererConfig,
    stats: RenderStats,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RectInstance {
    rect: [f32; 4],
    color: [f32; 4],
}

impl Renderer {
    pub fn new(
        context: &GpuContext,
        surface: &GpuSurface<'_>,
        config: RendererConfig,
    ) -> Result<Self, RendererError> {
        let device = context.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Elcarax Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("rect.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Elcarax Rect Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Elcarax Rect Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[vertex_layout(), instance_layout()],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface.format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let vertices = [
            Vertex {
                position: [0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
        ];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Elcarax Rect Unit Quad"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Ok(Self {
            pipeline,
            vertex_buffer,
            config,
            stats: RenderStats::default(),
        })
    }
    pub fn stats(&self) -> RenderStats {
        self.stats
    }
    pub fn render(
        &mut self,
        surface: &mut GpuSurface<'_>,
        scene: &RenderScene,
    ) -> Result<(), RendererError> {
        let started = Instant::now();
        let batches = batch_scene(scene);
        let instances = collect_rect_instances(scene, surface.size());
        let uploaded_bytes = (instances.len() * std::mem::size_of::<RectInstance>()) as u64;
        let device = surface.device();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Elcarax Rect Instances"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let clear = ClearColor {
            red: self.config.clear_color.r as f64,
            green: self.config.clear_color.g as f64,
            blue: self.config.clear_color.b as f64,
            alpha: self.config.clear_color.a as f64,
        };
        surface.render_with_clear(clear, "Elcarax Render Pass", |encoder, view| {
            if instances.is_empty() {
                return;
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Elcarax Primitive Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, instance_buffer.slice(..));
            pass.draw(0..4, 0..instances.len() as u32);
        })?;
        let text_stats = text_stats(scene);
        self.stats = RenderStats {
            primitive_count: scene.primitives.len(),
            batch_count: batches.len(),
            text_primitive_count: text_stats.text_primitive_count,
            glyph_count: text_stats.glyph_count,
            glyph_atlas_upload_bytes: text_stats.glyph_atlas_upload_bytes,
            glyph_cache_hits: text_stats.glyph_cache_hits,
            glyph_cache_misses: text_stats.glyph_cache_misses,
            shaped_text_cache_hits: text_stats.shaped_text_cache_hits,
            shaped_text_cache_misses: text_stats.shaped_text_cache_misses,
            uploaded_bytes,
            frame_count: self.stats.frame_count.saturating_add(1),
            last_frame_duration: Some(started.elapsed()),
        };
        Ok(())
    }
}

pub fn batch_scene(scene: &RenderScene) -> Vec<RenderBatch> {
    let mut batches: Vec<RenderBatch> = Vec::new();
    for (layer, primitive) in &scene.primitives {
        if let Some(last) = batches.last_mut()
            && last.layer == *layer
            && last.clip == primitive.clip
        {
            last.primitive_count += 1;
            continue;
        }
        batches.push(RenderBatch {
            layer: *layer,
            clip: primitive.clip,
            primitive_count: 1,
        });
    }
    batches
}

fn collect_rect_instances(scene: &RenderScene, size: SurfaceSize) -> Vec<RectInstance> {
    scene
        .primitives
        .iter()
        .flat_map(|(_, p)| primitive_instances(p, size))
        .collect()
}
fn primitive_instances(primitive: &RenderPrimitive, size: SurfaceSize) -> Vec<RectInstance> {
    match primitive.kind {
        RenderPrimitiveKind::SolidRect { rect, color }
        | RenderPrimitiveKind::RoundedRect { rect, color, .. } => {
            rect_instance(rect, color, size).into_iter().collect()
        }
        RenderPrimitiveKind::BorderRect { rect, border } => border_instances(rect, border, size),
        RenderPrimitiveKind::Text(_) => Vec::new(),
        RenderPrimitiveKind::Line {
            from,
            to,
            width,
            color,
        } => line_instance(from, to, width, color, size)
            .into_iter()
            .collect(),
    }
}
fn rect_instance(rect: Rect, color: Color, size: SurfaceSize) -> Option<RectInstance> {
    let r = rect.normalized();
    if !r.is_visible() {
        return None;
    }
    Some(RectInstance {
        rect: [
            to_ndc_x(r.x, size),
            to_ndc_y(r.y, size),
            r.width * 2.0 / size.width.max(1) as f32,
            r.height * -2.0 / size.height.max(1) as f32,
        ],
        color: [color.r, color.g, color.b, color.a],
    })
}
fn border_instances(rect: Rect, border: Border, size: SurfaceSize) -> Vec<RectInstance> {
    let r = rect.normalized();
    let w = border.width.min(r.width / 2.0).min(r.height / 2.0);
    if w <= 0.0 {
        return Vec::new();
    }
    [
        Rect::new(r.x, r.y, r.width, w),
        Rect::new(r.x, r.y + r.height - w, r.width, w),
        Rect::new(r.x, r.y, w, r.height),
        Rect::new(r.x + r.width - w, r.y, w, r.height),
    ]
    .into_iter()
    .filter_map(|part| rect_instance(part, border.color, size))
    .collect()
}
fn line_instance(
    from: [f32; 2],
    to: [f32; 2],
    width: f32,
    color: Color,
    size: SurfaceSize,
) -> Option<RectInstance> {
    if (from[0] - to[0]).abs() < f32::EPSILON {
        rect_instance(
            Rect::new(
                from[0] - width / 2.0,
                from[1].min(to[1]),
                width,
                (from[1] - to[1]).abs(),
            ),
            color,
            size,
        )
    } else {
        rect_instance(
            Rect::new(
                from[0].min(to[0]),
                from[1] - width / 2.0,
                (from[0] - to[0]).abs(),
                width,
            ),
            color,
            size,
        )
    }
}
fn to_ndc_x(x: f32, size: SurfaceSize) -> f32 {
    (x / size.width.max(1) as f32) * 2.0 - 1.0
}
fn to_ndc_y(y: f32, size: SurfaceSize) -> f32 {
    1.0 - (y / size.height.max(1) as f32) * 2.0
}
fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}
fn instance_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![1 => Float32x4, 2 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<RectInstance>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
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
    pub layer: RenderLayer,
    pub clip: Option<ClipRect>,
}

#[must_use]
pub fn build_text_batches(scene: &RenderScene) -> Vec<TextBatch> {
    scene
        .primitives
        .iter()
        .filter_map(|(layer, primitive)| match &primitive.kind {
            RenderPrimitiveKind::Text(text) if !text.content.is_empty() => Some(TextBatch {
                glyph_count: text.content.chars().count(),
                layer: *layer,
                clip: primitive.clip,
            }),
            _ => None,
        })
        .collect()
}

#[must_use]
pub fn text_stats(scene: &RenderScene) -> RenderStats {
    let batches = build_text_batches(scene);
    let glyph_count = batches.iter().map(|batch| batch.glyph_count).sum();
    RenderStats {
        primitive_count: scene.primitives.len(),
        batch_count: usize::from(!batches.is_empty()),
        text_primitive_count: batches.len(),
        glyph_count,
        glyph_cache_misses: glyph_count as u64,
        shaped_text_cache_misses: batches.len() as u64,
        ..RenderStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene_keeps_stable_order() {
        let mut s = RenderScene::new();
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::solid_rect(
                Rect::new(0.0, 0.0, 1.0, 1.0),
                Color::srgb(1.0, 0.0, 0.0, 1.0),
            )
            .with_debug_label("first"),
        );
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::line([0.0, 0.0], [1.0, 0.0], 1.0, Color::srgb(0.0, 1.0, 0.0, 1.0))
                .with_debug_label("second"),
        );
        assert_eq!(s.primitives()[0].1.debug_label.as_deref(), Some("first"));
        assert_eq!(s.primitives()[1].1.debug_label.as_deref(), Some("second"));
    }
    #[test]
    fn batching_groups_compatible_neighbors() {
        let mut s = RenderScene::new();
        let c = Color::srgb(1.0, 1.0, 1.0, 1.0);
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::solid_rect(Rect::new(0.0, 0.0, 1.0, 1.0), c),
        );
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::solid_rect(Rect::new(1.0, 0.0, 1.0, 1.0), c),
        );
        s.push(
            RenderLayer::Overlay,
            RenderPrimitive::solid_rect(Rect::new(2.0, 0.0, 1.0, 1.0), c),
        );
        let b = batch_scene(&s);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].primitive_count, 2);
    }
    #[test]
    fn empty_scene_has_zero_batches() {
        assert!(batch_scene(&RenderScene::new()).is_empty());
    }
    #[test]
    fn primitive_and_batch_counts_are_correct() {
        let mut s = RenderScene::new();
        let c = Color::srgb(1.0, 1.0, 1.0, 1.0);
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::solid_rect(Rect::new(0.0, 0.0, 1.0, 1.0), c),
        );
        assert_eq!(s.primitives().len(), 1);
        assert_eq!(batch_scene(&s).len(), 1);
    }
    #[test]
    fn color_and_rect_normalize() {
        assert_eq!(
            Color::srgb(2.0, -1.0, 0.5, 3.0).normalized(),
            Color::srgb(1.0, 0.0, 0.5, 1.0)
        );
        assert_eq!(
            Rect::new(10.0, 5.0, -2.0, -3.0).normalized(),
            Rect::new(8.0, 2.0, 2.0, 3.0)
        );
    }
    #[test]
    fn clipping_data_attaches_to_primitive() {
        let p = RenderPrimitive::solid_rect(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Color::srgb(1.0, 1.0, 1.0, 1.0),
        )
        .with_clip(ClipRect::new(Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert!(p.clip.is_some());
    }

    #[test]
    fn text_primitives_keep_stable_order() {
        let mut s = RenderScene::new();
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::text("A", 0.0, 0.0, 14.0, Color::srgb(1.0, 1.0, 1.0, 1.0)),
        );
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::solid_rect(
                Rect::new(0.0, 0.0, 1.0, 1.0),
                Color::srgb(1.0, 1.0, 1.0, 1.0),
            ),
        );
        assert!(
            matches!(&s.primitives()[0].1.kind, RenderPrimitiveKind::Text(text) if text.content == "A")
        );
    }

    #[test]
    fn text_contributes_to_stats() {
        let mut s = RenderScene::new();
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::text("abc", 0.0, 0.0, 14.0, Color::srgb(1.0, 1.0, 1.0, 1.0)),
        );
        let stats = text_stats(&s);
        assert_eq!(stats.text_primitive_count, 1);
        assert_eq!(stats.glyph_count, 3);
    }

    #[test]
    fn empty_text_has_no_batch() {
        let mut s = RenderScene::new();
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::text("", 0.0, 0.0, 14.0, Color::srgb(1.0, 1.0, 1.0, 1.0)),
        );
        assert!(build_text_batches(&s).is_empty());
    }

    #[test]
    fn text_clipping_metadata_is_preserved() {
        let clip = ClipRect::new(Rect::new(1.0, 2.0, 3.0, 4.0));
        let mut s = RenderScene::new();
        s.push(
            RenderLayer::Chrome,
            RenderPrimitive::text("clip", 0.0, 0.0, 14.0, Color::srgb(1.0, 1.0, 1.0, 1.0))
                .with_clip(clip),
        );
        let batches = build_text_batches(&s);
        assert_eq!(batches.first().and_then(|b| b.clip), Some(clip));
    }
}
