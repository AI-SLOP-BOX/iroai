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
    pub layer_search_query: String,
    pub right_panel_tab: RightPanelTab,
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
            layer_search_query: String::new(),
            right_panel_tab: RightPanelTab::Layers,
        }
    }

    pub fn current_doc_mut(&mut self) -> &mut Document {
        &mut self.documents[self.active_doc_index]
    }

    pub fn current_doc(&self) -> &Document {
        &self.documents[self.active_doc_index]
    }
}

impl eframe::App for IroaiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
        egui::SidePanel::right("right_panels").default_width(300.0).show(ctx, |ui| {
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
            CanvasWidget::ui(ui, doc, &mut self.canvas_state, &mut self.brush);
        });
    }
}
