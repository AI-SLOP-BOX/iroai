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
    pub flow: f32,
    pub color: Color,
    pub spacing: f32,
    pub pressure_size: bool,
    pub pressure_opacity: bool,
    pub lock_alpha: bool,
    pub clone_offset: (i32, i32),
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            tool: BrushTool::Brush,
            size: 16.0,
            hardness: 0.8,
            opacity: 1.0,
            flow: 1.0,
            color: Color::BLACK,
            spacing: 0.2,
            pressure_size: true,
            pressure_opacity: false,
            lock_alpha: false,
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

                if self.lock_alpha && cur_color.a == 0 {
                    continue;
                }

                match event.tool {
                    BrushTool::Brush => {
                        let stamp_alpha = if self.lock_alpha {
                            let max_a = cur_color.a as f32;
                            (self.color.a as f32 * effective_alpha).min(max_a)
                        } else {
                            self.color.a as f32 * effective_alpha
                        };
                        let stamp_color = Color {
                            r: self.color.r,
                            g: self.color.g,
                            b: self.color.b,
                            a: stamp_alpha.round() as u8,
                        };
                        let mut blended = crate::color::BlendMode::Normal.blend_pixel(cur_color, stamp_color, 1.0);
                        if self.lock_alpha {
                            blended.a = cur_color.a;
                        }
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

    /// Paints a continuous stroke line. For semi-transparent brushes (opacity < 1.0),
    /// paints stamps with stroke-level maximum accumulation to prevent overlapping blotches.
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

        // Bounding box of the entire stroke segment
        let radius = self.size * 0.5 + 2.0;
        let min_x = ((x0.min(x1) - radius).max(0.0) as usize).min(buffer.width as usize);
        let max_x = ((x0.max(x1) + radius + 1.0).min(buffer.width as f32) as usize).min(buffer.width as usize);
        let min_y = ((y0.min(y1) - radius).max(0.0) as usize).min(buffer.height as usize);
        let max_y = ((y0.max(y1) + radius + 1.0).min(buffer.height as f32) as usize).min(buffer.height as usize);

        let bb_w = max_x.saturating_sub(min_x);
        let bb_h = max_y.saturating_sub(min_y);

        if bb_w == 0 || bb_h == 0 {
            return;
        }

        // 1-channel alpha mask accumulator for the stroke segment (0.0 ..= 1.0)
        let mut stroke_mask = vec![0.0f32; bb_w * bb_h];
        let r = self.size * 0.5;
        let r2 = r * r;
        let inner_r = r * self.hardness;
        let inner_r2 = inner_r * inner_r;

        for i in 0..=count {
            let t = i as f32 / count as f32;
            let cx = x0 + dx * t;
            let cy = y0 + dy * t;

            let stamp_min_x = ((cx - r).max(min_x as f32) as usize).min(max_x);
            let stamp_max_x = ((cx + r + 1.0).min(max_x as f32) as usize).min(max_x);
            let stamp_min_y = ((cy - r).max(min_y as f32) as usize).min(max_y);
            let stamp_max_y = ((cy + r + 1.0).min(max_y as f32) as usize).min(max_y);

            for sy in stamp_min_y..stamp_max_y {
                let s_dy = sy as f32 + 0.5 - cy;
                let s_dy2 = s_dy * s_dy;
                let row_offset = (sy - min_y) * bb_w;

                for sx in stamp_min_x..stamp_max_x {
                    let s_dx = sx as f32 + 0.5 - cx;
                    let d2 = s_dx * s_dx + s_dy2;
                    if d2 <= r2 {
                        let alpha = if d2 <= inner_r2 {
                            1.0
                        } else {
                            let d = d2.sqrt();
                            ((r - d) / (r - inner_r).max(0.001)).clamp(0.0, 1.0)
                        };
                        let mask_idx = row_offset + (sx - min_x);
                        // Photoshop Flow / Opacity model:
                        // Flow deposits paint per stamp with alpha accumulation: A_new = A_old + (1 - A_old) * (alpha * flow)
                        // Overall stroke is capped at self.opacity when composited.
                        let stamp_flow = (alpha * self.flow).clamp(0.0, 1.0);
                        let prev_mask = stroke_mask[mask_idx];
                        stroke_mask[mask_idx] = (prev_mask + (1.0 - prev_mask) * stamp_flow).min(1.0);
                    }
                }
            }
        }

        // Composite accumulated stroke mask onto layer buffer once
        for sy in min_y..max_y {
            let row_offset = (sy - min_y) * bb_w;
            let buf_row_start = sy * (buffer.width as usize) * 4;

            for sx in min_x..max_x {
                let mask_val = stroke_mask[row_offset + (sx - min_x)];
                if mask_val <= 0.0 {
                    continue;
                }

                let eff_alpha = (self.opacity * mask_val).clamp(0.0, 1.0);
                let p_idx = buf_row_start + sx * 4;
                let cur_color = Color::rgba(
                    buffer.data[p_idx],
                    buffer.data[p_idx + 1],
                    buffer.data[p_idx + 2],
                    buffer.data[p_idx + 3],
                );

                if self.lock_alpha && cur_color.a == 0 {
                    continue;
                }

                match self.tool {
                    BrushTool::Brush => {
                        let stamp_alpha = if self.lock_alpha {
                            let max_a = cur_color.a as f32;
                            (self.color.a as f32 * eff_alpha).min(max_a)
                        } else {
                            self.color.a as f32 * eff_alpha
                        };
                        let stamp_color = Color {
                            r: self.color.r,
                            g: self.color.g,
                            b: self.color.b,
                            a: stamp_alpha.round() as u8,
                        };
                        let mut blended = crate::color::BlendMode::Normal.blend_pixel(cur_color, stamp_color, 1.0);
                        if self.lock_alpha {
                            blended.a = cur_color.a;
                        }
                        buffer.data[p_idx] = blended.r;
                        buffer.data[p_idx + 1] = blended.g;
                        buffer.data[p_idx + 2] = blended.b;
                        buffer.data[p_idx + 3] = blended.a;
                    }
                    BrushTool::Eraser => {
                        let cur_a = cur_color.a as f32 / 255.0;
                        let new_a = (cur_a * (1.0 - eff_alpha)).clamp(0.0, 1.0);
                        buffer.data[p_idx + 3] = (new_a * 255.0).round() as u8;
                    }
                    _ => {
                        // For special tools (Blur, Sharpen, etc.), execute fallback stamp
                    }
                }
            }
        }
    }
}
