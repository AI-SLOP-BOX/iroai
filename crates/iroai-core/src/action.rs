use crate::color::Color;
use crate::document::Document;
use crate::filter::Filters;
use crate::photo::{PhotoAdjustments, PhotoProcessor};
use crate::transform::Transform;
use serde::{Deserialize, Serialize};

/// 記録・再実行可能なアクションコマンド
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionCommand {
    Resize {
        width: u32,
        height: u32,
    },
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    FlipHorizontal,
    FlipVertical,
    Rotate90Cw,
    Rotate90Ccw,
    ApplyPhotoAdjustments(PhotoAdjustments),
    ApplyLevels {
        black_point: u8,
        gamma: f32,
        white_point: u8,
    },
    ApplyGaussianBlur {
        radius: u32,
    },
    ApplySharpen {
        strength: f32,
    },
    ApplyInvert,
    ApplyGrayscale,
    ApplyWatermark {
        text: String,
        opacity: f32,
        position_bottom_right: bool,
    },
    MergeVisible,
}

impl ActionCommand {
    pub fn execute(&self, doc: &mut Document) -> Result<(), String> {
        match self {
            ActionCommand::Resize { width, height } => {
                for layer in &mut doc.layers {
                    layer.buffer = Transform::resize_bilinear(&layer.buffer, *width, *height);
                }
                doc.width = *width;
                doc.height = *height;
                doc.selection = crate::selection::SelectionMask::new(*width, *height);
            }
            ActionCommand::Crop { x, y, width, height } => {
                for layer in &mut doc.layers {
                    layer.buffer = Transform::crop(&layer.buffer, *x, *y, *width, *height);
                }
                doc.width = *width;
                doc.height = *height;
                doc.selection = crate::selection::SelectionMask::new(*width, *height);
            }
            ActionCommand::FlipHorizontal => {
                if let Some(layer) = doc.active_layer_mut() {
                    Transform::flip_horizontal(&mut layer.buffer);
                }
            }
            ActionCommand::FlipVertical => {
                if let Some(layer) = doc.active_layer_mut() {
                    Transform::flip_vertical(&mut layer.buffer);
                }
            }
            ActionCommand::Rotate90Cw => {
                for layer in &mut doc.layers {
                    layer.buffer = Transform::rotate_90_cw(&layer.buffer);
                }
                let old_w = doc.width;
                doc.width = doc.height;
                doc.height = old_w;
            }
            ActionCommand::Rotate90Ccw => {
                for layer in &mut doc.layers {
                    layer.buffer = Transform::rotate_90_ccw(&layer.buffer);
                }
                let old_w = doc.width;
                doc.width = doc.height;
                doc.height = old_w;
            }
            ActionCommand::ApplyPhotoAdjustments(adj) => {
                if let Some(layer) = doc.active_layer_mut() {
                    PhotoProcessor::apply_photo_adjustments(&mut layer.buffer, adj);
                }
            }
            ActionCommand::ApplyLevels { black_point, gamma, white_point } => {
                if let Some(layer) = doc.active_layer_mut() {
                    PhotoProcessor::apply_levels(&mut layer.buffer, *black_point, *gamma, *white_point);
                }
            }
            ActionCommand::ApplyGaussianBlur { radius } => {
                if let Some(layer) = doc.active_layer_mut() {
                    Filters::apply_gaussian_blur(&mut layer.buffer, *radius);
                }
            }
            ActionCommand::ApplySharpen { strength } => {
                if let Some(layer) = doc.active_layer_mut() {
                    Filters::apply_sharpen(&mut layer.buffer, *strength);
                }
            }
            ActionCommand::ApplyInvert => {
                if let Some(layer) = doc.active_layer_mut() {
                    Filters::apply_invert(&mut layer.buffer);
                }
            }
            ActionCommand::ApplyGrayscale => {
                if let Some(layer) = doc.active_layer_mut() {
                    Filters::apply_grayscale(&mut layer.buffer);
                }
            }
            ActionCommand::ApplyWatermark { opacity, .. } => {
                // 透かしスタンプ（右下または指定位置）
                if let Some(layer) = doc.active_layer_mut() {
                    let w = layer.buffer.width;
                    let h = layer.buffer.height;
                    let mark_color = Color::rgba(255, 255, 255, (*opacity * 255.0).clamp(0.0, 255.0) as u8);
                    let start_x = w.saturating_sub(60);
                    let start_y = h.saturating_sub(30);
                    Transform::draw_rect_outline(&mut layer.buffer, start_x as i32, start_y as i32, 50, 20, mark_color);
                }
            }
            ActionCommand::MergeVisible => {
                doc.merge_visible();
            }
        }
        Ok(())
    }
}

/// 一連のアクション操作セット（Photoshopのアクション相当）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionSequence {
    pub name: String,
    pub description: String,
    pub commands: Vec<ActionCommand>,
}

impl ActionSequence {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            commands: Vec::new(),
        }
    }

    pub fn add_command(&mut self, cmd: ActionCommand) {
        self.commands.push(cmd);
    }

    pub fn execute_all(&self, doc: &mut Document) -> Result<(), String> {
        for cmd in &self.commands {
            cmd.execute(doc)?;
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
