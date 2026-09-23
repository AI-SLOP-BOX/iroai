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

/// 3D Color Lookup Table (CLUT) for multidimensional non-linear color conversion (mft1/mft2/mAB/mBA)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clut3D {
    pub grid_points: usize, // e.g. 17 or 33
    pub output_channels: usize, // e.g. 3 (RGB/XYZ) or 4 (CMYK)
    pub table: Vec<f32>, // Flat array of normalized values [0.0, 1.0]
}

impl Clut3D {
    pub fn new(grid_points: usize, output_channels: usize, table: Vec<f32>) -> Self {
        Self {
            grid_points,
            output_channels,
            table,
        }
    }

    /// Trilinear interpolation over 3D cube [r, g, b] in range [0.0, 1.0]
    pub fn sample_trilinear(&self, r: f32, g: f32, b: f32) -> Vec<f32> {
        let n = self.grid_points;
        if n < 2 || self.table.is_empty() {
            return vec![r, g, b];
        }

        let max_idx = (n - 1) as f32;
        let rx = (r.clamp(0.0, 1.0) * max_idx);
        let gy = (g.clamp(0.0, 1.0) * max_idx);
        let bz = (b.clamp(0.0, 1.0) * max_idx);

        let x0 = rx.floor() as usize;
        let y0 = gy.floor() as usize;
        let z0 = bz.floor() as usize;

        let x1 = (x0 + 1).min(n - 1);
        let y1 = (y0 + 1).min(n - 1);
        let z1 = (z0 + 1).min(n - 1);

        let fx = rx - x0 as f32;
        let fy = gy - y0 as f32;
        let fz = bz - z0 as f32;

        let get_entry = |x: usize, y: usize, z: usize| -> &[f32] {
            let idx = (z * n * n + y * n + x) * self.output_channels;
            if idx + self.output_channels <= self.table.len() {
                &self.table[idx..idx + self.output_channels]
            } else {
                &[]
            }
        };

        let c000 = get_entry(x0, y0, z0);
        let c100 = get_entry(x1, y0, z0);
        let c010 = get_entry(x0, y1, z0);
        let c110 = get_entry(x1, y1, z0);
        let c001 = get_entry(x0, y0, z1);
        let c101 = get_entry(x1, y0, z1);
        let c011 = get_entry(x0, y1, z1);
        let c111 = get_entry(x1, y1, z1);

        let mut out = vec![0.0; self.output_channels];
        for ch in 0..self.output_channels {
            let v000 = c000.get(ch).copied().unwrap_or(0.0);
            let v100 = c100.get(ch).copied().unwrap_or(0.0);
            let v010 = c010.get(ch).copied().unwrap_or(0.0);
            let v110 = c110.get(ch).copied().unwrap_or(0.0);
            let v001 = c001.get(ch).copied().unwrap_or(0.0);
            let v101 = c101.get(ch).copied().unwrap_or(0.0);
            let v011 = c011.get(ch).copied().unwrap_or(0.0);
            let v111 = c111.get(ch).copied().unwrap_or(0.0);

            // Interpolate along X
            let c00 = v000 * (1.0 - fx) + v100 * fx;
            let c10 = v010 * (1.0 - fx) + v110 * fx;
            let c01 = v001 * (1.0 - fx) + v101 * fx;
            let c11 = v011 * (1.0 - fx) + v111 * fx;

            // Interpolate along Y
            let c0 = c00 * (1.0 - fy) + c10 * fy;
            let c1 = c01 * (1.0 - fy) + c11 * fy;

            // Interpolate along Z
            out[ch] = c0 * (1.0 - fz) + c1 * fz;
        }

        out
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
    pub clut_a2b: Option<Clut3D>,
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
            clut_a2b: None,
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
        let mut clut_3d = None;

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
                } else if tag_data.len() >= 52 && (&tag_data[0..4] == b"mft1" || &tag_data[0..4] == b"mft2") {
                    // Multi-function table with 3D CLUT
                    let input_ch = tag_data[8] as usize;
                    let output_ch = tag_data[9] as usize;
                    let grid_pts = tag_data[10] as usize;
                    if input_ch == 3 && grid_pts >= 2 {
                        let total_entries = grid_pts * grid_pts * grid_pts * output_ch;
                        let mut clut_table = Vec::with_capacity(total_entries);
                        let clut_start = 52;
                        if &tag_data[0..4] == b"mft1" && tag_data.len() >= clut_start + total_entries {
                            for &byte in &tag_data[clut_start..clut_start + total_entries] {
                                clut_table.push(byte as f32 / 255.0);
                            }
                        } else if &tag_data[0..4] == b"mft2" && tag_data.len() >= clut_start + total_entries * 2 {
                            for chunk in tag_data[clut_start..clut_start + total_entries * 2].chunks_exact(2) {
                                let val = u16::from_be_bytes([chunk[0], chunk[1]]) as f32 / 65535.0;
                                clut_table.push(val);
                            }
                        }
                        if clut_table.len() == total_entries {
                            clut_3d = Some(Clut3D::new(grid_pts, output_ch, clut_table));
                        }
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
            clut_a2b: clut_3d,
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
