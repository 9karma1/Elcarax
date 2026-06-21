//! Static text layout boundary for Elcarax.

use std::{collections::BTreeMap, error::Error, fmt};

use cosmic_text::{Attrs, Buffer, Color as CosmicColor, Metrics, Shaping, SwashCache};

#[derive(Debug)]
pub struct FontSystem {
    inner: cosmic_text::FontSystem,
}

impl FontSystem {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: cosmic_text::FontSystem::new(),
        }
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FontSize(pub f32);
impl FontSize {
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl TextColor {
    #[must_use]
    pub const fn srgb(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    fn to_cosmic(self) -> CosmicColor {
        CosmicColor::rgba(
            channel_to_u8(self.r),
            channel_to_u8(self.g),
            channel_to_u8(self.b),
            channel_to_u8(self.a),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub content: String,
    pub family: FontFamily,
    pub size: FontSize,
    pub color: TextColor,
}
impl TextRun {
    #[must_use]
    pub fn new(content: impl Into<String>, size: FontSize, color: TextColor) -> Self {
        Self {
            content: content.into(),
            family: FontFamily::SansSerif,
            size,
            color,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphPlacement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub line_height: f32,
    pub glyph_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    pub run: TextRun,
    pub metrics: TextMetrics,
    pub glyphs: Vec<GlyphPlacement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextPixel {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub color: TextColor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RasterizedText {
    pub layout: TextLayout,
    pub pixels: Vec<TextPixel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextError {
    InvalidFontSize,
}
impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFontSize => write!(f, "font size must be finite and positive"),
        }
    }
}
impl Error for TextError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CacheKey {
    content: String,
    family: FontFamily,
    size_bits: u32,
    width_bits: Option<u32>,
}

#[derive(Debug, Default)]
pub struct TextLayoutCache {
    entries: BTreeMap<CacheKey, TextLayout>,
    hits: u64,
    misses: u64,
}
impl TextLayoutCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn layout(
        &mut self,
        fonts: &mut FontSystem,
        run: &TextRun,
        max_width: Option<f32>,
    ) -> Result<TextLayout, TextError> {
        let key = CacheKey {
            content: run.content.clone(),
            family: run.family.clone(),
            size_bits: run.size.0.to_bits(),
            width_bits: max_width.map(f32::to_bits),
        };
        if let Some(layout) = self.entries.get(&key) {
            self.hits += 1;
            return Ok(layout.clone());
        }
        self.misses += 1;
        let layout = shape_static_text(fonts, run, max_width)?;
        self.entries.insert(key, layout.clone());
        Ok(layout)
    }
    #[must_use]
    pub fn hits(&self) -> u64 {
        self.hits
    }
    #[must_use]
    pub fn misses(&self) -> u64 {
        self.misses
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug)]
pub struct TextRasterizer {
    fonts: FontSystem,
    swash: SwashCache,
    layouts: TextLayoutCache,
}

impl TextRasterizer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            fonts: FontSystem::new(),
            swash: SwashCache::new(),
            layouts: TextLayoutCache::new(),
        }
    }

    pub fn rasterize(
        &mut self,
        run: &TextRun,
        max_width: Option<f32>,
    ) -> Result<RasterizedText, TextError> {
        let layout = self.layouts.layout(&mut self.fonts, run, max_width)?;
        let pixels = rasterize_pixels(&mut self.fonts, &mut self.swash, run, max_width)?;
        Ok(RasterizedText { layout, pixels })
    }

    #[must_use]
    pub fn cache_hits(&self) -> u64 {
        self.layouts.hits()
    }

    #[must_use]
    pub fn cache_misses(&self) -> u64 {
        self.layouts.misses()
    }
}

impl Default for TextRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn shape_static_text(
    fonts: &mut FontSystem,
    run: &TextRun,
    max_width: Option<f32>,
) -> Result<TextLayout, TextError> {
    if !run.size.0.is_finite() || run.size.0 <= 0.0 {
        return Err(TextError::InvalidFontSize);
    }
    let line_height = (run.size.0 * 1.25).ceil();
    let mut buffer = Buffer::new(&mut fonts.inner, Metrics::new(run.size.0, line_height));
    let mut borrowed = buffer.borrow_with(&mut fonts.inner);
    borrowed.set_size(max_width, Some(line_height));
    borrowed.set_text(&run.content, &Attrs::new(), Shaping::Advanced, None);
    let mut glyphs = Vec::new();
    let mut width: f32 = 0.0;
    for layout_run in borrowed.layout_runs() {
        for glyph in layout_run.glyphs {
            let w = (run.size.0 * 0.6).ceil().max(1.0);
            let h = run.size.0.ceil().max(1.0);
            glyphs.push(GlyphPlacement {
                x: glyph.x,
                y: layout_run.line_y,
                width: w,
                height: h,
                alpha: 255,
            });
            width = width.max(glyph.x + w);
        }
    }
    Ok(TextLayout {
        run: run.clone(),
        metrics: TextMetrics {
            width,
            height: if run.content.is_empty() {
                0.0
            } else {
                line_height
            },
            line_height,
            glyph_count: glyphs.len(),
        },
        glyphs,
    })
}

fn rasterize_pixels(
    fonts: &mut FontSystem,
    swash: &mut SwashCache,
    run: &TextRun,
    max_width: Option<f32>,
) -> Result<Vec<TextPixel>, TextError> {
    if !run.size.0.is_finite() || run.size.0 <= 0.0 {
        return Err(TextError::InvalidFontSize);
    }
    let line_height = (run.size.0 * 1.25).ceil();
    let mut buffer = Buffer::new(&mut fonts.inner, Metrics::new(run.size.0, line_height));
    let mut borrowed = buffer.borrow_with(&mut fonts.inner);
    borrowed.set_size(max_width, Some(line_height));
    borrowed.set_text(&run.content, &Attrs::new(), Shaping::Advanced, None);
    let mut pixels = Vec::new();
    borrowed.draw(
        swash,
        run.color.to_cosmic(),
        |x, y, width, height, color| {
            if width == 0 || height == 0 || color.a() == 0 {
                return;
            }
            pixels.push(TextPixel {
                x,
                y,
                width,
                height,
                color: TextColor::srgb(
                    f32::from(color.r()) / 255.0,
                    f32::from(color.g()) / 255.0,
                    f32::from(color.b()) / 255.0,
                    f32::from(color.a()) / 255.0,
                ),
            });
        },
    );
    Ok(pixels)
}

fn channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(text: &str, size: f32) -> TextRun {
        TextRun::new(
            text,
            FontSize::new(size),
            TextColor::srgb(1.0, 1.0, 1.0, 1.0),
        )
    }
    #[test]
    fn ascii_metrics_are_stable() {
        let mut fs = FontSystem::new();
        let l = shape_static_text(&mut fs, &run("Elcarax", 16.0), None)
            .unwrap_or_else(|error| panic!("layout failed: {error}"));
        assert!(l.metrics.width > 0.0);
        assert_eq!(l.metrics.glyph_count, 7);
    }
    #[test]
    fn empty_layout_is_valid() {
        let mut fs = FontSystem::new();
        let l = shape_static_text(&mut fs, &run("", 16.0), None)
            .unwrap_or_else(|error| panic!("layout failed: {error}"));
        assert_eq!(l.metrics.glyph_count, 0);
    }
    #[test]
    fn cache_hits_repeated_input() {
        let mut fs = FontSystem::new();
        let mut c = TextLayoutCache::new();
        let r = run("Project", 14.0);
        let _ = c
            .layout(&mut fs, &r, None)
            .unwrap_or_else(|error| panic!("layout failed: {error}"));
        let _ = c
            .layout(&mut fs, &r, None)
            .unwrap_or_else(|error| panic!("layout failed: {error}"));
        assert_eq!(c.hits(), 1);
    }
    #[test]
    fn font_size_distinguishes_cache_entries() {
        let mut fs = FontSystem::new();
        let mut c = TextLayoutCache::new();
        let _ = c
            .layout(&mut fs, &run("Project", 14.0), None)
            .unwrap_or_else(|error| panic!("layout failed: {error}"));
        let _ = c
            .layout(&mut fs, &run("Project", 18.0), None)
            .unwrap_or_else(|error| panic!("layout failed: {error}"));
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn rasterizer_uses_system_font_pixels() {
        let mut rasterizer = TextRasterizer::new();
        let rasterized = rasterizer
            .rasterize(&run("Elcarax", 16.0), None)
            .unwrap_or_else(|error| panic!("rasterize failed: {error}"));
        assert_eq!(rasterized.layout.metrics.glyph_count, 7);
        assert!(!rasterized.pixels.is_empty());
    }
}
