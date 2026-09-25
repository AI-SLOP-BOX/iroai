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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OuterGlow {
    pub enabled: bool,
    pub color: crate::color::Color,
    pub radius: u32,
    pub opacity: f32,
}

impl Default for OuterGlow {
    fn default() -> Self {
        Self {
            enabled: false,
            color: crate::color::Color::rgb(255, 230, 100),
            radius: 8,
            opacity: 0.8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BevelAndEmboss {
    pub enabled: bool,
    pub depth: f32, // 1.0 .. 10.0
    pub size: u32,  // 1 .. 20
    pub angle_deg: f32, // light angle e.g. 120.0
    pub highlight_opacity: f32,
    pub shadow_opacity: f32,
}

impl Default for BevelAndEmboss {
    fn default() -> Self {
        Self {
            enabled: false,
            depth: 3.0,
            size: 4,
            angle_deg: 120.0,
            highlight_opacity: 0.75,
            shadow_opacity: 0.75,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerStyle {
    pub drop_shadow: Option<DropShadow>,
    pub outer_glow: Option<OuterGlow>,
    pub stroke: Option<Stroke>,
    pub color_overlay: Option<ColorOverlay>,
    pub bevel_emboss: Option<BevelAndEmboss>,
}

impl LayerStyle {
    pub fn is_empty(&self) -> bool {
        let ds = self.drop_shadow.as_ref().map(|s| s.enabled).unwrap_or(false);
        let og = self.outer_glow.as_ref().map(|s| s.enabled).unwrap_or(false);
        let st = self.stroke.as_ref().map(|s| s.enabled).unwrap_or(false);
        let co = self.color_overlay.as_ref().map(|s| s.enabled).unwrap_or(false);
        let be = self.bevel_emboss.as_ref().map(|s| s.enabled).unwrap_or(false);
        !ds && !og && !st && !co && !be
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

        // 2. 光彩(外側) - Outer Glow (Radial Dilate / Distance field blur)
        if let Some(og) = &self.outer_glow {
            if og.enabled && og.opacity > 0.0 && og.radius > 0 {
                let rad = og.radius as i32;
                let rad_f = og.radius as f32;
                let glow_col = og.color;
                for y in 0..h {
                    for x in 0..w {
                        let base_idx = ((y * w + x) as usize) * 4;
                        if src.data[base_idx + 3] == 255 {
                            continue;
                        }
                        // Search nearby pixels for alpha
                        let mut max_alpha_weight = 0.0f32;
                        for dy in -rad..=rad {
                            let ny = y as i32 + dy;
                            if ny < 0 || ny >= h as i32 { continue; }
                            for dx in -rad..=rad {
                                let nx = x as i32 + dx;
                                if nx < 0 || nx >= w as i32 { continue; }
                                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                                if dist <= rad_f {
                                    let n_idx = (ny as usize * w as usize + nx as usize) * 4;
                                    let a = src.data[n_idx + 3] as f32 / 255.0;
                                    let falloff = 1.0 - (dist / rad_f);
                                    let w_val = a * falloff;
                                    if w_val > max_alpha_weight {
                                        max_alpha_weight = w_val;
                                    }
                                }
                            }
                        }
                        if max_alpha_weight > 0.0 {
                            let glow_a = (max_alpha_weight * og.opacity * 255.0).clamp(0.0, 255.0) as u8;
                            let glow_px = crate::color::Color::rgba(glow_col.r, glow_col.g, glow_col.b, glow_a);
                            let cur_px = result.get_pixel(x, y).unwrap_or(crate::color::Color::TRANSPARENT);
                            let blended = crate::color::BlendMode::Screen.blend_pixel(cur_px, glow_px, og.opacity);
                            result.set_pixel(x, y, blended);
                        }
                    }
                }
            }
        }

        // 3. 本体、カラーオーバーレイ、ベベル＆エンボス (Bevel & Emboss)
        let bevel_rad = self.bevel_emboss.as_ref().filter(|b| b.enabled).map(|b| b.size.max(1) as i32).unwrap_or(0);
        let (light_dx, light_dy) = if let Some(be) = &self.bevel_emboss {
            let rad = be.angle_deg.to_radians();
            (rad.cos(), -rad.sin())
        } else {
            (0.0, 0.0)
        };

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

                    // ベベル＆エンボス（陰影計算・ソベル傾斜法）
                    if let Some(be) = &self.bevel_emboss {
                        if be.enabled && bevel_rad > 0 {
                            let left_a = if x as i32 >= bevel_rad { src.data[idx - bevel_rad as usize * 4 + 3] as f32 } else { 0.0 };
                            let right_a = if (x as i32 + bevel_rad) < w as i32 { src.data[idx + bevel_rad as usize * 4 + 3] as f32 } else { 0.0 };
                            let up_a = if y as i32 >= bevel_rad { src.data[idx - (bevel_rad as usize * w as usize * 4) + 3] as f32 } else { 0.0 };
                            let down_a = if (y as i32 + bevel_rad) < h as i32 { src.data[idx + (bevel_rad as usize * w as usize * 4) + 3] as f32 } else { 0.0 };

                            let nx = (left_a - right_a) / 255.0 * be.depth;
                            let ny = (up_a - down_a) / 255.0 * be.depth;

                            let light = nx * light_dx + ny * light_dy;
                            if light > 0.0 {
                                // ハイライト (加算)
                                let factor = (light * be.highlight_opacity).clamp(0.0, 1.0);
                                px.r = (px.r as f32 + (255.0 - px.r as f32) * factor).clamp(0.0, 255.0) as u8;
                                px.g = (px.g as f32 + (255.0 - px.g as f32) * factor).clamp(0.0, 255.0) as u8;
                                px.b = (px.b as f32 + (255.0 - px.b as f32) * factor).clamp(0.0, 255.0) as u8;
                            } else if light < 0.0 {
                                // シャドウ (乗算)
                                let factor = 1.0 - (-light * be.shadow_opacity).clamp(0.0, 1.0);
                                px.r = (px.r as f32 * factor).clamp(0.0, 255.0) as u8;
                                px.g = (px.g as f32 * factor).clamp(0.0, 255.0) as u8;
                                px.b = (px.b as f32 * factor).clamp(0.0, 255.0) as u8;
                            }
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
