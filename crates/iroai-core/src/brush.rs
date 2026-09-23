use crate::buffer::PixelBuffer;
use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushTool {
    Brush,
    Eraser,
    Eyedropper,
    Bucket,
    RectSelect,
    EllipseSelect,
    LassoSelect,
    Line,
    ShapeRect,
    CloneStamp,
    Blur,
    Sharpen,
    Dodge,
    Burn,
    Sponge,
    Pen,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerCapabilities {
    pub has_pressure: bool,
    pub has_tilt: bool,
    pub has_twist: bool,
    pub is_eraser: bool,
}

impl Default for PointerCapabilities {
    fn default() -> Self {
        Self {
            has_pressure: false,
            has_tilt: false,
            has_twist: false,
            is_eraser: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub twist: f32,
    pub tool: BrushTool,
}

impl PointerEvent {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            pressure: 1.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0.0,
            tool: BrushTool::Brush,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Brush {
    pub tool: BrushTool,
    pub size: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub color: Color,
    pub spacing: f32,
    pub pressure_size: bool,
    pub pressure_opacity: bool,
    pub clone_offset: (i32, i32),
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            tool: BrushTool::Brush,
            size: 16.0,
            hardness: 0.8,
            opacity: 1.0,
            color: Color::BLACK,
            spacing: 0.2,
            pressure_size: true,
            pressure_opacity: false,
            clone_offset: (0, 0),
        }
    }
}

impl Brush {
    pub fn paint_pointer_stamp(&self, buffer: &mut PixelBuffer, event: &PointerEvent) {
        let size_multiplier = if self.pressure_size {
            0.1 + 0.9 * event.pressure
        } else {
            1.0
        };

        let opacity_multiplier = if self.pressure_opacity {
            0.1 + 0.9 * event.pressure
        } else {
            1.0
        };

        let radius = (self.size * size_multiplier) * 0.5;
        let inner_radius = radius * self.hardness;
        let r2 = radius * radius;
        let inner_r2 = inner_radius * inner_radius;

        let cx = event.x;
        let cy = event.y;

        let min_x = (cx - radius).max(0.0) as u32;
        let max_x = (cx + radius + 1.0).min(buffer.width as f32) as u32;
        let min_y = (cy - radius).max(0.0) as u32;
        let max_y = (cy + radius + 1.0).min(buffer.height as f32) as u32;

        for py in min_y..max_y {
            let dy = py as f32 + 0.5 - cy;
            let dy2 = dy * dy;

            for px in min_x..max_x {
                let dx = px as f32 + 0.5 - cx;
                let d2 = dx * dx + dy2;

                if d2 > r2 {
                    continue;
                }

                let alpha_factor = if d2 <= inner_r2 {
                    1.0
                } else {
                    let d = d2.sqrt();
                    ((radius - d) / (radius - inner_radius).max(0.001)).clamp(0.0, 1.0)
                };

                let effective_alpha = (self.opacity * opacity_multiplier).clamp(0.0, 1.0) * alpha_factor;
                if effective_alpha <= 0.0 {
                    continue;
                }

                let cur_color = buffer.get_pixel(px, py).unwrap_or(Color::TRANSPARENT);

                match event.tool {
                    BrushTool::Brush => {
                        let stamp_color = Color {
                            r: self.color.r,
                            g: self.color.g,
                            b: self.color.b,
                            a: (self.color.a as f32 * effective_alpha).round() as u8,
                        };
                        let blended = crate::color::BlendMode::Normal.blend_pixel(cur_color, stamp_color, 1.0);
                        buffer.set_pixel(px, py, blended);
                    }
                    BrushTool::Eraser => {
                        let cur_a = cur_color.a as f32 / 255.0;
                        let new_a = (cur_a * (1.0 - effective_alpha)).clamp(0.0, 1.0);
                        let updated = Color {
                            r: cur_color.r,
                            g: cur_color.g,
                            b: cur_color.b,
                            a: (new_a * 255.0).round() as u8,
                        };
                        buffer.set_pixel(px, py, updated);
                    }
                    BrushTool::CloneStamp => {
                        let sx = px as i32 + self.clone_offset.0;
                        let sy = py as i32 + self.clone_offset.1;
                        if sx >= 0 && sx < buffer.width as i32 && sy >= 0 && sy < buffer.height as i32 {
                            if let Some(src_px) = buffer.get_pixel(sx as u32, sy as u32) {
                                let blended = crate::color::BlendMode::Normal.blend_pixel(cur_color, src_px, effective_alpha);
                                buffer.set_pixel(px, py, blended);
                            }
                        }
                    }
                    BrushTool::Blur => {
                        let mut sum_r = 0u32;
                        let mut sum_g = 0u32;
                        let mut sum_b = 0u32;
                        let mut count = 0u32;
                        for ny in -1..=1 {
                            for nx in -1..=1 {
                                let spx = px as i32 + nx;
                                let spy = py as i32 + ny;
                                if spx >= 0 && spx < buffer.width as i32 && spy >= 0 && spy < buffer.height as i32 {
                                    if let Some(c) = buffer.get_pixel(spx as u32, spy as u32) {
                                        sum_r += c.r as u32;
                                        sum_g += c.g as u32;
                                        sum_b += c.b as u32;
                                        count += 1;
                                    }
                                }
                            }
                        }
                        if count > 0 {
                            let avg_col = Color::rgba((sum_r / count) as u8, (sum_g / count) as u8, (sum_b / count) as u8, cur_color.a);
                            let blended = crate::color::BlendMode::Normal.blend_pixel(cur_color, avg_col, effective_alpha * 0.5);
                            buffer.set_pixel(px, py, blended);
                        }
                    }
                    BrushTool::Sharpen => {
                        let factor = 1.0 + 0.5 * effective_alpha;
                        let r = ((cur_color.r as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8;
                        let g = ((cur_color.g as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8;
                        let b = ((cur_color.b as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8;
                        buffer.set_pixel(px, py, Color::rgba(r, g, b, cur_color.a));
                    }
                    BrushTool::Dodge => {
                        let factor = 1.0 + 0.3 * effective_alpha;
                        let r = (cur_color.r as f32 * factor).min(255.0) as u8;
                        let g = (cur_color.g as f32 * factor).min(255.0) as u8;
                        let b = (cur_color.b as f32 * factor).min(255.0) as u8;
                        buffer.set_pixel(px, py, Color::rgba(r, g, b, cur_color.a));
                    }
                    BrushTool::Burn => {
                        let factor = (1.0 - 0.3 * effective_alpha).max(0.0);
                        let r = (cur_color.r as f32 * factor) as u8;
                        let g = (cur_color.g as f32 * factor) as u8;
                        let b = (cur_color.b as f32 * factor) as u8;
                        buffer.set_pixel(px, py, Color::rgba(r, g, b, cur_color.a));
                    }
                    BrushTool::Sponge => {
                        let gray = cur_color.r as f32 * 0.299 + cur_color.g as f32 * 0.587 + cur_color.b as f32 * 0.114;
                        let sat_factor = 1.0 + 0.5 * effective_alpha;
                        let r = ((cur_color.r as f32 - gray) * sat_factor + gray).clamp(0.0, 255.0) as u8;
                        let g = ((cur_color.g as f32 - gray) * sat_factor + gray).clamp(0.0, 255.0) as u8;
                        let b = ((cur_color.b as f32 - gray) * sat_factor + gray).clamp(0.0, 255.0) as u8;
                        buffer.set_pixel(px, py, Color::rgba(r, g, b, cur_color.a));
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn paint_stamp(&self, buffer: &mut PixelBuffer, cx: f32, cy: f32) {
        let mut event = PointerEvent::new(cx, cy);
        event.tool = self.tool;
        self.paint_pointer_stamp(buffer, &event);
    }

    pub fn paint_line(&self, buffer: &mut PixelBuffer, x0: f32, y0: f32, x1: f32, y1: f32) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = (self.size * self.spacing).max(1.0);
        let count = (dist / step).ceil() as usize;

        if count == 0 {
            self.paint_stamp(buffer, x0, y0);
            return;
        }

        for i in 0..=count {
            let t = i as f32 / count as f32;
            let cx = x0 + dx * t;
            let cy = y0 + dy * t;
            self.paint_stamp(buffer, cx, cy);
        }
    }
}
