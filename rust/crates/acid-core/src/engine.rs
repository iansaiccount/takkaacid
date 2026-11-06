//! The audio engine — the only type the audio thread touches.
//!
//! Created with the device's sample rate and asked to fill buffers. Everything it needs is preallocated
//! at construction, so `render` never allocates and never locks.

pub struct Engine {
    sample_rate: f32,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        Self { sample_rate }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Fill `out` with the next block of mono samples.
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
    }

    pub fn is_playing(&self) -> bool {
        false
    }
}
