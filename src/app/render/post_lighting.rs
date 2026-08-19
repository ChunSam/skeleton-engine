use super::super::App;
use crate::app::render_state::RenderState;
#[cfg(not(target_arch = "wasm32"))]
use crate::ecs::World;
use crate::renderer::bloom::BloomRenderer;
use crate::renderer::PostProcessRenderer;

/// The three things about a `LightingRenderer` that a frame can invalidate.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct LightingState {
    pub format: wgpu::TextureFormat,
    pub max_lights: usize,
    pub width: u32,
    pub height: u32,
}

/// Which rebuild steps `setup_lighting` must run to bring an existing `LightingRenderer` from
/// `have` to `want`.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) struct LightingFixups {
    pub reconfigure: bool,
    pub set_max_lights: bool,
    pub resize: bool,
}

/// Decide the fix-up steps for one frame — **all of them**, not the first that applies.
///
/// The three `LightingRenderer` rebuilds each preserve what they do not change, which is what lets
/// them be independent flags applied in this fixed order:
///
/// - `reconfigure(w, h, fmt)` rebuilds at the new size and format but **keeps the current cap**;
/// - `set_max_lights(cap)` rebuilds at the **stored** size and format;
/// - `resize(w, h)` returns immediately when the size already matches.
///
/// So `reconfigure` already leaves the size correct (hence `resize` is only needed when
/// `reconfigure` did not run), and `set_max_lights` cannot undo either of the other two. Split out
/// as a pure function because every step below it needs a GPU device: this is the part of
/// `setup_lighting` that can be tested at all.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn lighting_fixups(have: LightingState, want: LightingState) -> LightingFixups {
    let reconfigure = have.format != want.format;
    LightingFixups {
        reconfigure,
        set_max_lights: have.max_lights != want.max_lights,
        // `reconfigure` rebuilt at `want`'s size already; otherwise the size stands as it was.
        resize: !reconfigure && (have.width != want.width || have.height != want.height),
    }
}

impl App {
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn ensure_intermediate_texture(
        slot: &mut Option<(
            wgpu::Texture,
            wgpu::TextureView,
            u32,
            u32,
            wgpu::TextureFormat,
        )>,
        device: &wgpu::Device,
        label: &'static str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> bool {
        let needs_new = match slot {
            Some((_, _, w, h, fmt)) => *w != width || *h != height || *fmt != format,
            None => true,
        };
        if needs_new {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            *slot = Some((tex, view, width, height, format));
        }
        needs_new
    }

    /// Lazily create / resize the post-process renderer for the current surface.
    /// Extracted from `render()` (pre-frame setup; no encoder/submit involved).
    /// `out_fmt` is the final display/swapchain format; `inter_fmt` is the scene intermediate format
    /// (== `out_fmt` normally, `Rgba16Float` when `PostProcessConfig::hdr` is on).
    pub(in crate::app) fn setup_post_renderer(
        render: &mut RenderState,
        device: &wgpu::Device,
        w: u32,
        h: u32,
        out_fmt: wgpu::TextureFormat,
        inter_fmt: wgpu::TextureFormat,
    ) {
        match &mut render.post_renderer {
            None => {
                render.post_renderer =
                    Some(PostProcessRenderer::new(device, w, h, out_fmt, inter_fmt));
            }
            Some(pr) if pr.output_format() != out_fmt || pr.intermediate_format() != inter_fmt => {
                pr.reconfigure(device, w, h, out_fmt, inter_fmt);
            }
            Some(pr) if pr.width != w || pr.height != h => {
                pr.resize(device, w, h);
            }
            _ => {}
        }
    }

    /// Lazily create / resize / reconfigure the bloom renderer for the scene intermediate.
    /// Called only when post-process is enabled and `PostProcessConfig::bloom` is on. `inter_fmt`
    /// is the scene intermediate format (== the post intermediate: `Rgba16Float` under HDR, else
    /// the surface format) so the bloom pipelines match. Mirrors `setup_post_renderer` /
    /// `setup_lighting`: one target per frame, so a format change rebuilds the renderer (there is
    /// no per-target-format pipeline cache, unlike the sprite/material/UI/GPU-particle passes).
    pub(in crate::app) fn setup_bloom_renderer(
        render: &mut RenderState,
        device: &wgpu::Device,
        w: u32,
        h: u32,
        inter_fmt: wgpu::TextureFormat,
    ) {
        match &mut render.bloom_renderer {
            None => {
                render.bloom_renderer = Some(BloomRenderer::new(device, w, h, inter_fmt));
            }
            Some(br) if br.format() != inter_fmt => br.reconfigure(device, w, h, inter_fmt),
            Some(br) if br.width() != w || br.height() != h => br.resize(device, w, h),
            _ => {}
        }
    }

    /// Lazily create / resize / disable the lighting renderer + its scene-intermediate
    /// texture, returning whether lighting is active this frame. Extracted from `render()`
    /// (pre-frame setup; no encoder/submit involved). Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn setup_lighting(
        render: &mut RenderState,
        world: &World,
        device: &wgpu::Device,
        w: u32,
        h: u32,
        fmt: wgpu::TextureFormat,
        use_post: bool,
    ) -> bool {
        let has_lighting = world.resource::<crate::resources::AmbientLight>().is_some();
        if has_lighting {
            // Point-light cap: opt-in LightingConfig, else the historical default (16).
            let max_lights = world
                .resource::<crate::resources::LightingConfig>()
                .map(|c| c.max_lights)
                .unwrap_or(crate::resources::DEFAULT_MAX_LIGHTS);
            match &mut render.lighting_renderer {
                None => {
                    render.lighting_renderer =
                        Some(crate::renderer::lighting::LightingRenderer::new(
                            device, w, h, fmt, max_lights,
                        ));
                }
                Some(lr) => {
                    // ⚠️ These were exclusive `match` arms until v0.153.2, so a frame that changed
                    // BOTH the format and the light cap took the format arm alone — and
                    // `reconfigure` preserves the current cap, so the new cap was swallowed until
                    // the next frame re-entered here and hit the cap arm. Self-healing, one frame
                    // late. They are independent fix-ups, so they are applied as independent
                    // steps; see `lighting_fixups` for why this order needs no re-checking.
                    let fx = lighting_fixups(
                        LightingState {
                            format: lr.format(),
                            max_lights: lr.max_lights(),
                            width: lr.width,
                            height: lr.height,
                        },
                        LightingState {
                            format: fmt,
                            max_lights,
                            width: w,
                            height: h,
                        },
                    );
                    if fx.reconfigure {
                        lr.reconfigure(device, w, h, fmt);
                    }
                    if fx.set_max_lights {
                        lr.set_max_lights(device, max_lights);
                    }
                    if fx.resize {
                        lr.resize(device, w, h);
                    }
                }
            }
            // Create / resize the scene intermediate texture (only needed when post_renderer is absent)
            let recreated = if !use_post {
                let recreated = Self::ensure_intermediate_texture(
                    &mut render.scene_texture_for_lighting,
                    device,
                    "scene_for_lighting",
                    w,
                    h,
                    fmt,
                );
                render.post_texture_for_lighting = None;
                recreated
            } else {
                render.scene_texture_for_lighting = None;
                Self::ensure_intermediate_texture(
                    &mut render.post_texture_for_lighting,
                    device,
                    "post_for_lighting",
                    w,
                    h,
                    fmt,
                )
            };
            // ⚠️ A recreated intermediate MUST invalidate the lighting bind group explicitly.
            // `run_pass` guards its cache with the address of the `&TextureView` it is handed, and
            // that address belongs to the `Option<(…)>` field above — which is written in place,
            // so the new view can arrive at the old address and be waved through. The cache was
            // sound only because every path reaching here also resized or reconfigured the
            // renderer; this makes it local instead of a standing invariant, and the failure it
            // prevents is silent (sampling last frame's texture, with no validation error).
            if recreated {
                if let Some(lr) = &mut render.lighting_renderer {
                    lr.invalidate_bind_group();
                }
            }
        } else {
            render.lighting_renderer = None;
            render.scene_texture_for_lighting = None;
            render.post_texture_for_lighting = None;
        }
        has_lighting
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{lighting_fixups, LightingState};
    use wgpu::TextureFormat;

    const SRGB: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    const HDR: TextureFormat = TextureFormat::Rgba16Float;

    fn state(format: TextureFormat, max_lights: usize, width: u32, height: u32) -> LightingState {
        LightingState {
            format,
            max_lights,
            width,
            height,
        }
    }

    /// Nothing changed: no rebuild of any kind. The control for every test below — without it,
    /// "the right steps ran" is indistinguishable from "every step always runs".
    #[test]
    fn an_unchanged_frame_rebuilds_nothing() {
        let s = state(SRGB, 16, 800, 600);
        let fx = lighting_fixups(s, s);
        assert!(!fx.reconfigure && !fx.set_max_lights && !fx.resize);
    }

    /// ⚠️ The regression this function exists for. A frame that turns on HDR *and* raises the
    /// light cap must apply both: `reconfigure` alone preserves the old cap, so the exclusive
    /// `match` arms this replaced left the scene lit with 16 lights for one frame after the game
    /// asked for 64.
    #[test]
    fn a_same_frame_format_and_cap_change_applies_both() {
        let fx = lighting_fixups(state(SRGB, 16, 800, 600), state(HDR, 64, 800, 600));
        assert!(fx.reconfigure, "the format change must rebuild");
        assert!(
            fx.set_max_lights,
            "the cap change must NOT wait for the next frame"
        );
        assert!(
            !fx.resize,
            "reconfigure already rebuilt at the current size; resizing again would be wasted work"
        );
    }

    /// All three at once — the resize is still redundant, because `reconfigure` took the new size.
    #[test]
    fn a_format_cap_and_size_change_needs_no_separate_resize() {
        let fx = lighting_fixups(state(SRGB, 16, 800, 600), state(HDR, 64, 1280, 720));
        assert!(fx.reconfigure && fx.set_max_lights);
        assert!(!fx.resize);
    }

    /// A cap change during a window drag: `set_max_lights` rebuilds at the *stored* size, so the
    /// resize is genuinely needed here and must not be skipped.
    #[test]
    fn a_cap_change_while_resizing_still_resizes() {
        let fx = lighting_fixups(state(SRGB, 16, 800, 600), state(SRGB, 64, 1280, 720));
        assert!(!fx.reconfigure, "the format did not change");
        assert!(fx.set_max_lights);
        assert!(
            fx.resize,
            "set_max_lights keeps the old size — resize must run"
        );
    }

    /// Each change on its own still does exactly one thing.
    #[test]
    fn a_single_change_runs_a_single_step() {
        let base = state(SRGB, 16, 800, 600);
        let only_fmt = lighting_fixups(base, state(HDR, 16, 800, 600));
        assert!(only_fmt.reconfigure && !only_fmt.set_max_lights && !only_fmt.resize);

        let only_cap = lighting_fixups(base, state(SRGB, 64, 800, 600));
        assert!(!only_cap.reconfigure && only_cap.set_max_lights && !only_cap.resize);

        let only_size = lighting_fixups(base, state(SRGB, 16, 1280, 720));
        assert!(!only_size.reconfigure && !only_size.set_max_lights && only_size.resize);
    }
}
