use eframe::egui;
use iroai_core::{
    BackupConfig, Brush, Document, ImageIo, PhotoAdjustments, PsdHandler, RecoveryManager,
};
use iroai_moufu::MoufuClient;

use crate::canvas::{CanvasState, CanvasWidget};
use crate::panels::Panels;
use crate::tablet_layout::TabletLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPanelTab {
    Layers,
    Adjust,
    Brush,
    Info,
}

pub struct IroaiApp {
    pub documents: Vec<Document>,
    pub active_doc_index: usize,
    pub canvas_state: CanvasState,
    pub brush: Brush,
    pub photo_adj: PhotoAdjustments,
    pub moufu: MoufuClient,
    pub recovery: RecoveryManager,
    pub is_tablet_mode: bool,
    pub status_message: String,
    pub toast_message: Option<(String, std::time::Instant)>,
    pub layer_search_query: String,
    pub right_panel_tab: RightPanelTab,
    pub transform_session: crate::transform_tool::TransformSession,
}

impl IroaiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::theme::apply_pro_theme(&cc.egui_ctx);
        let doc = Document::new(800, 600, "Untitled-1");
        let mut moufu = MoufuClient::new();
        let _ = moufu.try_connect();
        let recovery = RecoveryManager::new(BackupConfig::default());

        Self {
            documents: vec![doc],
            active_doc_index: 0,
            canvas_state: CanvasState::default(),
            brush: Brush::default(),
            photo_adj: PhotoAdjustments::default(),
            moufu,
            recovery,
            is_tablet_mode: false,
            status_message: "Ready".to_string(),
            toast_message: None,
            layer_search_query: String::new(),
            right_panel_tab: RightPanelTab::Layers,
            transform_session: crate::transform_tool::TransformSession::default(),
        }
    }

    pub fn current_doc_mut(&mut self) -> &mut Document {
        &mut self.documents[self.active_doc_index]
    }

    pub fn current_doc(&self) -> &Document {
        &self.documents[self.active_doc_index]
    }

    pub fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast_message = Some((msg.into(), std::time::Instant::now()));
    }
}

impl eframe::App for IroaiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Global Keyboard Shortcuts ---
        ctx.input(|i| {
            let cmd_or_ctrl = i.modifiers.command || i.modifiers.ctrl;
            let shift = i.modifiers.shift;

            // Undo / Redo
            if cmd_or_ctrl && !shift && i.key_pressed(egui::Key::Z) {
                if self.current_doc_mut().undo() {
                    self.show_toast("↶ Undo");
                }
            } else if (cmd_or_ctrl && shift && i.key_pressed(egui::Key::Z)) || (cmd_or_ctrl && i.key_pressed(egui::Key::Y)) {
                if self.current_doc_mut().redo() {
                    self.show_toast("↷ Redo");
                }
            }

            // Brush Size adjustments: [ and ]
            if i.key_pressed(egui::Key::OpenBracket) {
                self.brush.size = (self.brush.size * 0.8).max(1.0);
                self.show_toast(format!("Brush Size: {:.0} px", self.brush.size));
            } else if i.key_pressed(egui::Key::CloseBracket) {
                self.brush.size = (self.brush.size * 1.25).min(500.0);
                self.show_toast(format!("Brush Size: {:.0} px", self.brush.size));
            }

            // Tool Switching Shortcuts (without modifiers)
            if !cmd_or_ctrl && !i.modifiers.alt {
                if i.key_pressed(egui::Key::B) {
                    self.brush.tool = iroai_core::BrushTool::Brush;
                    self.show_toast("🖌 Brush Tool (B)");
                } else if i.key_pressed(egui::Key::E) {
                    self.brush.tool = iroai_core::BrushTool::Eraser;
                    self.show_toast("🧹 Eraser Tool (E)");
                } else if i.key_pressed(egui::Key::S) {
                    self.brush.tool = iroai_core::BrushTool::CloneStamp;
                    self.show_toast("📑 Clone Stamp (S)");
                } else if i.key_pressed(egui::Key::G) {
                    self.brush.tool = iroai_core::BrushTool::Bucket;
                    self.show_toast("🪣 Bucket Tool (G)");
                } else if i.key_pressed(egui::Key::I) {
                    self.brush.tool = iroai_core::BrushTool::Eyedropper;
                    self.show_toast("🔍 Eyedropper (I)");
                } else if i.key_pressed(egui::Key::M) {
                    self.brush.tool = iroai_core::BrushTool::RectSelect;
                    self.show_toast("🔲 Rect Marquee (M)");
                }
            }

            // Free Transform: Ctrl+T / Cmd+T
            if cmd_or_ctrl && i.key_pressed(egui::Key::T) {
                if self.transform_session.is_active {
                    self.show_toast("Transform already active (Enter to commit, Esc to cancel)");
                } else {
                    let doc = &self.documents[self.active_doc_index];
                    self.transform_session.begin(doc);
                    self.show_toast("Transform: Enter to Commit, Esc to Cancel");
                }
            }

            // Commit / Cancel Transform
            if self.transform_session.is_active {
                if i.key_pressed(egui::Key::Enter) {
                    let doc = &mut self.documents[self.active_doc_index];
                    self.transform_session.commit(doc);
                    self.show_toast("Transform Committed");
                } else if i.key_pressed(egui::Key::Escape) {
                    let doc = &mut self.documents[self.active_doc_index];
                    self.transform_session.cancel(doc);
                    self.show_toast("Transform Cancelled");
                }
            }

            // Zoom presets: Ctrl+0 (Fit), Ctrl+1 (100%)
            if cmd_or_ctrl && i.key_pressed(egui::Key::Num0) {
                self.canvas_state.zoom = 1.0;
                self.canvas_state.pan = egui::Vec2::ZERO;
                self.show_toast("View: Fit Screen (100%)");
            } else if cmd_or_ctrl && i.key_pressed(egui::Key::Num1) {
                self.canvas_state.zoom = 1.0;
                self.canvas_state.pan = egui::Vec2::ZERO;
                self.show_toast("View: Actual Pixels (1:1)");
            }
        });
        // 定期自動保存判定
        if self.recovery.should_auto_save() {
            let doc = &self.documents[self.active_doc_index];
            if let Ok(path) = self.recovery.create_auto_backup(doc) {
                self.status_message = format!("Auto-saved backup to {:?}", path.file_name());
            }
        }

        // 1. トップメニューバー
        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Document (800x600)").clicked() {
                        let count = self.documents.len() + 1;
                        let new_doc = Document::new(800, 600, format!("Untitled-{}", count));
                        self.documents.push(new_doc);
                        self.active_doc_index = self.documents.len() - 1;
                        self.canvas_state = CanvasState::default();
                        ui.close_menu();
                    }
                    if ui.button("Open Image...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "tif", "tiff"])
                            .pick_file()
                        {
                            if let Ok(new_doc) = ImageIo::load_image(&path) {
                                self.documents.push(new_doc);
                                self.active_doc_index = self.documents.len() - 1;
                                self.canvas_state = CanvasState::default();
                                self.status_message = format!("Loaded image: {:?}", path.file_name());
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Import PSD...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("Photoshop PSD", &["psd"]).pick_file() {
                            if let Ok(new_doc) = PsdHandler::load_psd(&path) {
                                self.documents.push(new_doc);
                                self.active_doc_index = self.documents.len() - 1;
                                self.canvas_state = CanvasState::default();
                                self.status_message = format!("Imported PSD: {:?}", path.file_name());
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Save Project (.iroai)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("Iroai Project", &["iroai"]).save_file() {
                            let doc = self.current_doc();
                            let _ = ImageIo::save_project(doc, &path);
                            self.status_message = "Project saved.".to_string();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Export PNG...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("PNG", &["png"]).save_file() {
                            let doc = self.current_doc();
                            let _ = ImageIo::export_png(doc, &path);
                            self.status_message = "Exported PNG.".to_string();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Export PSD...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("Photoshop PSD", &["psd"]).save_file() {
                            let doc = self.current_doc();
                            let _ = PsdHandler::export_psd(doc, &path);
                            self.status_message = "Exported PSD.".to_string();
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        self.current_doc_mut().undo();
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked() {
                        self.current_doc_mut().redo();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Fill Foreground Color").clicked() {
                        let col = self.brush.color;
                        if let Some(layer) = self.current_doc_mut().active_layer_mut() {
                            layer.buffer.fill(col);
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("Image", |ui| {
                    if ui.button("Flip Horizontal").clicked() {
                        if let Some(layer) = self.current_doc_mut().active_layer_mut() {
                            iroai_core::Transform::flip_horizontal(&mut layer.buffer);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Flip Vertical").clicked() {
                        if let Some(layer) = self.current_doc_mut().active_layer_mut() {
                            iroai_core::Transform::flip_vertical(&mut layer.buffer);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Rotate 90° CW").clicked() {
                        if let Some(layer) = self.current_doc_mut().active_layer_mut() {
                            layer.buffer = iroai_core::Transform::rotate_90_cw(&layer.buffer);
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("Filter", |ui| {
                    if ui.button("Gaussian Blur").clicked() {
                        if let Some(layer) = self.current_doc_mut().active_layer_mut() {
                            iroai_core::Filters::apply_gaussian_blur(&mut layer.buffer, 3);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Sharpen").clicked() {
                        if let Some(layer) = self.current_doc_mut().active_layer_mut() {
                            iroai_core::Filters::apply_sharpen(&mut layer.buffer, 1.5);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Invert").clicked() {
                        if let Some(layer) = self.current_doc_mut().active_layer_mut() {
                            iroai_core::Filters::apply_invert(&mut layer.buffer);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Grayscale").clicked() {
                        if let Some(layer) = self.current_doc_mut().active_layer_mut() {
                            iroai_core::Filters::apply_grayscale(&mut layer.buffer);
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("100% Zoom (1:1)").clicked() {
                        self.canvas_state.zoom = 1.0;
                        self.canvas_state.pan = egui::Vec2::ZERO;
                        ui.close_menu();
                    }
                    if ui.button("Fit on Screen").clicked() {
                        self.canvas_state.zoom = 1.0;
                        self.canvas_state.pan = egui::Vec2::ZERO;
                        ui.close_menu();
                    }
                    ui.checkbox(&mut self.is_tablet_mode, "Tablet Mode UI");
                });

                ui.separator();
                let moufu_status = if self.moufu.is_connected { "🟢 Moufu Connected" } else { "⚪ Moufu Offline" };
                ui.label(moufu_status);
            });
        });

        // 1.5 水平コンテキスト・ツールオプションバー (Photoshop風)
        egui::TopBottomPanel::top("tool_options_bar").show(ctx, |ui| {
            Panels::render_tool_options_bar(ui, &mut self.brush);
        });

        // 2. ドキュメントタブバー (複数ドキュメント切り替え)
        egui::TopBottomPanel::top("doc_tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut doc_to_select = None;
                for (i, doc) in self.documents.iter().enumerate() {
                    let is_selected = i == self.active_doc_index;
                    let label = format!("📄 {}", doc.title);
                    if ui.selectable_label(is_selected, label).clicked() {
                        doc_to_select = Some(i);
                    }
                }
                if let Some(idx) = doc_to_select {
                    self.active_doc_index = idx;
                    self.canvas_state = CanvasState::default();
                }
            });
        });

        // 3. タブレットモード時の上部クイックバー
        if self.is_tablet_mode {
            egui::TopBottomPanel::top("tablet_quickbar").show(ctx, |ui| {
                let can_undo = self.current_doc().history.can_undo();
                let can_redo = self.current_doc().history.can_redo();
                TabletLayout::render_touch_quickbar(
                    ui,
                    &mut self.brush.tool,
                    can_undo,
                    can_redo,
                    || {},
                    || {},
                );
            });
        }

        // 4. ボトムステータスバー
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let doc = self.current_doc();
            ui.horizontal(|ui| {
                ui.label(format!("Doc: {}x{} px | Zoom: {:.0}% | {}", doc.width, doc.height, self.canvas_state.zoom * 100.0, self.status_message));
            });
        });

        // 5. 左ツールバー (2列コンパクトパレット: 68px幅)
        egui::SidePanel::left("left_tools").exact_width(68.0).resizable(false).show(ctx, |ui| {
            Panels::render_toolbar(ui, &mut self.brush);
        });

        // 6. 右パネル (Photoshop/Affinity風 タブ式ドック)
        egui::SidePanel::right("right_panels").default_width(300.0).min_width(240.0).max_width(500.0).resizable(true).show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.right_panel_tab, RightPanelTab::Layers, "Layers");
                ui.selectable_value(&mut self.right_panel_tab, RightPanelTab::Adjust, "Adjust");
                ui.selectable_value(&mut self.right_panel_tab, RightPanelTab::Brush, "Brush");
                ui.selectable_value(&mut self.right_panel_tab, RightPanelTab::Info, "Info");
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let doc = &mut self.documents[self.active_doc_index];
                match self.right_panel_tab {
                    RightPanelTab::Layers => {
                        Panels::render_layer_panel(ui, doc, &mut self.layer_search_query);
                    }
                    RightPanelTab::Adjust => {
                        Panels::render_photo_settings(ui, &mut self.photo_adj, doc);
                    }
                    RightPanelTab::Brush => {
                        Panels::render_brush_settings(ui, &mut self.brush);
                    }
                    RightPanelTab::Info => {
                        Panels::render_histogram_and_info(ui, doc, self.canvas_state.hovered_pixel_info);
                    }
                }
            });
        });

        // 7. 中央キャンバスエリア
        egui::CentralPanel::default().show(ctx, |ui| {
            let doc = &mut self.documents[self.active_doc_index];
            let resp = CanvasWidget::ui(ui, doc, &mut self.canvas_state, &mut self.brush);

            // Render Transform Gizmo if active
            let center = resp.rect.center() + self.canvas_state.pan;
            let doc_w = doc.width as f32 * self.canvas_state.zoom;
            let doc_h = doc.height as f32 * self.canvas_state.zoom;
            let canvas_rect = egui::Rect::from_center_size(center, egui::Vec2::new(doc_w, doc_h));
            self.transform_session.render_gizmo(ui, canvas_rect, doc.width as f32, doc.height as f32);

            // Floating Toast HUD Overlay
            let mut remove_toast = false;
            if let Some((msg, instant)) = &self.toast_message {
                let elapsed = instant.elapsed().as_secs_f32();
                if elapsed > 2.5 {
                    remove_toast = true;
                } else {
                    let alpha = ((2.5 - elapsed) / 0.5).clamp(0.0, 1.0);
                    let toast_rect = egui::Rect::from_center_size(
                        resp.rect.center_bottom() - egui::vec2(0.0, 48.0),
                        egui::vec2(280.0, 34.0),
                    );
                    let p = ui.painter();
                    p.rect_filled(toast_rect, 4.0, egui::Color32::from_black_alpha((190.0 * alpha) as u8));
                    p.rect_stroke(toast_rect, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0, 150, 255)));
                    p.text(
                        toast_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        msg,
                        egui::FontId::proportional(13.0),
                        egui::Color32::from_white_alpha((240.0 * alpha) as u8),
                    );
                }
            }
            if remove_toast {
                self.toast_message = None;
            }
        });
    }
}
