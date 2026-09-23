use crate::buffer::PixelBuffer;
use crate::color::{Color, ColorHdr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Srgb,
    LinearSrgb,
    DisplayP3,
    AdobeRgb,
}

impl ColorSpace {
    pub fn name(&self) -> &'static str {
        match self {
            ColorSpace::Srgb => "sRGB IEC61966-2.1",
            ColorSpace::LinearSrgb => "Linear sRGB (scRGB)",
            ColorSpace::DisplayP3 => "Display P3 (Wide Gamut)",
            ColorSpace::AdobeRgb => "Adobe RGB (1998)",
        }
    }
}

/// ICC プロファイル情報とメタデータ保持
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IccProfile {
    pub name: String,
    pub color_space: ColorSpace,
    pub raw_profile_data: Option<Vec<u8>>,
}

impl Default for IccProfile {
    fn default() -> Self {
        Self {
            name: "sRGB IEC61966-2.1".to_string(),
            color_space: ColorSpace::Srgb,
            raw_profile_data: None,
        }
    }
}

/// 16-bit / 32-bit 高精度浮動小数点バッファ (HDR / 広色域対応基盤)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HdrBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<ColorHdr>, // 32-bit float RGBA per pixel
}

impl HdrBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            data: vec![ColorHdr::default(); size],
        }
    }

    pub fn from_pixel_buffer(buffer: &PixelBuffer) -> Self {
        let count = (buffer.width as usize) * (buffer.height as usize);
        let mut data = Vec::with_capacity(count);
        for chunk in buffer.data.chunks_exact(4) {
            let c = Color::rgba(chunk[0], chunk[1], chunk[2], chunk[3]);
            data.push(ColorHdr::from_color8(c));
        }
        Self {
            width: buffer.width,
            height: buffer.height,
            data,
        }
    }

    pub fn to_pixel_buffer(&self) -> PixelBuffer {
        let mut buffer = PixelBuffer::new(self.width, self.height);
        for (i, hdr) in self.data.iter().enumerate() {
            let c = hdr.to_color8();
            let idx = i * 4;
            buffer.data[idx] = c.r;
            buffer.data[idx + 1] = c.g;
            buffer.data[idx + 2] = c.b;
            buffer.data[idx + 3] = c.a;
        }
        buffer
    }

    pub fn apply_exposure(&mut self, ev: f32) {
        let factor = 2.0f32.powf(ev);
        for px in &mut self.data {
            px.r *= factor;
            px.g *= factor;
            px.b *= factor;
        }
    }
}

/// 写真補正用パラメータ（非破壊設定保持用）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoAdjustments {
    pub exposure: f32,       // -5.0 ..= +5.0 EV
    pub highlights: f32,     // -100.0 ..= +100.0
    pub shadows: f32,        // -100.0 ..= +100.0
    pub whites: f32,         // -100.0 ..= +100.0
    pub blacks: f32,         // -100.0 ..= +100.0
    pub temperature: f32,    // -100.0 (Cold/Blue) ..= +100.0 (Warm/Yellow)
    pub tint: f32,           // -100.0 (Green) ..= +100.0 (Magenta)
    pub vibrance: f32,       // -100.0 ..= +100.0
    pub saturation: f32,     // -100.0 ..= +100.0
    pub clarity: f32,        // -100.0 ..= +100.0
    pub sharpness: f32,      // 0.0 ..= 100.0
    pub noise_reduction: f32,// 0.0 ..= 100.0
    pub vignette: f32,       // -100.0 (Dark edge) ..= +100.0 (Light edge)
    pub distortion: f32,     // -100.0 (Barrel) ..= +100.0 (Pincushion)
}

impl Default for PhotoAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            temperature: 0.0,
            tint: 0.0,
            vibrance: 0.0,
            saturation: 0.0,
            clarity: 0.0,
            sharpness: 0.0,
            noise_reduction: 0.0,
            vignette: 0.0,
            distortion: 0.0,
        }
    }
}

pub struct PhotoProcessor;

impl PhotoProcessor {
    /// 露光量、ハイライト・シャドウ、白レベル・黒レベル、WB、自然な彩度、周辺減光の一括リニア/浮動小数点処理
    pub fn apply_photo_adjustments(buffer: &mut PixelBuffer, adj: &PhotoAdjustments) {
        let exp_factor = 2.0f32.powf(adj.exposure);
        let temp_r = 1.0 + (adj.temperature / 200.0);
        let temp_b = 1.0 - (adj.temperature / 200.0);
        let tint_g = 1.0 - (adj.tint / 200.0);

        let hl_adj = adj.highlights / 100.0;
        let sh_adj = adj.shadows / 100.0;
        let w_adj = adj.whites / 100.0;
        let blk_adj = adj.blacks / 100.0;

        let sat_mult = 1.0 + adj.saturation / 100.0;
        let vib_factor = adj.vibrance / 100.0;

        let w = buffer.width as f32;
        let h = buffer.height as f32;
        let cx = w * 0.5;
        let cy = h * 0.5;
        let max_r = (cx * cx + cy * cy).sqrt().max(1.0);

        let vig_amount = adj.vignette / 100.0;

        for y in 0..buffer.height {
            let row_offset = (y as usize) * (buffer.width as usize) * 4;
            let dy = y as f32 - cy;

            for x in 0..buffer.width {
                let idx = row_offset + (x as usize) * 4;
                if buffer.data[idx + 3] == 0 {
                    continue;
                }

                let color = Color::rgba(
                    buffer.data[idx],
                    buffer.data[idx + 1],
                    buffer.data[idx + 2],
                    buffer.data[idx + 3],
                );

                // sRGB -> Linear RGB f32
                let (mut lr, mut lg, mut lb, la) = color.to_linear_f32();

                // 1. 露光量 (Exposure in linear space)
                lr *= exp_factor;
                lg *= exp_factor;
                lb *= exp_factor;

                // 2. ホワイトバランス (Color Temperature & Tint in linear space)
                lr *= temp_r;
                lg *= tint_g;
                lb *= temp_b;

                // 3. 輝度に基づくハイライト・シャドウ・白レベル・黒レベル
                let lum = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;

                if hl_adj != 0.0 && lum > 0.5 {
                    let weight = ((lum - 0.5) * 2.0).clamp(0.0, 1.0);
                    let scale = (1.0 + hl_adj * weight * 0.5).max(0.0);
                    lr *= scale;
                    lg *= scale;
                    lb *= scale;
                }

                if sh_adj != 0.0 && lum < 0.5 {
                    let weight = ((0.5 - lum) * 2.0).clamp(0.0, 1.0);
                    let scale = (1.0 + sh_adj * weight * 0.6).max(0.0);
                    lr *= scale;
                    lg *= scale;
                    lb *= scale;
                }

                if w_adj != 0.0 && lum > 0.7 {
                    let weight = ((lum - 0.7) / 0.3).clamp(0.0, 1.0);
                    let scale = (1.0 + w_adj * weight * 0.4).max(0.0);
                    lr *= scale;
                    lg *= scale;
                    lb *= scale;
                }

                if blk_adj != 0.0 && lum < 0.3 {
                    let weight = ((0.3 - lum) / 0.3).clamp(0.0, 1.0);
                    let offset = blk_adj * weight * 0.1;
                    lr = (lr + offset).max(0.0);
                    lg = (lg + offset).max(0.0);
                    lb = (lb + offset).max(0.0);
                }

                // 4. 周辺光量 (Vignette)
                if vig_amount != 0.0 {
                    let dx = x as f32 - cx;
                    let dist = (dx * dx + dy * dy).sqrt() / max_r;
                    let factor = (1.0 + vig_amount * dist * dist).clamp(0.0, 2.0);
                    lr *= factor;
                    lg *= factor;
                    lb *= factor;
                }

                // Linear RGB -> sRGB 8-bit
                let mut adjusted_col = Color::from_linear_f32(lr, lg, lb, la);

                // 5. 彩度・自然な彩度 (Vibrance & Saturation)
                if sat_mult != 1.0 || vib_factor != 0.0 {
                    let cr = adjusted_col.r as f32;
                    let cg = adjusted_col.g as f32;
                    let cb = adjusted_col.b as f32;
                    let current_sat = (cr.max(cg).max(cb) - cr.min(cg).min(cb)) / 255.0;
                    let gray = 0.299 * cr + 0.587 * cg + 0.114 * cb;

                    // Vibrance applies more to less-saturated colors
                    let effective_mult = sat_mult + vib_factor * (1.0 - current_sat);

                    adjusted_col.r = (gray + (cr - gray) * effective_mult).clamp(0.0, 255.0).round() as u8;
                    adjusted_col.g = (gray + (cg - gray) * effective_mult).clamp(0.0, 255.0).round() as u8;
                    adjusted_col.b = (gray + (cb - gray) * effective_mult).clamp(0.0, 255.0).round() as u8;
                }

                buffer.data[idx] = adjusted_col.r;
                buffer.data[idx + 1] = adjusted_col.g;
                buffer.data[idx + 2] = adjusted_col.b;
            }
        }

        // 6. シャープネス
        if adj.sharpness > 0.0 {
            crate::filter::Filters::apply_sharpen(buffer, adj.sharpness / 50.0);
        }

        // 7. ノイズ低減
        if adj.noise_reduction > 0.0 {
            let radius = (adj.noise_reduction / 35.0).ceil() as u32;
            crate::filter::Filters::apply_gaussian_blur(buffer, radius.max(1));
        }
    }

    /// トーンカーブ補正 (0..=255 のLUTマップ適用)
    pub fn apply_tone_curve(buffer: &mut PixelBuffer, curve_lut: &[u8; 256]) {
        for chunk in buffer.data.chunks_exact_mut(4) {
            chunk[0] = curve_lut[chunk[0] as usize];
            chunk[1] = curve_lut[chunk[1] as usize];
            chunk[2] = curve_lut[chunk[2] as usize];
        }
    }

    /// レベル補正 (Black Point, Midtone Gamma, White Point)
    pub fn apply_levels(buffer: &mut PixelBuffer, black_point: u8, gamma: f32, white_point: u8) {
        let bp = black_point as f32;
        let wp = (white_point as f32).max(bp + 1.0);
        let g = gamma.max(0.01);

        let mut lut = [0u8; 256];
        for i in 0..256 {
            let v = i as f32;
            let norm = ((v - bp) / (wp - bp)).clamp(0.0, 1.0);
            let mapped = norm.powf(1.0 / g);
            lut[i] = (mapped * 255.0).round() as u8;
        }

        Self::apply_tone_curve(buffer, &lut);
    }

    /// チャンネルミキサー (RGBマトリクス変換)
    pub fn apply_channel_mixer(
        buffer: &mut PixelBuffer,
        matrix: [[f32; 3]; 3],
    ) {
        for chunk in buffer.data.chunks_exact_mut(4) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;

            let out_r = matrix[0][0] * r + matrix[0][1] * g + matrix[0][2] * b;
            let out_g = matrix[1][0] * r + matrix[1][1] * g + matrix[1][2] * b;
            let out_b = matrix[2][0] * r + matrix[2][1] * g + matrix[2][2] * b;

            chunk[0] = out_r.clamp(0.0, 255.0).round() as u8;
            chunk[1] = out_g.clamp(0.0, 255.0).round() as u8;
            chunk[2] = out_b.clamp(0.0, 255.0).round() as u8;
        }
    }

    /// 色域外警告（Gamut Warning）ピクセルマスクの生成
    pub fn detect_out_of_gamut(buffer: &PixelBuffer, target_space: ColorSpace) -> Vec<bool> {
        let count = (buffer.width as usize) * (buffer.height as usize);
        let mut out_of_gamut = vec![false; count];

        for (i, chunk) in buffer.data.chunks_exact(4).enumerate() {
            match target_space {
                ColorSpace::DisplayP3 | ColorSpace::Srgb => {
                    let r = chunk[0];
                    let g = chunk[1];
                    let b = chunk[2];
                    if (r == 255 || g == 255 || b == 255 || (r == 0 && g == 0 && b == 0)) && chunk[3] > 0 {
                        out_of_gamut[i] = true;
                    }
                }
                _ => {}
            }
        }
        out_of_gamut
    }
}
