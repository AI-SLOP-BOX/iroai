use crate::buffer::{PixelBuffer, TILE_SIZE};
use crate::color::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// タイル単位でのオンデマンド・スパース割り当てバッファ
/// 白紙領域や無変更領域にメモリ（400MB〜数GB）を浪費しないPhotoshop相当の省メモリ基盤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseTileBuffer {
    pub width: u32,
    pub height: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub tiles: HashMap<(u32, u32), Vec<u8>>, // (tile_x, tile_y) -> 64x64x4 bytes (16KB)
    pub default_color: Color,
}

impl SparseTileBuffer {
    pub fn new(width: u32, height: u32, default_color: Color) -> Self {
        let tiles_x = (width + (TILE_SIZE as u32) - 1) / (TILE_SIZE as u32);
        let tiles_y = (height + (TILE_SIZE as u32) - 1) / (TILE_SIZE as u32);
        Self {
            width,
            height,
            tiles_x,
            tiles_y,
            tiles: HashMap::new(),
            default_color,
        }
    }

    /// 現在実際に割り当てられているメモリバイト数
    pub fn allocated_bytes(&self) -> usize {
        self.tiles.len() * TILE_SIZE * TILE_SIZE * 4
    }

    /// 単一ピクセルの取得（タイル未割り当てならデフォルト色）
    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        if x >= self.width || y >= self.height {
            return Color::TRANSPARENT;
        }

        let tx = x / (TILE_SIZE as u32);
        let ty = y / (TILE_SIZE as u32);

        if let Some(tile) = self.tiles.get(&(tx, ty)) {
            let lx = (x % (TILE_SIZE as u32)) as usize;
            let ly = (y % (TILE_SIZE as u32)) as usize;
            let idx = (ly * TILE_SIZE + lx) * 4;
            if idx + 3 < tile.len() {
                Color::rgba(tile[idx], tile[idx + 1], tile[idx + 2], tile[idx + 3])
            } else {
                self.default_color
            }
        } else {
            self.default_color
        }
    }

    /// 単一ピクセルの設定（必要になったタイルだけオンデマンドで16KB確保）
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }

        let tx = x / (TILE_SIZE as u32);
        let ty = y / (TILE_SIZE as u32);

        let lx = (x % (TILE_SIZE as u32)) as usize;
        let ly = (y % (TILE_SIZE as u32)) as usize;
        let idx = (ly * TILE_SIZE + lx) * 4;

        let default_col = self.default_color;
        let tile = self.tiles.entry((tx, ty)).or_insert_with(|| {
            let mut data = vec![0u8; TILE_SIZE * TILE_SIZE * 4];
            let def_px = [default_col.r, default_col.g, default_col.b, default_col.a];
            for chunk in data.chunks_exact_mut(4) {
                chunk.copy_from_slice(&def_px);
            }
            data
        });

        if idx + 3 < tile.len() {
            tile[idx] = color.r;
            tile[idx + 1] = color.g;
            tile[idx + 2] = color.b;
            tile[idx + 3] = color.a;
        }
    }

    /// 通常の PixelBuffer へのフラット化
    pub fn to_pixel_buffer(&self) -> PixelBuffer {
        let mut buf = PixelBuffer::new(self.width, self.height);
        buf.fill(self.default_color);

        for (&(tx, ty), tile) in &self.tiles {
            let start_x = tx * (TILE_SIZE as u32);
            let start_y = ty * (TILE_SIZE as u32);
            let w = (self.width - start_x).min(TILE_SIZE as u32) as usize;
            let h = (self.height - start_y).min(TILE_SIZE as u32) as usize;

            for row in 0..h {
                let src_offset = row * TILE_SIZE * 4;
                let dst_y = (start_y as usize) + row;
                let dst_idx = (dst_y * (self.width as usize) + (start_x as usize)) * 4;
                buf.data[dst_idx..dst_idx + w * 4].copy_from_slice(&tile[src_offset..src_offset + w * 4]);
            }
        }
        buf
    }
}
