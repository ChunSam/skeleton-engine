//! The single color type used throughout the engine: [`Color`].

use serde::{Deserialize, Serialize};

/// RGBA color (each channel 0.0–1.0).
///
/// Used by every color API in the engine. `From` conversions are provided for
/// various inputs: `[f32; 4]`, `[f32; 3]` (alpha 1.0), `[u8; 4]`. At render
/// boundaries, convert with [`Color::to_array`]/[`Color::to_u8`]/[`Color::to_rgb`].
///
/// ```
/// use engine::Color;
/// let red = Color::rgb(1.0, 0.0, 0.0);
/// assert_eq!(red, Color::hex(0xFF0000));
/// assert_eq!(red.to_array(), [1.0, 0.0, 0.0, 1.0]);
/// let from_arr: Color = [0.2, 0.4, 0.6, 1.0].into();
/// assert_eq!(from_arr.r, 0.2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);

    /// Opaque color (alpha 1.0).
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Color including alpha.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Constructs from 0–255 integer channels.
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// `0xRRGGBB` hex color (alpha 1.0).
    pub fn hex(rgb: u32) -> Self {
        Self::rgba_u8(
            ((rgb >> 16) & 0xff) as u8,
            ((rgb >> 8) & 0xff) as u8,
            (rgb & 0xff) as u8,
            255,
        )
    }

    /// `[f32; 4]` for render/instance data.
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// `[u8; 4]` for 8-bit channel uses such as text rendering.
    pub fn to_u8(self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    /// `[f32; 3]` without alpha, for uses such as lighting.
    pub fn to_rgb(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl From<[f32; 4]> for Color {
    fn from(c: [f32; 4]) -> Self {
        Self {
            r: c[0],
            g: c[1],
            b: c[2],
            a: c[3],
        }
    }
}

impl From<[f32; 3]> for Color {
    fn from(c: [f32; 3]) -> Self {
        Self::rgb(c[0], c[1], c[2])
    }
}

impl From<[u8; 4]> for Color {
    fn from(c: [u8; 4]) -> Self {
        Self::rgba_u8(c[0], c[1], c[2], c[3])
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        c.to_array()
    }
}

impl From<Color> for [u8; 4] {
    fn from(c: Color) -> Self {
        c.to_u8()
    }
}

impl From<Color> for [f32; 3] {
    fn from(c: Color) -> Self {
        c.to_rgb()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_sets_opaque_alpha() {
        assert_eq!(Color::rgb(0.1, 0.2, 0.3).a, 1.0);
    }

    #[test]
    fn array_roundtrip() {
        let c = Color::rgba(0.25, 0.5, 0.75, 0.5);
        assert_eq!(c.to_array(), [0.25, 0.5, 0.75, 0.5]);
        assert_eq!(Color::from([0.25, 0.5, 0.75, 0.5]), c);
    }

    #[test]
    fn u8_and_hex() {
        assert_eq!(Color::rgba_u8(255, 0, 0, 255), Color::RED);
        assert_eq!(Color::hex(0xFF0000), Color::RED);
        assert_eq!(Color::WHITE.to_u8(), [255, 255, 255, 255]);
        assert_eq!(Color::from([0u8, 255, 0, 255]), Color::GREEN);
    }

    #[test]
    fn rgb3_fills_alpha() {
        let c: Color = [0.4, 0.5, 0.6].into();
        assert_eq!(c.a, 1.0);
        assert_eq!(c.to_rgb(), [0.4, 0.5, 0.6]);
    }
}
