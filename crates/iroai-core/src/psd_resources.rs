use serde::{Deserialize, Serialize};

/// Photoshop 8BIM Image Resource Block (IRB) タグ定義
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageResourceId {
    ResolutionInfo,
    AlphaNames,
    GridGuides,
    IccProfile,
    GlobalAngle,
    ColorHalftoneInfo,
    PrintFlags,
    Unknown(u16),
}

impl From<u16> for ImageResourceId {
    fn from(val: u16) -> Self {
        match val {
            0x03ED => ImageResourceId::ResolutionInfo,
            0x03EE => ImageResourceId::AlphaNames,
            0x0408 => ImageResourceId::GridGuides,
            0x040F => ImageResourceId::IccProfile,
            0x043D => ImageResourceId::GlobalAngle,
            0x03F3 => ImageResourceId::ColorHalftoneInfo,
            0x03F1 => ImageResourceId::PrintFlags,
            other => ImageResourceId::Unknown(other),
        }
    }
}

/// 解像度メタデータ (8BIM 0x03ED)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolutionInfo {
    pub h_res: f32, // DPI
    pub v_res: f32, // DPI
    pub is_inches: bool,
}

impl Default for ResolutionInfo {
    fn default() -> Self {
        Self {
            h_res: 72.0,
            v_res: 72.0,
            is_inches: true,
        }
    }
}

/// 解析済み 8BIM イメージリソース
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedPsdResources {
    pub resolution: Option<ResolutionInfo>,
    pub icc_profile_raw: Option<Vec<u8>>,
    pub global_angle: Option<i32>,
    pub alpha_channel_names: Vec<String>,
}

pub struct PsdResourceParser;

impl PsdResourceParser {
    /// 8BIM イメージリソースセクションの網羅的パース
    pub fn parse_resources(mut data: &[u8]) -> ParsedPsdResources {
        let mut resources = ParsedPsdResources::default();

        while data.len() >= 4 {
            if &data[0..4] != b"8BIM" {
                break;
            }
            data = &data[4..];

            if data.len() < 2 {
                break;
            }
            let res_id_raw = u16::from_be_bytes([data[0], data[1]]);
            let res_id = ImageResourceId::from(res_id_raw);
            data = &data[2..];

            // リソース名 (Pascal string, 偶数パディング)
            if data.is_empty() {
                break;
            }
            let name_len = data[0] as usize;
            let name_block_len = ((name_len + 1 + 1) & !1).max(2);
            if data.len() < name_block_len + 4 {
                break;
            }
            data = &data[name_block_len..];

            // データ長
            let data_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
            let padded_data_len = (data_len + 1) & !1;
            data = &data[4..];

            if data.len() < data_len {
                break;
            }
            let res_payload = &data[..data_len];

            // リソースIDごとの解析
            match res_id {
                ImageResourceId::ResolutionInfo => {
                    if res_payload.len() >= 16 {
                        // 32-bit fixed-point (16.16)
                        let h_int = i16::from_be_bytes([res_payload[0], res_payload[1]]) as f32;
                        let h_frac = u16::from_be_bytes([res_payload[2], res_payload[3]]) as f32 / 65536.0;
                        let v_int = i16::from_be_bytes([res_payload[8], res_payload[9]]) as f32;
                        let v_frac = u16::from_be_bytes([res_payload[10], res_payload[11]]) as f32 / 65536.0;
                        let unit = u16::from_be_bytes([res_payload[4], res_payload[5]]);
                        resources.resolution = Some(ResolutionInfo {
                            h_res: h_int + h_frac,
                            v_res: v_int + v_frac,
                            is_inches: unit == 1 || unit == 0,
                        });
                    }
                }
                ImageResourceId::IccProfile => {
                    resources.icc_profile_raw = Some(res_payload.to_vec());
                }
                ImageResourceId::GlobalAngle => {
                    if res_payload.len() >= 4 {
                        let angle = i32::from_be_bytes([res_payload[0], res_payload[1], res_payload[2], res_payload[3]]);
                        resources.global_angle = Some(angle);
                    }
                }
                _ => {}
            }

            if data.len() >= padded_data_len {
                data = &data[padded_data_len..];
            } else {
                break;
            }
        }

        resources
    }
}
