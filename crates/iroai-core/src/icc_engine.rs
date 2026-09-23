use crate::color::Color;
use serde::{Deserialize, Serialize};

/// ICC プロファイル内の CIE XYZ トリプレット (s15Fixed16Number)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct IccXyz {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl IccXyz {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// 解析済み ICC v2/v4 プロファイル
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedIccProfile {
    pub profile_class: String,
    pub color_space: String,
    pub connection_space: String,
    pub red_colorant: IccXyz,
    pub green_colorant: IccXyz,
    pub blue_colorant: IccXyz,
    pub media_white_point: IccXyz,
    pub gamma: f32,
}

impl Default for ParsedIccProfile {
    fn default() -> Self {
        // Standard sRGB D65 Matrix Profile
        Self {
            profile_class: "mntr".to_string(),
            color_space: "RGB ".to_string(),
            connection_space: "XYZ ".to_string(),
            red_colorant: IccXyz::new(0.4360657, 0.2224884, 0.0139160),
            green_colorant: IccXyz::new(0.3851471, 0.7168732, 0.0970764),
            blue_colorant: IccXyz::new(0.1430664, 0.0606079, 0.7140961),
            media_white_point: IccXyz::new(0.9504547, 1.0000000, 1.0890503), // D65
            gamma: 2.2,
        }
    }
}

/// 本格カラーマネジメントエンジン (CMM: Color Management Module)
pub struct ColorManagementEngine;

impl ColorManagementEngine {
    /// ICC 生バイナリ（128バイトヘッダ＋タグテーブル）の解析
    pub fn parse_icc(data: &[u8]) -> Result<ParsedIccProfile, String> {
        if data.len() < 128 {
            return Err("ICC profile too small (less than 128 bytes header)".into());
        }

        // 'acsp' シグネチャ検査 (offset 36..40)
        if &data[36..40] != b"acsp" {
            return Err("Invalid ICC profile: missing 'acsp' signature".into());
        }

        let profile_class = String::from_utf8_lossy(&data[12..16]).to_string();
        let color_space = String::from_utf8_lossy(&data[16..20]).to_string();
        let connection_space = String::from_utf8_lossy(&data[20..24]).to_string();

        let parse_s15_fixed_16 = |slice: &[u8]| -> f32 {
            if slice.len() < 4 {
                return 0.0;
            }
            let int_part = i16::from_be_bytes([slice[0], slice[1]]) as f32;
            let frac_part = u16::from_be_bytes([slice[2], slice[3]]) as f32 / 65536.0;
            int_part + frac_part
        };

        let illuminant_x = parse_s15_fixed_16(&data[68..72]);
        let illuminant_y = parse_s15_fixed_16(&data[72..76]);
        let illuminant_z = parse_s15_fixed_16(&data[76..80]);

        // タグテーブルの走査 (offset 128..)
        let tag_count = if data.len() >= 132 {
            u32::from_be_bytes([data[128], data[129], data[130], data[131]]) as usize
        } else {
            0
        };

        let mut red_col = IccXyz::new(0.436, 0.222, 0.014);
        let mut green_col = IccXyz::new(0.385, 0.717, 0.097);
        let mut blue_col = IccXyz::new(0.143, 0.061, 0.714);
        let mut gamma = 2.2;

        let mut offset = 132;
        for _ in 0..tag_count {
            if offset + 12 > data.len() {
                break;
            }
            let tag_sig = &data[offset..offset + 4];
            let tag_offset = u32::from_be_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]) as usize;
            let tag_size = u32::from_be_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]]) as usize;
            offset += 12;

            if tag_offset + tag_size <= data.len() {
                let tag_data = &data[tag_offset..tag_offset + tag_size];
                if tag_data.len() >= 20 && &tag_data[0..4] == b"XYZ " {
                    let xyz = IccXyz::new(
                        parse_s15_fixed_16(&tag_data[8..12]),
                        parse_s15_fixed_16(&tag_data[12..16]),
                        parse_s15_fixed_16(&tag_data[16..20]),
                    );
                    match tag_sig {
                        b"rXYZ" => red_col = xyz,
                        b"gXYZ" => green_col = xyz,
                        b"bXYZ" => blue_col = xyz,
                        _ => {}
                    }
                } else if tag_data.len() >= 12 && &tag_data[0..4] == b"curv" {
                    let count = u32::from_be_bytes([tag_data[8], tag_data[9], tag_data[10], tag_data[11]]);
                    if count == 1 && tag_data.len() >= 14 {
                        let g_u8_8 = u16::from_be_bytes([tag_data[12], tag_data[13]]) as f32 / 256.0;
                        gamma = g_u8_8;
                    }
                }
            }
        }

        Ok(ParsedIccProfile {
            profile_class,
            color_space,
            connection_space,
            red_colorant: red_col,
            green_colorant: green_col,
            blue_colorant: blue_col,
            media_white_point: IccXyz::new(illuminant_x, illuminant_y, illuminant_z),
            gamma,
        })
    }

    /// リニアRGBから CIE XYZ (PCS: Profile Connection Space) への変換
    pub fn linear_rgb_to_xyz(r: f32, g: f32, b: f32, profile: &ParsedIccProfile) -> (f32, f32, f32) {
        let x = r * profile.red_colorant.x + g * profile.green_colorant.x + b * profile.blue_colorant.x;
        let y = r * profile.red_colorant.y + g * profile.green_colorant.y + b * profile.blue_colorant.y;
        let z = r * profile.red_colorant.z + g * profile.green_colorant.z + b * profile.blue_colorant.z;
        (x, y, z)
    }

    /// Color (sRGB/linear) から PCS XYZ への変換
    pub fn rgb_to_pcs_xyz(color: Color, profile: &ParsedIccProfile) -> IccXyz {
        let (lr, lg, lb, _) = color.to_linear_f32();
        let (x, y, z) = Self::linear_rgb_to_xyz(lr, lg, lb, profile);
        IccXyz::new(x, y, z)
    }

    /// PCS XYZ から Color への変換 (マトリクス逆変換)
    pub fn pcs_xyz_to_rgb(xyz: IccXyz, profile: &ParsedIccProfile) -> Color {
        let m00 = profile.red_colorant.x;
        let m01 = profile.green_colorant.x;
        let m02 = profile.blue_colorant.x;
        let m10 = profile.red_colorant.y;
        let m11 = profile.green_colorant.y;
        let m12 = profile.blue_colorant.y;
        let m20 = profile.red_colorant.z;
        let m21 = profile.green_colorant.z;
        let m22 = profile.blue_colorant.z;

        let det = m00 * (m11 * m22 - m12 * m21) - m01 * (m10 * m22 - m12 * m20) + m02 * (m10 * m21 - m11 * m20);
        if det.abs() <= 1e-6 {
            return Color::BLACK;
        }

        let inv_det = 1.0 / det;
        let r_dst = ((m11 * m22 - m12 * m21) * xyz.x + (m02 * m21 - m01 * m22) * xyz.y + (m01 * m12 - m02 * m11) * xyz.z) * inv_det;
        let g_dst = ((m12 * m20 - m10 * m22) * xyz.x + (m00 * m22 - m02 * m20) * xyz.y + (m02 * m10 - m00 * m12) * xyz.z) * inv_det;
        let b_dst = ((m10 * m21 - m11 * m20) * xyz.x + (m01 * m20 - m00 * m21) * xyz.y + (m00 * m11 - m01 * m10) * xyz.z) * inv_det;

        Color::from_linear_f32(r_dst, g_dst, b_dst, 1.0)
    }

    /// 入力プロファイルから出力プロファイル（モニター表示・プルーフ）への厳密色変換
    pub fn transform_color(
        src: Color,
        src_profile: &ParsedIccProfile,
        dst_profile: &ParsedIccProfile,
    ) -> Color {
        let xyz = Self::rgb_to_pcs_xyz(src, src_profile);
        let mut out = Self::pcs_xyz_to_rgb(xyz, dst_profile);
        out.a = src.a;
        out
    }
}
