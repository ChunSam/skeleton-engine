//! Headless rendering — render frames to an offscreen target and read them back, with **no
//! window and no display** (works with the monitor off/asleep/locked, and on a machine with no
//! display attached). Backs golden-image / CI / monitor-off screenshot tests.

use std::path::Path;

use crate::renderer::context::GpuContext;
use crate::resources::{ViewportSize, WindowConfig};

use super::App;

impl App {
    /// Renders `frames` frames headlessly (no window/surface/display) and returns the final frame
    /// as tightly-packed sRGB **`RGBA8`** bytes: `(width, height, pixels)` with row stride
    /// `width * 4`.
    ///
    /// The render size is the [`WindowConfig`] width/height (default 1280×720). Each frame runs the
    /// app's systems via `update` then the full render path into an offscreen texture, so the image
    /// matches what the windowed app would draw. Run several frames (e.g. 30) so physics settles and
    /// animations advance before the capture.
    ///
    /// This builds a fresh headless [`GpuContext`] and installs it on the app, so call it **instead
    /// of** [`run`](Self::run) (not after). Native-only — wasm has no offscreen read-back path.
    ///
    /// # Panics
    /// Panics if headless GPU initialization fails (no usable adapter).
    pub fn screenshot_headless(&mut self, frames: u32) -> (u32, u32, Vec<u8>) {
        let (w, h) = self
            .world
            .resource::<WindowConfig>()
            .map(|c| (c.width, c.height))
            .unwrap_or((1280, 720));

        let gpu = pollster::block_on(GpuContext::new_headless(w, h))
            .expect("headless GPU initialization failed");
        // The render path reads ViewportSize for projection/letterbox; match the render size.
        self.world.insert_resource(ViewportSize::new(w, h));
        self.init_gpu_renderers(&gpu);
        self.gpu = Some(gpu);

        let dt = 1.0 / 60.0;
        for _ in 0..frames.max(1) {
            self.update(dt);
            // Headless render targets the offscreen texture and never presents, so it cannot
            // fail with a surface error (the Err arm is windowed-only).
            let _ = self.render();
        }

        self.gpu
            .as_ref()
            .expect("headless gpu present")
            .read_headless_rgba()
    }

    /// Renders `frames` frames headlessly and saves the final frame as a PNG at `path`.
    ///
    /// Convenience wrapper over [`screenshot_headless`](Self::screenshot_headless) that needs no
    /// `image` dependency in the caller. Native-only.
    pub fn save_screenshot_headless(
        &mut self,
        frames: u32,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        let (w, h, pixels) = self.screenshot_headless(frames);
        let img = image::RgbaImage::from_raw(w, h, pixels)
            .ok_or_else(|| "read-back produced an unexpected byte count".to_string())?;
        img.save(path.as_ref()).map_err(|e| e.to_string())
    }
}
