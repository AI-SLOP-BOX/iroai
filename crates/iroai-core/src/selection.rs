use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionOp {
    New,
    Add,
    Subtract,
    Intersect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionMask {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // 0 (未選択) ..= 255 (完全選択)
}

impl SelectionMask {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            data: vec![0; size],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.iter().all(|&v| v == 0)
    }

    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    pub fn select_all(&mut self) {
        self.data.fill(255);
    }

    pub fn invert(&mut self) {
        for v in &mut self.data {
            *v = 255 - *v;
        }
    }

    #[inline]
    pub fn get_value(&self, x: u32, y: u32) -> u8 {
        if x < self.width && y < self.height {
            self.data[(y as usize) * (self.width as usize) + (x as usize)]
        } else {
            0
        }
    }

    #[inline]
    pub fn is_selected(&self, x: u32, y: u32) -> bool {
        self.get_value(x, y) > 0
    }

    #[inline]
    pub fn set_value(&mut self, x: u32, y: u32, val: u8) {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            self.data[idx] = val;
        }
    }

    pub fn select_rect(&mut self, x: u32, y: u32, w: u32, h: u32, op: SelectionOp) {
        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);

        match op {
            SelectionOp::New => {
                self.clear();
                for py in y..max_y {
                    for px in x..max_x {
                        self.set_value(px, py, 255);
                    }
                }
            }
            SelectionOp::Add => {
                for py in y..max_y {
                    for px in x..max_x {
                        self.set_value(px, py, 255);
                    }
                }
            }
            SelectionOp::Subtract => {
                for py in y..max_y {
                    for px in x..max_x {
                        self.set_value(px, py, 0);
                    }
                }
            }
            SelectionOp::Intersect => {
                for py in 0..self.height {
                    for px in 0..self.width {
                        if px >= x && px < max_x && py >= y && py < max_y {
                            // 既存値を維持
                        } else {
                            self.set_value(px, py, 0);
                        }
                    }
                }
            }
        }
    }

    pub fn select_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, op: SelectionOp) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let min_x = (cx - rx).max(0.0) as u32;
        let max_x = (cx + rx + 1.0).min(self.width as f32) as u32;
        let min_y = (cy - ry).max(0.0) as u32;
        let max_y = (cy + ry + 1.0).min(self.height as f32) as u32;

        if op == SelectionOp::New {
            self.clear();
        }

        for py in min_y..max_y {
            let dy = py as f32 + 0.5 - cy;
            let norm_y = dy / ry;
            let norm_y2 = norm_y * norm_y;
            if norm_y2 > 1.0 {
                continue;
            }

            for px in min_x..max_x {
                let dx = px as f32 + 0.5 - cx;
                let norm_x = dx / rx;
                if norm_x * norm_x + norm_y2 <= 1.0 {
                    match op {
                        SelectionOp::New | SelectionOp::Add => self.set_value(px, py, 255),
                        SelectionOp::Subtract => self.set_value(px, py, 0),
                        SelectionOp::Intersect => {}
                    }
                } else if op == SelectionOp::Intersect {
                    self.set_value(px, py, 0);
                }
            }
        }
    }

    /// Feathers (smooth Gaussian blur) the selection boundary into 8-bit soft gradients.
    pub fn feather(&mut self, radius: f32) {
        if radius <= 0.0 || self.data.is_empty() {
            return;
        }

        let r = radius.ceil() as i32;
        let sigma = radius / 2.0;
        let two_sigma2 = 2.0 * sigma * sigma;

        // Generate 1D Gaussian kernel
        let mut kernel = Vec::with_capacity((r * 2 + 1) as usize);
        let mut sum = 0.0f32;
        for i in -r..=r {
            let val = (-(i * i) as f32 / two_sigma2).exp();
            kernel.push(val);
            sum += val;
        }
        for k in &mut kernel {
            *k /= sum;
        }

        let w = self.width as usize;
        let h = self.height as usize;

        // Horizontal pass
        let mut temp = vec![0.0f32; w * h];
        for y in 0..h {
            let row_start = y * w;
            for x in 0..w {
                let mut acc = 0.0f32;
                for (k_idx, ki) in (-r..=r).enumerate() {
                    let sx = (x as i32 + ki).clamp(0, (w - 1) as i32) as usize;
                    acc += self.data[row_start + sx] as f32 * kernel[k_idx];
                }
                temp[row_start + x] = acc;
            }
        }

        // Vertical pass
        for y in 0..h {
            let row_start = y * w;
            for x in 0..w {
                let mut acc = 0.0f32;
                for (k_idx, ki) in (-r..=r).enumerate() {
                    let sy = (y as i32 + ki).clamp(0, (h - 1) as i32) as usize;
                    acc += temp[sy * w + x] * kernel[k_idx];
                }
                self.data[row_start + x] = acc.clamp(0.0, 255.0).round() as u8;
            }
        }
    }

    /// Expands (radius > 0) or contracts (radius < 0) the selection boundary via morphological filtering.
    pub fn expand_contract(&mut self, radius: i32) {
        if radius == 0 || self.data.is_empty() {
            return;
        }
        let w = self.width as usize;
        let h = self.height as usize;
        let orig = self.data.clone();
        let r_abs = radius.abs() as i32;

        for y in 0..h {
            for x in 0..w {
                let mut target_val = if radius > 0 { 0u8 } else { 255u8 };
                for dy in -r_abs..=r_abs {
                    let sy = (y as i32 + dy).clamp(0, (h - 1) as i32) as usize;
                    for dx in -r_abs..=r_abs {
                        if dx * dx + dy * dy <= r_abs * r_abs {
                            let sx = (x as i32 + dx).clamp(0, (w - 1) as i32) as usize;
                            let val = orig[sy * w + sx];
                            if radius > 0 {
                                target_val = target_val.max(val);
                            } else {
                                target_val = target_val.min(val);
                            }
                        }
                    }
                }
                self.data[y * w + x] = target_val;
            }
        }
    }
}
