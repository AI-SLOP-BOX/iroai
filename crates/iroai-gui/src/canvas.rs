use egui::{Color32, ColorImage, Pos2, Rect, Response, Sense, TextureHandle, TextureOptions, Ui, Vec2};
use iroai_core::{
    Brush, BrushTool, Document, PalmRejectionFilter, PointerEvent, SelectionOp,
};

pub struct CanvasState {
    pub zoom: f32,
    pub pan: Vec2,
    pub texture: Option<TextureHandle>,
    pub texture_version: u64,
    pub palm_filter: PalmRejectionFilter,
    pub last_pos: Option<Pos2>,
    pub split_view: bool,
    pub split_position: f32,
    pub hovered_pixel_info: Option<(u32, u32, iroai_core::Color)>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            texture: None,
            texture_version: 0,
            palm_filter: PalmRejectionFilter::new(),
            last_pos: None,
            split_view: false,
            split_position: 0.5,
            hovered_pixel_info: None,
        }
    }
}

pub struct CanvasWidget;

impl CanvasWidget {
    pub fn ui(
        ui: &mut Ui,
        doc: &mut Document,
        state: &mut CanvasState,
        brush: &mut Brush,
    ) -> Response {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

        // 1. マルチタッチ・ピンチズーム・パン (iPadOS / Android タブレット対応)
        let multi_touch = ui.input(|i| i.multi_touch());
        if let Some(touch) = multi_touch {
            state.zoom = (state.zoom * touch.zoom_delta).clamp(0.05, 50.0);
            state.pan += touch.translation_delta;
        }

        // 2. マウスホイールによるズーム・パン
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
        if ui.input(|i| i.modifiers.ctrl || i.modifiers.command) && scroll_delta.y != 0.0 {
            let factor = (scroll_delta.y * 0.005).exp();
            state.zoom = (state.zoom * factor).clamp(0.05, 50.0);
        } else if scroll_delta != Vec2::ZERO {
            state.pan += scroll_delta;
        }

        // 3. テクスチャのアップロード / 更新
        let composite = doc.composite();
        let w = composite.width as usize;
        let h = composite.height as usize;
        let image = ColorImage::from_rgba_unmultiplied([w, h], &composite.data);

        let texture = state.texture.get_or_insert_with(|| {
            ui.ctx().load_texture("canvas_composite", image.clone(), TextureOptions::NEAREST)
        });

        if state.texture_version != doc.history.undo_count() as u64 {
            texture.set(image, TextureOptions::NEAREST);
            state.texture_version = doc.history.undo_count() as u64;
        }

        // 4. キャンバス描画領域の計算 (アスペクト比維持・センタリング)
        let center = rect.center() + state.pan;
        let doc_w = doc.width as f32 * state.zoom;
        let doc_h = doc.height as f32 * state.zoom;
        let canvas_rect = Rect::from_center_size(center, Vec2::new(doc_w, doc_h));

        let painter = ui.painter().with_clip_rect(rect);

        // 背景
        painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(200, 200, 200));

        // 画像の描画
        painter.image(
            texture.id(),
            canvas_rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );

        // 5. ポインター入力・描画処理
        if let Some(pos) = response.hover_pos() {
            if canvas_rect.contains(pos) {
                let norm_x = (pos.x - canvas_rect.min.x) / canvas_rect.width();
                let norm_y = (pos.y - canvas_rect.min.y) / canvas_rect.height();
                let px = (norm_x * doc.width as f32) as u32;
                let py = (norm_y * doc.height as f32) as u32;

                if let Some(col) = composite.get_pixel(px, py) {
                    state.hovered_pixel_info = Some((px, py, col));
                }

                let mut ev = PointerEvent::new(px as f32, py as f32);
                ev.pressure = 1.0;
                ev.tool = brush.tool;

                if response.drag_started() {
                    state.last_pos = Some(pos);

                    if brush.tool == BrushTool::Eyedropper {
                        if let Some((_, _, col)) = state.hovered_pixel_info {
                            brush.color = col;
                        }
                    } else if brush.tool == BrushTool::Bucket {
                        if let Some(layer) = doc.active_layer_mut() {
                            iroai_core::Filters::flood_fill(&mut layer.buffer, px, py, brush.color, 20);
                        }
                    } else if brush.tool == BrushTool::RectSelect {
                        doc.selection.select_rect(px.saturating_sub(10), py.saturating_sub(10), 20, 20, SelectionOp::New);
                    } else if let Some(layer) = doc.active_layer_mut() {
                        brush.paint_pointer_stamp(&mut layer.buffer, &ev);
                    }
                } else if response.dragged() {
                    if let Some(prev) = state.last_pos {
                        let prev_nx = (prev.x - canvas_rect.min.x) / canvas_rect.width();
                        let prev_ny = (prev.y - canvas_rect.min.y) / canvas_rect.height();
                        let ppx = prev_nx * doc.width as f32;
                        let ppy = prev_ny * doc.height as f32;

                        if let Some(layer) = doc.active_layer_mut() {
                            brush.paint_line(&mut layer.buffer, ppx, ppy, px as f32, py as f32);
                        }
                    }
                    state.last_pos = Some(pos);
                } else if response.drag_stopped() {
                    state.last_pos = None;
                }
            } else {
                state.hovered_pixel_info = None;
            }
        }

        response
    }
}
