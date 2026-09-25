use egui::{Ui, Vec2};
use iroai_core::BrushTool;

pub struct TabletLayout;

impl TabletLayout {
    pub const MIN_TOUCH_TARGET: f32 = 44.0;

    pub fn render_touch_quickbar(
        ui: &mut Ui,
        current_tool: &mut BrushTool,
        can_undo: bool,
        can_redo: bool,
        on_undo: impl FnOnce(),
        on_redo: impl FnOnce(),
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);

            let undo_btn = ui.add_sized(
                [Self::MIN_TOUCH_TARGET, Self::MIN_TOUCH_TARGET],
                egui::Button::new("↩").sense(if can_undo { egui::Sense::click() } else { egui::Sense::hover() }),
            );
            if can_undo && undo_btn.clicked() {
                on_undo();
            }

            let redo_btn = ui.add_sized(
                [Self::MIN_TOUCH_TARGET, Self::MIN_TOUCH_TARGET],
                egui::Button::new("↪").sense(if can_redo { egui::Sense::click() } else { egui::Sense::hover() }),
            );
            if can_redo && redo_btn.clicked() {
                on_redo();
            }

            ui.separator();

            let tools = [
                (BrushTool::Brush, "🖌 Brush"),
                (BrushTool::Eraser, "🧹 Eraser"),
                (BrushTool::CloneStamp, "📑 Clone"),
                (BrushTool::Blur, "💧 Blur"),
                (BrushTool::Dodge, "☀️ Dodge"),
                (BrushTool::Burn, "🌑 Burn"),
                (BrushTool::Eyedropper, "🔍 Pick"),
                (BrushTool::Bucket, "🪣 Fill"),
            ];

            for (tool, label) in tools {
                let is_active = *current_tool == tool;
                if ui
                    .add_sized(
                        [Self::MIN_TOUCH_TARGET * 1.5, Self::MIN_TOUCH_TARGET],
                        egui::SelectableLabel::new(is_active, label),
                    )
                    .clicked()
                {
                    *current_tool = tool;
                }
            }
        });
    }
}
