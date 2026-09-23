use crate::buffer::PixelBuffer;
use crate::color::Color;
use crate::path::PathPoint;

/// 3次ベジェ曲線（Cubic Bezier）評価および描画エンジン
pub struct BezierEngine;

impl BezierEngine {
    /// 3次ベジェ曲線上の特定 t (0.0 ..= 1.0) の座標を計算
    pub fn evaluate_cubic(
        p0: (f32, f32),
        p1: (f32, f32), // Handle out of P0
        p2: (f32, f32), // Handle in of P3
        p3: (f32, f32),
        t: f32,
    ) -> (f32, f32) {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        let x = uuu * p0.0 + 3.0 * uu * t * p1.0 + 3.0 * u * tt * p2.0 + ttt * p3.0;
        let y = uuu * p0.1 + 3.0 * uu * t * p1.1 + 3.0 * u * tt * p2.1 + ttt * p3.1;

        (x, y)
    }

    /// ベジェ曲線のサンプリング点列の生成
    pub fn sample_curve(
        pt_start: &PathPoint,
        pt_end: &PathPoint,
        steps: usize,
    ) -> Vec<(f32, f32)> {
        let p0 = pt_start.anchor;
        let p1 = pt_start.handle_out.unwrap_or(pt_start.anchor);
        let p2 = pt_end.handle_in.unwrap_or(pt_end.anchor);
        let p3 = pt_end.anchor;

        let n = steps.max(2);
        let mut points = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            points.push(Self::evaluate_cubic(p0, p1, p2, p3, t));
        }
        points
    }

    /// アンチエイリアス付きベジェ曲線のラスター描画
    pub fn rasterize_curve(
        buffer: &mut PixelBuffer,
        pt_start: &PathPoint,
        pt_end: &PathPoint,
        color: Color,
    ) {
        let samples = Self::sample_curve(pt_start, pt_end, 32);
        for pair in samples.windows(2) {
            crate::transform::Transform::draw_line(
                buffer,
                pair[0].0.round() as i32,
                pair[0].1.round() as i32,
                pair[1].0.round() as i32,
                pair[1].1.round() as i32,
                color,
            );
        }
    }
}
