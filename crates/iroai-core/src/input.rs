use crate::brush::BrushTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDeviceSource {
    Mouse,
    Stylus,
    Touch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchGesture {
    Pinch { zoom_delta: f32 },
    Pan { delta_x: f32, delta_y: f32 },
    Rotate { angle_delta: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StylusData {
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub twist: f32,
    pub is_eraser: bool,
}

impl Default for StylusData {
    fn default() -> Self {
        Self {
            pressure: 1.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0.0,
            is_eraser: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnifiedInputEvent {
    PointerDown {
        x: f32,
        y: f32,
        source: InputDeviceSource,
        stylus: Option<StylusData>,
        tool: BrushTool,
    },
    PointerMove {
        x: f32,
        y: f32,
        source: InputDeviceSource,
        stylus: Option<StylusData>,
        tool: BrushTool,
    },
    PointerUp {
        x: f32,
        y: f32,
        source: InputDeviceSource,
        tool: BrushTool,
    },
}

pub struct PalmRejectionFilter {
    pub is_stylus_active: bool,
}

impl PalmRejectionFilter {
    pub fn new() -> Self {
        Self {
            is_stylus_active: false,
        }
    }

    pub fn should_reject_touch(&self, source: InputDeviceSource) -> bool {
        self.is_stylus_active && source == InputDeviceSource::Touch
    }

    pub fn on_event(&mut self, event: &UnifiedInputEvent) {
        match event {
            UnifiedInputEvent::PointerDown { source, .. } => {
                if *source == InputDeviceSource::Stylus {
                    self.is_stylus_active = true;
                }
            }
            UnifiedInputEvent::PointerUp { source, .. } => {
                if *source == InputDeviceSource::Stylus {
                    self.is_stylus_active = false;
                }
            }
            _ => {}
        }
    }
}
