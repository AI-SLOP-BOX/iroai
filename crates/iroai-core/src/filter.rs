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
    MotionBlur { angle_deg: f32, distance: u32 },
    RadialBlur { center_x: f32, center_y: f32, strength: f32 },
    UnsharpMask { radius: u32, amount: f32, threshold: u8 },
    SobelEdge,
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

    /// Professional Gaussian Blur with Clamp-to-Edge boundary condition and Alpha-Weighted Normalization.
    /// Prevents dark edges and boundary shrinkage.
    pub fn apply_gaussian_blur(buffer: &mut PixelBuffer, radius: u32) {
        if radius == 0 || buffer.width == 0 || buffer.height == 0 {
            return;
        }
        let w = buffer.width as usize;
        let h = buffer.height as usize;
        let r = radius as i32;

        // Precompute 1D Gaussian kernel
        let sigma = (radius as f32) / 2.0;
        let two_sigma_sq = 2.0 * sigma * sigma;
        let mut kernel = Vec::with_capacity((r * 2 + 1) as usize);
        for i in -r..=r {
            let weight = (-((i * i) as f32) / two_sigma_sq).exp();
            kernel.push(weight);
        }

        let mut temp = vec![0.0f32; w * h * 4];

        // Horizontal Pass (Clamp-to-edge + alpha weighted)
        for y in 0..h {
            let row_offset = y * w;
            for x in 0..w {
                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                let mut sum_a = 0.0f32;
                let mut total_w = 0.0f32;

                for (idx, &k_w) in kernel.iter().enumerate() {
                    let dx = -r + idx as i32;
                    let nx = (x as i32 + dx).clamp(0, (w - 1) as i32) as usize;
                    let p_idx = (row_offset + nx) * 4;

                    let a = buffer.data[p_idx + 3] as f32 / 255.0;
                    let eff_w = k_w * a;

                    sum_r += buffer.data[p_idx] as f32 * eff_w;
                    sum_g += buffer.data[p_idx + 1] as f32 * eff_w;
                    sum_b += buffer.data[p_idx + 2] as f32 * eff_w;
                    sum_a += buffer.data[p_idx + 3] as f32 * k_w;
                    total_w += eff_w;
                }

                let dst_idx = (row_offset + x) * 4;
                let norm = if total_w > 0.001 { total_w } else { 1.0 };
                let k_norm: f32 = kernel.iter().sum();

                temp[dst_idx] = sum_r / norm;
                temp[dst_idx + 1] = sum_g / norm;
                temp[dst_idx + 2] = sum_b / norm;
                temp[dst_idx + 3] = sum_a / k_norm;
            }
        }

        // Vertical Pass (Clamp-to-edge + alpha weighted)
        for y in 0..h {
            for x in 0..w {
                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                let mut sum_a = 0.0f32;
                let mut total_w = 0.0f32;

                for (idx, &k_w) in kernel.iter().enumerate() {
                    let dy = -r + idx as i32;
                    let ny = (y as i32 + dy).clamp(0, (h - 1) as i32) as usize;
                    let p_idx = (ny * w + x) * 4;

                    let a = temp[p_idx + 3] / 255.0;
                    let eff_w = k_w * a;

                    sum_r += temp[p_idx] * eff_w;
                    sum_g += temp[p_idx + 1] * eff_w;
                    sum_b += temp[p_idx + 2] * eff_w;
                    sum_a += temp[p_idx + 3] * k_w;
                    total_w += eff_w;
                }

                let dst_idx = (y * w + x) * 4;
                let norm = if total_w > 0.001 { total_w } else { 1.0 };
                let k_norm: f32 = kernel.iter().sum();

                buffer.data[dst_idx] = (sum_r / norm).clamp(0.0, 255.0).round() as u8;
                buffer.data[dst_idx + 1] = (sum_g / norm).clamp(0.0, 255.0).round() as u8;
                buffer.data[dst_idx + 2] = (sum_b / norm).clamp(0.0, 255.0).round() as u8;
                buffer.data[dst_idx + 3] = (sum_a / k_norm).clamp(0.0, 255.0).round() as u8;
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

    pub fn apply_motion_blur(buffer: &mut PixelBuffer, angle_deg: f32, distance: u32) {
        if distance == 0 || buffer.width == 0 || buffer.height == 0 {
            return;
        }
        let rad = angle_deg.to_radians();
        let dx = rad.cos();
        let dy = rad.sin();
        let w = buffer.width as i32;
        let h = buffer.height as i32;
        let dist = distance as i32;
        let samples = (dist * 2 + 1) as usize;

        let src = buffer.clone();

        for y in 0..h {
            for x in 0..w {
                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                let mut sum_a = 0.0f32;

                for step in -dist..=dist {
                    let sx = ((x as f32 + step as f32 * dx).round() as i32).clamp(0, w - 1) as u32;
                    let sy = ((y as f32 + step as f32 * dy).round() as i32).clamp(0, h - 1) as u32;
                    if let Some(col) = src.get_pixel(sx, sy) {
                        sum_r += col.r as f32;
                        sum_g += col.g as f32;
                        sum_b += col.b as f32;
                        sum_a += col.a as f32;
                    }
                }

                let inv = 1.0 / samples as f32;
                let out_col = crate::color::Color::rgba(
                    (sum_r * inv).clamp(0.0, 255.0).round() as u8,
                    (sum_g * inv).clamp(0.0, 255.0).round() as u8,
                    (sum_b * inv).clamp(0.0, 255.0).round() as u8,
                    (sum_a * inv).clamp(0.0, 255.0).round() as u8,
                );
                buffer.set_pixel(x as u32, y as u32, out_col);
            }
        }
    }

    pub fn apply_radial_blur(buffer: &mut PixelBuffer, center_x: f32, center_y: f32, strength: f32) {
        if strength <= 0.0 || buffer.width == 0 || buffer.height == 0 {
            return;
        }
        let w = buffer.width as i32;
        let h = buffer.height as i32;
        let steps = 16;
        let src = buffer.clone();

        for y in 0..h {
            for x in 0..w {
                let vx = x as f32 - center_x;
                let vy = y as f32 - center_y;
                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                let mut sum_a = 0.0f32;

                for i in 0..steps {
                    let scale = 1.0 - (strength * 0.02) * (i as f32 / steps as f32);
                    let sx = ((center_x + vx * scale).round() as i32).clamp(0, w - 1) as u32;
                    let sy = ((center_y + vy * scale).round() as i32).clamp(0, h - 1) as u32;
                    if let Some(col) = src.get_pixel(sx, sy) {
                        sum_r += col.r as f32;
                        sum_g += col.g as f32;
                        sum_b += col.b as f32;
                        sum_a += col.a as f32;
                    }
                }

                let inv = 1.0 / steps as f32;
                buffer.set_pixel(
                    x as u32,
                    y as u32,
                    crate::color::Color::rgba(
                        (sum_r * inv).clamp(0.0, 255.0).round() as u8,
                        (sum_g * inv).clamp(0.0, 255.0).round() as u8,
                        (sum_b * inv).clamp(0.0, 255.0).round() as u8,
                        (sum_a * inv).clamp(0.0, 255.0).round() as u8,
                    ),
                );
            }
        }
    }

    pub fn apply_unsharp_mask(buffer: &mut PixelBuffer, radius: u32, amount: f32, threshold: u8) {
        let mut blurred = buffer.clone();
        Self::apply_gaussian_blur(&mut blurred, radius);
        let th = threshold as i32;

        for (orig_chunk, blur_chunk) in buffer.data.chunks_exact_mut(4).zip(blurred.data.chunks_exact(4)) {
            for i in 0..3 {
                let orig = orig_chunk[i] as i32;
                let blur = blur_chunk[i] as i32;
                let diff = orig - blur;
                if diff.abs() >= th {
                    let sharpened = orig as f32 + diff as f32 * amount;
                    orig_chunk[i] = sharpened.clamp(0.0, 255.0).round() as u8;
                }
            }
        }
    }

    pub fn apply_sobel_edge(buffer: &mut PixelBuffer) {
        let w = buffer.width as i32;
        let h = buffer.height as i32;
        if w < 3 || h < 3 {
            return;
        }
        let src = buffer.clone();

        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let mut gx = 0.0f32;
                let mut gy = 0.0f32;

                // Sobel Kernels:
                // Gx: [-1 0 1, -2 0 2, -1 0 1]
                // Gy: [-1 -2 -1,  0  0  0,  1  2  1]
                let kernel_x = [
                    (-1, -1, -1.0), (1, -1, 1.0),
                    (-1,  0, -2.0), (1,  0, 2.0),
                    (-1,  1, -1.0), (1,  1, 1.0),
                ];
                let kernel_y = [
                    (-1, -1, -1.0), (0, -1, -2.0), (1, -1, -1.0),
                    (-1,  1,  1.0), (0,  1,  2.0), (1,  1,  1.0),
                ];

                for (dx, dy, k) in kernel_x {
                    if let Some(col) = src.get_pixel((x + dx) as u32, (y + dy) as u32) {
                        let lum = (col.r as f32 * 0.299) + (col.g as f32 * 0.587) + (col.b as f32 * 0.114);
                        gx += lum * k;
                    }
                }
                for (dx, dy, k) in kernel_y {
                    if let Some(col) = src.get_pixel((x + dx) as u32, (y + dy) as u32) {
                        let lum = (col.r as f32 * 0.299) + (col.g as f32 * 0.587) + (col.b as f32 * 0.114);
                        gy += lum * k;
                    }
                }

                let mag = (gx * gx + gy * gy).sqrt().clamp(0.0, 255.0).round() as u8;
                buffer.set_pixel(x as u32, y as u32, crate::color::Color::rgba(mag, mag, mag, 255));
            }
        }
    }

    /// Photoshop-grade Spot Healing (Texture Repair) using annular boundary sampling and bilateral harmonic blending.
    pub fn apply_spot_heal(buffer: &mut PixelBuffer, center_x: f32, center_y: f32, radius: f32) {
        if radius <= 1.0 || buffer.width == 0 || buffer.height == 0 {
            return;
        }

        let r = radius;
        let r_outer = r * 1.4;
        let r2_inner = r * r;
        let r2_outer = r_outer * r_outer;

        let min_x = (center_x - r_outer).floor().max(0.0) as u32;
        let max_x = (center_x + r_outer).ceil().min(buffer.width as f32 - 1.0).max(0.0) as u32;
        let min_y = (center_y - r_outer).floor().max(0.0) as u32;
        let max_y = (center_y + r_outer).ceil().min(buffer.height as f32 - 1.0).max(0.0) as u32;

        if min_x >= max_x || min_y >= max_y {
            return;
        }

        // 1. Sample boundary ring pixels outside inner radius to compute surrounding texture & color
        let mut sum_r = 0.0f32;
        let mut sum_g = 0.0f32;
        let mut sum_b = 0.0f32;
        let mut ring_samples = Vec::new();

        for y in min_y..=max_y {
            let dy = y as f32 - center_y;
            for x in min_x..=max_x {
                let dx = x as f32 - center_x;
                let d2 = dx * dx + dy * dy;
                if d2 >= r2_inner && d2 <= r2_outer {
                    if let Some(col) = buffer.get_pixel(x, y) {
                        sum_r += col.r as f32;
                        sum_g += col.g as f32;
                        sum_b += col.b as f32;
                        ring_samples.push((dx, dy, col));
                    }
                }
            }
        }

        if ring_samples.is_empty() {
            return;
        }

        let avg_r = sum_r / ring_samples.len() as f32;
        let avg_g = sum_g / ring_samples.len() as f32;
        let avg_b = sum_b / ring_samples.len() as f32;

        // 2. Synthesize inner flawed region via Inverse-Distance Weighting (IDW) + edge feathering
        let src = buffer.clone();
        for y in min_y..=max_y {
            let dy = y as f32 - center_y;
            for x in min_x..=max_x {
                let dx = x as f32 - center_x;
                let d2 = dx * dx + dy * dy;

                if d2 < r2_inner {
                    let d = d2.sqrt();
                    let norm_d = d / r; // 0.0 at center, 1.0 at boundary

                    // Calculate distance-weighted boundary color
                    let mut weight_sum = 0.0f32;
                    let mut blend_r = 0.0f32;
                    let mut blend_g = 0.0f32;
                    let mut blend_b = 0.0f32;

                    for &(sx, sy, col) in &ring_samples {
                        let dist_sq = (dx - sx) * (dx - sx) + (dy - sy) * (dy - sy) + 1.0;
                        let w = 1.0 / dist_sq;
                        blend_r += col.r as f32 * w;
                        blend_g += col.g as f32 * w;
                        blend_b += col.b as f32 * w;
                        weight_sum += w;
                    }

                    let idw_r = if weight_sum > 0.0 { blend_r / weight_sum } else { avg_r };
                    let idw_g = if weight_sum > 0.0 { blend_g / weight_sum } else { avg_g };
                    let idw_b = if weight_sum > 0.0 { blend_b / weight_sum } else { avg_b };

                    // Bell-shaped smooth blending towards boundaries
                    let t = (1.0 - norm_d).powi(2);
                    let orig = src.get_pixel(x, y).unwrap_or(crate::color::Color::BLACK);

                    let out_r = (orig.r as f32 * (1.0 - t) + idw_r * t).clamp(0.0, 255.0).round() as u8;
                    let out_g = (orig.g as f32 * (1.0 - t) + idw_g * t).clamp(0.0, 255.0).round() as u8;
                    let out_b = (orig.b as f32 * (1.0 - t) + idw_b * t).clamp(0.0, 255.0).round() as u8;

                    buffer.set_pixel(x, y, crate::color::Color::rgba(out_r, out_g, out_b, orig.a));
                }
            }
        }
    }

    /// Content-Aware Scaling (Seam Carving)
    /// Dynamically carves out low-energy vertical/horizontal seams to preserve important focal objects.
    pub fn seam_carve_resize(buffer: &PixelBuffer, target_width: u32, target_height: u32) -> PixelBuffer {
        let mut cur = buffer.clone();
        if target_width == 0 || target_height == 0 {
            return PixelBuffer::new(target_width, target_height);
        }

        // Reduce width if needed
        while cur.width > target_width {
            let w = cur.width as usize;
            let h = cur.height as usize;

            // 1. Calculate dual-gradient energy map e(x, y) = dx^2 + dy^2
            let mut energy = vec![0.0f32; w * h];
            for y in 0..h {
                for x in 0..w {
                    let left_x = if x == 0 { w - 1 } else { x - 1 };
                    let right_x = if x == w - 1 { 0 } else { x + 1 };
                    let up_y = if y == 0 { h - 1 } else { y - 1 };
                    let down_y = if y == h - 1 { 0 } else { y + 1 };

                    let p_left = (y * w + left_x) * 4;
                    let p_right = (y * w + right_x) * 4;
                    let p_up = (up_y * w + x) * 4;
                    let p_down = (down_y * w + x) * 4;

                    let mut dx2 = 0.0f32;
                    let mut dy2 = 0.0f32;
                    for c in 0..3 {
                        let rx = cur.data[p_right + c] as f32 - cur.data[p_left + c] as f32;
                        let ry = cur.data[p_down + c] as f32 - cur.data[p_up + c] as f32;
                        dx2 += rx * rx;
                        dy2 += ry * ry;
                    }
                    energy[y * w + x] = dx2 + dy2;
                }
            }

            // 2. Dynamic programming: compute cumulative minimum energy matrix M
            let mut dp = vec![0.0f32; w * h];
            for x in 0..w {
                dp[x] = energy[x];
            }
            for y in 1..h {
                for x in 0..w {
                    let mut min_prev = dp[(y - 1) * w + x];
                    if x > 0 {
                        min_prev = min_prev.min(dp[(y - 1) * w + (x - 1)]);
                    }
                    if x + 1 < w {
                        min_prev = min_prev.min(dp[(y - 1) * w + (x + 1)]);
                    }
                    dp[y * w + x] = energy[y * w + x] + min_prev;
                }
            }

            // 3. Find minimum energy seam starting at the bottom row
            let mut min_x = 0;
            let mut min_val = f32::INFINITY;
            for x in 0..w {
                let val = dp[(h - 1) * w + x];
                if val < min_val {
                    min_val = val;
                    min_x = x;
                }
            }

            // Backtrack seam path
            let mut seam = vec![0usize; h];
            seam[h - 1] = min_x;
            for y in (0..h - 1).rev() {
                let prev_x = seam[y + 1];
                let mut best_x = prev_x;
                let mut best_val = dp[y * w + prev_x];
                if prev_x > 0 && dp[y * w + (prev_x - 1)] < best_val {
                    best_val = dp[y * w + (prev_x - 1)];
                    best_x = prev_x - 1;
                }
                if prev_x + 1 < w && dp[y * w + (prev_x + 1)] < best_val {
                    best_x = prev_x + 1;
                }
                seam[y] = best_x;
            }

            // 4. Carve seam out: copy remaining pixels into a buffer of width w - 1
            let new_w = (w - 1) as u32;
            let mut next_buf = PixelBuffer::new(new_w, h as u32);
            for y in 0..h {
                let sx = seam[y];
                let mut dst_x = 0;
                for x in 0..w {
                    if x == sx {
                        continue;
                    }
                    let src_idx = (y * w + x) * 4;
                    let dst_idx = (y * (new_w as usize) + dst_x) * 4;
                    next_buf.data[dst_idx..dst_idx + 4].copy_from_slice(&cur.data[src_idx..src_idx + 4]);
                    dst_x += 1;
                }
            }
            cur = next_buf;
        }

        // Reduce height if needed (by carving horizontal seams)
        while cur.height > target_height {
            let w = cur.width as usize;
            let h = cur.height as usize;

            // 1. Dual-gradient energy
            let mut energy = vec![0.0f32; w * h];
            for y in 0..h {
                for x in 0..w {
                    let left_x = if x == 0 { w - 1 } else { x - 1 };
                    let right_x = if x == w - 1 { 0 } else { x + 1 };
                    let up_y = if y == 0 { h - 1 } else { y - 1 };
                    let down_y = if y == h - 1 { 0 } else { y + 1 };

                    let p_left = (y * w + left_x) * 4;
                    let p_right = (y * w + right_x) * 4;
                    let p_up = (up_y * w + x) * 4;
                    let p_down = (down_y * w + x) * 4;

                    let mut dx2 = 0.0f32;
                    let mut dy2 = 0.0f32;
                    for c in 0..3 {
                        let rx = cur.data[p_right + c] as f32 - cur.data[p_left + c] as f32;
                        let ry = cur.data[p_down + c] as f32 - cur.data[p_up + c] as f32;
                        dx2 += rx * rx;
                        dy2 += ry * ry;
                    }
                    energy[y * w + x] = dx2 + dy2;
                }
            }

            // 2. DP left-to-right
            let mut dp = vec![0.0f32; w * h];
            for y in 0..h {
                dp[y * w] = energy[y * w];
            }
            for x in 1..w {
                for y in 0..h {
                    let mut min_prev = dp[y * w + (x - 1)];
                    if y > 0 {
                        min_prev = min_prev.min(dp[(y - 1) * w + (x - 1)]);
                    }
                    if y + 1 < h {
                        min_prev = min_prev.min(dp[(y + 1) * w + (x - 1)]);
                    }
                    dp[y * w + x] = energy[y * w + x] + min_prev;
                }
            }

            // 3. Find horizontal seam starting at rightmost column
            let mut min_y = 0;
            let mut min_val = f32::INFINITY;
            for y in 0..h {
                let val = dp[y * w + (w - 1)];
                if val < min_val {
                    min_val = val;
                    min_y = y;
                }
            }

            let mut seam = vec![0usize; w];
            seam[w - 1] = min_y;
            for x in (0..w - 1).rev() {
                let prev_y = seam[x + 1];
                let mut best_y = prev_y;
                let mut best_val = dp[prev_y * w + x];
                if prev_y > 0 && dp[(prev_y - 1) * w + x] < best_val {
                    best_val = dp[(prev_y - 1) * w + x];
                    best_y = prev_y - 1;
                }
                if prev_y + 1 < h && dp[(prev_y + 1) * w + x] < best_val {
                    best_y = prev_y + 1;
                }
                seam[x] = best_y;
            }

            // 4. Carve horizontal seam
            let new_h = (h - 1) as u32;
            let mut next_buf = PixelBuffer::new(w as u32, new_h);
            for x in 0..w {
                let sy = seam[x];
                let mut dst_y = 0;
                for y in 0..h {
                    if y == sy {
                        continue;
                    }
                    let src_idx = (y * w + x) * 4;
                    let dst_idx = (dst_y * w + x) * 4;
                    next_buf.data[dst_idx..dst_idx + 4].copy_from_slice(&cur.data[src_idx..src_idx + 4]);
                    dst_y += 1;
                }
            }
            cur = next_buf;
        }

        // If target size is larger than current, bicubic scale up
        if cur.width != target_width || cur.height != target_height {
            cur = crate::transform::Transform::resize_bicubic(&cur, target_width, target_height);
        }

        cur
    }
}
