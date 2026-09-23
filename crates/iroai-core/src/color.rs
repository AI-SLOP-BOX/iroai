use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Pod, Zeroable)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// sRGB 8-bit から Linear RGB f32 (0.0 ..= 1.0) への変換
    pub fn to_linear_f32(&self) -> (f32, f32, f32, f32) {
        let to_lin = |v: u8| -> f32 {
            let s = v as f32 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        (to_lin(self.r), to_lin(self.g), to_lin(self.b), self.a as f32 / 255.0)
    }

    /// Linear RGB f32 (0.0 ..= 1.0) から sRGB 8-bit への変換
    pub fn from_linear_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        let to_srgb = |l: f32| -> u8 {
            let l_clamped = l.clamp(0.0, 1.0);
            let s = if l_clamped <= 0.0031308 {
                l_clamped * 12.92
            } else {
                1.055 * l_clamped.powf(1.0 / 2.4) - 0.055
            };
            (s * 255.0).round() as u8
        };
        Self {
            r: to_srgb(r),
            g: to_srgb(g),
            b: to_srgb(b),
            a: (a.clamp(0.0, 1.0) * 255.0).round() as u8,
        }
    }
}

/// 16-bit / 32-bit HDR 拡張カラー
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ColorHdr {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorHdr {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_color8(&self) -> Color {
        Color::from_linear_f32(self.r, self.g, self.b, self.a)
    }

    pub fn from_color8(c: Color) -> Self {
        let (r, g, b, a) = c.to_linear_f32();
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Add,
}

impl BlendMode {
    pub fn all() -> &'static [BlendMode] {
        &[
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::ColorDodge,
            BlendMode::ColorBurn,
            BlendMode::HardLight,
            BlendMode::SoftLight,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::Add,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::ColorDodge => "Color Dodge",
            BlendMode::ColorBurn => "Color Burn",
            BlendMode::HardLight => "Hard Light",
            BlendMode::SoftLight => "Soft Light",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Add => "Add",
        }
    }

    /// 厳密なアルファ合成 (Porter-Duff Over 則) に基づくブレンド計算
    pub fn blend_pixel(&self, base: Color, src: Color, opacity: f32) -> Color {
        let src_a = (src.a as f32 / 255.0) * opacity.clamp(0.0, 1.0);
        if src_a <= 0.0 {
            return base;
        }

        let dst_a = base.a as f32 / 255.0;
        let out_a = src_a + dst_a * (1.0 - src_a);
        if out_a <= 0.0 {
            return Color::TRANSPARENT;
        }

        let br = base.r as f32;
        let bg = base.g as f32;
        let bb = base.b as f32;
        let sr = src.r as f32;
        let sg = src.g as f32;
        let sb = src.b as f32;

        let blend_ch = |b: f32, s: f32| -> f32 {
            match self {
                BlendMode::Normal => s,
                BlendMode::Multiply => (b * s) / 255.0,
                BlendMode::Screen => 255.0 - ((255.0 - b) * (255.0 - s)) / 255.0,
                BlendMode::Overlay => {
                    if b < 128.0 {
                        (2.0 * b * s) / 255.0
                    } else {
                        255.0 - (2.0 * (255.0 - b) * (255.0 - s)) / 255.0
                    }
                }
                BlendMode::Darken => b.min(s),
                BlendMode::Lighten => b.max(s),
                BlendMode::ColorDodge => {
                    if s >= 255.0 { 255.0 } else { ((b * 255.0) / (255.0 - s)).min(255.0) }
                }
                BlendMode::ColorBurn => {
                    if s <= 0.0 { 0.0 } else { (255.0 - ((255.0 - b) * 255.0) / s).max(0.0) }
                }
                BlendMode::HardLight => {
                    if s < 128.0 { (2.0 * b * s) / 255.0 } else { 255.0 - (2.0 * (255.0 - b) * (255.0 - s)) / 255.0 }
                }
                BlendMode::SoftLight => {
                    let s_norm = s / 255.0;
                    let b_norm = b / 255.0;
                    (255.0 * ((1.0 - 2.0 * s_norm) * b_norm * b_norm + 2.0 * s_norm * b_norm)).clamp(0.0, 255.0)
                }
                BlendMode::Difference => (b - s).abs(),
                BlendMode::Exclusion => b + s - (2.0 * b * s) / 255.0,
                BlendMode::Add => (b + s).min(255.0),
            }
        };

        // W3C Compositing and Blending Level 1 式:
        // out_col = ( (1 - src_a) * dst_a * base + (1 - dst_a) * src_a * src + src_a * dst_a * blend(base, src) ) / out_a
        let comp_ch = |b: f32, s: f32| -> f32 {
            let blended = blend_ch(b, s);
            let num = (1.0 - src_a) * dst_a * b + (1.0 - dst_a) * src_a * s + src_a * dst_a * blended;
            (num / out_a).clamp(0.0, 255.0)
        };

        Color {
            r: comp_ch(br, sr).round() as u8,
            g: comp_ch(bg, sg).round() as u8,
            b: comp_ch(bb, sb).round() as u8,
            a: (out_a * 255.0).round() as u8,
        }
    }

    /// Photoshop「ガンマ1.0によるブレンド（リニアブレンド）」に対応した精密合成
    pub fn blend_pixel_linear(&self, base: Color, src: Color, opacity: f32) -> Color {
        let (br, bg, bb, ba) = base.to_linear_f32();
        let (sr, sg, sb, sa) = src.to_linear_f32();
        let effective_src_a = sa * opacity.clamp(0.0, 1.0);

        if effective_src_a <= 0.0 {
            return base;
        }

        let out_a = effective_src_a + ba * (1.0 - effective_src_a);
        if out_a <= 0.0 {
            return Color::TRANSPARENT;
        }

        let blend_linear_ch = |b: f32, s: f32| -> f32 {
            match self {
                BlendMode::Normal => s,
                BlendMode::Multiply => b * s,
                BlendMode::Screen => 1.0 - (1.0 - b) * (1.0 - s),
                BlendMode::Overlay => {
                    if b < 0.5 { 2.0 * b * s } else { 1.0 - 2.0 * (1.0 - b) * (1.0 - s) }
                }
                BlendMode::Darken => b.min(s),
                BlendMode::Lighten => b.max(s),
                BlendMode::Add => (b + s).min(1.0),
                BlendMode::Difference => (b - s).abs(),
                _ => s,
            }
        };

        let comp_ch = |b: f32, s: f32| -> f32 {
            let blended = blend_linear_ch(b, s);
            let num = (1.0 - effective_src_a) * ba * b + (1.0 - ba) * effective_src_a * s + effective_src_a * ba * blended;
            (num / out_a).clamp(0.0, 1.0)
        };

        let out_r = comp_ch(br, sr);
        let out_g = comp_ch(bg, sg);
        let out_b = comp_ch(bb, sb);

        Color::from_linear_f32(out_r, out_g, out_b, out_a)
    }
}
