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
