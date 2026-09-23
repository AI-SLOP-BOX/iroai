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
}
