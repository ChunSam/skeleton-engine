use std::sync::Arc;
use winit::{dpi::PhysicalSize, window::Window};

/// Reason for GPU initialization failure.
#[derive(Debug)]
pub enum GpuContextError {
    Surface(wgpu::CreateSurfaceError),
    AdapterNotFound,
    Device(wgpu::RequestDeviceError),
    NoSurfaceFormat,
    NoAlphaMode,
}

impl std::fmt::Display for GpuContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Surface(err) => write!(f, "surface creation failed: {err}"),
            Self::AdapterNotFound => write!(f, "no compatible GPU adapter found"),
            Self::Device(err) => write!(f, "device creation failed: {err}"),
            Self::NoSurfaceFormat => write!(f, "surface reported no supported formats"),
            Self::NoAlphaMode => write!(f, "surface reported no supported alpha modes"),
        }
    }
}

impl std::error::Error for GpuContextError {}

/// GPU context bundling the core wgpu objects.
pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
}

impl GpuContext {
    /// Initializes a wgpu Surface/Device/Queue from the given window.
    /// wgpu init is async; wrap with `pollster::block_on` when calling from sync code.
    pub async fn new(window: Arc<Window>) -> Result<Self, GpuContextError> {
        // WASM: inner_size() can return 1×1 immediately after winit attaches the canvas.
        // Read the canvas width/height attributes directly from the DOM for the real size.
        #[cfg(not(target_arch = "wasm32"))]
        let size = window.inner_size();
        #[cfg(target_arch = "wasm32")]
        let size = {
            use wasm_bindgen::JsCast;
            web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("game-canvas"))
                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                .map(|c| winit::dpi::PhysicalSize::new(c.width().max(1), c.height().max(1)))
                .unwrap_or_else(|| window.inner_size())
        };

        // 1. Instance: select backend per platform.
        // WASM: WebGPU adapter rejects unsupported limits like maxInterStageShaderComponents,
        //       so we force the WebGL2 backend (GL). Enabled via the "webgl" feature in Cargo.toml.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        // 2. Surface: render target tied to the window.
        let surface = instance
            .create_surface(window)
            .map_err(GpuContextError::Surface)?;

        // 3. Adapter: select the physical GPU.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| GpuContextError::AdapterNotFound)?;

        // 4. Logical device + command queue.
        // wgpu 22 added the memory_hints field to DeviceDescriptor.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("main device"),
                required_features: wgpu::Features::empty(),
                required_limits: {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        wgpu::Limits::default()
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    }
                },
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(GpuContextError::Device)?;

        // 5. Surface format and configuration.
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or(GpuContextError::NoSurfaceFormat)?;
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .ok_or(GpuContextError::NoAlphaMode)?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            // AutoVsync + frame_latency=1: minimizes frame queuing without tearing.
            // AutoNoVsync (low latency) was tested but the latency gain was marginal while
            // frames ran unbounded (battery/heat), making it unsuitable as a default.
            // Remaining input latency on macOS (event loop stalling during live window
            // drag, etc.) is deferred to a follow-up optimization.
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
        })
    }

    /// Reconfigures the surface when the window is resized.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Reconfigures the surface after it is lost (`SurfaceError::Lost`).
    pub fn reconfigure(&self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// Clears the screen to a solid color. Used to show the background when there are no sprites.
    pub fn clear(&mut self, color: wgpu::Color) -> Result<(), String> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            e => return Err(format!("{e:?}")),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear"),
            });
        {
            let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }
        self.queue.submit(std::iter::once(enc.finish()));
        frame.present();
        Ok(())
    }
}
