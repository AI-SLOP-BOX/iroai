use iroai_core::buffer::PixelBuffer;

pub struct GpuRenderer;

impl Default for GpuRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn upload_texture(&self, _buffer: &PixelBuffer) {
        // GPU texture upload stub
    }
}
