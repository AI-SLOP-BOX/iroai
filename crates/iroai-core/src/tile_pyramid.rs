use crate::buffer::PixelBuffer;
use serde::{Deserialize, Serialize};

/// 超巨大キャンバス（8K/16K）向けマルチスケール・Mipmapピラミッド
/// レンダリングやズーム縮小表示の負荷を劇的に削減
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilePyramid {
    pub levels: Vec<PixelBuffer>, // level 0 = 100%, level 1 = 50%, level 2 = 25%...
}

impl TilePyramid {
    pub fn build(original: &PixelBuffer, max_levels: usize) -> Self {
        let mut levels = vec![original.clone()];
        let mut current = original.clone();

        for _ in 1..max_levels {
            let next_w = (current.width / 2).max(1);
            let next_h = (current.height / 2).max(1);
            if current.width <= 1 && current.height <= 1 {
                break;
            }

            let mut downscaled = PixelBuffer::new(next_w, next_h);
            for y in 0..next_h {
                for x in 0..next_w {
                    let sx = x * 2;
                    let sy = y * 2;

                    // 2x2 アルファ加重平均ボックスフィルタ (透明境界の黒ずみフリンジを防止)
                    let mut sum_r = 0.0f32;
                    let mut sum_g = 0.0f32;
                    let mut sum_b = 0.0f32;
                    let mut sum_a = 0.0f32;
                    let mut total_alpha_weight = 0.0f32;
                    let mut count = 0u32;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let px = (sx + dx).min(current.width - 1);
                            let py = (sy + dy).min(current.height - 1);
                            if let Some(col) = current.get_pixel(px, py) {
                                let a = col.a as f32 / 255.0;
                                sum_r += col.r as f32 * a;
                                sum_g += col.g as f32 * a;
                                sum_b += col.b as f32 * a;
                                sum_a += col.a as f32;
                                total_alpha_weight += a;
                                count += 1;
                            }
                        }
                    }

                    if count > 0 {
                        let avg_a = (sum_a / count as f32).round() as u8;
                        let (out_r, out_g, out_b) = if total_alpha_weight > 0.001 {
                            (
                                (sum_r / total_alpha_weight).clamp(0.0, 255.0).round() as u8,
                                (sum_g / total_alpha_weight).clamp(0.0, 255.0).round() as u8,
                                (sum_b / total_alpha_weight).clamp(0.0, 255.0).round() as u8,
                            )
                        } else {
                            (0, 0, 0)
                        };

                        downscaled.set_pixel(
                            x,
                            y,
                            crate::color::Color::rgba(out_r, out_g, out_b, avg_a),
                        );
                    }
                }
            }

            levels.push(downscaled.clone());
            current = downscaled;
        }

        Self { levels }
    }

    /// ズーム倍率に応じた最適Mipmapレベルバッファの取得
    pub fn get_level_for_zoom(&self, zoom: f32) -> &PixelBuffer {
        if zoom >= 0.75 || self.levels.len() <= 1 {
            &self.levels[0]
        } else if zoom >= 0.35 || self.levels.len() <= 2 {
            &self.levels[1]
        } else if zoom >= 0.15 || self.levels.len() <= 3 {
            &self.levels[2]
        } else {
            self.levels.last().unwrap()
        }
    }
}
