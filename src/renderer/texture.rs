use wgpu::util::DeviceExt;

/// Reason a texture load failed
#[derive(Debug)]
pub enum TextureError {
    Io(std::io::Error),
    Decode(image::ImageError),
}

impl std::fmt::Display for TextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureError::Io(e) => write!(f, "IO error: {e}"),
            TextureError::Decode(e) => write!(f, "decode error: {e}"),
        }
    }
}

/// A GPU-resident texture together with its sampler and bind group
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

impl Texture {
    /// Reads a PNG file and creates a GPU texture. Falls back to a magenta 1×1 texture + warn log on failure.
    pub fn from_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: &str,
    ) -> Self {
        Self::from_path_with_format(
            device,
            queue,
            layout,
            path,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }

    /// Reads a PNG file and creates a GPU texture with a caller-chosen pixel format.
    ///
    /// Falls back to a magenta 1×1 texture + warn log on failure. See
    /// [`Texture::from_rgba_with_format`] for when a non-sRGB (linear) format is wanted.
    pub(crate) fn from_path_with_format(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: &str,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self::try_from_path_with_format(device, queue, layout, path, format).unwrap_or_else(|e| {
            // A failed texture was the engine's loudest silent failure: the magenta 1×1 is
            // substituted, nothing aborts, and a full-screen textured quad turns the whole window
            // magenta. Record it so `asset_path::asset_failures()` surfaces it and strict mode can
            // stop here, instead of leaving the player staring at a magenta screen.
            crate::asset_path::record_failure(path, &e);
            // magenta 1×1: makes missing textures visually identifiable at a glance
            Self::from_rgba_with_format(
                device,
                queue,
                layout,
                &[255u8, 0, 255, 255],
                1,
                1,
                Some("fallback"),
                format,
            )
        })
    }

    /// Reads a PNG file and creates a GPU texture. Returns `TextureError` on failure.
    pub fn try_from_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: &str,
    ) -> Result<Self, TextureError> {
        Self::try_from_path_with_format(
            device,
            queue,
            layout,
            path,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }

    /// Reads a PNG file and creates a GPU texture with a caller-chosen pixel format.
    /// Returns `TextureError` on failure.
    pub(crate) fn try_from_path_with_format(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        path: &str,
        format: wgpu::TextureFormat,
    ) -> Result<Self, TextureError> {
        // Resolved only here, at the filesystem edge — `path` stays the texture's cache key.
        let bytes = std::fs::read(crate::asset_path::resolve(path)).map_err(TextureError::Io)?;
        let img = image::load_from_memory(&bytes).map_err(TextureError::Decode)?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Ok(Self::from_rgba_with_format(
            device,
            queue,
            layout,
            &rgba,
            w,
            h,
            Some(path),
            format,
        ))
    }

    /// Uploads CPU-side `ImageAsset` data as a GPU texture (used when async loading completes).
    pub fn from_image_asset(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        asset: &crate::asset::ImageAsset,
        label: Option<&str>,
    ) -> Self {
        Self::from_rgba(
            device,
            queue,
            layout,
            &asset.data,
            asset.width,
            asset.height,
            label,
        )
    }

    /// Creates a default white 1×1 pixel texture (used for solid-color sprites)
    pub fn white(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        Self::from_rgba(
            device,
            queue,
            layout,
            &[255u8, 255, 255, 255],
            1,
            1,
            Some("white"),
        )
    }

    fn from_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        data: &[u8],
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> Self {
        // sRGB is the right default for color art: the sampler decodes to linear so
        // shading is correct, and the surface re-encodes on write.
        Self::from_rgba_with_format(
            device,
            queue,
            layout,
            data,
            width,
            height,
            label,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }

    /// Uploads tightly-packed RGBA8 bytes as a GPU texture with a caller-chosen pixel format.
    ///
    /// The default [`Texture::from_rgba`] hardcodes `Rgba8UnormSrgb` (correct for color art).
    /// Use this with `Rgba8Unorm` (linear) for **data textures** — normal maps, masks, height
    /// or lookup tables — whose bytes are *not* sRGB-encoded color and must be sampled verbatim
    /// without the sRGB→linear decode. The resulting texture stays sampleable by the sprite
    /// pipeline: both formats satisfy the `Float { filterable }` bind-group layout.
    // Mirrors the wgpu texture-descriptor argument set (the default `from_rgba` is already
    // at the 7-arg boundary); bundling these into a struct would only obscure an internal helper.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_rgba_with_format(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        data: &[u8],
        width: u32,
        height: u32,
        label: Option<&str>,
        format: wgpu::TextureFormat,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Nearest filtering = crisp pixel art (sprite/atlas textures).
        //
        // One sampler per texture, and every one of them identical. Left that way on purpose:
        // this runs at *load* time, not per frame, and de-duplicating it means either a new
        // public constructor taking a borrowed sampler or a device-keyed cache — real API
        // surface for a skeleton engine to carry, bought with no measurement behind it. If it
        // ever needs doing, `wgpu::Sampler` is `Clone` (an Arc-backed handle), so one shared
        // sampler cloned into each `Texture` is the whole fix and the `pub sampler` field keeps
        // working unchanged. The case that would force it is a backend with a hard sampler
        // ceiling (D3D12's descriptor heap) under a texture count high enough to reach it —
        // which needs that box to confirm, and this repo has no Windows runtime coverage.
        let sampler = super::common::create_clamp_sampler(device, None, wgpu::FilterMode::Nearest);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self {
            texture,
            view,
            sampler,
            bind_group,
        }
    }

    /// Returns the texture bind group layout (shared when creating render pipelines)
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture layout"),
            entries: &[
                super::common::filterable_texture_entry(0),
                super::common::filtering_sampler_entry(1),
            ],
        })
    }
}

/// Pure helper that validates file-to-RGBA decoding without a GPU (for tests and diagnostics)
pub fn decode_image_bytes(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), TextureError> {
    let img = image::load_from_memory(bytes).map_err(TextureError::Decode)?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((rgba.into_raw(), w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_path_missing_file_returns_io_error() {
        // Verify file-read failure without a GPU
        let result = std::fs::read("/nonexistent/__does_not_exist__.png").map_err(TextureError::Io);
        assert!(matches!(result, Err(TextureError::Io(_))));
    }

    #[test]
    fn decode_broken_bytes_returns_decode_error() {
        let broken = b"this is not a valid image";
        let result = decode_image_bytes(broken);
        assert!(matches!(result, Err(TextureError::Decode(_))));
    }

    /// The one check that the *success* path of `decode_image_bytes` works at all —
    /// `decode_broken_bytes_returns_decode_error` covers only the failure side.
    ///
    /// Assert the decoded pixel, not just `is_ok()`. This test used to throw the result away
    /// (`let _ = …; // Ok or Err both acceptable`), and that hid a fixture whose IDAT chunk
    /// carried a **wrong CRC** — the "minimal valid PNG" had never decoded once, so the success
    /// path had no coverage at all. A vacuous assertion does not merely fail to test the code;
    /// it hides the test's own rot.
    #[test]
    fn decode_valid_png_returns_rgba() {
        // 1×1 opaque red, 8-bit RGB. Regenerate with:
        //   python3 -c 'import zlib,struct
        //   c=lambda t,d:struct.pack(">I",len(d))+t+d+struct.pack(">I",zlib.crc32(t+d))
        //   print((b"\x89PNG\r\n\x1a\n"+c(b"IHDR",struct.pack(">IIBBBBB",1,1,8,2,0,0,0))
        //         +c(b"IDAT",zlib.compress(b"\x00\xff\x00\x00"))+c(b"IEND",b"")).hex())'
        #[rustfmt::skip]
        let png_bytes: &[u8] = &[
            // signature
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
            // IHDR: length + type, 1×1, bit depth 8, colour type 2 (RGB), then its CRC
            0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xde,
            // IDAT: length + type, zlib stream, then its CRC
            0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54,
            0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00,
            0xc9, 0xfe, 0x92, 0xef,
            // IEND: length + type, then its CRC
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
            0xae, 0x42, 0x60, 0x82,
        ];

        let (pixels, w, h) = decode_image_bytes(png_bytes).expect("the fixture must decode");
        assert_eq!((w, h), (1, 1), "dimensions come from IHDR");
        // `to_rgba8` widens the RGB source, so the alpha byte is synthesised as opaque.
        assert_eq!(pixels, vec![255, 0, 0, 255], "1×1 opaque red as RGBA8");
    }
}
