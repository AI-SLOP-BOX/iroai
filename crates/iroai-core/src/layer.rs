use crate::buffer::PixelBuffer;
use crate::color::{BlendMode, Color};
use crate::selection::SelectionMask;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub Uuid);

impl LayerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LayerId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AdjustmentKind {
    BrightnessContrast { brightness: f32, contrast: f32 },
    HueSaturation { hue_shift: f32, saturation: f32 },
    Invert,
    Grayscale,
    Threshold { cutoff: u8 },
    Posterize { levels: u8 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    Raster,
    Group { is_open: bool },
    Adjustment(AdjustmentKind),
    Text {
        text: String,
        font_size: f32,
        color: Color,
    },
    SmartObject(crate::smart_object::SmartObject),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub lock_alpha: bool,
    pub opacity: f32, // 0.0 ..= 1.0
    pub blend_mode: BlendMode,
    pub buffer: PixelBuffer,
    pub kind: LayerKind,
    pub clipping_mask: bool,
    pub mask: Option<SelectionMask>,
    pub mask_enabled: bool,
    pub style: Option<crate::style::LayerStyle>,
}

impl Layer {
    pub fn new_empty(width: u32, height: u32, name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            lock_alpha: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            buffer: PixelBuffer::new(width, height),
            kind: LayerKind::Raster,
            clipping_mask: false,
            mask: None,
            mask_enabled: true,
            style: None,
        }
    }

    pub fn new_with_color(width: u32, height: u32, name: impl Into<String>, color: Color) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            lock_alpha: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            buffer: PixelBuffer::from_color(width, height, color),
            kind: LayerKind::Raster,
            clipping_mask: false,
            mask: None,
            mask_enabled: true,
            style: None,
        }
    }

    pub fn new_from_buffer(name: impl Into<String>, buffer: PixelBuffer) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            lock_alpha: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            buffer,
            kind: LayerKind::Raster,
            clipping_mask: false,
            mask: None,
            mask_enabled: true,
            style: None,
        }
    }

    pub fn new_group(name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            lock_alpha: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            buffer: PixelBuffer::new(0, 0),
            kind: LayerKind::Group { is_open: true },
            clipping_mask: false,
            mask: None,
            mask_enabled: true,
            style: None,
        }
    }

    pub fn new_adjustment(name: impl Into<String>, kind: AdjustmentKind, width: u32, height: u32) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            lock_alpha: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            buffer: PixelBuffer::new(width, height),
            kind: LayerKind::Adjustment(kind),
            clipping_mask: false,
            mask: None,
            mask_enabled: true,
            style: None,
        }
    }

    pub fn new_smart_object(name: impl Into<String>, original_buffer: PixelBuffer, canvas_w: u32, canvas_h: u32) -> Self {
        let sm = crate::smart_object::SmartObject::new(original_buffer);
        let rendered = sm.render(canvas_w, canvas_h);
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            locked: false,
            lock_alpha: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            buffer: rendered,
            kind: LayerKind::SmartObject(sm),
            clipping_mask: false,
            mask: None,
            mask_enabled: true,
            style: None,
        }
    }

    pub fn duplicate(&self) -> Self {
        let mut copy = self.clone();
        copy.id = LayerId::new();
        copy.name = format!("{} (Copy)", self.name);
        copy
    }

    pub fn add_mask(&mut self, width: u32, height: u32) {
        if self.mask.is_none() {
            let mut mask = SelectionMask::new(width, height);
            mask.select_all();
            self.mask = Some(mask);
            self.mask_enabled = true;
        }
    }

    pub fn remove_mask(&mut self) {
        self.mask = None;
    }

    pub fn invert_mask(&mut self) {
        if let Some(ref mut mask) = self.mask {
            mask.invert();
        }
    }
}
