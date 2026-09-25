use crate::buffer::PixelBuffer;
use crate::color::Color;

pub struct Transform;

impl Transform {
    pub fn flip_horizontal(buffer: &mut PixelBuffer) {
        let w = buffer.width as usize;
        let h = buffer.height as usize;
        for y in 0..h {
            let row_offset = y * w * 4;
            for x in 0..(w / 2) {
                let left_idx = row_offset + x * 4;
                let right_idx = row_offset + (w - 1 - x) * 4;
                for c in 0..4 {
                    buffer.data.swap(left_idx + c, right_idx + c);
                }
            }
        }
    }

    pub fn flip_vertical(buffer: &mut PixelBuffer) {
        let w = buffer.width as usize;
        let h = buffer.height as usize;
        for y in 0..(h / 2) {
            let top_row = y * w * 4;
            let bot_row = (h - 1 - y) * w * 4;
            for x in 0..(w * 4) {
                buffer.data.swap(top_row + x, bot_row + x);
            }
        }
    }

    pub fn rotate_90_cw(buffer: &PixelBuffer) -> PixelBuffer {
        let new_w = buffer.height;
        let new_h = buffer.width;
        let mut out = PixelBuffer::new(new_w, new_h);
        for y in 0..buffer.height {
            for x in 0..buffer.width {
                if let Some(c) = buffer.get_pixel(x, y) {
                    out.set_pixel(buffer.height - 1 - y, x, c);
                }
            }
        }
        out
    }

    pub fn rotate_90_ccw(buffer: &PixelBuffer) -> PixelBuffer {
        let new_w = buffer.height;
        let new_h = buffer.width;
        let mut out = PixelBuffer::new(new_w, new_h);
        for y in 0..buffer.height {
            for x in 0..buffer.width {
                if let Some(c) = buffer.get_pixel(x, y) {
                    out.set_pixel(y, buffer.width - 1 - x, c);
                }
            }
        }
        out
    }

    /// Bicubic spline interpolation kernel (Catmull-Rom variant, a = -0.5)
    fn cubic_hermite(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
        let a_coeff = -0.5 * a + 1.5 * b - 1.5 * c + 0.5 * d;
        let b_coeff = a - 2.5 * b + 2.0 * c - 0.5 * d;
        let c_coeff = -0.5 * a + 0.5 * c;
        let d_coeff = b;
        a_coeff * t * t * t + b_coeff * t * t + c_coeff * t + d_coeff
    }

    /// High-precision Bicubic interpolation for high-fidelity scaling and rotation.
    pub fn sample_bicubic(buffer: &PixelBuffer, fx: f32, fy: f32) -> Color {
        let x = fx.floor() as i32;
        let y = fy.floor() as i32;
        let tx = fx - x as f32;
        let ty = fy - y as f32;

        let get_px = |ix: i32, iy: i32| -> [f32; 4] {
            let cx = ix.clamp(0, (buffer.width - 1) as i32) as u32;
            let cy = iy.clamp(0, (buffer.height - 1) as i32) as u32;
            let c = buffer.get_pixel(cx, cy).unwrap_or(Color::TRANSPARENT);
            [c.r as f32, c.g as f32, c.b as f32, c.a as f32]
        };

        let mut col = [0.0f32; 4];
        for ch in 0..4 {
            let mut row = [0.0f32; 4];
            for j in 0..4 {
                let iy = y - 1 + j as i32;
                let p0 = get_px(x - 1, iy)[ch];
                let p1 = get_px(x, iy)[ch];
                let p2 = get_px(x + 1, iy)[ch];
                let p3 = get_px(x + 2, iy)[ch];
                row[j] = Self::cubic_hermite(p0, p1, p2, p3, tx);
            }
            col[ch] = Self::cubic_hermite(row[0], row[1], row[2], row[3], ty).clamp(0.0, 255.0);
        }

        Color::rgba(col[0] as u8, col[1] as u8, col[2] as u8, col[3] as u8)
    }

    /// Resize using high-precision Bicubic sampling (smoother curves, reduced aliasing).
    pub fn resize_bicubic(buffer: &PixelBuffer, new_width: u32, new_height: u32) -> PixelBuffer {
        if new_width == 0 || new_height == 0 {
            return PixelBuffer::new(1, 1);
        }
        let mut out = PixelBuffer::new(new_width, new_height);
        let x_scale = (buffer.width as f32) / (new_width as f32);
        let y_scale = (buffer.height as f32) / (new_height as f32);

        for y in 0..new_height {
            let src_y = (y as f32 + 0.5) * y_scale - 0.5;
            for x in 0..new_width {
                let src_x = (x as f32 + 0.5) * x_scale - 0.5;
                let c = Self::sample_bicubic(buffer, src_x, src_y);
                out.set_pixel(x, y, c);
            }
        }
        out
    }

    /// Bilinear interpolation sampling of a single coordinate
    pub fn sample_bilinear(buffer: &PixelBuffer, fx: f32, fy: f32) -> Color {
        let x0 = (fx.floor() as i32).clamp(0, (buffer.width - 1) as i32) as u32;
        let y0 = (fy.floor() as i32).clamp(0, (buffer.height - 1) as i32) as u32;
        let x1 = (x0 + 1).min(buffer.width - 1);
        let y1 = (y0 + 1).min(buffer.height - 1);

        let tx = (fx - fx.floor()).clamp(0.0, 1.0);
        let ty = (fy - fy.floor()).clamp(0.0, 1.0);

        let c00 = buffer.get_pixel(x0, y0).unwrap_or(Color::TRANSPARENT);
        let c10 = buffer.get_pixel(x1, y0).unwrap_or(Color::TRANSPARENT);
        let c01 = buffer.get_pixel(x0, y1).unwrap_or(Color::TRANSPARENT);
        let c11 = buffer.get_pixel(x1, y1).unwrap_or(Color::TRANSPARENT);

        let lerp = |a: u8, b: u8, t: f32| -> f32 {
            a as f32 * (1.0 - t) + b as f32 * t
        };

        let r0 = lerp(c00.r, c10.r, tx);
        let r1 = lerp(c01.r, c11.r, tx);
        let r = lerp(r0 as u8, r1 as u8, ty).round() as u8;

        let g0 = lerp(c00.g, c10.g, tx);
        let g1 = lerp(c01.g, c11.g, tx);
        let g = lerp(g0 as u8, g1 as u8, ty).round() as u8;

        let b0 = lerp(c00.b, c10.b, tx);
        let b1 = lerp(c01.b, c11.b, tx);
        let b = lerp(b0 as u8, b1 as u8, ty).round() as u8;

        let a0 = lerp(c00.a, c10.a, tx);
        let a1 = lerp(c01.a, c11.a, tx);
        let a = lerp(a0 as u8, a1 as u8, ty).round() as u8;

        Color::rgba(r, g, b, a)
    }

    pub fn resize_bilinear(buffer: &PixelBuffer, new_width: u32, new_height: u32) -> PixelBuffer {
        if new_width == 0 || new_height == 0 {
            return PixelBuffer::new(1, 1);
        }
        let mut out = PixelBuffer::new(new_width, new_height);
        let x_ratio = (buffer.width as f32) / (new_width as f32);
        let y_ratio = (buffer.height as f32) / (new_height as f32);

        for y in 0..new_height {
            let src_y = (y as f32 * y_ratio).clamp(0.0, (buffer.height - 1) as f32) as u32;
            for x in 0..new_width {
                let src_x = (x as f32 * x_ratio).clamp(0.0, (buffer.width - 1) as f32) as u32;
                if let Some(col) = buffer.get_pixel(src_x, src_y) {
                    out.set_pixel(x, y, col);
                }
            }
        }
        out
    }

    pub fn crop(buffer: &PixelBuffer, crop_x: u32, crop_y: u32, crop_w: u32, crop_h: u32) -> PixelBuffer {
        let mut out = PixelBuffer::new(crop_w, crop_h);
        for y in 0..crop_h {
            let sy = crop_y + y;
            if sy >= buffer.height {
                continue;
            }
            for x in 0..crop_w {
                let sx = crop_x + x;
                if sx >= buffer.width {
                    continue;
                }
                if let Some(col) = buffer.get_pixel(sx, sy) {
                    out.set_pixel(x, y, col);
                }
            }
        }
        out
    }

    pub fn draw_line(buffer: &mut PixelBuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut cx = x0;
        let mut cy = y0;

        loop {
            if cx >= 0 && cx < buffer.width as i32 && cy >= 0 && cy < buffer.height as i32 {
                buffer.set_pixel(cx as u32, cy as u32, color);
            }
            if cx == x1 && cy == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
            }
        }
    }

    pub fn draw_rect_outline(buffer: &mut PixelBuffer, x: i32, y: i32, w: i32, h: i32, color: Color) {
        Self::draw_line(buffer, x, y, x + w, y, color);
        Self::draw_line(buffer, x + w, y, x + w, y + h, color);
        Self::draw_line(buffer, x + w, y + h, x, y + h, color);
        Self::draw_line(buffer, x, y + h, x, y, color);
    }

    /// Computes the bounding box of non-transparent pixels in the buffer.
    pub fn content_bounds(buffer: &PixelBuffer) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = u32::MAX;
        let mut max_x = 0;
        let mut min_y = u32::MAX;
        let mut max_y = 0;
        let mut found = false;

        for y in 0..buffer.height {
            for x in 0..buffer.width {
                if let Some(c) = buffer.get_pixel(x, y) {
                    if c.a > 0 {
                        found = true;
                        min_x = min_x.min(x);
                        max_x = max_x.max(x);
                        min_y = min_y.min(y);
                        max_y = max_y.max(y);
                    }
                }
            }
        }

        if found {
            Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
        } else {
            None
        }
    }

    /// Transforms a buffer using 2D affine transformation (scale, rotate, translate).
    /// Center of rotation and scaling is (cx, cy).
    /// angle_rad: rotation angle in radians
    /// scale_x, scale_y: scaling factors
    /// offset_x, offset_y: translation delta
    pub fn transform_affine_bicubic(
        src: &PixelBuffer,
        cx: f32,
        cy: f32,
        angle_rad: f32,
        scale_x: f32,
        scale_y: f32,
        offset_x: f32,
        offset_y: f32,
    ) -> PixelBuffer {
        let mut dst = PixelBuffer::new(src.width, src.height);
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let inv_sx = if scale_x.abs() < 1e-5 { 1.0 } else { 1.0 / scale_x };
        let inv_sy = if scale_y.abs() < 1e-5 { 1.0 } else { 1.0 / scale_y };

        for y in 0..src.height {
            let dy = y as f32 - cy - offset_y;
            for x in 0..src.width {
                let dx = x as f32 - cx - offset_x;

                // Inverse rotation & scale
                let unrot_x = dx * cos_a + dy * sin_a;
                let unrot_y = -dx * sin_a + dy * cos_a;

                let src_x = unrot_x * inv_sx + cx;
                let src_y = unrot_y * inv_sy + cy;

                if src_x >= -1.0 && src_x <= src.width as f32 && src_y >= -1.0 && src_y <= src.height as f32 {
                    let c = Self::sample_bicubic(src, src_x, src_y);
                    if c.a > 0 {
                        dst.set_pixel(x, y, c);
                    }
                }
            }
        }
        dst
    }

    /// Photoshop-style Liquify (Forward Warp / Push)
    /// Displaces pixels smoothly under the brush radius towards the movement vector (dx, dy).
    pub fn apply_liquify_push(
        buffer: &mut PixelBuffer,
        center_x: f32,
        center_y: f32,
        dx: f32,
        dy: f32,
        radius: f32,
        strength: f32,
    ) {
        if radius <= 1.0 || (dx.abs() < 1e-4 && dy.abs() < 1e-4) {
            return;
        }

        let r2 = radius * radius;
        let min_x = (center_x - radius).floor().max(0.0) as u32;
        let max_x = (center_x + radius).ceil().min(buffer.width as f32 - 1.0).max(0.0) as u32;
        let min_y = (center_y - radius).floor().max(0.0) as u32;
        let max_y = (center_y + radius).ceil().min(buffer.height as f32 - 1.0).max(0.0) as u32;

        if min_x > max_x || min_y > max_y || min_x >= buffer.width || min_y >= buffer.height {
            return;
        }

        let src = buffer.clone();

        for y in min_y..=max_y {
            let py = y as f32;
            let dist_y = py - center_y;

            for x in min_x..=max_x {
                let px = x as f32;
                let dist_x = px - center_x;
                let dist_sq = dist_x * dist_x + dist_y * dist_y;

                if dist_sq < r2 {
                    // Smooth bell-shaped falloff (1 - (d/r)^2)^2
                    let factor = (1.0 - dist_sq / r2).powi(2) * strength.clamp(0.0, 1.0);
                    let sx = px - dx * factor;
                    let sy = py - dy * factor;

                    let sample_col = Self::sample_bilinear(&src, sx, sy);
                    buffer.set_pixel(x, y, sample_col);
                }
            }
        }
    }

    /// Photoshop-style Liquify Expand / Bloat
    pub fn apply_liquify_bloat(
        buffer: &mut PixelBuffer,
        center_x: f32,
        center_y: f32,
        radius: f32,
        strength: f32,
    ) {
        if radius <= 1.0 || strength.abs() < 1e-4 {
            return;
        }
        let r2 = radius * radius;
        let min_x = (center_x - radius).floor().max(0.0) as u32;
        let max_x = (center_x + radius).ceil().min(buffer.width as f32 - 1.0).max(0.0) as u32;
        let min_y = (center_y - radius).floor().max(0.0) as u32;
        let max_y = (center_y + radius).ceil().min(buffer.height as f32 - 1.0).max(0.0) as u32;

        if min_x > max_x || min_y > max_y || min_x >= buffer.width || min_y >= buffer.height {
            return;
        }

        let src = buffer.clone();

        for y in min_y..=max_y {
            let py = y as f32;
            let dist_y = py - center_y;

            for x in min_x..=max_x {
                let px = x as f32;
                let dist_x = px - center_x;
                let dist_sq = dist_x * dist_x + dist_y * dist_y;

                if dist_sq < r2 {
                    let d = dist_sq.sqrt();
                    let norm_d = d / radius;
                    let factor = (1.0 - norm_d).powi(2) * strength.clamp(-1.0, 1.0);

                    // Pull inward or push outward
                    let scale = 1.0 - factor * 0.4;
                    let sx = center_x + dist_x * scale;
                    let sy = center_y + dist_y * scale;

                    let sample_col = Self::sample_bilinear(&src, sx, sy);
                    buffer.set_pixel(x, y, sample_col);
                }
            }
        }
    }

    /// Photoshop-style 4-corner Perspective / Free Distort transformation.
    /// Maps a rectangular buffer to an arbitrary destination quadrilateral specified by
    /// 4 corner points: [top-left, top-right, bottom-right, bottom-left].
    /// Uses inverse projective homography mapping with high-precision bicubic sampling.
    pub fn transform_perspective_quad(
        src: &PixelBuffer,
        dst_w: u32,
        dst_h: u32,
        quad: [(f32, f32); 4], // [tl, tr, br, bl]
    ) -> PixelBuffer {
        let mut dst = PixelBuffer::new(dst_w, dst_h);
        let (w, h) = (src.width as f32, src.height as f32);

        // Source rectangle corners: (0,0), (w,0), (w,h), (0,h)
        // Quad destination corners: p0, p1, p2, p3
        // Compute homography matrix H that maps quad (dst) -> rect (src)
        if let Some(h_inv) = Self::compute_homography_inverse(quad, [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]) {
            for y in 0..dst_h {
                let dy = y as f32 + 0.5;
                for x in 0..dst_w {
                    let dx = x as f32 + 0.5;

                    // Project (dx, dy) back to source buffer space
                    let denom = h_inv[6] * dx + h_inv[7] * dy + h_inv[8];
                    if denom.abs() < 1e-7 {
                        continue;
                    }
                    let inv_denom = 1.0 / denom;
                    let sx = (h_inv[0] * dx + h_inv[1] * dy + h_inv[2]) * inv_denom;
                    let sy = (h_inv[3] * dx + h_inv[4] * dy + h_inv[5]) * inv_denom;

                    if sx >= 0.0 && sx < w && sy >= 0.0 && sy < h {
                        let col = Self::sample_bicubic(src, sx, sy);
                        if col.a > 0 {
                            dst.set_pixel(x, y, col);
                        }
                    }
                }
            }
        }
        dst
    }

    /// Computes the 3x3 homography matrix mapping quad 1 -> quad 2 using Gaussian elimination
    fn compute_homography_inverse(src_pts: [(f32, f32); 4], dst_pts: [(f32, f32); 4]) -> Option<[f32; 9]> {
        let mut a = [[0.0f32; 9]; 8];

        for i in 0..4 {
            let (x, y) = src_pts[i];
            let (u, v) = dst_pts[i];

            a[i * 2][0] = x;
            a[i * 2][1] = y;
            a[i * 2][2] = 1.0;
            a[i * 2][3] = 0.0;
            a[i * 2][4] = 0.0;
            a[i * 2][5] = 0.0;
            a[i * 2][6] = -x * u;
            a[i * 2][7] = -y * u;
            a[i * 2][8] = u;

            a[i * 2 + 1][0] = 0.0;
            a[i * 2 + 1][1] = 0.0;
            a[i * 2 + 1][2] = 0.0;
            a[i * 2 + 1][3] = x;
            a[i * 2 + 1][4] = y;
            a[i * 2 + 1][5] = 1.0;
            a[i * 2 + 1][6] = -x * v;
            a[i * 2 + 1][7] = -y * v;
            a[i * 2 + 1][8] = v;
        }

        // Gaussian elimination with partial pivoting (solve for h0..h7 with h8 = 1)
        for i in 0..8 {
            let mut max_row = i;
            let mut max_val = a[i][i].abs();
            for k in (i + 1)..8 {
                if a[k][i].abs() > max_val {
                    max_val = a[k][i].abs();
                    max_row = k;
                }
            }
            if max_val < 1e-7 {
                return None;
            }
            a.swap(i, max_row);

            let pivot = a[i][i];
            for j in i..=8 {
                a[i][j] /= pivot;
            }
            for k in 0..8 {
                if k != i {
                    let factor = a[k][i];
                    for j in i..=8 {
                        a[k][j] -= factor * a[i][j];
                    }
                }
            }
        }

        Some([
            a[0][8], a[1][8], a[2][8],
            a[3][8], a[4][8], a[5][8],
            a[6][8], a[7][8], 1.0,
        ])
    }
}

