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
}
