//! Stylus Dynamics & Drawing Engine for Iroai
//!
//! Provides hardware-grade pen input handling:
//! - Non-linear cubic pressure response curves with deadzones and hardness control
//! - Real-time stroke stabilization via exponential moving average (EMA)
//! - Catmull-Rom spline interpolation for smooth sub-pixel continuous lines
//! - Stylus tilt and angle dynamics for natural brush angle/shading

/// Stylus raw event from OS tablet driver (Wacom, Apple Pencil, Surface Pen, etc.)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StylusInput {
    pub x: f64,
    pub y: f64,
    /// Raw hardware pressure, normalized to [0.0, 1.0]
    pub pressure: f64,
    /// Tilt altitude angle in radians [0, pi/2]
    pub tilt_altitude: f64,
    /// Tilt azimuth angle in radians [0, 2*pi]
    pub tilt_azimuth: f64,
    /// Timestamp in milliseconds or monotonic ticks
    pub timestamp_ms: u64,
}

/// Customizable Bezier/Cubic Pressure Curve.
#[derive(Debug, Clone)]
pub struct PressureCurve {
    /// Minimum threshold to register a stroke (ignores light contact noise)
    pub deadzone_low: f64,
    /// Pressure at which 100% output is reached
    pub deadzone_high: f64,
    /// Curve exponent: < 1.0 = soft (ramps up quickly), 1.0 = linear, > 1.0 = hard (requires firm press)
    pub gamma: f64,
}

impl Default for PressureCurve {
    fn default() -> Self {
        Self {
            deadzone_low: 0.02,
            deadzone_high: 0.98,
            gamma: 1.0,
        }
    }
}

impl PressureCurve {
    /// Maps raw hardware pressure to dynamic brush pressure.
    pub fn evaluate(&self, raw: f64) -> f64 {
        if raw <= self.deadzone_low {
            return 0.0;
        }
        if raw >= self.deadzone_high {
            return 1.0;
        }

        let normalized = (raw - self.deadzone_low) / (self.deadzone_high - self.deadzone_low);
        normalized.clamp(0.0, 1.0).powf(self.gamma)
    }
}

/// Real-time stroke stabilizer and jitter reduction filter.
pub struct StrokeStabilizer {
    /// Smoothing factor in range [0.0, 1.0].
    /// 0.0 = raw passthrough, 0.9 = heavy lazy-mouse smoothing
    pub smoothing: f64,
    current_pos: Option<(f64, f64)>,
    current_pressure: f64,
}

impl StrokeStabilizer {
    pub fn new(smoothing: f64) -> Self {
        Self {
            smoothing: smoothing.clamp(0.0, 0.98),
            current_pos: None,
            current_pressure: 0.0,
        }
    }

    /// Resets the stabilizer at the start of a new stroke (`pointer_down`).
    pub fn begin_stroke(&mut self, input: StylusInput) -> StylusInput {
        self.current_pos = Some((input.x, input.y));
        self.current_pressure = input.pressure;
        input
    }

    /// Filters an incoming raw pointer event with Exponential Moving Average.
    pub fn update(&mut self, input: StylusInput) -> StylusInput {
        let (cur_x, cur_y) = match self.current_pos {
            Some(pos) => pos,
            None => {
                return self.begin_stroke(input);
            }
        };

        let alpha = 1.0 - self.smoothing;
        let smooth_x = cur_x + (input.x - cur_x) * alpha;
        let smooth_y = cur_y + (input.y - cur_y) * alpha;
        let smooth_p = self.current_pressure + (input.pressure - self.current_pressure) * alpha;

        self.current_pos = Some((smooth_x, smooth_y));
        self.current_pressure = smooth_p;

        StylusInput {
            x: smooth_x,
            y: smooth_y,
            pressure: smooth_p,
            tilt_altitude: input.tilt_altitude,
            tilt_azimuth: input.tilt_azimuth,
            timestamp_ms: input.timestamp_ms,
        }
    }
}

/// Catmull-Rom Spline interpolator for generating smooth, high-fidelity sub-pixel dab points
/// between discrete stylus polling packets.
pub fn interpolate_catmull_rom(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    steps: usize,
) -> Vec<(f64, f64)> {
    if steps == 0 {
        return vec![p1];
    }

    let mut points = Vec::with_capacity(steps + 1);

    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let t2 = t * t;
        let t3 = t2 * t;

        // Standard Catmull-Rom basis matrix (tension = 0.5)
        let x = 0.5
            * ((2.0 * p1.0)
                + (-p0.0 + p2.0) * t
                + (2.0 * p0.0 - 5.0 * p1.0 + 4.0 * p2.0 - p3.0) * t2
                + (-p0.0 + 3.0 * p1.0 - 3.0 * p2.0 + p3.0) * t3);

        let y = 0.5
            * ((2.0 * p1.1)
                + (-p0.1 + p2.1) * t
                + (2.0 * p0.1 - 5.0 * p1.1 + 4.0 * p2.1 - p3.1) * t2
                + (-p0.1 + 3.0 * p1.1 - 3.0 * p2.1 + p3.1) * t3);

        points.push((x, y));
    }

    points
}

/// Wet Media mixing engine (Paint load, color pickup, and drag simulation)
#[derive(Debug, Clone)]
pub struct WetMediaMixer {
    /// How much wet paint is currently loaded on the brush tip (0.0 = dry, 1.0 = full load)
    pub paint_load: f32,
    /// Water content (wetness): higher values blend more aggressively with existing canvas colors
    pub wetness: f32,
    /// Current pigment color on the brush bristles (linear RGBA)
    pub current_color: [f32; 4],
}

impl WetMediaMixer {
    pub fn new(initial_color: [f32; 4], wetness: f32, paint_load: f32) -> Self {
        Self {
            paint_load: paint_load.clamp(0.0, 1.0),
            wetness: wetness.clamp(0.0, 1.0),
            current_color: initial_color,
        }
    }

    /// Blends incoming canvas pixel color into the brush tip and returns the deposited color.
    pub fn deposit_and_pickup(&mut self, canvas_color: [f32; 4]) -> [f32; 4] {
        if self.paint_load <= 0.001 {
            // Dry brush: purely picks up canvas color and drags it
            self.current_color = canvas_color;
            return canvas_color;
        }

        // Color pickup: existing canvas pigment rubs into brush tip proportional to wetness
        let pickup_factor = self.wetness * (1.0 - self.paint_load * 0.5);
        for ch in 0..3 {
            self.current_color[ch] =
                self.current_color[ch] * (1.0 - pickup_factor) + canvas_color[ch] * pickup_factor;
        }

        // Deposited color onto canvas
        let mut deposited = self.current_color;
        deposited[3] = self.current_color[3] * self.paint_load;

        // Brush paint depletes slightly along the stroke
        self.paint_load = (self.paint_load - 0.002).max(0.0);

        deposited
    }
}

/// Particle Scatter Generator for realistic bristle and airbrush effects
pub struct ScatterGenerator {
    /// Scatter radius perpendicular and parallel to the stroke
    pub scatter_amount: f64,
    /// Number of dab particles generated per step
    pub count: usize,
    seed: u64,
}

impl ScatterGenerator {
    pub fn new(scatter_amount: f64, count: usize) -> Self {
        Self {
            scatter_amount,
            count: count.max(1),
            seed: 0x123456789ABCDEF0,
        }
    }

    fn next_random(&mut self) -> f64 {
        // Linear Congruential Generator for deterministic high-speed scatter
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.seed >> 32) as u32 as f64) / (u32::MAX as f64)
    }

    /// Generates scattered sub-pixel offsets around a base coordinate (x, y)
    pub fn generate_scatter(&mut self, center_x: f64, center_y: f64) -> Vec<(f64, f64)> {
        let mut particles = Vec::with_capacity(self.count);
        for _ in 0..self.count {
            let angle = self.next_random() * std::f64::consts::PI * 2.0;
            let radius = self.next_random().sqrt() * self.scatter_amount;
            let sx = center_x + radius * angle.cos();
            let sy = center_y + radius * angle.sin();
            particles.push((sx, sy));
        }
        particles
    }
}

/// Canvas paper grain texture modulator (e.g. cold-press watercolor paper or linen canvas)
pub struct PaperTextureMask {
    pub scale: f64,
    pub contrast: f64,
}

impl PaperTextureMask {
    pub fn new(scale: f64, contrast: f64) -> Self {
        Self {
            scale: scale.max(1.0),
            contrast: contrast.clamp(0.0, 2.0),
        }
    }

    /// Evaluates grain density factor in range [0.0, 1.0] at canvas coordinate (x, y)
    pub fn evaluate_density(&self, x: f64, y: f64) -> f32 {
        let sx = x / self.scale;
        let sy = y / self.scale;
        // High-frequency procedural texture synthesized via sinusoidal octaves
        let n1 = (sx.sin() * sy.cos()) * 0.5 + 0.5;
        let n2 = ((sx * 2.3).cos() * (sy * 2.7).sin()) * 0.5 + 0.5;
        let grain = (n1 * 0.6 + n2 * 0.4).powf(self.contrast);
        grain.clamp(0.0, 1.0) as f32
    }
}
