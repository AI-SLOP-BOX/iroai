use egui::{Color32, Margin, Rounding, Stroke, Visuals};

/// Professional Charcoal Theme & Visual Styling for Iroai
/// Eliminates the default bubble/hobbyist look of egui, delivering
/// a razor-sharp, flat, industrial Photoshop/Affinity dark studio aesthetic.
pub fn apply_pro_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // 1. Precise, compact sizing and sharp borders
    style.visuals = Visuals::dark();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.window_margin = Margin::same(6.0);
    style.spacing.button_padding = egui::vec2(6.0, 4.0);
    style.spacing.indent = 12.0;

    // 2. High-end Charcoal DCC Color Palette
    let bg_canvas = Color32::from_rgb(24, 24, 24);      // Deep viewport charcoal #181818
    let bg_panel = Color32::from_rgb(34, 34, 34);       // Studio side panel #222222
    let bg_widget = Color32::from_rgb(45, 45, 45);      // Sharp button base #2d2d2d
    let bg_widget_hover = Color32::from_rgb(60, 60, 60);// Subtle highlight #3c3c3c
    let bg_widget_active = Color32::from_rgb(38, 79, 120); // Professional Adobe blue accent
    let border_color = Color32::from_rgb(52, 52, 52);   // 1px hairline border #343434
    let text_primary = Color32::from_rgb(225, 225, 225);// High-contrast clean white-gray
    let text_secondary = Color32::from_rgb(150, 150, 150);

    // Flat hairline corners (Rounding 2.0)
    let rounding = Rounding::same(2.0);

    style.visuals.dark_mode = true;
    style.visuals.panel_fill = bg_panel;
    style.visuals.window_fill = bg_panel;
    style.visuals.extreme_bg_color = bg_canvas;

    // Non-interactive widgets
    style.visuals.widgets.noninteractive.bg_fill = bg_panel;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, border_color);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0f32, text_secondary);
    style.visuals.widgets.noninteractive.rounding = rounding;

    // Inactive (normal button / field)
    style.visuals.widgets.inactive.bg_fill = bg_widget;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0f32, border_color);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0f32, text_primary);
    style.visuals.widgets.inactive.rounding = rounding;

    // Hovered
    style.visuals.widgets.hovered.bg_fill = bg_widget_hover;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(80, 80, 80));
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0f32, Color32::WHITE);
    style.visuals.widgets.hovered.rounding = rounding;

    // Active / Pressed
    style.visuals.widgets.active.bg_fill = bg_widget_active;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(60, 120, 180));
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0f32, Color32::WHITE);
    style.visuals.widgets.active.rounding = rounding;

    // Selection
    style.visuals.selection.bg_fill = Color32::from_rgb(38, 79, 120);
    style.visuals.selection.stroke = Stroke::new(1.0f32, Color32::from_rgb(60, 130, 200));

    ctx.set_style(style);
}
