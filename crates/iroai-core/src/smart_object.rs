use crate::buffer::PixelBuffer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmartObject {
    pub original_buffer: PixelBuffer,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation_deg: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl SmartObject {
    pub fn new(original_buffer: PixelBuffer) -> Self {
        Self {
            original_buffer,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_deg: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    pub fn render(&self, canvas_width: u32, canvas_height: u32) -> PixelBuffer {
        let mut rendered = PixelBuffer::new(canvas_width, canvas_height);
        let orig_w = self.original_buffer.width as f32;
        let orig_h = self.original_buffer.height as f32;

        if orig_w == 0.0 || orig_h == 0.0 || self.scale_x == 0.0 || self.scale_y == 0.0 {
            return rendered;
        }

        let center_x = self.offset_x + (orig_w * self.scale_x) / 2.0;
        let center_y = self.offset_y + (orig_h * self.scale_y) / 2.0;

        for y in 0..canvas_height {
            for x in 0..canvas_width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;

                let orig_px = dx / self.scale_x + orig_w / 2.0;
                let orig_py = dy / self.scale_y + orig_h / 2.0;

                if orig_px >= -0.5 && orig_px < orig_w + 0.5 && orig_py >= -0.5 && orig_py < orig_h + 0.5 {
                    let c = crate::transform::Transform::sample_bicubic(&self.original_buffer, orig_px, orig_py);
                    rendered.set_pixel(x, y, c);
                }
            }
        }

        rendered
    }
}
