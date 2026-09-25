use crate::selection::SelectionMask;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathPoint {
    pub anchor: (f32, f32),
    pub handle_in: Option<(f32, f32)>,
    pub handle_out: Option<(f32, f32)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubPath {
    pub points: Vec<PathPoint>,
    pub closed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VectorPath {
    pub name: String,
    pub subpaths: Vec<SubPath>,
}

impl VectorPath {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            subpaths: Vec::new(),
        }
    }

    pub fn to_selection_mask(&self, width: u32, height: u32) -> SelectionMask {
        let mut mask = SelectionMask::new(width, height);
        // 多角形スキャンライン
        for subpath in &self.subpaths {
            if subpath.points.len() < 3 {
                continue;
            }
            for y in 0..height {
                let py = y as f32 + 0.5;
                let mut intersections = Vec::new();
                let count = subpath.points.len();

                for i in 0..count {
                    let p1 = subpath.points[i].anchor;
                    let p2 = subpath.points[(i + 1) % count].anchor;

                    if (p1.1 <= py && p2.1 > py) || (p2.1 <= py && p1.1 > py) {
                        let t = (py - p1.1) / (p2.1 - p1.1);
                        let ix = p1.0 + t * (p2.0 - p1.0);
                        intersections.push(ix);
                    }
                }

                intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
                for pair in intersections.chunks(2) {
                    if pair.len() == 2 {
                        let start_x = (pair[0].max(0.0) as u32).min(width);
                        let end_x = (pair[1].max(0.0).ceil() as u32).min(width);
                        for x in start_x..end_x {
                            mask.set_value(x, y, 255);
                        }
                    }
                }
            }
        }
        mask
    }

    /// Stroke the vector path onto a PixelBuffer with Bezier curve interpolation
    pub fn rasterize_stroke(
        &self,
        buffer: &mut crate::buffer::PixelBuffer,
        color: crate::color::Color,
        stroke_width: f32,
    ) {
        let half_w = (stroke_width * 0.5).max(0.5);
        for subpath in &self.subpaths {
            if subpath.points.is_empty() {
                continue;
            }
            let count = subpath.points.len();
            let limit = if subpath.closed { count } else { count.saturating_sub(1) };

            for i in 0..limit {
                let p_start = &subpath.points[i];
                let p_end = &subpath.points[(i + 1) % count];
                let samples = crate::bezier::BezierEngine::sample_curve(p_start, p_end, 32);

                for pair in samples.windows(2) {
                    let (x0, y0) = pair[0];
                    let (x1, y1) = pair[1];
                    let steps = ((x1 - x0).hypot(y1 - y0).ceil() as usize).max(1);

                    for s in 0..=steps {
                        let t = s as f32 / steps as f32;
                        let cx = x0 + t * (x1 - x0);
                        let cy = y0 + t * (y1 - y0);

                        let min_x = (cx - half_w).max(0.0) as i32;
                        let max_x = (cx + half_w).min(buffer.width as f32 - 1.0) as i32;
                        let min_y = (cy - half_w).max(0.0) as i32;
                        let max_y = (cy + half_w).min(buffer.height as f32 - 1.0) as i32;

                        for py in min_y..=max_y {
                            for px in min_x..=max_x {
                                let d = (px as f32 - cx).hypot(py as f32 - cy);
                                if d <= half_w {
                                    buffer.set_pixel(px as u32, py as u32, color);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Fill the closed vector path onto a PixelBuffer
    pub fn rasterize_fill(
        &self,
        buffer: &mut crate::buffer::PixelBuffer,
        color: crate::color::Color,
    ) {
        let mask = self.to_selection_mask(buffer.width, buffer.height);
        for y in 0..buffer.height {
            for x in 0..buffer.width {
                if mask.is_selected(x, y) {
                    buffer.set_pixel(x, y, color);
                }
            }
        }
    }
}
