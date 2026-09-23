use crate::buffer::PixelBuffer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistogramData {
    pub red: [u32; 256],
    pub green: [u32; 256],
    pub blue: [u32; 256],
    pub luminance: [u32; 256],
    pub max_count: u32,
}

pub struct ColorChannelManager;

impl ColorChannelManager {
    pub fn calculate_histogram(buffer: &PixelBuffer) -> HistogramData {
        let mut red = [0u32; 256];
        let mut green = [0u32; 256];
        let mut blue = [0u32; 256];
        let mut luminance = [0u32; 256];

        for chunk in buffer.data.chunks_exact(4) {
            if chunk[3] > 0 {
                let r = chunk[0];
                let g = chunk[1];
                let b = chunk[2];
                let lum = ((r as u32 * 77 + g as u32 * 150 + b as u32 * 29) >> 8) as usize;

                red[r as usize] += 1;
                green[g as usize] += 1;
                blue[b as usize] += 1;
                luminance[lum.min(255)] += 1;
            }
        }

        let mut max_count = 0;
        for i in 0..256 {
            max_count = max_count.max(red[i]).max(green[i]).max(blue[i]).max(luminance[i]);
        }

        HistogramData { red, green, blue, luminance, max_count }
    }

    pub fn apply_color_balance(
        buffer: &mut PixelBuffer,
        cyan_red: f32,
        magenta_green: f32,
        yellow_blue: f32,
    ) {
        let dr = cyan_red * 1.28;
        let dg = magenta_green * 1.28;
        let db = yellow_blue * 1.28;

        for chunk in buffer.data.chunks_exact_mut(4) {
            if chunk[3] > 0 {
                let r = (chunk[0] as f32 + dr).clamp(0.0, 255.0);
                let g = (chunk[1] as f32 + dg).clamp(0.0, 255.0);
                let b = (chunk[2] as f32 + db).clamp(0.0, 255.0);
                chunk[0] = r.round() as u8;
                chunk[1] = g.round() as u8;
                chunk[2] = b.round() as u8;
            }
        }
    }
}
