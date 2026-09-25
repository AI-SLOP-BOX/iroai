use crate::buffer::PixelBuffer;
use crate::color::Color;
use fontdue::{Font, FontSettings};
use std::sync::OnceLock;

/// High-performance TrueType & OpenType typography rasterizer with anti-aliasing
pub struct TextRenderer;

static NOTO_JP_FONT: OnceLock<Option<Font>> = OnceLock::new();

impl TextRenderer {
    /// Load system or bundled TrueType/OpenType font
    pub fn get_default_font() -> Option<&'static Font> {
        NOTO_JP_FONT
            .get_or_init(|| {
                // 1. Check custom environment variable IROAI_FONT_PATH
                if let Ok(env_path) = std::env::var("IROAI_FONT_PATH") {
                    if let Ok(bytes) = std::fs::read(&env_path) {
                        if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
                            return Some(font);
                        }
                    }
                }

                // 2. Cross-platform candidate font paths (macOS, Windows, Linux, and local assets)
                let candidate_paths = [
                    // Bundled / workspace assets
                    "../Amata/assets/fonts/NotoSansJP-Regular.ttf",
                    "assets/fonts/NotoSansJP-Regular.ttf",
                    // macOS
                    "/System/Library/Fonts/PingFang.ttc",
                    "/System/Library/Fonts/Hiragino Sans GB.ttc",
                    "/System/Library/Fonts/Supplemental/Arial.ttf",
                    "/Library/Fonts/Arial Unicode.ttf",
                    // Linux (Debian, Ubuntu, Fedora, Arch)
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                    "/usr/share/fonts/TTF/DejaVuSans.ttf",
                    // Windows
                    "C:\\Windows\\Fonts\\msgothic.ttc",
                    "C:\\Windows\\Fonts\\meiryo.ttc",
                    "C:\\Windows\\Fonts\\arial.ttf",
                ];

                for path in candidate_paths {
                    if let Ok(bytes) = std::fs::read(path) {
                        if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
                            return Some(font);
                        }
                    }
                }
                None
            })
            .as_ref()
    }

    /// Load custom font from arbitrary byte slice (OTF or TTF)
    pub fn load_font_from_bytes(bytes: &[u8]) -> Result<Font, &'static str> {
        Font::from_bytes(bytes, FontSettings::default())
    }

    /// 5x7 Basic Bitmap Font fallback for ASCII if no OTF/TTF is available
    fn get_fallback_bitmap(ch: char) -> &'static [u8; 7] {
        match ch {
            'A' => &[0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'B' => &[0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
            'C' => &[0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
            'D' => &[0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
            'E' => &[0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
            'F' => &[0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
            'G' => &[0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
            'H' => &[0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'I' => &[0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            'J' => &[0b00001, 0b00001, 0b00001, 0b00001, 0b10001, 0b10001, 0b01110],
            'K' => &[0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
            'L' => &[0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
            'M' => &[0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
            'N' => &[0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
            'O' => &[0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'P' => &[0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            'Q' => &[0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10011, 0b01111],
            'R' => &[0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
            'S' => &[0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
            'T' => &[0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            'U' => &[0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'V' => &[0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
            'W' => &[0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
            'X' => &[0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
            'Y' => &[0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
            'Z' => &[0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
            _ => &[0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        }
    }

    /// Render text with OpenType / TrueType vector rasterization, tracking (letter spacing), and sub-pixel alpha blending
    pub fn render_text(
        buffer: &mut PixelBuffer,
        text: &str,
        origin_x: i32,
        origin_y: i32,
        font_size: f32,
        color: Color,
    ) {
        Self::render_text_advanced(buffer, text, origin_x, origin_y, font_size, color, 0.0, 1.25);
    }

    /// Advanced typography renderer supporting letter tracking (px) and line height multiplier
    #[allow(clippy::too_many_arguments)]
    pub fn render_text_advanced(
        buffer: &mut PixelBuffer,
        text: &str,
        origin_x: i32,
        origin_y: i32,
        font_size: f32,
        color: Color,
        tracking_px: f32,
        line_height_mult: f32,
    ) {
        if let Some(font) = Self::get_default_font() {
            Self::render_with_font_advanced(buffer, font, text, origin_x, origin_y, font_size, color, tracking_px, line_height_mult);
        } else {
            Self::render_fallback_bitmap(buffer, text, origin_x, origin_y, font_size, color);
        }
    }

    /// High-fidelity TrueType/OpenType raster rendering using `fontdue`
    pub fn render_with_font(
        buffer: &mut PixelBuffer,
        font: &Font,
        text: &str,
        origin_x: i32,
        origin_y: i32,
        font_size: f32,
        color: Color,
    ) {
        Self::render_with_font_advanced(buffer, font, text, origin_x, origin_y, font_size, color, 0.0, 1.25);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_with_font_advanced(
        buffer: &mut PixelBuffer,
        font: &Font,
        text: &str,
        origin_x: i32,
        origin_y: i32,
        font_size: f32,
        color: Color,
        tracking_px: f32,
        line_height_mult: f32,
    ) {
        let mut cur_x = origin_x as f32;
        let mut cur_y = origin_y as f32;
        let line_height = font_size * line_height_mult;

        for ch in text.chars() {
            if ch == '\n' {
                cur_x = origin_x as f32;
                cur_y += line_height;
                continue;
            }

            let (metrics, bitmap) = font.rasterize(ch, font_size);
            let gx0 = (cur_x + metrics.xmin as f32).round() as i32;
            let gy0 = (cur_y + font_size - metrics.ymin as f32 - metrics.height as f32).round() as i32;

            for r in 0..metrics.height {
                for c in 0..metrics.width {
                    let coverage = bitmap[r * metrics.width + c];
                    if coverage > 0 {
                        let px = gx0 + c as i32;
                        let py = gy0 + r as i32;

                        if px >= 0 && px < buffer.width as i32 && py >= 0 && py < buffer.height as i32 {
                            let opacity_factor = (coverage as f32 / 255.0) * (color.a as f32 / 255.0);
                            let cur_pixel = buffer.get_pixel(px as u32, py as u32).unwrap_or(Color::TRANSPARENT);
                            let stamp_color = Color::rgba(color.r, color.g, color.b, 255);
                            let blended = crate::color::BlendMode::Normal.blend_pixel(cur_pixel, stamp_color, opacity_factor);
                            buffer.set_pixel(px as u32, py as u32, blended);
                        }
                    }
                }
            }

            cur_x += metrics.advance_width + tracking_px;
        }
    }

    fn render_fallback_bitmap(
        buffer: &mut PixelBuffer,
        text: &str,
        origin_x: i32,
        origin_y: i32,
        scale: f32,
        color: Color,
    ) {
        let s = (scale / 8.0).max(1.0);
        let mut cur_x = origin_x as f32;
        let mut cur_y = origin_y as f32;
        let char_w = 6.0 * s;
        let char_h = 8.0 * s;

        for ch in text.chars() {
            if ch == '\n' {
                cur_x = origin_x as f32;
                cur_y += char_h + 2.0 * s;
                continue;
            }
            if ch == ' ' {
                cur_x += char_w;
                continue;
            }

            let bitmap = Self::get_fallback_bitmap(ch.to_ascii_uppercase());
            for (row_idx, &row_bits) in bitmap.iter().enumerate() {
                for col_idx in 0..5 {
                    if (row_bits & (1 << (4 - col_idx))) != 0 {
                        let px0 = (cur_x + col_idx as f32 * s).round() as i32;
                        let py0 = (cur_y + row_idx as f32 * s).round() as i32;
                        let s_int = s.ceil() as i32;

                        for dy in 0..s_int {
                            for dx in 0..s_int {
                                let px = px0 + dx;
                                let py = py0 + dy;
                                if px >= 0 && px < buffer.width as i32 && py >= 0 && py < buffer.height as i32 {
                                    buffer.set_pixel(px as u32, py as u32, color);
                                }
                            }
                        }
                    }
                }
            }
            cur_x += char_w;
        }
    }

    /// Measure text dimensions in pixels
    pub fn measure_text(text: &str, font_size: f32) -> (u32, u32) {
        if let Some(font) = Self::get_default_font() {
            let mut max_w = 0.0f32;
            let mut cur_w = 0.0f32;
            let mut lines = 1;
            let line_height = font_size * 1.25;

            for ch in text.chars() {
                if ch == '\n' {
                    if cur_w > max_w {
                        max_w = cur_w;
                    }
                    cur_w = 0.0;
                    lines += 1;
                } else {
                    let metrics = font.metrics(ch, font_size);
                    cur_w += metrics.advance_width;
                }
            }
            if cur_w > max_w {
                max_w = cur_w;
            }
            (max_w.ceil() as u32, (lines as f32 * line_height).ceil() as u32)
        } else {
            let s = (font_size / 8.0).max(1.0);
            let char_w = 6.0 * s;
            let char_h = 8.0 * s;
            let mut max_w = 0.0f32;
            let mut cur_w = 0.0f32;
            let mut lines = 1;
            for ch in text.chars() {
                if ch == '\n' {
                    if cur_w > max_w { max_w = cur_w; }
                    cur_w = 0.0;
                    lines += 1;
                } else {
                    cur_w += char_w;
                }
            }
            if cur_w > max_w { max_w = cur_w; }
            (max_w.ceil() as u32, (lines as f32 * (char_h + 2.0 * s)).ceil() as u32)
        }
    }
}
