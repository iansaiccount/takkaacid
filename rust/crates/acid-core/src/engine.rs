//! The audio engine — the only type the audio thread touches.
//!
//! Created with the device's sample rate and asked to fill buffers. Everything it needs is preallocated
//! at construction, so `render` never allocates and never locks.

use crate::voice::Voice;

pub struct Engine {
    sample_rate: f32,
    voice: Voice,
    playing: bool,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            voice: Voice::new(sample_rate),
            playing: false,
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Fill `out` with the next block of mono samples.
    pub fn render(&mut self, out: &mut [f32]) {
        if !self.playing {
            out.fill(0.0);
            return;
        }
        self.voice.render(out);
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Play a note directly, outside the sequencer (used by the first audible milestone and MIDI keys).
    pub fn note_on(&mut self, hz: f32, accent: bool) {
        self.voice.note_on(hz, accent);
    }

    pub fn note_off(&mut self) {
        self.voice.note_off();
    }
}
