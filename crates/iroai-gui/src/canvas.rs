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
    pub stroke_recorder: Option<iroai_core::StrokeRecorder>,
    pub active_path: Option<iroai_core::VectorPath>,
    pub selected_anchor_idx: Option<usize>,
    pub text_input_prompt: Option<(u32, u32, String)>,
    pub preview_plate: Option<iroai_core::CmykPlate>,
    pub horizontal_guides: Vec<f32>,
    pub vertical_guides: Vec<f32>,
    pub dragging_guide: Option<(bool, f32)>, // (is_horizontal, current_doc_pos)
    pub show_rulers: bool,
    pub show_pixel_grid: bool,
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
            stroke_recorder: None,
            active_path: None,
            selected_anchor_idx: None,
            text_input_prompt: None,
            preview_plate: None,
            horizontal_guides: Vec::new(),
            vertical_guides: Vec::new(),
            dragging_guide: None,
            show_rulers: true,
            show_pixel_grid: true,
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

        // 3. テクスチャのアップロード / 更新 (ゼロコピー・遅延評価: 変更時のみ composite & upload)
        let needs_texture_update = state.texture.is_none() || state.texture_version != doc.history.undo_count() as u64;
        if needs_texture_update {
            let composite = doc.composite();
            let display_buffer = if let Some(plate) = state.preview_plate {
                iroai_core::CmykManager::extract_plate(&composite, plate)
            } else {
                composite
            };
            let w = display_buffer.width as usize;
            let h = display_buffer.height as usize;
            let image = ColorImage::from_rgba_unmultiplied([w, h], &display_buffer.data);

            if let Some(tex) = &mut state.texture {
                tex.set(image, TextureOptions::LINEAR);
            } else {
                state.texture = Some(ui.ctx().load_texture("canvas_composite", image, TextureOptions::LINEAR));
            }
            state.texture_version = doc.history.undo_count() as u64;
        }

        let texture = state.texture.as_ref().unwrap();

        // 4. キャンバス描画領域の計算 (アスペクト比維持・センタリング)
        let center = rect.center() + state.pan;
        let doc_w = doc.width as f32 * state.zoom;
        let doc_h = doc.height as f32 * state.zoom;
        let canvas_rect = Rect::from_center_size(center, Vec2::new(doc_w, doc_h));

        let painter = ui.painter().with_clip_rect(rect);

        // 1. Dark viewport canvas backdrop
        painter.rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 24));

        // 2. Subtle drop shadow behind canvas
        let shadow_rect = canvas_rect.expand(4.0).translate(Vec2::new(0.0, 2.0));
        painter.rect_filled(shadow_rect, 2.0, Color32::from_black_alpha(120));

        // 3. Studio Checkerboard (Transparency grid)
        painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(240, 240, 240));
        let grid_size = 12.0;
        let start_x = canvas_rect.min.x;
        let start_y = canvas_rect.min.y;
        let cols = (canvas_rect.width() / grid_size).ceil() as usize;
        let rows = (canvas_rect.height() / grid_size).ceil() as usize;
        for r in 0..rows {
            for c in 0..cols {
                if (r + c) % 2 == 1 {
                    let rx = (start_x + c as f32 * grid_size).min(canvas_rect.max.x);
                    let ry = (start_y + r as f32 * grid_size).min(canvas_rect.max.y);
                    let rw = grid_size.min(canvas_rect.max.x - rx);
                    let rh = grid_size.min(canvas_rect.max.y - ry);
                    if rw > 0.0 && rh > 0.0 {
                        let cell_rect = Rect::from_min_size(egui::pos2(rx, ry), Vec2::new(rw, rh));
                        painter.rect_filled(cell_rect, 0.0, Color32::from_rgb(205, 205, 205));
                    }
                }
            }
        }

        // 4. 画像の描画
        painter.image(
            texture.id(),
            canvas_rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );

        // 5. 1px hairline canvas boundary border
        painter.rect_stroke(canvas_rect, 0.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 60)));

        // 6. Vector Paths & Pen Tool Bezier Curve Overlay (Photoshop-grade Anchor & Handle Gizmos)
        if let Some(ref path) = state.active_path {
            let doc_to_screen = |x: f32, y: f32| -> Pos2 {
                let sx = canvas_rect.min.x + (x / doc.width as f32) * canvas_rect.width();
                let sy = canvas_rect.min.y + (y / doc.height as f32) * canvas_rect.height();
                Pos2::new(sx, sy)
            };

            for subpath in &path.subpaths {
                if subpath.points.is_empty() {
                    continue;
                }
                let count = subpath.points.len();
                let limit = if subpath.closed { count } else { count.saturating_sub(1) };

                // Draw Bezier Curves
                for i in 0..limit {
                    let p0 = &subpath.points[i];
                    let p1 = &subpath.points[(i + 1) % count];
                    let samples = iroai_core::BezierEngine::sample_curve(p0, p1, 24);
                    for pair in samples.windows(2) {
                        let pt_a = doc_to_screen(pair[0].0, pair[0].1);
                        let pt_b = doc_to_screen(pair[1].0, pair[1].1);
                        painter.line_segment([pt_a, pt_b], egui::Stroke::new(1.5_f32, Color32::from_rgb(0, 160, 255)));
                    }
                }

                // Draw Anchor Points & Direction Handles
                for (idx, pt) in subpath.points.iter().enumerate() {
                    let anchor_pos = doc_to_screen(pt.anchor.0, pt.anchor.1);

                    // Direction Handles
                    if let Some(h_out) = pt.handle_out {
                        let handle_pos = doc_to_screen(h_out.0, h_out.1);
                        painter.line_segment([anchor_pos, handle_pos], egui::Stroke::new(1.0_f32, Color32::from_rgb(120, 200, 255)));
                        painter.circle_filled(handle_pos, 3.0, Color32::from_rgb(0, 180, 255));
                    }
                    if let Some(h_in) = pt.handle_in {
                        let handle_pos = doc_to_screen(h_in.0, h_in.1);
                        painter.line_segment([anchor_pos, handle_pos], egui::Stroke::new(1.0_f32, Color32::from_rgb(120, 200, 255)));
                        painter.circle_filled(handle_pos, 3.0, Color32::from_rgb(0, 180, 255));
                    }

                    // Anchor Box (Square)
                    let is_selected = state.selected_anchor_idx == Some(idx);
                    let box_rect = Rect::from_center_size(anchor_pos, Vec2::splat(7.0));
                    let fill_col = if is_selected { Color32::from_rgb(0, 140, 255) } else { Color32::WHITE };
                    painter.rect_filled(box_rect, 1.0, fill_col);
                    painter.rect_stroke(box_rect, 1.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(30, 30, 30)));
                }
            }
        }

        // 7. Selection Marquee (Marching Ants / Stencil Boundary)
        if !doc.selection.is_empty() {
            if let Some((sx, sy, sw, sh)) = doc.selection.get_bounding_box() {
                let p0 = Pos2::new(
                    canvas_rect.min.x + (sx as f32 / doc.width as f32) * canvas_rect.width(),
                    canvas_rect.min.y + (sy as f32 / doc.height as f32) * canvas_rect.height(),
                );
                let p1 = Pos2::new(
                    canvas_rect.min.x + ((sx + sw) as f32 / doc.width as f32) * canvas_rect.width(),
                    canvas_rect.min.y + ((sy + sh) as f32 / doc.height as f32) * canvas_rect.height(),
                );
                let sel_rect = Rect::from_min_max(p0, p1);
                // Subtle blue tint to indicate active stencil mask
                painter.rect_filled(sel_rect, 0.0, Color32::from_rgba_unmultiplied(0, 140, 255, 20));
                // High contrast two-tone dashed boundary line
                painter.rect_stroke(sel_rect, 0.0, egui::Stroke::new(1.5_f32, Color32::from_rgb(20, 20, 20)));
                painter.rect_stroke(sel_rect.shrink(1.0), 0.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 255, 255)));
            }
        }

        // 8. Photoshop-style Pixel Grid (Automatically displayed when zoomed >= 500%)
        if state.show_pixel_grid && state.zoom >= 5.0 {
            let px_step_x = canvas_rect.width() / doc.width as f32;
            let px_step_y = canvas_rect.height() / doc.height as f32;
            let grid_color = Color32::from_white_alpha(35);
            let stroke = egui::Stroke::new(1.0_f32, grid_color);

            // Vertical pixel grid lines
            for x in 0..=doc.width {
                let sx = canvas_rect.min.x + x as f32 * px_step_x;
                if sx >= rect.min.x && sx <= rect.max.x {
                    painter.line_segment([Pos2::new(sx, canvas_rect.min.y), Pos2::new(sx, canvas_rect.max.y)], stroke);
                }
            }
            // Horizontal pixel grid lines
            for y in 0..=doc.height {
                let sy = canvas_rect.min.y + y as f32 * px_step_y;
                if sy >= rect.min.y && sy <= rect.max.y {
                    painter.line_segment([Pos2::new(canvas_rect.min.x, sy), Pos2::new(canvas_rect.max.x, sy)], stroke);
                }
            }
        }

        // 8. Cyan Guide Lines (Photoshop-standard #00e5ff)
        let guide_color = Color32::from_rgb(0, 229, 255);
        let guide_stroke = egui::Stroke::new(1.0_f32, guide_color);
        for &gy in &state.horizontal_guides {
            let sy = canvas_rect.min.y + (gy / doc.height as f32) * canvas_rect.height();
            painter.line_segment([Pos2::new(rect.min.x, sy), Pos2::new(rect.max.x, sy)], guide_stroke);
        }
        for &gx in &state.vertical_guides {
            let sx = canvas_rect.min.x + (gx / doc.width as f32) * canvas_rect.width();
            painter.line_segment([Pos2::new(sx, rect.min.y), Pos2::new(sx, rect.max.y)], guide_stroke);
        }

        // 9. Precision Rulers (Top & Left edge measurement headers)
        if state.show_rulers {
            let ruler_bg = Color32::from_rgb(32, 32, 32);
            let ruler_fg = Color32::from_rgb(160, 160, 160);
            let ruler_size = 16.0;

            // Top Ruler
            let top_ruler_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), ruler_size));
            painter.rect_filled(top_ruler_rect, 0.0, ruler_bg);
            painter.line_segment([Pos2::new(rect.min.x, rect.min.y + ruler_size), Pos2::new(rect.max.x, rect.min.y + ruler_size)], egui::Stroke::new(1.0_f32, Color32::from_rgb(50, 50, 50)));

            // Left Ruler
            let left_ruler_rect = Rect::from_min_size(rect.min, Vec2::new(ruler_size, rect.height()));
            painter.rect_filled(left_ruler_rect, 0.0, ruler_bg);
            painter.line_segment([Pos2::new(rect.min.x + ruler_size, rect.min.y), Pos2::new(rect.min.x + ruler_size, rect.max.y)], egui::Stroke::new(1.0_f32, Color32::from_rgb(50, 50, 50)));

            // Handle guide drag initiation from rulers
            if let Some(pos) = response.hover_pos() {
                if response.drag_started() {
                    if top_ruler_rect.contains(pos) && pos.x > rect.min.x + ruler_size {
                        let doc_y = (pos.y - canvas_rect.min.y) / canvas_rect.height() * doc.height as f32;
                        state.dragging_guide = Some((true, doc_y));
                    } else if left_ruler_rect.contains(pos) && pos.y > rect.min.y + ruler_size {
                        let doc_x = (pos.x - canvas_rect.min.x) / canvas_rect.width() * doc.width as f32;
                        state.dragging_guide = Some((false, doc_x));
                    }
                }
            }

            // Top Ruler Tick Marks & Labels
            let tick_interval_doc = if state.zoom > 4.0 { 10.0 } else if state.zoom > 1.0 { 50.0 } else { 100.0 };
            let start_val_x = ((rect.min.x - canvas_rect.min.x) / (canvas_rect.width() / doc.width as f32) / tick_interval_doc).floor() * tick_interval_doc;
            let end_val_x = ((rect.max.x - canvas_rect.min.x) / (canvas_rect.width() / doc.width as f32) / tick_interval_doc).ceil() * tick_interval_doc;

            let mut cur_x = start_val_x;
            while cur_x <= end_val_x {
                let sx = canvas_rect.min.x + (cur_x / doc.width as f32) * canvas_rect.width();
                if sx >= rect.min.x + ruler_size && sx <= rect.max.x {
                    painter.line_segment([Pos2::new(sx, rect.min.y + 10.0), Pos2::new(sx, rect.min.y + ruler_size)], egui::Stroke::new(1.0_f32, ruler_fg));
                    if cur_x >= 0.0 && cur_x <= doc.width as f32 {
                        painter.text(Pos2::new(sx + 2.0, rect.min.y + 1.0), egui::Align2::LEFT_TOP, format!("{:.0}", cur_x), egui::FontId::monospace(9.0), ruler_fg);
                    }
                }
                cur_x += tick_interval_doc;
            }

            // Left Ruler Tick Marks
            let mut cur_y = 0.0f32;
            while cur_y <= doc.height as f32 {
                let sy = canvas_rect.min.y + (cur_y / doc.height as f32) * canvas_rect.height();
                if sy >= rect.min.y + ruler_size && sy <= rect.max.y {
                    painter.line_segment([Pos2::new(rect.min.x + 10.0, sy), Pos2::new(rect.min.x + ruler_size, sy)], egui::Stroke::new(1.0_f32, ruler_fg));
                }
                cur_y += tick_interval_doc;
            }

            // Top-left Corner Square
            painter.rect_filled(Rect::from_min_size(rect.min, Vec2::splat(ruler_size)), 0.0, Color32::from_rgb(45, 45, 45));
        }

        // Active Guide Dragging feedback & release
        if let Some((is_horizontal, cur_doc_pos)) = state.dragging_guide {
            let guide_preview_stroke = egui::Stroke::new(1.5_f32, Color32::from_rgb(0, 240, 255));
            if is_horizontal {
                let sy = canvas_rect.min.y + (cur_doc_pos / doc.height as f32) * canvas_rect.height();
                painter.line_segment([Pos2::new(rect.min.x, sy), Pos2::new(rect.max.x, sy)], guide_preview_stroke);
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            } else {
                let sx = canvas_rect.min.x + (cur_doc_pos / doc.width as f32) * canvas_rect.width();
                painter.line_segment([Pos2::new(sx, rect.min.y), Pos2::new(sx, rect.max.y)], guide_preview_stroke);
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }

            if let Some(pos) = response.hover_pos() {
                if is_horizontal {
                    let doc_y = (pos.y - canvas_rect.min.y) / canvas_rect.height() * doc.height as f32;
                    state.dragging_guide = Some((true, doc_y.round()));
                } else {
                    let doc_x = (pos.x - canvas_rect.min.x) / canvas_rect.width() * doc.width as f32;
                    state.dragging_guide = Some((false, doc_x.round()));
                }
            }

            if response.drag_stopped() {
                if let Some((is_h, final_pos)) = state.dragging_guide.take() {
                    if is_h {
                        if final_pos >= 0.0 && final_pos <= doc.height as f32 {
                            state.horizontal_guides.push(final_pos);
                        }
                    } else {
                        if final_pos >= 0.0 && final_pos <= doc.width as f32 {
                            state.vertical_guides.push(final_pos);
                        }
                    }
                }
            }
        }

        let is_space_down = ui.input(|i| i.key_down(egui::Key::Space));

        if is_space_down {
            if response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                state.pan += response.drag_delta();
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
            state.last_pos = None;
        }

        // 5. ポインター入力・描画処理
        if let Some(pos) = response.hover_pos() {
            if canvas_rect.contains(pos) {
                let norm_x = (pos.x - canvas_rect.min.x) / canvas_rect.width();
                let norm_y = (pos.y - canvas_rect.min.y) / canvas_rect.height();
                let px = (norm_x * doc.width as f32) as u32;
                let py = (norm_y * doc.height as f32) as u32;

                if let Some(layer) = doc.active_layer() {
                    if let Some(col) = layer.buffer.get_pixel(px, py) {
                        state.hovered_pixel_info = Some((px, py, col));
                    }
                }

                if !is_space_down {
                    // ブラシ・消しゴム等のカーソルプレビュー円を描画 (Photoshop / クリスタ風)
                    match brush.tool {
                        BrushTool::Brush | BrushTool::Eraser | BrushTool::Pen => {
                            let radius = (brush.size * state.zoom * 0.5).max(2.0);
                            // 外枠 (サイズ)
                            painter.circle_stroke(
                                pos,
                                radius,
                                egui::Stroke::new(1.0_f32, Color32::from_white_alpha(200)),
                            );
                            // 硬さ（Hardness）のインナー円プレビュー (Hardness < 1.0 の場合)
                            if brush.hardness < 0.99  {
                                let inner_radius = radius * brush.hardness;
                                if inner_radius > 1.0 {
                                    painter.circle_stroke(
                                        pos,
                                        inner_radius,
                                        egui::Stroke::new(0.8_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 90)),
                                    );
                                }
                            }
                            // センタードット
                            painter.circle_filled(pos, 1.0, Color32::from_white_alpha(220));
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        }
                        BrushTool::Eyedropper => {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        }
                        _ => {}
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
                        } else if brush.tool == BrushTool::Pen {
                            let path = state.active_path.get_or_insert_with(|| {
                                let mut vp = iroai_core::VectorPath::new("Work Path");
                                vp.subpaths.push(iroai_core::SubPath {
                                    points: Vec::new(),
                                    closed: false,
                                });
                                vp
                            });
                            if let Some(sub) = path.subpaths.first_mut() {
                                let anchor_pt = iroai_core::PathPoint {
                                    anchor: (px as f32, py as f32),
                                    handle_in: None,
                                    handle_out: None,
                                };
                                sub.points.push(anchor_pt);
                                state.selected_anchor_idx = Some(sub.points.len() - 1);
                            }
                        } else if brush.tool == BrushTool::Text {
                            let initial_text = if let Some(layer) = doc.active_layer() {
                                if let iroai_core::LayerKind::Text { ref text, .. } = layer.kind {
                                    text.clone()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            state.text_input_prompt = Some((px, py, initial_text));
                        } else if brush.tool == BrushTool::Bucket {
                            if let Some(layer) = doc.active_layer_mut() {
                                let mut rec = iroai_core::StrokeRecorder::new(layer.id);
                                rec.record_area_before_touch(&layer.buffer, 0, 0, layer.buffer.width, layer.buffer.height);
                                iroai_core::Filters::flood_fill(&mut layer.buffer, px, py, brush.color, 20);
                                if let Some(action) = rec.finish(&layer.buffer, "Paint Bucket") {
                                    doc.history.push(action);
                                }
                            }
                        } else if brush.tool == BrushTool::RectSelect {
                            doc.selection.select_rect(px.saturating_sub(10), py.saturating_sub(10), 20, 20, SelectionOp::New);
                        } else {
                            let sel_ref = doc.selection.clone();
                            if let Some(layer) = doc.active_layer_mut() {
                                let mut recorder = iroai_core::StrokeRecorder::new(layer.id);
                                let r = (brush.size * 0.5 + 4.0).ceil() as u32;
                                recorder.record_area_before_touch(
                                    &layer.buffer,
                                    px.saturating_sub(r * 2),
                                    py.saturating_sub(r * 2),
                                    (px + r * 2).min(layer.buffer.width),
                                    (py + r * 2).min(layer.buffer.height),
                                );
                                if brush.tool == BrushTool::SpotHealing {
                                    iroai_core::Filters::apply_spot_heal(&mut layer.buffer, px as f32, py as f32, brush.size * 0.5);
                                } else if brush.tool != BrushTool::LiquifyPush && brush.tool != BrushTool::LiquifyBloat {
                                    brush.paint_pointer_stamp_masked(&mut layer.buffer, &ev, Some(&sel_ref));
                                }
                                state.stroke_recorder = Some(recorder);
                            }
                        }
                    } else if response.dragged() {
                        if brush.tool == BrushTool::Pen {
                            if let Some(ref mut path) = state.active_path {
                                if let Some(sub) = path.subpaths.first_mut() {
                                    if let Some(idx) = state.selected_anchor_idx {
                                        if let Some(pt) = sub.points.get_mut(idx) {
                                            pt.handle_out = Some((px as f32, py as f32));
                                            let dx = px as f32 - pt.anchor.0;
                                            let dy = py as f32 - pt.anchor.1;
                                            pt.handle_in = Some((pt.anchor.0 - dx, pt.anchor.1 - dy));
                                        }
                                    }
                                }
                            }
                        } else if let Some(prev) = state.last_pos {
                            let prev_nx = (prev.x - canvas_rect.min.x) / canvas_rect.width();
                            let prev_ny = (prev.y - canvas_rect.min.y) / canvas_rect.height();
                            let ppx = prev_nx * doc.width as f32;
                            let ppy = prev_ny * doc.height as f32;

                            let sel_ref = doc.selection.clone();
                            if let Some(layer) = doc.active_layer_mut() {
                                if let Some(ref mut recorder) = state.stroke_recorder {
                                    let min_x = ((ppx.min(px as f32) - brush.size * 0.5 - 4.0).max(0.0) as u32).min(layer.buffer.width);
                                    let min_y = ((ppy.min(py as f32) - brush.size * 0.5 - 4.0).max(0.0) as u32).min(layer.buffer.height);
                                    let max_x = ((ppx.max(px as f32) + brush.size * 0.5 + 4.0).min(layer.buffer.width as f32) as u32).min(layer.buffer.width);
                                    let max_y = ((ppy.max(py as f32) + brush.size * 0.5 + 4.0).min(layer.buffer.height as f32) as u32).min(layer.buffer.height);
                                    recorder.record_area_before_touch(&layer.buffer, min_x, min_y, max_x, max_y);
                                }
                                if brush.tool == BrushTool::LiquifyPush {
                                    let dx = px as f32 - ppx;
                                    let dy = py as f32 - ppy;
                                    iroai_core::Transform::apply_liquify_push(&mut layer.buffer, px as f32, py as f32, dx, dy, brush.size * 0.5, brush.flow);
                                } else if brush.tool == BrushTool::LiquifyBloat {
                                    iroai_core::Transform::apply_liquify_bloat(&mut layer.buffer, px as f32, py as f32, brush.size * 0.5, brush.flow);
                                } else if brush.tool == BrushTool::SpotHealing {
                                    iroai_core::Filters::apply_spot_heal(&mut layer.buffer, px as f32, py as f32, brush.size * 0.5);
                                } else {
                                    brush.paint_line_masked(&mut layer.buffer, ppx, ppy, px as f32, py as f32, Some(&sel_ref));
                                }
                            }
                        }
                        state.last_pos = Some(pos);
                    } else if response.drag_stopped() {
                        if let Some(recorder) = state.stroke_recorder.take() {
                            if let Some(layer) = doc.active_layer() {
                                let action_name = match brush.tool {
                                    BrushTool::LiquifyPush => "Liquify Forward Warp",
                                    BrushTool::LiquifyBloat => "Liquify Bloat / Pinch",
                                    BrushTool::SpotHealing => "Spot Healing",
                                    BrushTool::Eraser => "Eraser",
                                    BrushTool::CloneStamp => "Clone Stamp",
                                    _ => "Brush Stroke",
                                };
                                if let Some(action) = recorder.finish(&layer.buffer, action_name) {
                                    doc.history.push(action);
                                }
                            }
                        }
                        state.last_pos = None;
                    }
                }
            } else {
                state.hovered_pixel_info = None;
            }
        }

        // Render In-Canvas Text Tool Floating Editor (Photoshop on-canvas type prompt)
        let mut commit_text = None;
        let mut close_text = false;

        if let Some((tx, ty, ref mut text_str)) = state.text_input_prompt {
            let sx = canvas_rect.min.x + (tx as f32 / doc.width as f32) * canvas_rect.width();
            let sy = canvas_rect.min.y + (ty as f32 / doc.height as f32) * canvas_rect.height();

            egui::Area::new("text_tool_editor".into())
                .fixed_pos(Pos2::new(sx, sy))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label("🅃 Type Text (Enter: Newline, Ctrl+Enter: Commit):");
                            let te = ui.add(
                                egui::TextEdit::multiline(text_str)
                                    .desired_rows(3)
                                    .desired_width(240.0)
                            );
                            if te.has_focus() && ui.input(|i| (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::Enter)) {
                                commit_text = Some((tx, ty, text_str.clone()));
                            }
                            ui.horizontal(|ui| {
                                if ui.button("Commit (✓)").clicked() {
                                    commit_text = Some((tx, ty, text_str.clone()));
                                }
                                if ui.button("Cancel (✗)").clicked() {
                                    close_text = true;
                                }
                            });
                        });
                    });
                });
        }

        if let Some((tx, ty, text_content)) = commit_text {
            if !text_content.is_empty() {
                let font_size = brush.size.max(8.0);
                // Check if active layer is already a text layer: if so, update it
                let mut updated_existing = false;
                if let Some(layer) = doc.active_layer_mut() {
                    if matches!(layer.kind, iroai_core::LayerKind::Text { .. }) {
                        layer.update_text(&text_content, font_size, brush.color);
                        updated_existing = true;
                    }
                }

                if !updated_existing {
                    // Create a dedicated non-destructive Text Layer (Photoshop-standard behavior)
                    let text_layer_name = if text_content.len() > 16 {
                        format!("\"{}...\"", &text_content[..16])
                    } else {
                        format!("\"{}\"", text_content)
                    };
                    let text_layer = iroai_core::Layer::new_text(
                        text_layer_name,
                        &text_content,
                        font_size,
                        brush.color,
                        tx as i32,
                        ty as i32,
                        doc.width,
                        doc.height,
                    );
                    let new_id = text_layer.id;
                    let index = doc.layers.len();
                    doc.layers.push(text_layer);
                    doc.active_layer_id = Some(new_id);
                    doc.history.push(iroai_core::history::HistoryAction::LayerCreated {
                        index,
                        id: new_id,
                        name: format!("Text: {}", text_content),
                        width: doc.width,
                        height: doc.height,
                        kind: iroai_core::LayerKind::Text {
                            text: text_content,
                            font_size,
                            color: brush.color,
                            x: tx as i32,
                            y: ty as i32,
                        },
                    });
                }
            }
            state.text_input_prompt = None;
        } else if close_text {
            state.text_input_prompt = None;
        }

        response
    }
}
