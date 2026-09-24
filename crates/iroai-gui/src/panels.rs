use egui::{Color32, Ui, Vec2};
use iroai_core::{
    BlendMode, Brush, BrushTool, Color, ColorChannelManager,
    Document, DropShadow, LayerStyle,
    PhotoAdjustments, PhotoProcessor, Stroke,
};

pub struct Panels;

impl Panels {
    /// 水平コンテキスト・ツールオプションバー（Photoshopのトップ下部プロパティバー）
    pub fn render_tool_options_bar(ui: &mut Ui, brush: &mut Brush) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(12.0, 0.0);

            let tool_name = match brush.tool {
                BrushTool::Pen => "✒ Pen",
                BrushTool::Brush => "🖌 Brush",
                BrushTool::Eraser => "🧹 Eraser",
                BrushTool::CloneStamp => "📑 Clone Stamp",
                BrushTool::Blur => "💧 Blur",
                BrushTool::Sharpen => "🔺 Sharpen",
                BrushTool::Dodge => "☀️ Dodge",
                BrushTool::Burn => "🌑 Burn",
                BrushTool::Sponge => "🧽 Sponge",
                BrushTool::Eyedropper => "🔍 Eyedropper",
                BrushTool::Bucket => "🪣 Bucket",
                BrushTool::RectSelect => "🔲 Rect Marquee",
                BrushTool::EllipseSelect => "⚪ Ellipse Marquee",
                BrushTool::LassoSelect => "➰ Lasso",
                BrushTool::Line => "📏 Line",
                BrushTool::ShapeRect => "⬜ Rectangle",
            };
            ui.label(egui::RichText::new(tool_name).strong().color(Color32::from_rgb(220, 220, 220)));
            ui.separator();

            match brush.tool {
                BrushTool::Brush | BrushTool::Eraser | BrushTool::CloneStamp | BrushTool::Pen => {
                    ui.label("Size:");
                    ui.add(egui::Slider::new(&mut brush.size, 1.0..=200.0).suffix(" px").logarithmic(true));

                    ui.label("Hardness:");
                    ui.add(egui::Slider::new(&mut brush.hardness, 0.0..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0)));

                    ui.label("Opacity:");
                    ui.add(egui::Slider::new(&mut brush.opacity, 0.0..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0)));

                    ui.label("Flow:");
                    ui.add(egui::Slider::new(&mut brush.flow, 0.01..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0)));

                    ui.label("Spacing:");
                    ui.add(egui::Slider::new(&mut brush.spacing, 0.05..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0)));

                    ui.separator();
                    ui.checkbox(&mut brush.pressure_size, "Pen Pressure");
                }
                BrushTool::Blur | BrushTool::Sharpen | BrushTool::Dodge | BrushTool::Burn | BrushTool::Sponge => {
                    ui.label("Size:");
                    ui.add(egui::Slider::new(&mut brush.size, 1.0..=200.0).suffix(" px"));
                    ui.label("Strength:");
                    ui.add(egui::Slider::new(&mut brush.flow, 0.01..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0)));
                }
                BrushTool::RectSelect | BrushTool::EllipseSelect | BrushTool::LassoSelect => {
                    ui.label("Mode: New Selection");
                    ui.separator();
                    ui.label("Feather: 0 px");
                    ui.separator();
                    ui.label("Anti-alias: On");
                }
                BrushTool::Bucket => {
                    ui.label("Tolerance: 20");
                    ui.separator();
                    ui.label("Contiguous: On");
                    ui.separator();
                    ui.label("All Layers: Off");
                }
                BrushTool::Eyedropper => {
                    ui.label("Sample: All Layers");
                    ui.separator();
                    ui.label("Sample Size: Point Sample");
                }
                BrushTool::Line | BrushTool::ShapeRect => {
                    ui.label("Stroke Width:");
                    ui.add(egui::Slider::new(&mut brush.size, 1.0..=50.0).suffix(" px"));
                    ui.separator();
                    ui.label("Anti-alias: On");
                }
            }
        });
    }

    /// 左ツールバー（Photoshop準拠の2列コンパクト・アイコンパレット）
    pub fn render_toolbar(ui: &mut Ui, brush: &mut Brush) {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            let tools = [
                (BrushTool::Brush, "🖌", "Brush Tool (B)"),
                (BrushTool::Eraser, "🧹", "Eraser Tool (E)"),
                (BrushTool::CloneStamp, "📑", "Clone Stamp (S)"),
                (BrushTool::Blur, "💧", "Blur Tool"),
                (BrushTool::Sharpen, "🔺", "Sharpen Tool"),
                (BrushTool::Dodge, "☀️", "Dodge Tool (O)"),
                (BrushTool::Burn, "🌑", "Burn Tool"),
                (BrushTool::Sponge, "🧽", "Sponge Tool"),
                (BrushTool::Eyedropper, "🔍", "Eyedropper (I)"),
                (BrushTool::Bucket, "🪣", "Paint Bucket (G)"),
                (BrushTool::RectSelect, "🔲", "Rect Marquee (M)"),
                (BrushTool::EllipseSelect, "⚪", "Ellipse Marquee"),
            ];

            egui::Grid::new("tool_grid").spacing(egui::vec2(2.0, 2.0)).show(ui, |ui| {
                for (i, (tool, icon, tip)) in tools.iter().enumerate() {
                    let is_selected = brush.tool == *tool;
                    let btn = egui::Button::new(*icon).min_size(Vec2::new(26.0, 26.0));
                    let resp = if is_selected {
                        ui.add(btn.fill(Color32::from_rgb(38, 79, 120)))
                    } else {
                        ui.add(btn)
                    };
                    if resp.on_hover_text(*tip).clicked() {
                        brush.tool = *tool;
                    }
                    if (i + 1) % 2 == 0 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(8.0);
            ui.separator();

            // Color Swatch
            let (rect_fg, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::hover());
            ui.painter().rect_filled(rect_fg, 2.0, Color32::from_rgba_unmultiplied(brush.color.r, brush.color.g, brush.color.b, brush.color.a));
            ui.painter().rect_stroke(rect_fg, 2.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(80, 80, 80)));
        });
    }

    /// ブラシ設定パネル
    pub fn render_brush_settings(ui: &mut Ui, brush: &mut Brush) {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.heading("Brush Dynamics");
            ui.separator();
            ui.add(egui::Slider::new(&mut brush.size, 1.0..=200.0).text("Size"));
            ui.add(egui::Slider::new(&mut brush.hardness, 0.0..=1.0).text("Hardness"));
            ui.add(egui::Slider::new(&mut brush.opacity, 0.0..=1.0).text("Opacity"));
            ui.add(egui::Slider::new(&mut brush.flow, 0.0..=1.0).text("Flow"));
            ui.add(egui::Slider::new(&mut brush.spacing, 0.01..=1.0).text("Spacing"));
            ui.checkbox(&mut brush.pressure_size, "Pressure Size");
            ui.checkbox(&mut brush.pressure_opacity, "Pressure Opacity");
        });
    }

    /// 写真調整パネル (Photoshop/Lightroom風 フラットスライダー)
    pub fn render_photo_settings(
        ui: &mut Ui,
        photo_adj: &mut PhotoAdjustments,
        doc: &mut Document,
    ) {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.heading("Adjustments");
            ui.separator();

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

            ui.add_space(8.0);
            if ui.button("Apply to Active Layer").clicked() || changed {
                if let Some(layer) = doc.active_layer_mut() {
                    PhotoProcessor::apply_photo_adjustments(&mut layer.buffer, photo_adj);
                }
            }

            if ui.button("Reset All Sliders").clicked() {
                *photo_adj = PhotoAdjustments::default();
            }
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
