use crate::buffer::PixelBuffer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterType {
    BrightnessContrast { brightness: f32, contrast: f32 },
    HueSaturation { hue_shift: f32, saturation: f32 },
    Exposure { ev: f32 },
    Invert,
    Grayscale,
    Threshold { cutoff: u8 },
    Posterize { levels: u8 },
    GaussianBlur { radius: u32 },
    Sharpen { strength: f32 },
}

pub struct Filters;

impl Filters {
    pub fn apply_brightness_contrast(buffer: &mut PixelBuffer, brightness: f32, contrast: f32) {
        let b = brightness * 255.0;
        let c = contrast.clamp(-1.0, 1.0);
        let factor = (259.0 * (c * 255.0 + 255.0)) / (255.0 * (259.0 - c * 255.0));

        for chunk in buffer.data.chunks_exact_mut(4) {
            for i in 0..3 {
                let val = chunk[i] as f32;
                let adjusted = factor * (val - 128.0) + 128.0 + b;
                chunk[i] = adjusted.clamp(0.0, 255.0).round() as u8;
            }
        }
    }

    pub fn apply_exposure(buffer: &mut PixelBuffer, ev: f32) {
        let mult = 2.0f32.powf(ev);
        for chunk in buffer.data.chunks_exact_mut(4) {
            for i in 0..3 {
                let val = chunk[i] as f32 * mult;
                chunk[i] = val.clamp(0.0, 255.0).round() as u8;
            }
        }
    }

    pub fn apply_hue_saturation(buffer: &mut PixelBuffer, hue_shift_deg: f32, sat_mult: f32) {
        for chunk in buffer.data.chunks_exact_mut(4) {
            let r = chunk[0] as f32 / 255.0;
            let g = chunk[1] as f32 / 255.0;
            let b = chunk[2] as f32 / 255.0;

            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let delta = max - min;

            let mut h = if delta == 0.0 {
                0.0
            } else if max == r {
                60.0 * (((g - b) / delta) % 6.0)
            } else if max == g {
                60.0 * (((b - r) / delta) + 2.0)
            } else {
                60.0 * (((r - g) / delta) + 4.0)
            };
            if h < 0.0 {
                h += 360.0;
            }

            let mut s = if max == 0.0 { 0.0 } else { delta / max };
            let v = max;

            h = (h + hue_shift_deg).rem_euclid(360.0);
            s = (s * sat_mult).clamp(0.0, 1.0);

            let c = v * s;
            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
            let m = v - c;

            let (r1, g1, b1) = match (h / 60.0) as u32 {
                0 => (c, x, 0.0),
                1 => (x, c, 0.0),
                2 => (0.0, c, x),
                3 => (0.0, x, c),
                4 => (x, 0.0, c),
                _ => (c, 0.0, x),
            };

            chunk[0] = ((r1 + m) * 255.0).round() as u8;
            chunk[1] = ((g1 + m) * 255.0).round() as u8;
            chunk[2] = ((b1 + m) * 255.0).round() as u8;
        }
    }

    pub fn apply_invert(buffer: &mut PixelBuffer) {
        for chunk in buffer.data.chunks_exact_mut(4) {
            chunk[0] = 255 - chunk[0];
            chunk[1] = 255 - chunk[1];
            chunk[2] = 255 - chunk[2];
        }
    }

    pub fn apply_grayscale(buffer: &mut PixelBuffer) {
        for chunk in buffer.data.chunks_exact_mut(4) {
            let gray = (chunk[0] as u32 * 77 + chunk[1] as u32 * 150 + chunk[2] as u32 * 29) >> 8;
            let g8 = gray as u8;
            chunk[0] = g8;
            chunk[1] = g8;
            chunk[2] = g8;
        }
    }

    pub fn apply_threshold(buffer: &mut PixelBuffer, cutoff: u8) {
        for chunk in buffer.data.chunks_exact_mut(4) {
            let lum = (chunk[0] as u32 * 77 + chunk[1] as u32 * 150 + chunk[2] as u32 * 29) >> 8;
            let out = if lum as u8 >= cutoff { 255 } else { 0 };
            chunk[0] = out;
            chunk[1] = out;
            chunk[2] = out;
        }
    }

    pub fn apply_posterize(buffer: &mut PixelBuffer, levels: u8) {
        let lvl = levels.clamp(2, 32) as f32;
        for chunk in buffer.data.chunks_exact_mut(4) {
            for i in 0..3 {
                let v = chunk[i] as f32 / 255.0;
                let q = (v * (lvl - 1.0)).round() / (lvl - 1.0);
                chunk[i] = (q * 255.0).clamp(0.0, 255.0).round() as u8;
            }
        }
    }

    pub fn apply_gaussian_blur(buffer: &mut PixelBuffer, radius: u32) {
        if radius == 0 {
            return;
        }
        let w = buffer.width as usize;
        let h = buffer.height as usize;
        let r = radius as i32;

        let mut temp = buffer.data.clone();

        // 水平
        for y in 0..h {
            let row_offset = y * w * 4;
            for x in 0..w {
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;
                let mut sum_a = 0u32;
                let mut count = 0u32;

                for dx in -r..=r {
                    let nx = x as i32 + dx;
                    if nx >= 0 && nx < w as i32 {
                        let idx = row_offset + (nx as usize) * 4;
                        sum_r += buffer.data[idx] as u32;
                        sum_g += buffer.data[idx + 1] as u32;
                        sum_b += buffer.data[idx + 2] as u32;
                        sum_a += buffer.data[idx + 3] as u32;
                        count += 1;
                    }
                }

                let dst_idx = row_offset + x * 4;
                temp[dst_idx] = (sum_r / count) as u8;
                temp[dst_idx + 1] = (sum_g / count) as u8;
                temp[dst_idx + 2] = (sum_b / count) as u8;
                temp[dst_idx + 3] = (sum_a / count) as u8;
            }
        }

        // 垂直
        for y in 0..h {
            for x in 0..w {
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;
                let mut sum_a = 0u32;
                let mut count = 0u32;

                for dy in -r..=r {
                    let ny = y as i32 + dy;
                    if ny >= 0 && ny < h as i32 {
                        let idx = ((ny as usize) * w + x) * 4;
                        sum_r += temp[idx] as u32;
                        sum_g += temp[idx + 1] as u32;
                        sum_b += temp[idx + 2] as u32;
                        sum_a += temp[idx + 3] as u32;
                        count += 1;
                    }
                }

                let dst_idx = (y * w + x) * 4;
                buffer.data[dst_idx] = (sum_r / count) as u8;
                buffer.data[dst_idx + 1] = (sum_g / count) as u8;
                buffer.data[dst_idx + 2] = (sum_b / count) as u8;
                buffer.data[dst_idx + 3] = (sum_a / count) as u8;
            }
        }
    }

    pub fn apply_sharpen(buffer: &mut PixelBuffer, strength: f32) {
        let mut temp = buffer.clone();
        Self::apply_gaussian_blur(&mut temp, 1);
        for i in (0..buffer.data.len()).step_by(4) {
            for c in 0..3 {
                let orig = buffer.data[i + c] as f32;
                let blurred = temp.data[i + c] as f32;
                let high_pass = orig - blurred;
                let sharpened = orig + high_pass * strength;
                buffer.data[i + c] = sharpened.clamp(0.0, 255.0).round() as u8;
            }
        }
    }

    pub fn flood_fill(buffer: &mut PixelBuffer, start_x: u32, start_y: u32, fill_color: crate::color::Color, tolerance: u8) {
        if start_x >= buffer.width || start_y >= buffer.height {
            return;
        }
        let target_col = buffer.get_pixel(start_x, start_y).unwrap_or(crate::color::Color::TRANSPARENT);
        if target_col == fill_color {
            return;
        }

        let w = buffer.width;
        let h = buffer.height;
        let mut queue = std::collections::VecDeque::new();
        let mut visited = vec![false; (w * h) as usize];

        queue.push_back((start_x, start_y));
        visited[(start_y * w + start_x) as usize] = true;

        let tol = tolerance as i32;

        while let Some((x, y)) = queue.pop_front() {
            buffer.set_pixel(x, y, fill_color);

            let neighbors = [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ];

            for (nx, ny) in neighbors {
                if nx < w && ny < h {
                    let idx = (ny * w + nx) as usize;
                    if !visited[idx] {
                        if let Some(col) = buffer.get_pixel(nx, ny) {
                            let dr = (col.r as i32 - target_col.r as i32).abs();
                            let dg = (col.g as i32 - target_col.g as i32).abs();
                            let db = (col.b as i32 - target_col.b as i32).abs();
                            let da = (col.a as i32 - target_col.a as i32).abs();

                            if dr <= tol && dg <= tol && db <= tol && da <= tol {
                                visited[idx] = true;
                                queue.push_back((nx, ny));
                            }
                        }
                    }
                }
            }
        }
    }
}
