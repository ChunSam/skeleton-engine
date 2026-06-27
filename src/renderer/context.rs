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
    /// The window surface (swapchain). `None` in **headless** mode (no window), where the
    /// final frame is rendered into [`headless_color`](Self::headless_color) instead — see
    /// [`GpuContext::new_headless`] and [`crate::App::save_screenshot_headless`].
    pub surface: Option<wgpu::Surface<'static>>,
    /// Offscreen final-frame color texture used in headless mode (`Some` iff `surface` is `None`).
    /// Format matches [`config`](Self::config)`.format`; usable as a render attachment and a copy
    /// source so the rendered frame can be read back to a PNG.
    pub headless_color: Option<wgpu::Texture>,
    /// The physical GPU adapter. Retained (not just used at init) so render-format
    /// capabilities can be queried at runtime — see [`GpuContext::supports_render_target`]
    /// and [`RenderCapabilities`].
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
}

/// Returns `true` if `format` can be used as a color render attachment on `adapter`'s GPU/backend.
///
/// Single source of truth shared by [`GpuContext::supports_render_target`] and
/// [`RenderCapabilities::supports_render_target`].
fn format_supports_render_target(adapter: &wgpu::Adapter, format: wgpu::TextureFormat) -> bool {
    adapter
        .get_texture_format_features(format)
        .allowed_usages
        .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
}

/// Pure render-target format decision (no GPU): the requested format when it is renderable,
/// otherwise the surface fallback. Extracted from [`GpuContext::resolve_render_target_format`]
/// so the fallback policy is unit-testable without a live GPU.
fn pick_render_target_format(
    requested: Option<wgpu::TextureFormat>,
    requested_is_renderable: bool,
    surface_format: wgpu::TextureFormat,
) -> wgpu::TextureFormat {
    match requested {
        Some(format) if requested_is_renderable => format,
        _ => surface_format,
    }
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
                .and_then(|d| d.get_element_by_id(crate::DEFAULT_CANVAS_ID))
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
            // AutoVsync (no tearing) + frame_latency=2: lets the GPU keep ~1 frame queued so
            // `get_current_texture()` does NOT block the main thread on vsync for most of each
            // frame. With latency=1 the main thread (= macOS AppKit run loop) was blocked the
            // full frame, which — combined with the old `Poll` busy-spin — starved input and
            // made window drags laggy. The remaining pacing is handled by WaitUntil in the
            // event loop (see `App::about_to_wait`). AutoNoVsync was rejected: marginal latency
            // gain but unbounded frames (battery/heat).
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            surface: Some(surface),
            headless_color: None,
            adapter,
            device,
            queue,
            config,
            size,
        })
    }

    /// Initializes a **headless** GPU context — no window, no surface, no display dependency.
    ///
    /// Renders into an offscreen `Rgba8UnormSrgb` color texture instead of a swapchain, so it
    /// works with the monitor off/asleep/locked and on a machine with no display attached. Drives
    /// [`crate::App::save_screenshot_headless`]; read the rendered frame back with
    /// [`read_headless_rgba`](Self::read_headless_rgba).
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn new_headless(width: u32, height: u32) -> Result<Self, GpuContextError> {
        let width = width.max(1);
        let height = height.max(1);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });
        // No surface: pass `compatible_surface: None` so the adapter is selected purely on the GPU.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| GpuContextError::AdapterNotFound)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("headless device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(GpuContextError::Device)?;

        // Rgba8UnormSrgb so the read-back bytes are directly RGBA8 (sRGB) for a PNG — no channel
        // swizzle, and an sRGB color target matching the usual surface encoding.
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        let headless_color = Self::create_headless_color(&device, format, width, height);

        Ok(Self {
            surface: None,
            headless_color: Some(headless_color),
            adapter,
            device,
            queue,
            config,
            size: PhysicalSize::new(width, height),
        })
    }

    /// Builds the offscreen final-frame texture (render attachment + copy source).
    #[cfg(not(target_arch = "wasm32"))]
    fn create_headless_color(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless final color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// Whether this context renders headlessly (no window/surface).
    pub fn is_headless(&self) -> bool {
        self.surface.is_none()
    }

    /// A view of the headless final-frame texture (the render target in headless mode).
    ///
    /// # Panics
    /// Panics if called on a windowed (non-headless) context.
    pub fn headless_view(&self) -> wgpu::TextureView {
        self.headless_color
            .as_ref()
            .expect("headless_view called on a windowed GpuContext")
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Returns `true` if `format` can be used as a color render attachment on this GPU/backend.
    ///
    /// Backs [`RenderCapabilities`] and the automatic render-target fallback. Most relevant on
    /// wasm/WebGL2, where float formats like `Rgba16Float` are renderable only when the
    /// `EXT_color_buffer_float` extension is present.
    pub fn supports_render_target(&self, format: wgpu::TextureFormat) -> bool {
        format_supports_render_target(&self.adapter, format)
    }

    /// Resolves a requested render-target format to one this GPU can actually render into.
    ///
    /// `None` resolves to the surface format. A `Some(format)` that is not renderable falls back
    /// to the surface format with a warning (e.g. `Rgba16Float` on a WebGL2 context lacking
    /// `EXT_color_buffer_float`), so [`crate::App::create_render_target_with_format`] degrades
    /// gracefully instead of creating an invalid texture. Query ahead of time with
    /// [`RenderCapabilities`].
    pub fn resolve_render_target_format(
        &self,
        requested: Option<wgpu::TextureFormat>,
        name: &str,
    ) -> wgpu::TextureFormat {
        let renderable = requested.is_none_or(|f| self.supports_render_target(f));
        if let Some(format) = requested {
            if !renderable {
                log::warn!(
                    "render target '{name}': {format:?} is not a renderable format on this GPU; \
                     falling back to the surface format {:?}",
                    self.config.format
                );
            }
        }
        pick_render_target_format(requested, renderable, self.config.format)
    }

    /// Reconfigures the surface when the window is resized.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        if let Some(surface) = &self.surface {
            surface.configure(&self.device, &self.config);
        }
        // Headless: rebuild the offscreen final-frame texture at the new size.
        #[cfg(not(target_arch = "wasm32"))]
        if self.headless_color.is_some() {
            self.headless_color = Some(Self::create_headless_color(
                &self.device,
                self.config.format,
                new_size.width,
                new_size.height,
            ));
        }
    }

    /// Reconfigures the surface after it is lost (`SurfaceError::Lost`). No-op when headless.
    pub fn reconfigure(&self) {
        if let Some(surface) = &self.surface {
            surface.configure(&self.device, &self.config);
        }
    }

    /// Copies the headless final-frame texture back to CPU memory as tightly-packed RGBA8 bytes
    /// (row stride = `width * 4`, sRGB-encoded). Returns `(width, height, pixels)`.
    ///
    /// # Panics
    /// Panics if called on a windowed (non-headless) context.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_headless_rgba(&self) -> (u32, u32, Vec<u8>) {
        let texture = self
            .headless_color
            .as_ref()
            .expect("read_headless_rgba called on a windowed GpuContext");
        let (w, h) = (self.config.width, self.config.height);
        let unpadded = w * 4;
        // wgpu requires each copied row to be 256-byte aligned.
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded.div_ceil(align) * align;

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("headless readback"),
            size: (padded * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("headless readback"),
            });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(enc.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll for readback");
        rx.recv().expect("readback channel").expect("map readback");

        let data = slice.get_mapped_range();
        // Drop the row padding into a tight width*4 stride.
        let mut pixels = Vec::with_capacity((unpadded * h) as usize);
        for row in 0..h {
            let start = (row * padded) as usize;
            pixels.extend_from_slice(&data[start..start + unpadded as usize]);
        }
        drop(data);
        buffer.unmap();
        (w, h, pixels)
    }

    /// Clears the screen to a solid color. Used to show the background when there are no sprites.
    pub fn clear(&mut self, color: wgpu::Color) -> Result<(), String> {
        // No surface to clear in headless mode (the render path clears the offscreen target).
        let Some(surface) = &self.surface else {
            return Ok(());
        };
        let frame = match surface.get_current_texture() {
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

/// GPU render-format capabilities, queryable from game code.
///
/// Inserted into the [`World`](crate::World) as a resource at GPU initialization, so a system or
/// `Scene::on_enter` can branch on what the GPU/backend can render into — e.g. request an HDR
/// (`Rgba16Float`) render target only where it is renderable. (When a requested format is *not*
/// renderable, [`crate::App::create_render_target_with_format`] also falls back to the surface
/// format automatically, with a warning — so the query is for deliberate branching, not crash
/// avoidance.)
///
/// Most useful on wasm/WebGL2, where float render targets require `EXT_color_buffer_float`; on a
/// desktop Vulkan/Metal/DX12 backend, `Rgba16Float` is renderable.
///
/// ```rust,ignore
/// fn setup(world: &mut World) {
///     let hdr_ok = world
///         .resource::<RenderCapabilities>()
///         .is_some_and(|caps| caps.supports_render_target(wgpu::TextureFormat::Rgba16Float));
///     // request an HDR render target only when `hdr_ok`
/// }
/// ```
#[derive(Clone)]
pub struct RenderCapabilities {
    adapter: wgpu::Adapter,
    surface_format: wgpu::TextureFormat,
}

impl RenderCapabilities {
    pub(crate) fn new(adapter: wgpu::Adapter, surface_format: wgpu::TextureFormat) -> Self {
        Self {
            adapter,
            surface_format,
        }
    }

    /// The window surface's pixel format — the default render-target format, and the fallback
    /// used when a requested format is not renderable.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }

    /// Returns `true` if `format` can be used as a color render attachment on this GPU/backend.
    pub fn supports_render_target(&self, format: wgpu::TextureFormat) -> bool {
        format_supports_render_target(&self.adapter, format)
    }
}

#[cfg(test)]
mod tests {
    use super::pick_render_target_format;
    use wgpu::TextureFormat;

    // The renderability query itself needs a live GPU (verified by the native render smoke);
    // here we pin the pure fallback policy, which CI can run without a GPU.

    #[test]
    fn pick_format_none_uses_surface() {
        assert_eq!(
            pick_render_target_format(None, true, TextureFormat::Bgra8UnormSrgb),
            TextureFormat::Bgra8UnormSrgb
        );
    }

    #[test]
    fn pick_format_renderable_uses_requested() {
        assert_eq!(
            pick_render_target_format(
                Some(TextureFormat::Rgba16Float),
                true,
                TextureFormat::Bgra8UnormSrgb
            ),
            TextureFormat::Rgba16Float
        );
    }

    #[test]
    fn pick_format_unrenderable_falls_back_to_surface() {
        assert_eq!(
            pick_render_target_format(
                Some(TextureFormat::Rgba16Float),
                false,
                TextureFormat::Bgra8UnormSrgb
            ),
            TextureFormat::Bgra8UnormSrgb
        );
    }
}
