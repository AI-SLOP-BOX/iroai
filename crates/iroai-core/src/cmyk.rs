use crate::color::Color;
use serde::{Deserialize, Serialize};

/// CMYK 印刷用色モデル (0.0 ..= 1.0)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ColorCmyk {
    pub c: f32, // Cyan
    pub m: f32, // Magenta
    pub y: f32, // Yellow
    pub k: f32, // Key / Black
}

impl ColorCmyk {
    pub fn new(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self {
            c: c.clamp(0.0, 1.0),
            m: m.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            k: k.clamp(0.0, 1.0),
        }
    }

    /// sRGB 8-bit から標準 CMYK への変換
    pub fn from_rgb(rgb: Color) -> Self {
        let r = rgb.r as f32 / 255.0;
        let g = rgb.g as f32 / 255.0;
        let b = rgb.b as f32 / 255.0;

        let k = 1.0 - r.max(g).max(b);
        if k >= 0.9999 {
            return Self { c: 0.0, m: 0.0, y: 0.0, k: 1.0 };
        }

        let inv_k = 1.0 - k;
        let c = (1.0 - r - k) / inv_k;
        let m = (1.0 - g - k) / inv_k;
        let y = (1.0 - b - k) / inv_k;

        Self {
            c: c.clamp(0.0, 1.0),
            m: m.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            k: k.clamp(0.0, 1.0),
        }
    }

    /// CMYK から sRGB 8-bit への変換 (ソフトプルーフ / 印刷プレビュー用)
    pub fn to_rgb(&self, a: u8) -> Color {
        let inv_k = 1.0 - self.k;
        let r = (1.0 - self.c) * inv_k;
        let g = (1.0 - self.m) * inv_k;
        let b = (1.0 - self.y) * inv_k;

        Color {
            r: (r.clamp(0.0, 1.0) * 255.0).round() as u8,
            g: (g.clamp(0.0, 1.0) * 255.0).round() as u8,
            b: (b.clamp(0.0, 1.0) * 255.0).round() as u8,
            a,
        }
    }

    /// 総インキ量 (Total Area Coverage: TAC / TIC)
    /// 日本の商業オフセット印刷基準 (Japan Color 2001 Coated) では 320% 〜 350% 以下が推奨
    pub fn total_ink_coverage(&self) -> f32 {
        (self.c + self.m + self.y + self.k) * 100.0
    }
}

/// 印刷校正・分版マネージャー
pub struct CmykManager;

impl CmykManager {
    /// 印刷総インキ量オーバー (TAC警告) ピクセルマスクの検出
    pub fn detect_tac_overrun(buffer: &crate::buffer::PixelBuffer, max_tac_percent: f32) -> Vec<bool> {
        let count = (buffer.width as usize) * (buffer.height as usize);
        let mut overrun = vec![false; count];

        for (i, chunk) in buffer.data.chunks_exact(4).enumerate() {
            if chunk[3] > 0 {
                let rgb = Color::rgba(chunk[0], chunk[1], chunk[2], chunk[3]);
                let cmyk = ColorCmyk::from_rgb(rgb);
                if cmyk.total_ink_coverage() > max_tac_percent {
                    overrun[i] = true;
                }
            }
        }
        overrun
    }
}
