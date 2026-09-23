use crate::buffer::PixelBuffer;
use crate::color::Color;
use crate::history::{HistoryAction, HistoryManager};
use crate::layer::{Layer, LayerId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Document {
    pub id: DocumentId,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
    pub layers: Vec<Layer>,
    pub active_layer_id: Option<LayerId>,
    pub selection: crate::selection::SelectionMask,
    pub paths: Vec<crate::path::VectorPath>,
    pub history: HistoryManager,
}

impl Document {
    pub fn new(width: u32, height: u32, title: impl Into<String>) -> Self {
        let base_layer = Layer::new_with_color(width, height, "Background", Color::WHITE);
        let base_id = base_layer.id;
        Self {
            id: DocumentId::new(),
            title: title.into(),
            width,
            height,
            dpi: 72.0,
            layers: vec![base_layer],
            active_layer_id: Some(base_id),
            selection: crate::selection::SelectionMask::new(width, height),
            paths: Vec::new(),
            history: HistoryManager::new(50),
        }
    }

    pub fn new_with_layer(layer: Layer, title: impl Into<String>) -> Self {
        let w = layer.buffer.width;
        let h = layer.buffer.height;
        let id = layer.id;
        Self {
            id: DocumentId::new(),
            title: title.into(),
            width: w,
            height: h,
            dpi: 72.0,
            layers: vec![layer],
            active_layer_id: Some(id),
            selection: crate::selection::SelectionMask::new(w, h),
            paths: Vec::new(),
            history: HistoryManager::new(50),
        }
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn active_layer(&self) -> Option<&Layer> {
        let id = self.active_layer_id?;
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut Layer> {
        let id = self.active_layer_id?;
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> LayerId {
        let layer = Layer::new_empty(self.width, self.height, name);
        let id = layer.id;
        let index = self.layers.len();
        self.layers.push(layer.clone());
        self.active_layer_id = Some(id);
        self.history.push(HistoryAction::LayerAdded { index, layer: Box::new(layer) });
        id
    }

    pub fn add_adjustment_layer(&mut self, name: impl Into<String>, kind: crate::layer::AdjustmentKind) -> LayerId {
        let layer = Layer::new_adjustment(name, kind, self.width, self.height);
        let id = layer.id;
        let index = self.layers.len();
        self.layers.push(layer.clone());
        self.active_layer_id = Some(id);
        self.history.push(HistoryAction::LayerAdded { index, layer: Box::new(layer) });
        id
    }

    pub fn duplicate_layer(&mut self, id: LayerId) -> Option<LayerId> {
        if let Some(pos) = self.layers.iter().position(|l| l.id == id) {
            let dup = self.layers[pos].duplicate();
            let new_id = dup.id;
            let insert_pos = pos + 1;
            self.layers.insert(insert_pos, dup.clone());
            self.active_layer_id = Some(new_id);
            self.history.push(HistoryAction::LayerAdded { index: insert_pos, layer: Box::new(dup) });
            Some(new_id)
        } else {
            None
        }
    }

    pub fn merge_down(&mut self, id: LayerId) -> bool {
        if let Some(top_idx) = self.layers.iter().position(|l| l.id == id) {
            if top_idx == 0 {
                return false;
            }
            let top_layer = self.layers.remove(top_idx);
            let bottom_layer = &mut self.layers[top_idx - 1];

            for y in 0..self.height {
                for x in 0..self.width {
                    if let Some(top_px) = top_layer.buffer.get_pixel(x, y) {
                        if top_px.a > 0 {
                            let bot_px = bottom_layer.buffer.get_pixel(x, y).unwrap_or(Color::TRANSPARENT);
                            let blended = top_layer.blend_mode.blend_pixel(bot_px, top_px, top_layer.opacity);
                            bottom_layer.buffer.set_pixel(x, y, blended);
                        }
                    }
                }
            }
            self.active_layer_id = Some(bottom_layer.id);
            true
        } else {
            false
        }
    }

    pub fn merge_visible(&mut self) {
        if self.layers.len() <= 1 {
            return;
        }
        let comp = self.composite();
        let flattened = Layer::new_from_buffer("Merged", comp);
        let id = flattened.id;
        self.layers.clear();
        self.layers.push(flattened);
        self.active_layer_id = Some(id);
    }

    pub fn composite(&self) -> PixelBuffer {
        let mut composite_buf = PixelBuffer::new(self.width, self.height);

        for (i, layer) in self.layers.iter().enumerate() {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }

            match &layer.kind {
                crate::layer::LayerKind::Group { .. } => continue,
                crate::layer::LayerKind::Adjustment(adj) => {
                    let mut temp = composite_buf.clone();
                    match adj {
                        crate::layer::AdjustmentKind::BrightnessContrast { brightness, contrast } => {
                            crate::filter::Filters::apply_brightness_contrast(&mut temp, *brightness, *contrast);
                        }
                        crate::layer::AdjustmentKind::HueSaturation { hue_shift, saturation } => {
                            crate::filter::Filters::apply_hue_saturation(&mut temp, *hue_shift, *saturation);
                        }
                        crate::layer::AdjustmentKind::Invert => {
                            crate::filter::Filters::apply_invert(&mut temp);
                        }
                        crate::layer::AdjustmentKind::Grayscale => {
                            crate::filter::Filters::apply_grayscale(&mut temp);
                        }
                        crate::layer::AdjustmentKind::Threshold { cutoff } => {
                            crate::filter::Filters::apply_threshold(&mut temp, *cutoff);
                        }
                        crate::layer::AdjustmentKind::Posterize { levels } => {
                            crate::filter::Filters::apply_posterize(&mut temp, *levels);
                        }
                    }
                    let op = layer.opacity;
                    for idx in (0..composite_buf.data.len()).step_by(4) {
                        for c in 0..3 {
                            let orig = composite_buf.data[idx + c] as f32;
                            let adj_val = temp.data[idx + c] as f32;
                            composite_buf.data[idx + c] = (orig * (1.0 - op) + adj_val * op).round() as u8;
                        }
                    }
                    continue;
                }
                _ => {}
            }

            let opacity = layer.opacity;
            let blend = layer.blend_mode;

            let styled_buf = if let Some(style) = &layer.style {
                style.render_styled(&layer.buffer)
            } else {
                layer.buffer.clone()
            };

            let clip_target = if layer.clipping_mask && i > 0 {
                self.layers.get(i - 1)
            } else {
                None
            };

            for y in 0..self.height {
                let row_start = (y as usize) * (self.width as usize) * 4;
                for x in 0..self.width {
                    let idx = row_start + (x as usize) * 4;

                    if let Some(target) = clip_target {
                        if target.buffer.data.len() > idx + 3 && target.buffer.data[idx + 3] == 0 {
                            continue;
                        }
                    }

                    let mask_factor = if layer.mask_enabled {
                        layer.mask.as_ref().map(|m| m.get_value(x, y) as f32 / 255.0).unwrap_or(1.0)
                    } else {
                        1.0
                    };
                    if mask_factor <= 0.0 {
                        continue;
                    }

                    let effective_opacity = opacity * mask_factor;
                    let src_a = styled_buf.data[idx + 3];
                    if src_a > 0 {
                        let src_px = Color {
                            r: styled_buf.data[idx],
                            g: styled_buf.data[idx + 1],
                            b: styled_buf.data[idx + 2],
                            a: src_a,
                        };
                        let base_px = Color {
                            r: composite_buf.data[idx],
                            g: composite_buf.data[idx + 1],
                            b: composite_buf.data[idx + 2],
                            a: composite_buf.data[idx + 3],
                        };
                        let blended = blend.blend_pixel(base_px, src_px, effective_opacity);
                        composite_buf.data[idx] = blended.r;
                        composite_buf.data[idx + 1] = blended.g;
                        composite_buf.data[idx + 2] = blended.b;
                        composite_buf.data[idx + 3] = blended.a;
                    }
                }
            }
        }

        composite_buf
    }

    pub fn undo(&mut self) -> bool {
        if let Some(action) = self.history.pop_undo() {
            match action {
                HistoryAction::PixelChange { patches, .. } => {
                    for patch in patches {
                        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == patch.layer_id) {
                            layer.buffer.restore_tile(patch.tile_x, patch.tile_y, &patch.before_data);
                        }
                    }
                    true
                }
                HistoryAction::LayerAdded { index, .. } => {
                    if index < self.layers.len() {
                        self.layers.remove(index);
                        self.active_layer_id = self.layers.last().map(|l| l.id);
                        true
                    } else {
                        false
                    }
                }
                HistoryAction::LayerRemoved { index, layer } => {
                    let insert_pos = index.min(self.layers.len());
                    self.active_layer_id = Some(layer.id);
                    self.layers.insert(insert_pos, *layer);
                    true
                }
                HistoryAction::LayerReordered { from, to } => {
                    if to < self.layers.len() && from < self.layers.len() {
                        let item = self.layers.remove(to);
                        self.layers.insert(from, item);
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(action) = self.history.pop_redo() {
            match action {
                HistoryAction::PixelChange { patches, .. } => {
                    for patch in patches {
                        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == patch.layer_id) {
                            layer.buffer.restore_tile(patch.tile_x, patch.tile_y, &patch.after_data);
                        }
                    }
                    true
                }
                HistoryAction::LayerAdded { index, layer } => {
                    let insert_pos = index.min(self.layers.len());
                    self.active_layer_id = Some(layer.id);
                    self.layers.insert(insert_pos, *layer);
                    true
                }
                HistoryAction::LayerRemoved { index, .. } => {
                    if index < self.layers.len() {
                        self.layers.remove(index);
                        self.active_layer_id = self.layers.last().map(|l| l.id);
                        true
                    } else {
                        false
                    }
                }
                HistoryAction::LayerReordered { from, to } => {
                    if from < self.layers.len() && to < self.layers.len() {
                        let item = self.layers.remove(from);
                        self.layers.insert(to, item);
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }
}
