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
}
