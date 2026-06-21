//! GPU boundary types for Elcarax.

use std::{error::Error, fmt, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackendPreference {
    Auto,
    Vulkan,
    Metal,
    Direct3D12,
    OpenGl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuContextSpec {
    pub backend: GpuBackendPreference,
    pub enable_timestamps: bool,
    pub enable_validation: bool,
}

impl GpuContextSpec {
    pub const fn editor_default() -> Self {
        Self {
            backend: GpuBackendPreference::Auto,
            enable_timestamps: true,
            enable_validation: cfg!(debug_assertions),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClearColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl ClearColor {
    pub const ELCARAX_DARK: Self = Self {
        red: 0.035,
        green: 0.039,
        blue: 0.055,
        alpha: 1.0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
}

impl SurfaceSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    pub const fn is_drawable(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    pub cpu_frame_ms: f32,
    pub gpu_frame_ms: Option<f32>,
    pub uploaded_bytes: u64,
}

impl FrameStats {
    pub const fn empty() -> Self {
        Self {
            cpu_frame_ms: 0.0,
            gpu_frame_ms: None,
            uploaded_bytes: 0,
        }
    }
}

#[derive(Debug)]
pub enum RenderError {
    AdapterUnavailable,
    Device(String),
    Surface(String),
    SurfaceLost,
    OutOfMemory,
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterUnavailable => write!(formatter, "no compatible GPU adapter found"),
            Self::Device(message) => write!(formatter, "GPU device error: {message}"),
            Self::Surface(message) => write!(formatter, "GPU surface error: {message}"),
            Self::SurfaceLost => write!(formatter, "GPU surface was lost"),
            Self::OutOfMemory => write!(formatter, "GPU is out of memory"),
        }
    }
}

impl Error for RenderError {}

pub struct GpuContext {
    instance: wgpu::Instance,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl GpuContext {
    pub fn device(&self) -> Arc<wgpu::Device> {
        Arc::clone(&self.device)
    }
    pub fn queue(&self) -> Arc<wgpu::Queue> {
        Arc::clone(&self.queue)
    }
}

pub struct GpuSurface<'window> {
    surface: wgpu::Surface<'window>,
    config: wgpu::SurfaceConfiguration,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

pub struct GpuFrame;

impl GpuContext {
    pub async fn for_window(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        size: SurfaceSize,
        spec: &GpuContextSpec,
    ) -> Result<(Self, GpuSurface<'static>), RenderError> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backends(spec.backend);
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance
            .create_surface(window)
            .map_err(|error| RenderError::Surface(error.to_string()))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| RenderError::AdapterUnavailable)?;
        let features = if spec.enable_timestamps {
            wgpu::Features::TIMESTAMP_QUERY
        } else {
            wgpu::Features::empty()
        } & adapter.features();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Elcarax GPU Device"),
                required_features: features,
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| RenderError::Device(error.to_string()))?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let config = surface_config(&surface, &adapter, size)?;
        if size.is_drawable() {
            surface.configure(&device, &config);
        }
        Ok((
            Self {
                instance,
                device: Arc::clone(&device),
                queue: Arc::clone(&queue),
            },
            GpuSurface {
                surface,
                config,
                device,
                queue,
            },
        ))
    }

    pub fn keep_alive(&self) {
        let _ = &self.instance;
    }
}

impl GpuSurface<'_> {
    pub fn device(&self) -> Arc<wgpu::Device> {
        Arc::clone(&self.device)
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn size(&self) -> SurfaceSize {
        SurfaceSize::new(self.config.width, self.config.height)
    }

    pub fn resize(&mut self, size: SurfaceSize) {
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        if size.is_drawable() {
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render_clear(&mut self, color: ClearColor) -> Result<GpuFrame, RenderError> {
        self.render_with_clear(color, "Elcarax Clear Pass", |_encoder, _view| {})
    }

    pub fn render_with_clear(
        &mut self,
        color: ClearColor,
        label: &'static str,
        draw: impl FnOnce(&mut wgpu::CommandEncoder, &wgpu::TextureView),
    ) -> Result<GpuFrame, RenderError> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                return Err(RenderError::SurfaceLost);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Err(RenderError::Surface(
                    "surface is temporarily unavailable".to_owned(),
                ));
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::Surface("surface validation failed".to_owned()));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: color.red,
                            g: color.green,
                            b: color.blue,
                            a: color.alpha,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        draw(&mut encoder, &view);
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(GpuFrame)
    }
}

fn surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    size: SurfaceSize,
) -> Result<wgpu::SurfaceConfiguration, RenderError> {
    let capabilities = surface.get_capabilities(adapter);
    let Some(format) = capabilities.formats.first().copied() else {
        return Err(RenderError::Surface(
            "surface has no supported formats".to_owned(),
        ));
    };
    let present_mode = capabilities
        .present_modes
        .iter()
        .copied()
        .find(|mode| *mode == wgpu::PresentMode::Fifo)
        .unwrap_or(wgpu::PresentMode::AutoVsync);
    let alpha_mode = capabilities
        .alpha_modes
        .first()
        .copied()
        .unwrap_or(wgpu::CompositeAlphaMode::Auto);
    Ok(wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode,
        desired_maximum_frame_latency: 2,
        alpha_mode,
        view_formats: vec![],
    })
}

fn backends(preference: GpuBackendPreference) -> wgpu::Backends {
    match preference {
        GpuBackendPreference::Auto => wgpu::Backends::PRIMARY,
        GpuBackendPreference::Vulkan => wgpu::Backends::VULKAN,
        GpuBackendPreference::Metal => wgpu::Backends::METAL,
        GpuBackendPreference::Direct3D12 => wgpu::Backends::DX12,
        GpuBackendPreference::OpenGl => wgpu::Backends::GL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_clear_color_is_opaque() {
        assert_eq!(ClearColor::ELCARAX_DARK.alpha, 1.0);
    }
    #[test]
    fn zero_surface_is_not_drawable() {
        assert!(!SurfaceSize::new(0, 100).is_drawable());
    }
}
