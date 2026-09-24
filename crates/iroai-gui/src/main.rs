pub mod color_picker;
pub mod transform_tool;
pub mod app;
pub mod canvas;
pub mod panels;
pub mod tablet_layout;
pub mod theme;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Iroai — Professional Image Editor"),
        ..Default::default()
    };
    eframe::run_native(
        "Iroai",
        native_options,
        Box::new(|cc| Ok(Box::new(app::IroaiApp::new(cc)))),
    )
}
