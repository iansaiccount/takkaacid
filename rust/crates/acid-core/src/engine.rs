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
        // Keep rendering through the release tail after a stop, so it never cuts mid-cycle.
        if self.playing || self.voice.is_active() {
            self.voice.render(out);
        } else {
            out.fill(0.0);
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// True while a note is sounding, regardless of what the transport is doing.
    pub fn is_playing_something(&self) -> bool {
        self.voice.is_active()
    }

    /// Start the transport. Until the sequencer exists (Phase 05) this holds the root note.
    pub fn start(&mut self) {
        self.playing = true;
        self.voice
            .note_on(crate::pitch::step_hz(crate::pitch::Pitch::C, 0), false);
    }

    /// Stop the transport and close the gate.
    pub fn stop(&mut self) {
        self.playing = false;
        self.voice.note_off();
    }

    /// Play a note directly, outside the sequencer (used by the first audible milestone and MIDI keys).
    pub fn note_on(&mut self, hz: f32, accent: bool) {
        self.voice.note_on(hz, accent);
    }

    pub fn note_off(&mut self) {
        self.voice.note_off();
    }

    /// WAVEFORM switch.
    pub fn set_waveform(&mut self, waveform: crate::oscillator::Waveform) {
        self.voice.set_waveform(waveform);
    }

    pub fn waveform(&self) -> crate::oscillator::Waveform {
        self.voice.waveform()
    }

    /// TUNING knob, in knob domain: `0.5` is centre.
    pub fn set_tuning(&mut self, knob: f32) {
        self.voice
            .set_tuning_semitones(crate::param::tuning_semitones(knob));
    }

    /// CUT OFF knob.
    pub fn set_cutoff(&mut self, knob: f32) {
        self.voice.set_cutoff_knob(knob);
    }

    /// RESONANCE knob.
    pub fn set_resonance(&mut self, knob: f32) {
        self.voice.set_resonance_knob(knob);
    }

    /// ENV MOD knob.
    pub fn set_env_mod(&mut self, knob: f32) {
        self.voice.set_env_mod_knob(knob);
    }

    /// DECAY knob.
    pub fn set_decay(&mut self, knob: f32) {
        self.voice.set_decay_knob(knob);
    }

    /// ACCENT knob.
    pub fn set_accent(&mut self, knob: f32) {
        self.voice.set_accent_knob(knob);
    }
}
