use egui::{Color32, Ui, Vec2};
use iroai_core::{
    BlendMode, Brush, BrushTool, Color, ColorChannelManager,
    Document, DropShadow, Layer, LayerStyle,
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
                BrushTool::Text => "🅃 Horizontal Type Tool",
                BrushTool::LiquifyPush => "🌀 Liquify Forward Warp",
                BrushTool::LiquifyBloat => "🫧 Liquify Bloat / Pinch",
                BrushTool::SpotHealing => "🩹 Spot Healing Brush",
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
                    ui.separator();
                    ui.checkbox(&mut brush.lock_alpha, "🔒 Lock Alpha");
                }
                BrushTool::SpotHealing => {
                    ui.label("Heal Radius:");
                    ui.add(egui::Slider::new(&mut brush.size, 4.0..=100.0).suffix(" px").logarithmic(true));
                    ui.separator();
                    ui.label("Type: Proximity Texture Match (Harmonic Blend)");
                }
                BrushTool::LiquifyPush | BrushTool::LiquifyBloat => {
                    ui.label("Brush Size:");
                    ui.add(egui::Slider::new(&mut brush.size, 10.0..=300.0).suffix(" px").logarithmic(true));
                    ui.label("Pressure / Strength:");
                    ui.add(egui::Slider::new(&mut brush.flow, 0.05..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0)));
                    ui.separator();
                    ui.label("Interactive Mesh Warp");
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
                BrushTool::Text => {
                    ui.label("Font Size:");
                    ui.add(egui::Slider::new(&mut brush.size, 1.0..=120.0).suffix(" pt").logarithmic(true));
                    ui.separator();
                    ui.label("Font: OpenType / TrueType Pro");
                    ui.separator();
                    ui.label("Anti-aliasing: Subpixel Fontdue");
                }
            }
        });
    }

    /// 左ツールバー（Photoshop準拠の2列コンパクト・アイコンパレット）
    pub fn render_toolbar(ui: &mut Ui, brush: &mut Brush) {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            let tools = [
                (BrushTool::Pen, "✒", "Pen Tool (P)"),
                (BrushTool::Text, "🅃", "Horizontal Type Tool (T)"),
                (BrushTool::Brush, "🖌", "Brush Tool (B)"),
                (BrushTool::SpotHealing, "🩹", "Spot Healing Brush (J)"),
                (BrushTool::Eraser, "🧹", "Eraser Tool (E)"),
                (BrushTool::LiquifyPush, "🌀", "Liquify Forward Warp (W)"),
                (BrushTool::LiquifyBloat, "🫧", "Liquify Bloat / Pinch"),
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
                (BrushTool::LassoSelect, "➰", "Lasso Tool (L)"),
                (BrushTool::Line, "📏", "Line Tool"),
                (BrushTool::ShapeRect, "⬜", "Rectangle Shape (U)"),
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

            // Color Swatch with HSV Wheel popup
            let (rect_fg, resp_fg) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::click());
            ui.painter().rect_filled(rect_fg, 2.0, Color32::from_rgba_unmultiplied(brush.color.r, brush.color.g, brush.color.b, brush.color.a));
            ui.painter().rect_stroke(rect_fg, 2.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(140, 140, 140)));
            resp_fg.clone().on_hover_text("Foreground Color (Click to open HSV Color Wheel)");

            let popup_id = ui.make_persistent_id("color_picker_popup");
            if resp_fg.clicked() {
                ui.memory_mut(|m| m.toggle_popup(popup_id));
            }
            egui::popup_below_widget(ui, popup_id, &resp_fg, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                ui.set_min_width(220.0);
                ui.heading("Color Wheel");
                ui.separator();
                crate::color_picker::ColorPickerWheel::show(ui, &mut brush.color);
            });
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
            ui.checkbox(&mut brush.lock_alpha, "🔒 Lock Transparent Pixels (Alpha Lock)");
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
                ui.menu_button("+ Adj ▾", |ui| {
                    let w = doc.width;
                    let h = doc.height;
                    if ui.button("☀️ Brightness / Contrast").clicked() {
                        let layer = Layer::new_adjustment("Brightness/Contrast", iroai_core::AdjustmentKind::BrightnessContrast { brightness: 0.0, contrast: 0.0 }, w, h);
                        let id = layer.id;
                        doc.layers.push(layer);
                        doc.active_layer_id = Some(id);
                        ui.close_menu();
                    }
                    if ui.button("🎨 Hue / Saturation").clicked() {
                        let layer = Layer::new_adjustment("Hue/Saturation", iroai_core::AdjustmentKind::HueSaturation { hue_shift: 0.0, saturation: 1.0 }, w, h);
                        let id = layer.id;
                        doc.layers.push(layer);
                        doc.active_layer_id = Some(id);
                        ui.close_menu();
                    }
                    if ui.button("⚡ Exposure").clicked() {
                        let layer = Layer::new_adjustment("Exposure", iroai_core::AdjustmentKind::Exposure { ev: 0.0 }, w, h);
                        let id = layer.id;
                        doc.layers.push(layer);
                        doc.active_layer_id = Some(id);
                        ui.close_menu();
                    }
                    if ui.button("📊 Levels").clicked() {
                        let layer = Layer::new_adjustment("Levels", iroai_core::AdjustmentKind::Levels { black_point: 0, gamma: 1.0, white_point: 255 }, w, h);
                        let id = layer.id;
                        doc.layers.push(layer);
                        doc.active_layer_id = Some(id);
                        ui.close_menu();
                    }
                    if ui.button("📷 Photo Master").clicked() {
                        let layer = Layer::new_adjustment("Photo Master", iroai_core::AdjustmentKind::Photo(iroai_core::PhotoAdjustments::default()), w, h);
                        let id = layer.id;
                        doc.layers.push(layer);
                        doc.active_layer_id = Some(id);
                        ui.close_menu();
                    }
                    if ui.button("⬛ Invert").clicked() {
                        let layer = Layer::new_adjustment("Invert", iroai_core::AdjustmentKind::Invert, w, h);
                        let id = layer.id;
                        doc.layers.push(layer);
                        doc.active_layer_id = Some(id);
                        ui.close_menu();
                    }
                    if ui.button("⚪ Grayscale").clicked() {
                        let layer = Layer::new_adjustment("Grayscale", iroai_core::AdjustmentKind::Grayscale, w, h);
                        let id = layer.id;
                        doc.layers.push(layer);
                        doc.active_layer_id = Some(id);
                        ui.close_menu();
                    }
                });
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

                ui.horizontal(|ui| {
                    let mut lock_a = layer.lock_alpha;
                    if ui.checkbox(&mut lock_a, "🔒 Lock Transparent").changed() {
                        layer.lock_alpha = lock_a;
                    }
                    let mut is_locked = layer.locked;
                    if ui.checkbox(&mut is_locked, "Lock All").changed() {
                        layer.locked = is_locked;
                    }
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

                        let mut glow_enabled = style.outer_glow.is_some();
                        if ui.checkbox(&mut glow_enabled, "Outer Glow (光彩・外側)").changed() {
                            if glow_enabled {
                                style.outer_glow = Some(iroai_core::style::OuterGlow::default());
                            } else {
                                style.outer_glow = None;
                            }
                        }
                        if let Some(ref mut og) = style.outer_glow {
                            og.enabled = true;
                            ui.add(egui::Slider::new(&mut og.radius, 1..=30).text("Glow Radius"));
                            ui.add(egui::Slider::new(&mut og.opacity, 0.0..=1.0).text("Opacity"));
                        }

                        let mut bevel_enabled = style.bevel_emboss.is_some();
                        if ui.checkbox(&mut bevel_enabled, "Bevel & Emboss (ベベルとエンボス)").changed() {
                            if bevel_enabled {
                                style.bevel_emboss = Some(iroai_core::style::BevelAndEmboss::default());
                            } else {
                                style.bevel_emboss = None;
                            }
                        }
                        if let Some(ref mut be) = style.bevel_emboss {
                            be.enabled = true;
                            ui.add(egui::Slider::new(&mut be.depth, 0.5..=8.0).text("Depth"));
                            ui.add(egui::Slider::new(&mut be.size, 1..=15).text("Size"));
                            ui.add(egui::Slider::new(&mut be.angle_deg, 0.0..=360.0).text("Light Angle"));
                        }
                    }
                });

                // Non-destructive Text Layer live properties editor
                let mut text_update = None;
                if let iroai_core::LayerKind::Text { ref text, font_size, color, .. } = layer.kind {
                    let mut cur_text = text.clone();
                    let mut cur_size = font_size;
                    let mut cur_color = color;
                    let mut changed = false;

                    ui.collapsing("🅃 Live Text Properties", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Text:");
                            if ui.text_edit_singleline(&mut cur_text).changed() {
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            if ui.add(egui::Slider::new(&mut cur_size, 6.0..=120.0).suffix(" pt")).changed() {
                                changed = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Color:");
                            let mut egui_col = [cur_color.r as f32 / 255.0, cur_color.g as f32 / 255.0, cur_color.b as f32 / 255.0];
                            if ui.color_edit_button_rgb(&mut egui_col).changed() {
                                cur_color = Color::rgba(
                                    (egui_col[0] * 255.0).round() as u8,
                                    (egui_col[1] * 255.0).round() as u8,
                                    (egui_col[2] * 255.0).round() as u8,
                                    cur_color.a,
                                );
                                changed = true;
                            }
                        });
                    });

                    if changed {
                        text_update = Some((cur_text, cur_size, cur_color));
                    }
                }

                if let Some((new_txt, new_sz, new_col)) = text_update {
                    layer.update_text(&new_txt, new_sz, new_col);
                }

                // Non-destructive Adjustment Layer live properties editor
                if let iroai_core::LayerKind::Adjustment(ref mut adj) = layer.kind {
                    ui.collapsing("⚖ Live Adjustment Controls", |ui| {
                        match adj {
                            iroai_core::AdjustmentKind::BrightnessContrast { brightness, contrast } => {
                                ui.add(egui::Slider::new(brightness, -1.0..=1.0).text("Brightness"));
                                ui.add(egui::Slider::new(contrast, -1.0..=1.0).text("Contrast"));
                            }
                            iroai_core::AdjustmentKind::HueSaturation { hue_shift, saturation } => {
                                ui.add(egui::Slider::new(hue_shift, -180.0..=180.0).text("Hue Shift"));
                                ui.add(egui::Slider::new(saturation, 0.0..=3.0).text("Saturation"));
                            }
                            iroai_core::AdjustmentKind::Exposure { ev } => {
                                ui.add(egui::Slider::new(ev, -5.0..=5.0).text("Exposure (EV)"));
                            }
                            iroai_core::AdjustmentKind::Levels { black_point, gamma, white_point } => {
                                ui.add(egui::Slider::new(black_point, 0..=254).text("Black Point"));
                                ui.add(egui::Slider::new(gamma, 0.1..=5.0).text("Midtone Gamma"));
                                ui.add(egui::Slider::new(white_point, 1..=255).text("White Point"));
                            }
                            iroai_core::AdjustmentKind::Photo(photo_adj) => {
                                ui.add(egui::Slider::new(&mut photo_adj.exposure, -5.0..=5.0).text("Exposure"));
                                ui.add(egui::Slider::new(&mut photo_adj.highlights, -100.0..=100.0).text("Highlights"));
                                ui.add(egui::Slider::new(&mut photo_adj.shadows, -100.0..=100.0).text("Shadows"));
                                ui.add(egui::Slider::new(&mut photo_adj.whites, -100.0..=100.0).text("Whites"));
                                ui.add(egui::Slider::new(&mut photo_adj.blacks, -100.0..=100.0).text("Blacks"));
                                ui.add(egui::Slider::new(&mut photo_adj.temperature, -100.0..=100.0).text("Temp"));
                                ui.add(egui::Slider::new(&mut photo_adj.tint, -100.0..=100.0).text("Tint"));
                                ui.add(egui::Slider::new(&mut photo_adj.vibrance, -100.0..=100.0).text("Vibrance"));
                                ui.add(egui::Slider::new(&mut photo_adj.saturation, -100.0..=100.0).text("Saturation"));
                                ui.add(egui::Slider::new(&mut photo_adj.clarity, -100.0..=100.0).text("Clarity"));
                            }
                            iroai_core::AdjustmentKind::Threshold { cutoff } => {
                                ui.add(egui::Slider::new(cutoff, 0..=255).text("Cutoff"));
                            }
                            iroai_core::AdjustmentKind::Posterize { levels } => {
                                ui.add(egui::Slider::new(levels, 2..=32).text("Levels"));
                            }
                            iroai_core::AdjustmentKind::Invert => {
                                ui.label("Inverts all color channels below this layer.");
                            }
                            iroai_core::AdjustmentKind::Grayscale => {
                                ui.label("Converts composite below to perceptual luminance.");
                            }
                            iroai_core::AdjustmentKind::Curves { .. } => {
                                ui.label("Tone curve mapping active.");
                            }
                        }
                    });
                }

                ui.separator();
            }

            // フィルタリングされたレイヤーリスト (D&D / 並べ替え対応)
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut layer_to_select = None;
                let mut move_op: Option<(usize, usize)> = None;
                let num_layers = doc.layers.len();

                for rev_idx in 0..num_layers {
                    let actual_idx = num_layers - 1 - rev_idx;
                    let layer = &mut doc.layers[actual_idx];

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
                        let lock_icon = if layer.lock_alpha { "🔒 " } else { "" };
                        let type_icon = match layer.kind {
                            iroai_core::LayerKind::Text { .. } => "🅃 ",
                            iroai_core::LayerKind::Adjustment(_) => "⚖ ",
                            iroai_core::LayerKind::Group { .. } => "📁 ",
                            iroai_core::LayerKind::SmartObject(_) => "📦 ",
                            iroai_core::LayerKind::Raster => "",
                        };
                        let name_label = format!("{}{}{}{}", clip_prefix, lock_icon, type_icon, layer.name);

                        if ui.selectable_label(is_active, name_label).clicked() {
                            layer_to_select = Some(layer.id);
                        }

                        // クイック移動ボタン (D&D代替・即応性)
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if actual_idx < num_layers - 1 && ui.small_button("▲").on_hover_text("Move Layer Up").clicked() {
                                move_op = Some((actual_idx, actual_idx + 1));
                            }
                            if actual_idx > 0 && ui.small_button("▼").on_hover_text("Move Layer Down").clicked() {
                                move_op = Some((actual_idx, actual_idx - 1));
                            }
                        });
                    });
                }
                if let Some(new_id) = layer_to_select {
                    doc.active_layer_id = Some(new_id);
                }
                if let Some((from, to)) = move_op {
                    doc.move_layer(from, to);
                }
            });
        });
    }

    /// ベクターパスパネル (Photoshopのパスパネル同等: 選択範囲化・境界線描画・塗りつぶし・閉じる)
    pub fn render_paths_panel(
        ui: &mut Ui,
        doc: &mut Document,
        canvas_state: &mut crate::canvas::CanvasState,
        brush: &Brush,
    ) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Vector Paths");
                if ui.button("+ New Path").clicked() {
                    let mut vp = iroai_core::VectorPath::new(format!("Path {}", doc.paths.len() + 1));
                    vp.subpaths.push(iroai_core::SubPath { points: Vec::new(), closed: false });
                    canvas_state.active_path = Some(vp);
                }
                if ui.button("🗑 Clear").clicked() {
                    canvas_state.active_path = None;
                    canvas_state.selected_anchor_idx = None;
                }
            });
            ui.separator();

            if let Some(ref mut path) = canvas_state.active_path {
                ui.label(format!("Active: {}", path.name));
                if let Some(sub) = path.subpaths.first_mut() {
                    ui.label(format!("Anchor Points: {}", sub.points.len()));
                    ui.checkbox(&mut sub.closed, "Close Path");

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("🔳 Load as Selection").clicked() {
                            let mask = path.to_selection_mask(doc.width, doc.height);
                            doc.selection = mask;
                        }
                        if ui.button("🖋 Stroke Path").clicked() {
                            if let Some(layer) = doc.active_layer_mut() {
                                path.rasterize_stroke(&mut layer.buffer, brush.color, brush.size);
                            }
                        }
                        if ui.button("🪣 Fill Path").clicked() {
                            if let Some(layer) = doc.active_layer_mut() {
                                path.rasterize_fill(&mut layer.buffer, brush.color);
                            }
                        }
                    });
                }
            } else {
                ui.label("No active vector path.");
                ui.label("Select the Pen Tool (P) and click on the canvas to place anchor points.");
            }
        });
    }
}
