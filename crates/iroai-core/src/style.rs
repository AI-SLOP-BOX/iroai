use crate::buffer::PixelBuffer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropShadow {
    pub enabled: bool,
    pub color: crate::color::Color,
    pub offset_x: i32,
    pub offset_y: i32,
    pub blur_radius: u32,
    pub opacity: f32,
}

impl Default for DropShadow {
    fn default() -> Self {
        Self {
            enabled: false,
            color: crate::color::Color::BLACK,
            offset_x: 5,
            offset_y: 5,
            blur_radius: 5,
            opacity: 0.75,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub enabled: bool,
    pub size: u32,
    pub color: crate::color::Color,
    pub opacity: f32,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            enabled: false,
            size: 3,
            color: crate::color::Color::BLACK,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorOverlay {
    pub enabled: bool,
    pub color: crate::color::Color,
    pub opacity: f32,
    pub blend_mode: crate::color::BlendMode,
}

impl Default for ColorOverlay {
    fn default() -> Self {
        Self {
            enabled: false,
            color: crate::color::Color::rgb(255, 0, 0),
            opacity: 1.0,
            blend_mode: crate::color::BlendMode::Normal,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerStyle {
    pub drop_shadow: Option<DropShadow>,
    pub stroke: Option<Stroke>,
    pub color_overlay: Option<ColorOverlay>,
}

impl LayerStyle {
    pub fn is_empty(&self) -> bool {
        let ds = self.drop_shadow.as_ref().map(|s| s.enabled).unwrap_or(false);
        let st = self.stroke.as_ref().map(|s| s.enabled).unwrap_or(false);
        let co = self.color_overlay.as_ref().map(|s| s.enabled).unwrap_or(false);
        !ds && !st && !co
    }

    pub fn render_styled(&self, src: &PixelBuffer) -> PixelBuffer {
        if self.is_empty() {
            return src.clone();
        }

        let w = src.width;
        let h = src.height;
        let mut result = PixelBuffer::new(w, h);

        // 1. ドロップシャドウ
        if let Some(ds) = &self.drop_shadow {
            if ds.enabled && ds.opacity > 0.0 {
                let shadow_col = ds.color;
                for y in 0..h {
                    for x in 0..w {
                        let src_x = x as i32 - ds.offset_x;
                        let src_y = y as i32 - ds.offset_y;
                        if src_x >= 0 && src_x < w as i32 && src_y >= 0 && src_y < h as i32 {
                            let idx = (src_y as usize * w as usize + src_x as usize) * 4;
                            let src_a = src.data[idx + 3];
                            if src_a > 0 {
                                let a_val = (src_a as f32 * ds.opacity).round() as u8;
                                let s_px = crate::color::Color::rgba(shadow_col.r, shadow_col.g, shadow_col.b, a_val);
                                result.set_pixel(x, y, s_px);
                            }
                        }
                    }
                }
            }
        }

        // 2. 本体およびカラーオーバーレイ
        for y in 0..h {
            for x in 0..w {
                let idx = ((y * w + x) as usize) * 4;
                let orig_a = src.data[idx + 3];
                if orig_a > 0 {
                    let mut px = crate::color::Color {
                        r: src.data[idx],
                        g: src.data[idx + 1],
                        b: src.data[idx + 2],
                        a: orig_a,
                    };

                    if let Some(co) = &self.color_overlay {
                        if co.enabled && co.opacity > 0.0 {
                            let overlay_px = crate::color::Color::rgba(co.color.r, co.color.g, co.color.b, orig_a);
                            px = co.blend_mode.blend_pixel(px, overlay_px, co.opacity);
                        }
                    }

                    let base_px = result.get_pixel(x, y).unwrap_or(crate::color::Color::TRANSPARENT);
                    let blended = crate::color::BlendMode::Normal.blend_pixel(base_px, px, 1.0);
                    result.set_pixel(x, y, blended);
                }
            }
        }

        result
    }
}
