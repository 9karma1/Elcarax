//! Viewport camera, letterboxed frame layout, and normalized pointer coordinates.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ViewportRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn is_visible(self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }

    pub fn center(self) -> (f32, f32) {
        (self.x + self.width * 0.5, self.y + self.height * 0.5)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportCamera {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

impl ViewportCamera {
    pub const MIN_ZOOM: f32 = 0.1;
    pub const MAX_ZOOM: f32 = 16.0;
    pub const DEFAULT_ZOOM: f32 = 1.0;

    pub const fn default_editor() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: Self::DEFAULT_ZOOM,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default_editor();
    }

    pub fn pan_by(&mut self, delta_x: f32, delta_y: f32) {
        self.pan_x += delta_x;
        self.pan_y += delta_y;
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    pub fn zoom_percent(self) -> u32 {
        (self.zoom * 100.0).round() as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportFramePlacement {
    pub content: ViewportRect,
    pub fitted: ViewportRect,
    pub displayed: ViewportRect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedViewportCoord {
    pub u: f32,
    pub v: f32,
}

pub fn fit_frame_rect(
    content: ViewportRect,
    frame_width: u32,
    frame_height: u32,
) -> Option<ViewportRect> {
    if !content.is_visible() || frame_width == 0 || frame_height == 0 {
        return None;
    }
    let frame_aspect = frame_width as f32 / frame_height as f32;
    let content_aspect = content.width / content.height;
    let (width, height) = if frame_aspect > content_aspect {
        let width = content.width;
        let height = width / frame_aspect;
        (width, height)
    } else {
        let height = content.height;
        let width = height * frame_aspect;
        (width, height)
    };
    Some(ViewportRect::new(
        content.x + (content.width - width) * 0.5,
        content.y + (content.height - height) * 0.5,
        width,
        height,
    ))
}

pub fn apply_camera_to_rect(
    fitted: ViewportRect,
    content: ViewportRect,
    camera: &ViewportCamera,
) -> ViewportRect {
    let (center_x, center_y) = fitted.center();
    let width = fitted.width * camera.zoom;
    let height = fitted.height * camera.zoom;
    let x = center_x - width * 0.5 + camera.pan_x;
    let y = center_y - height * 0.5 + camera.pan_y;
    let mut displayed = ViewportRect::new(x, y, width, height);
    displayed = clamp_rect_to_content(displayed, content);
    displayed
}

pub fn layout_viewport_frame(
    content: ViewportRect,
    frame_width: u32,
    frame_height: u32,
    camera: &ViewportCamera,
) -> Option<ViewportFramePlacement> {
    let fitted = fit_frame_rect(content, frame_width, frame_height)?;
    let displayed = apply_camera_to_rect(fitted, content, camera);
    Some(ViewportFramePlacement {
        content,
        fitted,
        displayed,
    })
}

pub fn pointer_to_frame_uv(
    pointer_x: f32,
    pointer_y: f32,
    placement: &ViewportFramePlacement,
) -> Option<NormalizedViewportCoord> {
    let rect = placement.displayed;
    if !rect.is_visible() {
        return None;
    }
    if pointer_x < rect.x
        || pointer_y < rect.y
        || pointer_x >= rect.x + rect.width
        || pointer_y >= rect.y + rect.height
    {
        return None;
    }
    let u = (pointer_x - rect.x) / rect.width;
    let v = (pointer_y - rect.y) / rect.height;
    Some(NormalizedViewportCoord { u, v })
}

fn clamp_rect_to_content(mut rect: ViewportRect, content: ViewportRect) -> ViewportRect {
    if rect.width > content.width {
        rect.width = content.width;
    }
    if rect.height > content.height {
        rect.height = content.height;
    }
    if rect.x < content.x {
        rect.x = content.x;
    }
    if rect.y < content.y {
        rect.y = content.y;
    }
    if rect.x + rect.width > content.x + content.width {
        rect.x = content.x + content.width - rect.width;
    }
    if rect.y + rect.height > content.y + content.height {
        rect.y = content.y + content.height - rect.height;
    }
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_frame_preserves_aspect_ratio() {
        let content = ViewportRect::new(0.0, 0.0, 400.0, 200.0);
        let fitted = match fit_frame_rect(content, 128, 128) {
            Some(value) => value,
            None => panic!("fit should succeed"),
        };
        let aspect = fitted.width / fitted.height;
        assert!((aspect - 1.0).abs() < 0.01);
        assert!(fitted.width <= content.width);
        assert!(fitted.height <= content.height);
    }

    #[test]
    fn zoom_updates_camera_factor() {
        let mut camera = ViewportCamera::default_editor();
        camera.zoom_by(2.0);
        assert_eq!(camera.zoom, 2.0);
    }

    #[test]
    fn pointer_maps_to_frame_uv() {
        let placement = ViewportFramePlacement {
            content: ViewportRect::new(0.0, 0.0, 100.0, 100.0),
            fitted: ViewportRect::new(10.0, 10.0, 80.0, 80.0),
            displayed: ViewportRect::new(10.0, 10.0, 80.0, 80.0),
        };
        let uv = match pointer_to_frame_uv(50.0, 50.0, &placement) {
            Some(value) => value,
            None => panic!("pointer should map"),
        };
        assert!((uv.u - 0.5).abs() < 0.01);
        assert!((uv.v - 0.5).abs() < 0.01);
    }
}
