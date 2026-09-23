pub mod app;
pub mod canvas;
pub mod panels;
pub mod tablet_layout;

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
