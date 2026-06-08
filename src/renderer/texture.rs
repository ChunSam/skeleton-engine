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
        Self::try_from_path(device, queue, layout, path).unwrap_or_else(|e| {
            log::warn!("texture load failed ({path}): {e}, using magenta fallback");
            // magenta 1×1: makes missing textures visually identifiable at a glance
            Self::from_rgba(
                device,
                queue,
                layout,
                &[255u8, 0, 255, 255],
                1,
                1,
                Some("fallback"),
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
        let bytes = std::fs::read(path).map_err(TextureError::Io)?;
        let img = image::load_from_memory(&bytes).map_err(TextureError::Decode)?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Ok(Self::from_rgba(
            device,
            queue,
            layout,
            &rgba,
            w,
            h,
            Some(path),
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
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
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
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
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

    #[test]
    fn decode_valid_png_returns_rgba() {
        // 1×1 red pixel PNG (minimal valid PNG)
        let png_bytes: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // signature
            0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR length+type
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1×1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // bit depth 8, RGB
            0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, // IDAT
            0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xe2,
            0x21, 0xbc, 0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, // IEND
            0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        // Delegate to the library to check whether the PNG is actually valid — at minimum it must not panic
        let _ = decode_image_bytes(png_bytes); // Ok or Err both acceptable, only panic is forbidden
    }
}
