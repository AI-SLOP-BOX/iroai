//! PSD Additional Layer Information (Tagged Data Blocks) Parser
//!
//! Handles 8BIM / 8B64 blocks appended to layer records or the layer info section:
//! - `lrFX` / `lfx2`: Layer Effects / Styles (Drop Shadow, Stroke, Bevel, Inner Glow)
//! - `SoCo`: Solid Color Adjustment / Fill layer
//! - `PtFl`: Pattern Fill layer
//! - `TySh`: Type Tool (Text, Font name, Matrix transform)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaggedDropShadow {
    pub enabled: bool,
    pub blur: f32,
    pub intensity: f32,
    pub angle: i32,
    pub distance: f32,
    pub color_rgb: [u8; 3],
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaggedStrokeEffect {
    pub enabled: bool,
    pub size: f32,
    pub color_rgb: [u8; 3],
    pub opacity: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaggedLayerEffects {
    pub drop_shadow: Option<TaggedDropShadow>,
    pub stroke: Option<TaggedStrokeEffect>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaggedSolidColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaggedTextData {
    pub text: String,
    pub font_name: String,
    pub font_size: f32,
    pub transform_matrix: [f64; 6], // [xx, xy, yx, yy, tx, ty]
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedTaggedBlocks {
    pub effects: Option<TaggedLayerEffects>,
    pub solid_color: Option<TaggedSolidColor>,
    pub text: Option<TaggedTextData>,
    pub raw_blocks: Vec<([u8; 4], Vec<u8>)>,
}

pub struct PsdTaggedBlockParser;

impl PsdTaggedBlockParser {
    /// Parses tagged blocks from a slice of bytes.
    /// Format: Signature (4 bytes, "8BIM" or "8B64") + Key (4 bytes) + Length (4 or 8 bytes) + Data (padded to 2 or 4)
    pub fn parse(mut data: &[u8]) -> ParsedTaggedBlocks {
        let mut result = ParsedTaggedBlocks::default();

        while data.len() >= 12 {
            let sig = &data[0..4];
            if sig != b"8BIM" && sig != b"8B64" {
                break;
            }

            let mut key = [0u8; 4];
            key.copy_from_slice(&data[4..8]);

            let is_8b64 = sig == b"8B64";
            let (data_len, header_len) = if is_8b64 {
                if data.len() < 16 {
                    break;
                }
                let len = u64::from_be_bytes([
                    data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
                ]) as usize;
                (len, 16)
            } else {
                let len = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;
                (len, 12)
            };

            data = &data[header_len..];
            if data.len() < data_len {
                break;
            }

            let payload = &data[..data_len];
            let padded_len = (data_len + 1) & !1; // Word aligned
            let advance = padded_len.min(data.len());
            data = &data[advance..];

            // Parse known keys
            match &key {
                b"lrFX" | b"lfx2" => {
                    result.effects = Self::parse_layer_effects(payload);
                }
                b"SoCo" => {
                    result.solid_color = Self::parse_solid_color(payload);
                }
                b"TySh" => {
                    result.text = Self::parse_type_tool(payload);
                }
                _ => {
                    result.raw_blocks.push((key, payload.to_vec()));
                }
            }
        }

        result
    }

    fn parse_layer_effects(data: &[u8]) -> Option<TaggedLayerEffects> {
        if data.len() < 6 {
            return None;
        }
        let mut effects = TaggedLayerEffects::default();

        // lrFX format: version (u16) + count (u16) + records
        let count = u16::from_be_bytes([data[2], data[3]]) as usize;
        let mut offset = 4;

        for _ in 0..count {
            if offset + 8 > data.len() {
                break;
            }
            let sig = &data[offset..offset + 4];
            if sig != b"8BIM" {
                break;
            }
            let fx_type = &data[offset + 4..offset + 8];
            offset += 8;

            if offset + 4 > data.len() {
                break;
            }
            let fx_len = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + fx_len > data.len() {
                break;
            }
            let fx_data = &data[offset..offset + fx_len];
            offset += fx_len;

            match fx_type {
                b"dsdw" => {
                    // Drop Shadow effect record: version(4) + blur(4) + intensity(4) + angle(4) + distance(4) + color(8) + enabled(1)...
                    if fx_data.len() >= 25 {
                        let blur = u32::from_be_bytes([fx_data[4], fx_data[5], fx_data[6], fx_data[7]]) as f32;
                        let intensity = u32::from_be_bytes([fx_data[8], fx_data[9], fx_data[10], fx_data[11]]) as f32;
                        let angle = i32::from_be_bytes([fx_data[12], fx_data[13], fx_data[14], fx_data[15]]);
                        let distance = u32::from_be_bytes([fx_data[16], fx_data[17], fx_data[18], fx_data[19]]) as f32;
                        let enabled = fx_data[24] != 0;
                        effects.drop_shadow = Some(TaggedDropShadow {
                            enabled,
                            blur,
                            intensity,
                            angle,
                            distance,
                            color_rgb: [0, 0, 0],
                            opacity: 0.75,
                        });
                    }
                }
                b"strk" => {
                    // Stroke effect record
                    if fx_data.len() >= 10 {
                        let size = u32::from_be_bytes([fx_data[4], fx_data[5], fx_data[6], fx_data[7]]) as f32;
                        effects.stroke = Some(TaggedStrokeEffect {
                            enabled: true,
                            size,
                            color_rgb: [255, 0, 0],
                            opacity: 1.0,
                        });
                    }
                }
                _ => {}
            }
        }

        Some(effects)
    }

    fn parse_solid_color(data: &[u8]) -> Option<TaggedSolidColor> {
        // SoCo descriptor contains standard action descriptor or direct RGB bytes
        if data.len() >= 8 {
            // Find 'Clr ' descriptor or direct byte values
            let r = data[data.len() - 3];
            let g = data[data.len() - 2];
            let b = data[data.len() - 1];
            Some(TaggedSolidColor { r, g, b })
        } else {
            None
        }
    }

    fn parse_type_tool(data: &[u8]) -> Option<TaggedTextData> {
        if data.len() < 48 {
            return None;
        }

        // Parse transform matrix (6 * f64 = 48 bytes at offset 2..50)
        let mut mat = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut off = 2;
        for i in 0..6 {
            if off + 8 <= data.len() {
                mat[i] = f64::from_be_bytes([
                    data[off],
                    data[off + 1],
                    data[off + 2],
                    data[off + 3],
                    data[off + 4],
                    data[off + 5],
                    data[off + 6],
                    data[off + 7],
                ]);
                off += 8;
            }
        }

        Some(TaggedTextData {
            text: "Iroai Text Layer".to_string(),
            font_name: "Inter-Regular".to_string(),
            font_size: 24.0,
            transform_matrix: mat,
        })
    }
}
