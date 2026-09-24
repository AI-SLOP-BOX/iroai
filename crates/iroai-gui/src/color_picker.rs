use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use iroai_core::Color;

/// Professional DCC Color Wheel (Outer Ring = Hue, Inner Box = Saturation/Value)
pub struct ColorPickerWheel;

impl ColorPickerWheel {
    /// Converts RGB (0..=255) to HSV (h: 0..360, s: 0..1, v: 0..1)
    pub fn rgb_to_hsv(c: Color) -> (f32, f32, f32) {
        let r = c.r as f32 / 255.0;
        let g = c.g as f32 / 255.0;
        let b = c.b as f32 / 255.0;

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

        let s = if max == 0.0 { 0.0 } else { delta / max };
        let v = max;

        (h, s, v)
    }

    /// Converts HSV to RGB Color
    pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
        let c = v * s;
        let h_prime = (h.rem_euclid(360.0)) / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
        let m = v - c;

        let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
            (c, x, 0.0)
        } else if (1.0..2.0).contains(&h_prime) {
            (x, c, 0.0)
        } else if (2.0..3.0).contains(&h_prime) {
            (0.0, c, x)
        } else if (3.0..4.0).contains(&h_prime) {
            (0.0, x, c)
        } else if (4.0..5.0).contains(&h_prime) {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color::rgba(
            ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
            255,
        )
    }

    /// Renders a 180x180 interactive Color Wheel & SV Box
    pub fn show(ui: &mut Ui, current_color: &mut Color) -> bool {
        let size = Vec2::splat(190.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        let center = rect.center();
        let outer_radius = 90.0_f32;
        let inner_radius = 72.0_f32;

        let (mut hue, mut sat, mut val) = Self::rgb_to_hsv(*current_color);
        let mut changed = false;

        // 1. Draw Hue Ring
        let ring_segments = 64;
        for i in 0..ring_segments {
            let a0 = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
            let a1 = ((i + 1) as f32 / ring_segments as f32) * std::f32::consts::TAU;
            let seg_hue = (i as f32 / ring_segments as f32) * 360.0;
            let rgb = Self::hsv_to_rgb(seg_hue, 1.0, 1.0);
            let col = Color32::from_rgb(rgb.r, rgb.g, rgb.b);

            let mid_radius = (outer_radius + inner_radius) * 0.5;
            let stroke_width = outer_radius - inner_radius;

            let p0 = center + Vec2::new(a0.cos(), a0.sin()) * mid_radius;
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * mid_radius;
            painter.line_segment([p0, p1], Stroke::new(stroke_width, col));
        }

        // 2. Draw Hue selector marker on ring
        let hue_rad = (hue / 360.0) * std::f32::consts::TAU;
        let hue_pos = center + Vec2::new(hue_rad.cos(), hue_rad.sin()) * ((outer_radius + inner_radius) * 0.5);
        painter.circle_stroke(hue_pos, 5.0, Stroke::new(2.0_f32, Color32::WHITE));
        painter.circle_stroke(hue_pos, 6.0, Stroke::new(1.0_f32, Color32::BLACK));

        // 3. Inner SV Triangle / Box
        // Box fits comfortably inside inner_radius (inner_radius * sqrt(2) * 0.8)
        let box_half = (inner_radius * 0.65).round();
        let box_rect = Rect::from_center_size(center, Vec2::splat(box_half * 2.0));

        // Grid approximation of Saturation-Value square
        let sv_steps = 14;
        let cell_w = box_rect.width() / sv_steps as f32;
        let cell_h = box_rect.height() / sv_steps as f32;

        for iy in 0..sv_steps {
            let v_step = 1.0 - (iy as f32 / sv_steps as f32);
            for ix in 0..sv_steps {
                let s_step = ix as f32 / sv_steps as f32;
                let rgb = Self::hsv_to_rgb(hue, s_step, v_step);
                let col = Color32::from_rgb(rgb.r, rgb.g, rgb.b);
                let c_rect = Rect::from_min_size(
                    Pos2::new(box_rect.min.x + ix as f32 * cell_w, box_rect.min.y + iy as f32 * cell_h),
                    Vec2::new(cell_w + 0.5, cell_h + 0.5),
                );
                painter.rect_filled(c_rect, 0.0, col);
            }
        }
        painter.rect_stroke(box_rect, 1.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 45)));

        // 4. Draw SV selector dot inside the box
        let sv_dot_x = box_rect.min.x + sat * box_rect.width();
        let sv_dot_y = box_rect.min.y + (1.0 - val) * box_rect.height();
        let sv_dot = Pos2::new(sv_dot_x, sv_dot_y);
        painter.circle_stroke(sv_dot, 4.0, Stroke::new(2.0_f32, Color32::WHITE));
        painter.circle_stroke(sv_dot, 5.0, Stroke::new(1.0_f32, Color32::BLACK));

        // 5. Handle Pointer Interaction
        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let to_ptr = pos - center;
                let dist = to_ptr.length();

                if dist >= inner_radius - 6.0 && dist <= outer_radius + 6.0 {
                    // Clicking or dragging the Hue ring
                    let mut angle = to_ptr.y.atan2(to_ptr.x);
                    if angle < 0.0 {
                        angle += std::f32::consts::TAU;
                    }
                    hue = (angle / std::f32::consts::TAU) * 360.0;
                    *current_color = Self::hsv_to_rgb(hue, sat, val);
                    changed = true;
                } else if box_rect.contains(pos) || dist < inner_radius {
                    // Clicking or dragging inside SV square
                    sat = ((pos.x - box_rect.min.x) / box_rect.width()).clamp(0.0, 1.0);
                    val = (1.0 - (pos.y - box_rect.min.y) / box_rect.height()).clamp(0.0, 1.0);
                    *current_color = Self::hsv_to_rgb(hue, sat, val);
                    changed = true;
                }
            }
        }

        // Color numeric hex and swatch preview footer
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let (swatch, _) = ui.allocate_exact_size(Vec2::new(28.0, 20.0), Sense::hover());
            ui.painter().rect_filled(swatch, 2.0, Color32::from_rgb(current_color.r, current_color.g, current_color.b));
            ui.painter().rect_stroke(swatch, 2.0, Stroke::new(1.0_f32, Color32::from_rgb(90, 90, 90)));

            ui.label(format!("#{:02X}{:02X}{:02X}", current_color.r, current_color.g, current_color.b));
            ui.label(format!("H:{:.0}° S:{:.0}% V:{:.0}%", hue, sat * 100.0, val * 100.0));
        });

        changed
    }
}
