use crate::color::Color;
use serde::{Deserialize, Serialize};

pub const TILE_SIZE: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA 8-bit, 4 bytes per pixel
}

impl PixelBuffer {
    /// Safely computes the buffer size with overflow checks.
    #[inline]
    pub fn compute_buffer_size(width: u32, height: u32) -> Result<usize, String> {
        (width as usize)
            .checked_mul(height as usize)
            .and_then(|count| count.checked_mul(4))
            .ok_or_else(|| format!("Canvas dimension overflow: {}x{}", width, height))
    }

    pub fn new(width: u32, height: u32) -> Self {
        let size = Self::compute_buffer_size(width, height).unwrap_or(0);
        Self {
            width,
            height,
            data: vec![0; size],
        }
    }

    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Result<Self, String> {
        let expected = Self::compute_buffer_size(width, height)?;
        if data.len() != expected {
            return Err(format!("Buffer size mismatch: got {}, expected {}", data.len(), expected));
        }
        Ok(Self { width, height, data })
    }

    pub fn from_color(width: u32, height: u32, color: Color) -> Self {
        let count = (width as usize).checked_mul(height as usize).unwrap_or(0);
        let mut data = Vec::with_capacity(count * 4);
        let pixel = [color.r, color.g, color.b, color.a];
        for _ in 0..count {
            data.extend_from_slice(&pixel);
        }
        Self { width, height, data }
    }

    pub fn fill(&mut self, color: Color) {
        let pixel = [color.r, color.g, color.b, color.a];
        for chunk in self.data.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel);
        }
    }

    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    #[inline]
    pub fn pixel_index(&self, x: u32, y: u32) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(((y as usize) * (self.width as usize) + (x as usize)) * 4)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Color> {
        self.pixel_index(x, y).map(|idx| Color {
            r: self.data[idx],
            g: self.data[idx + 1],
            b: self.data[idx + 2],
            a: self.data[idx + 3],
        })
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        if let Some(idx) = self.pixel_index(x, y) {
            self.data[idx] = color.r;
            self.data[idx + 1] = color.g;
            self.data[idx + 2] = color.b;
            self.data[idx + 3] = color.a;
        }
    }

    pub fn extract_tile(&self, tile_x: u32, tile_y: u32) -> Option<Vec<u8>> {
        let start_x = tile_x * (TILE_SIZE as u32);
        let start_y = tile_y * (TILE_SIZE as u32);

        if start_x >= self.width || start_y >= self.height {
            return None;
        }

        let w = (self.width - start_x).min(TILE_SIZE as u32) as usize;
        let h = (self.height - start_y).min(TILE_SIZE as u32) as usize;

        let mut tile_data = Vec::with_capacity(w * h * 4);
        for row in 0..h {
            let src_y = (start_y as usize) + row;
            let src_idx = (src_y * (self.width as usize) + (start_x as usize)) * 4;
            tile_data.extend_from_slice(&self.data[src_idx..src_idx + w * 4]);
        }
        Some(tile_data)
    }

    pub fn restore_tile(&mut self, tile_x: u32, tile_y: u32, tile_data: &[u8]) {
        let start_x = tile_x * (TILE_SIZE as u32);
        let start_y = tile_y * (TILE_SIZE as u32);

        if start_x >= self.width || start_y >= self.height {
            return;
        }

        let w = (self.width - start_x).min(TILE_SIZE as u32) as usize;
        let h = (self.height - start_y).min(TILE_SIZE as u32) as usize;

        for row in 0..h {
            let src_offset = row * w * 4;
            let dst_y = (start_y as usize) + row;
            let dst_idx = (dst_y * (self.width as usize) + (start_x as usize)) * 4;
            self.data[dst_idx..dst_idx + w * 4].copy_from_slice(&tile_data[src_offset..src_offset + w * 4]);
        }
    }
}
