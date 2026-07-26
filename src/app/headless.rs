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
        save_rgba_png(w, h, pixels, path.as_ref())
    }

    /// Runs headlessly and writes a PNG at **each** listed frame — a whole verification pass in
    /// one run.
    ///
    /// `captures` are `(frame, path)` pairs; frame numbers count from 0 and may be given in any
    /// order. The run lasts until the highest listed frame, so pairing this with an
    /// [`InputScript`](crate::InputScript) lets a game drive itself through several screens and
    /// photograph each one — with **no window, no display and no OS automation permissions**.
    ///
    /// This is what the `ENGINE_CAPTURE=<frame>:<path>[,…]` environment variable calls, so a game
    /// gets the whole flow without writing any code (see [`App::run`](Self::run)).
    ///
    /// Returns the paths written, or the first error. Native-only.
    ///
    /// ```no_run
    /// # use engine::{App, InputScript};
    /// # let mut app = App::new();
    /// app.set_input_script(InputScript::load("assets/scripts/shop.ron").unwrap());
    /// app.capture_frames_headless(&[(60, "/tmp/menu.png"), (150, "/tmp/shop.png")])
    ///     .unwrap();
    /// ```
    ///
    /// # Panics
    /// Panics if headless GPU initialization fails (no usable adapter).
    pub fn capture_frames_headless(
        &mut self,
        captures: &[(u32, impl AsRef<Path>)],
    ) -> Result<Vec<std::path::PathBuf>, String> {
        if captures.is_empty() {
            return Ok(Vec::new());
        }
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

        // Frame `n` is captured after n+1 updates, so the frame a script acts on is the frame
        // photographed — `(frame: 10, KeyPress)` + a capture at 10 shows the pressed state.
        let last = captures.iter().map(|(f, _)| *f).max().unwrap_or(0);
        let dt = 1.0 / 60.0;
        let mut written = Vec::with_capacity(captures.len());
        for frame in 0..=last {
            self.update(dt);
            // Headless render targets the offscreen texture and never presents, so it cannot
            // fail with a surface error (the Err arm is windowed-only).
            let _ = self.render();
            for (at, path) in captures.iter().filter(|(at, _)| *at == frame) {
                let (cw, ch, pixels) = self
                    .gpu
                    .as_ref()
                    .expect("headless gpu present")
                    .read_headless_rgba();
                save_rgba_png(cw, ch, pixels, path.as_ref())
                    .map_err(|e| format!("frame {at}: {e}"))?;
                written.push(path.as_ref().to_path_buf());
            }
        }
        Ok(written)
    }
}

/// Write tightly-packed RGBA8 bytes to a PNG, so callers need no `image` dependency.
fn save_rgba_png(w: u32, h: u32, pixels: Vec<u8>, path: &Path) -> Result<(), String> {
    let img = image::RgbaImage::from_raw(w, h, pixels)
        .ok_or_else(|| "read-back produced an unexpected byte count".to_string())?;
    img.save(path).map_err(|e| e.to_string())
}

// ── Headless EDITOR screenshot (no window, drives egui manually) ────────────────
//
// `screenshot_headless` above renders only the game render path. The egui editor overlay is
// normally driven by the windowed `egui_winit` loop (`begin_egui_frame` needs a window + egui
// state), so it never appears in a headless capture. The methods below drive egui *without* a
// window — synthesize a `RawInput`, build the editor UI via [`App::update_editor_ui`], tessellate,
// and let the normal render path's egui pass composite it onto the offscreen texture — so the
// editor's panels and windows (Inspector, EngineStats, the shortcuts cheatsheet, …) can be
// visually verified with **no display** (monitor off / asleep / locked / headless CI).
#[cfg(not(target_arch = "wasm32"))]
impl App {
    /// Renders `frames` frames headlessly **with the editor egui overlay drawn on top**, returning
    /// the final frame as tightly-packed sRGB `RGBA8` `(width, height, pixels)`.
    ///
    /// Set up the editor state you want captured *before* calling — e.g.
    /// [`set_editor_shortcuts_visible(true)`](Self::set_editor_shortcuts_visible) to open the
    /// shortcuts cheatsheet, or select an entity to populate the Inspector. Uses **overlay** mode
    /// (egui windows over the scene), not the docked layout — for the docked layout use
    /// [`screenshot_editor_docked_headless_rgba`](Self::screenshot_editor_docked_headless_rgba).
    /// The unconditional editor chrome (the shortcuts cheatsheet + action toasts) draws in overlay
    /// mode; the EngineStats / Inspector windows are gated on `Overlay` mode and don't appear here
    /// (overlay headless leaves the editor mode `Off`). Call this **instead of** [`run`](Self::run).
    /// Native-only.
    ///
    /// # Panics
    /// Panics if headless GPU initialization fails (no usable adapter).
    pub fn screenshot_editor_headless_rgba(&mut self, frames: u32) -> (u32, u32, Vec<u8>) {
        self.editor_headless_capture(frames, false)
    }

    /// Renders `frames` frames headlessly with the editor overlay and saves the final frame as a PNG.
    /// Convenience wrapper over [`screenshot_editor_headless_rgba`](Self::screenshot_editor_headless_rgba).
    /// Native-only.
    pub fn screenshot_editor_headless(
        &mut self,
        frames: u32,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        let (w, h, pixels) = self.screenshot_editor_headless_rgba(frames);
        let img = image::RgbaImage::from_raw(w, h, pixels)
            .ok_or_else(|| "read-back produced an unexpected byte count".to_string())?;
        img.save(path.as_ref()).map_err(|e| e.to_string())
    }

    /// Renders `frames` frames headlessly **with the full docked editor layout** (top toolbar,
    /// left entity list, right inspector, bottom assets/data-tables, and the game scene in the
    /// central viewport), returning the final frame as tightly-packed sRGB `RGBA8`
    /// `(width, height, pixels)`.
    ///
    /// This is the docked counterpart to [`screenshot_editor_headless_rgba`](Self::screenshot_editor_headless_rgba):
    /// it enters docked editor mode (F2) before driving egui, so the **docked side panels** — which
    /// the overlay capture can't show — render with no window/display (monitor
    /// off / asleep / locked / headless CI). This is what unblocks visual verification + golden-image
    /// tests of docked-panel editor UI (the entity-list eye toggle, Data Tables, the inspector, …).
    ///
    /// **Warm-up:** the central game viewport is a debounced offscreen render target (recreated only
    /// after its size is stable for 3 frames, then shown by egui the following frame). Pass
    /// **`frames >= 5`** so the scene appears in the central panel; with fewer frames the side panels
    /// still render but the central panel shows the *(no game frame yet)* placeholder. Set up the
    /// scene + selection *before* calling. Call this **instead of** [`run`](Self::run). Native-only.
    ///
    /// # Panics
    /// Panics if headless GPU initialization fails (no usable adapter).
    pub fn screenshot_editor_docked_headless_rgba(&mut self, frames: u32) -> (u32, u32, Vec<u8>) {
        self.editor_headless_capture(frames, true)
    }

    /// Renders `frames` frames headlessly with the docked editor layout and saves the final frame
    /// as a PNG. Convenience wrapper over
    /// [`screenshot_editor_docked_headless_rgba`](Self::screenshot_editor_docked_headless_rgba)
    /// (pass `frames >= 5` for the central scene). Native-only.
    pub fn screenshot_editor_docked_headless(
        &mut self,
        frames: u32,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        let (w, h, pixels) = self.screenshot_editor_docked_headless_rgba(frames);
        let img = image::RgbaImage::from_raw(w, h, pixels)
            .ok_or_else(|| "read-back produced an unexpected byte count".to_string())?;
        img.save(path.as_ref()).map_err(|e| e.to_string())
    }

    /// Shared headless editor-capture driver for both the overlay and docked paths: stand up a
    /// headless GPU + an egui context with no window, optionally enter docked mode, then run the
    /// `update → drive egui → render` loop and read the offscreen texture back.
    fn editor_headless_capture(&mut self, frames: u32, docked: bool) -> (u32, u32, Vec<u8>) {
        let (w, h) = self
            .world
            .resource::<WindowConfig>()
            .map(|c| (c.width, c.height))
            .unwrap_or((1280, 720));

        let gpu = pollster::block_on(GpuContext::new_headless(w, h))
            .expect("headless GPU initialization failed");
        self.world.insert_resource(ViewportSize::new(w, h));
        self.init_gpu_renderers(&gpu);

        // Stand up egui without a window: a DebugUi (egui Context + Korean fallback fonts), enabled
        // so the overlay editor draws, plus an egui-wgpu renderer matched to the OFFSCREEN format
        // (`new_headless` renders into Rgba8UnormSrgb — see `context.rs`).
        let egui_ctx = egui::Context::default();
        let mut debug_ui = crate::debug_ui::DebugUi::new_with_ctx(egui_ctx.clone());
        debug_ui.set_enabled(true);
        self.world.insert_resource(debug_ui);
        self.render.egui_renderer = Some(egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.config.format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                dithering: false,
                ..Default::default()
            },
        ));
        self.gpu = Some(gpu);

        // Docked mode: enter the docked layout so `update_editor_ui` builds the full panel chrome
        // (the overlay capture leaves the mode `Off`). The central viewport's offscreen RT then
        // warms up over the debounce window — see the method doc's frame-count note.
        if docked {
            self.editor.mode = crate::app::editor::EditorMode::Docked;
        }

        let dt = 1.0 / 60.0;
        for _ in 0..frames.max(1) {
            // Advance the game (the windowed egui begin/build is a no-op headlessly — no window).
            self.update(dt);
            // Then manually drive one editor egui frame into `render.egui_output`...
            self.drive_editor_egui_headless(w, h, dt, &egui_ctx);
            // ...and render: the game passes draw the scene, the egui pass composites the editor on top.
            let _ = self.render();
        }

        self.gpu
            .as_ref()
            .expect("headless gpu present")
            .read_headless_rgba()
    }

    /// Show or hide the editor keyboard-shortcuts cheatsheet (the same window the `?` key / `? Keys`
    /// toolbar button toggle). Public so a headless capture or a game can open it programmatically.
    pub fn set_editor_shortcuts_visible(&mut self, visible: bool) {
        self.editor.show_shortcuts = visible;
    }

    /// Select `entity` in the editor: populate the Inspector with its components and make it the
    /// sole multi-selection. Public so a headless capture (to set up an Inspector screenshot) or a
    /// game can drive editor selection programmatically — the same state a click in the entity list
    /// produces. Native-only.
    pub fn editor_select_entity(&mut self, entity: crate::Entity) {
        self.editor.inspector_selected = Some(entity);
        self.editor.selected_entities = vec![entity];
    }

    /// Switch the docked left panel to the **Scene** tab (the parent→children tree, where
    /// drag-to-reparent lives), as if the user clicked its tab. Public so a headless capture can
    /// screenshot the hierarchy view (the default is the flat Entities tab). Native-only.
    pub fn editor_show_scene_tree(&mut self) {
        self.editor.inspector_tab = 2;
    }

    /// Drive one egui editor frame with a synthesized `RawInput` (no window): begin the pass, build
    /// the editor UI, tessellate, and stash the paint jobs in `render.egui_output` for the render
    /// pass's egui overlay to consume.
    fn drive_editor_egui_headless(&mut self, w: u32, h: u32, dt: f32, ctx: &egui::Context) {
        let mut raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(w as f32, h as f32),
            )),
            ..Default::default()
        };
        // Ensure the ROOT viewport advertises a pixels-per-point so layout/tessellation use 1.0.
        raw_input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(1.0);

        ctx.begin_pass(raw_input);
        self.update_editor_ui(&Some(ctx.clone()), dt);
        let full_output = ctx.end_pass();
        let paint_jobs = ctx.tessellate(full_output.shapes, 1.0);
        let textures_delta = crate::app::schedule::merge_textures_delta(
            self.render
                .egui_output
                .take()
                .map(|(_, pending, _)| pending),
            full_output.textures_delta,
        );
        self.render.egui_output = Some((paint_jobs, textures_delta, 1.0));
    }
}
