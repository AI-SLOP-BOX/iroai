use egui::{Color32, Ui, Vec2};
use iroai_core::{
    BlendMode, Brush, BrushTool, Color, ColorChannelManager,
    Document, DropShadow, LayerStyle,
    PhotoAdjustments, PhotoProcessor, Stroke,
};

pub struct Panels;

impl Panels {
    /// 左ツールバー（Photoshop準拠ツールセット）
    pub fn render_toolbar(ui: &mut Ui, brush: &mut Brush) {
        ui.vertical(|ui| {
            ui.heading("Tools");
            ui.separator();

            let tools = [
                (BrushTool::Brush, "🖌 Brush"),
                (BrushTool::Eraser, "🧹 Eraser"),
                (BrushTool::CloneStamp, "📑 Clone Stamp"),
                (BrushTool::Blur, "💧 Blur Tool"),
                (BrushTool::Sharpen, "🔺 Sharpen Tool"),
                (BrushTool::Dodge, "☀️ Dodge Tool"),
                (BrushTool::Burn, "🌑 Burn Tool"),
                (BrushTool::Sponge, "🧽 Sponge Tool"),
                (BrushTool::Eyedropper, "🔍 Eyedropper"),
                (BrushTool::Bucket, "🪣 Paint Bucket"),
                (BrushTool::RectSelect, "🔲 Rect Marquee"),
                (BrushTool::EllipseSelect, "⚪ Ellipse Marquee"),
            ];

            for (tool, label) in tools {
                let is_selected = brush.tool == tool;
                if ui.selectable_label(is_selected, label).clicked() {
                    brush.tool = tool;
                }
            }
        });
    }

    /// ブラシ設定・写真補正パネル
    pub fn render_brush_and_photo_settings(
        ui: &mut Ui,
        brush: &mut Brush,
        photo_adj: &mut PhotoAdjustments,
        doc: &mut Document,
    ) {
        ui.vertical(|ui| {
            ui.collapsing("🖌 Brush Dynamics", |ui| {
                ui.add(egui::Slider::new(&mut brush.size, 1.0..=200.0).text("Size"));
                ui.add(egui::Slider::new(&mut brush.hardness, 0.0..=1.0).text("Hardness"));
                ui.add(egui::Slider::new(&mut brush.opacity, 0.0..=1.0).text("Opacity"));
                ui.checkbox(&mut brush.pressure_size, "Pressure Size");
                ui.checkbox(&mut brush.pressure_opacity, "Pressure Opacity");
            });

            ui.separator();

            ui.collapsing("📷 Photo Adjustments (Live)", |ui| {
                let mut changed = false;

                changed |= ui.add(egui::Slider::new(&mut photo_adj.exposure, -5.0..=5.0).text("Exposure (EV)")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.temperature, -100.0..=100.0).text("Color Temp")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.tint, -100.0..=100.0).text("Tint")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.highlights, -100.0..=100.0).text("Highlights")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.shadows, -100.0..=100.0).text("Shadows")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.whites, -100.0..=100.0).text("Whites")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.blacks, -100.0..=100.0).text("Blacks")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.vibrance, -100.0..=100.0).text("Vibrance")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.saturation, -100.0..=100.0).text("Saturation")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.clarity, -100.0..=100.0).text("Clarity")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.sharpness, 0.0..=100.0).text("Sharpness")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.noise_reduction, 0.0..=100.0).text("Noise Red.")).changed();
                changed |= ui.add(egui::Slider::new(&mut photo_adj.vignette, -100.0..=100.0).text("Vignette")).changed();

                if ui.button("Apply to Active Layer").clicked() || changed {
                    if let Some(layer) = doc.active_layer_mut() {
                        PhotoProcessor::apply_photo_adjustments(&mut layer.buffer, photo_adj);
                    }
                }

                if ui.button("Reset Photo Adjustments").clicked() {
                    *photo_adj = PhotoAdjustments::default();
                }
            });
        });
    }

    /// ヒストグラム・情報パネル
    pub fn render_histogram_and_info(
        ui: &mut Ui,
        doc: &Document,
        pixel_info: Option<(u32, u32, Color)>,
    ) {
        ui.vertical(|ui| {
            ui.heading("Histogram & Color Info");
            ui.separator();

            let composite = doc.composite();
            let hist = ColorChannelManager::calculate_histogram(&composite);

            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 80.0), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(rect);
            painter.rect_filled(rect, 2.0, Color32::from_rgb(30, 30, 30));

            if hist.max_count > 0 {
                let max = hist.max_count as f32;
                let step = rect.width() / 256.0;

                for i in 0..256 {
                    let h_lum = (hist.luminance[i] as f32 / max) * rect.height();
                    let x = rect.min.x + (i as f32) * step;
                    painter.line_segment(
                        [egui::pos2(x, rect.max.y), egui::pos2(x, rect.max.y - h_lum)],
                        egui::Stroke::new(1.0_f32, Color32::from_white_alpha(100)),
                    );
                }
            }

            ui.add_space(4.0);

            // ピクセルインスペクター
            if let Some((x, y, col)) = pixel_info {
                ui.label(format!("Cursor: X: {}  Y: {}", x, y));
                ui.horizontal(|ui| {
                    ui.label(format!("R: {}  G: {}  B: {}  A: {}", col.r, col.g, col.b, col.a));
                    let (rect_c, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect_c, 2.0, Color32::from_rgba_unmultiplied(col.r, col.g, col.b, col.a));
                });
                let (lr, lg, lb, _) = col.to_linear_f32();
                ui.label(format!("Linear RGB: ({:.3}, {:.3}, {:.3})", lr, lg, lb));
            } else {
                ui.label("Hover over canvas to inspect pixels");
            }
        });
    }

    /// レイヤー管理パネル (レイヤー検索、一括操作、スタイル、マスク、クリッピング、ブレンドモード)
    pub fn render_layer_panel(ui: &mut Ui, doc: &mut Document, search_query: &mut String) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Layers");
                if ui.button("+ New").clicked() {
                    let count = doc.layers.len() + 1;
                    doc.add_layer(format!("Layer {}", count));
                }
                if ui.button("🗑 Del").clicked() {
                    if let Some(id) = doc.active_layer_id {
                        if let Some(pos) = doc.layers.iter().position(|l| l.id == id) {
                            if doc.layers.len() > 1 {
                                let removed = doc.layers.remove(pos);
                                doc.active_layer_id = doc.layers.last().map(|l| l.id);
                                doc.history.push(iroai_core::history::HistoryAction::LayerRemoved {
                                    index: pos,
                                    layer: Box::new(removed),
                                });
                            }
                        }
                    }
                }
                if ui.button("🗗 Dup").clicked() {
                    if let Some(id) = doc.active_layer_id {
                        doc.duplicate_layer(id);
                    }
                }
            });

            // レイヤー一括操作 & 検索バー
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(search_query);
                if ui.button("👁 All").clicked() {
                    for l in &mut doc.layers {
                        l.visible = true;
                    }
                }
                if ui.button("🔒 All").clicked() {
                    for l in &mut doc.layers {
                        l.locked = !l.locked;
                    }
                }
            });

            ui.separator();

            let active_id = doc.active_layer_id;

            // アクティブレイヤーの設定
            if let Some(layer) = doc.active_layer_mut() {
                ui.horizontal(|ui| {
                    ui.label("Blend:");
                    egui::ComboBox::from_id_salt("layer_blend_mode")
                        .selected_text(layer.blend_mode.name())
                        .show_ui(ui, |ui| {
                            for mode in BlendMode::all() {
                                ui.selectable_value(&mut layer.blend_mode, *mode, mode.name());
                            }
                        });

                    ui.label("Opacity:");
                    ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).show_value(false));
                });

                ui.collapsing("✨ Layer Style", |ui| {
                    let mut has_style = layer.style.is_some();
                    if ui.checkbox(&mut has_style, "Enable Layer Effects").changed() {
                        if has_style {
                            layer.style = Some(LayerStyle::default());
                        } else {
                            layer.style = None;
                        }
                    }

                    if let Some(ref mut style) = layer.style {
                        let mut shadow_enabled = style.drop_shadow.is_some();
                        if ui.checkbox(&mut shadow_enabled, "Drop Shadow").changed() {
                            if shadow_enabled {
                                style.drop_shadow = Some(DropShadow::default());
                            } else {
                                style.drop_shadow = None;
                            }
                        }
                        if let Some(ref mut ds) = style.drop_shadow {
                            ui.add(egui::Slider::new(&mut ds.offset_x, -50..=50).text("Offset X"));
                            ui.add(egui::Slider::new(&mut ds.offset_y, -50..=50).text("Offset Y"));
                            ui.add(egui::Slider::new(&mut ds.blur_radius, 0..=50).text("Blur"));
                            ui.add(egui::Slider::new(&mut ds.opacity, 0.0..=1.0).text("Opacity"));
                        }

                        let mut stroke_enabled = style.stroke.is_some();
                        if ui.checkbox(&mut stroke_enabled, "Stroke").changed() {
                            if stroke_enabled {
                                style.stroke = Some(Stroke::default());
                            } else {
                                style.stroke = None;
                            }
                        }
                        if let Some(ref mut st) = style.stroke {
                            ui.add(egui::Slider::new(&mut st.size, 1..=20).text("Stroke Size"));
                        }
                    }
                });

                ui.separator();
            }

            // フィルタリングされたレイヤーリスト
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut layer_to_select = None;
                for layer in doc.layers.iter_mut().rev() {
                    if !search_query.is_empty() && !layer.name.to_lowercase().contains(&search_query.to_lowercase()) {
                        continue;
                    }

                    let is_active = Some(layer.id) == active_id;

                    ui.horizontal(|ui| {
                        let mut vis = layer.visible;
                        if ui.checkbox(&mut vis, "").changed() {
                            layer.visible = vis;
                        }

                        let clip_prefix = if layer.clipping_mask { "  ↳ " } else { "" };
                        let name_label = format!("{}{}", clip_prefix, layer.name);

                        if ui.selectable_label(is_active, name_label).clicked() {
                            layer_to_select = Some(layer.id);
                        }
                    });
                }
                if let Some(new_id) = layer_to_select {
                    doc.active_layer_id = Some(new_id);
                }
            });
        });
    }
}
