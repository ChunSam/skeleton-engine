//! 엔진 전역에서 쓰는 단일 색상 타입 [`Color`].

use serde::{Deserialize, Serialize};

/// RGBA 색상 (각 채널 0.0–1.0).
///
/// 엔진의 모든 색상 API가 이 타입을 쓴다. 다양한 입력을 받도록 `From` 변환을
/// 제공한다: `[f32; 4]`, `[f32; 3]`(알파 1.0), `[u8; 4]`. 렌더 경계에서는
/// [`Color::to_array`]/[`Color::to_u8`]/[`Color::to_rgb`] 로 변환한다.
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

    /// 불투명 색 (알파 1.0).
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// 알파를 포함한 색.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// 0–255 정수 채널로 생성.
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// `0xRRGGBB` 16진수 색 (알파 1.0).
    pub fn hex(rgb: u32) -> Self {
        Self::rgba_u8(
            ((rgb >> 16) & 0xff) as u8,
            ((rgb >> 8) & 0xff) as u8,
            (rgb & 0xff) as u8,
            255,
        )
    }

    /// 렌더/인스턴스 데이터용 `[f32; 4]`.
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// 텍스트 등 8비트 채널용 `[u8; 4]`.
    pub fn to_u8(self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    /// 라이팅 등 알파 없는 `[f32; 3]`.
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
