use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};
use iroai_core::{Document, PixelBuffer, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleType {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopMid,
    BottomMid,
    LeftMid,
    RightMid,
    Rotate,
    Body,
}

pub struct TransformSession {
    pub is_active: bool,
    pub original_buffer: Option<PixelBuffer>,
    pub center: Pos2,
    pub width: f32,
    pub height: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub angle_rad: f32,
    pub offset: Vec2,
    pub active_handle: Option<HandleType>,
}

impl Default for TransformSession {
    fn default() -> Self {
        Self {
            is_active: false,
            original_buffer: None,
            center: Pos2::ZERO,
            width: 0.0,
            height: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            angle_rad: 0.0,
            offset: Vec2::ZERO,
            active_handle: None,
        }
    }
}

impl TransformSession {
    pub fn begin(&mut self, doc: &Document) {
        if let Some(layer) = doc.active_layer() {
            if let Some((min_x, min_y, w, h)) = Transform::content_bounds(&layer.buffer) {
                self.is_active = true;
                self.original_buffer = Some(layer.buffer.clone());
                self.center = Pos2::new(min_x as f32 + w as f32 * 0.5, min_y as f32 + h as f32 * 0.5);
                self.width = w as f32;
                self.height = h as f32;
                self.scale_x = 1.0;
                self.scale_y = 1.0;
                self.angle_rad = 0.0;
                self.offset = Vec2::ZERO;
            }
        }
    }

    pub fn commit(&mut self, doc: &mut Document) {
        if !self.is_active {
            return;
        }
        if let (Some(orig), Some(layer)) = (&self.original_buffer, doc.active_layer_mut()) {
            layer.buffer = Transform::transform_affine_bicubic(
                orig,
                self.center.x,
                self.center.y,
                self.angle_rad,
                self.scale_x,
                self.scale_y,
                self.offset.x,
                self.offset.y,
            );
        }
        self.is_active = false;
        self.original_buffer = None;
    }

    pub fn cancel(&mut self, doc: &mut Document) {
        if !self.is_active {
            return;
        }
        if let (Some(orig), Some(layer)) = (self.original_buffer.take(), doc.active_layer_mut()) {
            layer.buffer = orig;
        }
        self.is_active = false;
    }

    /// Renders bounding box and 8 handles in screen space
    pub fn render_gizmo(
        &mut self,
        ui: &mut Ui,
        canvas_rect: Rect,
        doc_w: f32,
        doc_h: f32,
    ) {
        if !self.is_active {
            return;
        }

        // Map document coordinate to canvas screen coordinate
        let to_screen = |pos: Pos2| -> Pos2 {
            Pos2::new(
                canvas_rect.min.x + (pos.x / doc_w) * canvas_rect.width(),
                canvas_rect.min.y + (pos.y / doc_h) * canvas_rect.height(),
            )
        };

        let scr_center = to_screen(self.center + self.offset);
        let half_w = (self.width * self.scale_x * 0.5) * (canvas_rect.width() / doc_w);
        let half_h = (self.height * self.scale_y * 0.5) * (canvas_rect.height() / doc_h);

        let cos_a = self.angle_rad.cos();
        let sin_a = self.angle_rad.sin();
        let rotate_vec = |v: Vec2| -> Vec2 {
            Vec2::new(v.x * cos_a - v.y * sin_a, v.x * sin_a + v.y * cos_a)
        };

        let p_tl = scr_center + rotate_vec(Vec2::new(-half_w, -half_h));
        let p_tr = scr_center + rotate_vec(Vec2::new(half_w, -half_h));
        let p_br = scr_center + rotate_vec(Vec2::new(half_w, half_h));
        let p_bl = scr_center + rotate_vec(Vec2::new(-half_w, half_h));
        let p_top = scr_center + rotate_vec(Vec2::new(0.0, -half_h));
        let p_rot = scr_center + rotate_vec(Vec2::new(0.0, -half_h - 24.0));

        let painter = ui.painter_at(canvas_rect);

        // Bounding wireframe
        painter.line_segment([p_tl, p_tr], Stroke::new(1.5_f32, Color32::from_rgb(0, 150, 255)));
        painter.line_segment([p_tr, p_br], Stroke::new(1.5_f32, Color32::from_rgb(0, 150, 255)));
        painter.line_segment([p_br, p_bl], Stroke::new(1.5_f32, Color32::from_rgb(0, 150, 255)));
        painter.line_segment([p_bl, p_tl], Stroke::new(1.5_f32, Color32::from_rgb(0, 150, 255)));
        painter.line_segment([p_top, p_rot], Stroke::new(1.0_f32, Color32::from_rgb(0, 150, 255)));

        // Draw handles
        let handle_size = 7.0;
        let draw_handle = |pos: Pos2, fill: Color32| {
            let h_rect = Rect::from_center_size(pos, Vec2::splat(handle_size));
            painter.rect_filled(h_rect, 1.0, fill);
            painter.rect_stroke(h_rect, 1.0, Stroke::new(1.0_f32, Color32::BLACK));
        };

        draw_handle(p_tl, Color32::WHITE);
        draw_handle(p_tr, Color32::WHITE);
        draw_handle(p_br, Color32::WHITE);
        draw_handle(p_bl, Color32::WHITE);
        painter.circle_filled(p_rot, 4.5, Color32::from_rgb(0, 255, 120));
        painter.circle_stroke(p_rot, 4.5, Stroke::new(1.0_f32, Color32::BLACK));
    }
}
