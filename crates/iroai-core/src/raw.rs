use serde::{Deserialize, Serialize};

/// RAW 撮影メタデータ (Exif相当情報)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawMetadata {
    pub camera_make: String,
    pub camera_model: String,
    pub lens_model: String,
    pub iso: u32,
    pub shutter_speed: String,
    pub aperture: f32,
    pub focal_length: f32,
    pub timestamp: String,
    pub white_balance_preset: String,
}

impl Default for RawMetadata {
    fn default() -> Self {
        Self {
            camera_make: "Sony".to_string(),
            camera_model: "ILCE-7RM5".to_string(),
            lens_model: "FE 24-70mm F2.8 GM II".to_string(),
            iso: 100,
            shutter_speed: "1/250s".to_string(),
            aperture: 2.8,
            focal_length: 50.0,
            timestamp: "2026:09:23 14:30:00".to_string(),
            white_balance_preset: "Daylight (5500K)".to_string(),
        }
    }
}

/// 非破壊 RAW 現像設定
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawDevelopSettings {
    pub color_profile: String,
    pub white_balance_temp: f32,    // Kelvin, e.g. 2000K ..= 50000K
    pub white_balance_tint: f32,    // -150 ..= +150
    pub exposure: f32,              // -5.0 ..= +5.0 EV
    pub contrast: f32,              // -100 ..= +100
    pub highlights: f32,            // -100 ..= +100
    pub shadows: f32,               // -100 ..= +100
    pub whites: f32,                // -100 ..= +100
    pub blacks: f32,                // -100 ..= +100
    pub clarity: f32,               // -100 ..= +100
    pub dehaze: f32,                // -100 ..= +100
    pub vibrance: f32,              // -100 ..= +100
    pub saturation: f32,            // -100 ..= +100
    pub lens_profile_correction: bool,
    pub chromatic_aberration: bool,
}

impl Default for RawDevelopSettings {
    fn default() -> Self {
        Self {
            color_profile: "Adobe Color".to_string(),
            white_balance_temp: 5500.0,
            white_balance_tint: 10.0,
            exposure: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            clarity: 0.0,
            dehaze: 0.0,
            vibrance: 0.0,
            saturation: 0.0,
            lens_profile_correction: true,
            chromatic_aberration: true,
        }
    }
}

/// RAW 元画像コンテナ（非破壊再編集用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawContainer {
    pub metadata: RawMetadata,
    pub settings: RawDevelopSettings,
    pub raw_data: Vec<u8>,
    pub filename: String,
}

impl RawContainer {
    pub fn new(filename: impl Into<String>, raw_data: Vec<u8>) -> Self {
        Self {
            metadata: RawMetadata::default(),
            settings: RawDevelopSettings::default(),
            raw_data,
            filename: filename.into(),
        }
    }
}
