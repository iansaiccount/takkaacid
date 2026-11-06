//! The audio engine — the only type the audio thread touches.
//!
//! Created with the device's sample rate and asked to fill buffers. Everything it needs is preallocated
//! at construction, so `render` never allocates and never locks. The engine also *stores* every knob
//! position: the UI writes knobs here, the panel reads them back, and Phase 08 persists them.

use std::sync::Arc;

use crate::oscillator::Waveform;
use crate::param;
use crate::drive::Drive;
use crate::pattern::Pattern;
use crate::sequencer::Sequencer;
use crate::voice::{
    DEFAULT_ACCENT_KNOB, DEFAULT_CUTOFF_KNOB, DEFAULT_DECAY_KNOB, DEFAULT_ENV_MOD_KNOB,
    DEFAULT_RESONANCE_KNOB, Voice,
};

/// TUNING knob at launch: centred, no transpose.
pub const DEFAULT_TUNING_KNOB: f32 = 0.5;
/// VOLUME knob at launch. Applied by the master section in Phase 09; stored from now on.
pub const DEFAULT_VOLUME_KNOB: f32 = 0.8;
/// DRIVE knob at launch: clean, so the first thing you hear is the voice, not the clipping.
pub const DEFAULT_DRIVE_KNOB: f32 = 0.0;

/// Every knob position, so readback and persistence have one source of truth.
#[derive(Copy, Clone, Debug)]
struct Knobs {
    waveform: Waveform,
    tuning: f32,
    cutoff: f32,
    resonance: f32,
    env_mod: f32,
    decay: f32,
    accent: f32,
    volume: f32,
    drive: f32,
}

impl Default for Knobs {
    fn default() -> Self {
        Self {
            waveform: Waveform::Saw,
            tuning: DEFAULT_TUNING_KNOB,
            cutoff: DEFAULT_CUTOFF_KNOB,
            resonance: DEFAULT_RESONANCE_KNOB,
            env_mod: DEFAULT_ENV_MOD_KNOB,
            decay: DEFAULT_DECAY_KNOB,
            accent: DEFAULT_ACCENT_KNOB,
            volume: DEFAULT_VOLUME_KNOB,
            drive: DEFAULT_DRIVE_KNOB,
        }
    }
}

pub struct Engine {
    sample_rate: f32,
    voice: Voice,
    sequencer: Sequencer,
    drive: Drive,
    playing: bool,
    knobs: Knobs,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        let knobs = Knobs::default();
        let mut voice = Voice::new(sample_rate);
        // Push the launch patch into the voice so a fresh engine already sounds like the panel.
        voice.set_waveform(knobs.waveform);
        voice.set_tuning_semitones(param::tuning_semitones(knobs.tuning));
        voice.set_cutoff_knob(knobs.cutoff);
        voice.set_resonance_knob(knobs.resonance);
        voice.set_env_mod_knob(knobs.env_mod);
        voice.set_decay_knob(knobs.decay);
        voice.set_accent_knob(knobs.accent);
        Self {
            sample_rate,
            voice,
            sequencer: Sequencer::new(sample_rate),
            drive: Drive::new(sample_rate),
            playing: false,
            knobs,
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Fill `out` with the next block of mono samples.
    pub fn render(&mut self, out: &mut [f32]) {
        // The sequencer owns the transport: it splits at step boundaries, triggers the steps, and keeps
        // rendering the release tail after a stop so it never cuts mid-cycle.
        self.sequencer.render(&mut self.voice, out);
        // Master chain: the voice feeds the drive; the limiter (Task 44) and the VOLUME knob (Task 46) come
        // after it, in that order.
        for sample in out.iter_mut() {
            *sample = self.drive.process(*sample);
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// True while a note is sounding, regardless of what the transport is doing.
    pub fn is_playing_something(&self) -> bool {
        self.voice.is_active()
    }

    /// Start the transport at step 0 of the current pattern.
    pub fn start(&mut self) {
        self.playing = true;
        self.sequencer.start();
    }

    /// Stop the transport and close the gate. Closing it here as well as in the sequencer means a stop
    /// silences immediately even if no render follows.
    pub fn stop(&mut self) {
        self.playing = false;
        self.sequencer.stop();
    }

    /// Swap the playing pattern without disturbing position, glide history or the clock.
    pub fn set_pattern(&mut self, pattern: Arc<Pattern>) {
        self.sequencer.set_pattern(pattern);
    }

    /// TEMPO, in BPM; the clock clamps it to the panel's range.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.sequencer.set_bpm(bpm);
    }

    pub fn bpm(&self) -> f32 {
        self.sequencer.clock().bpm()
    }

    /// SWING, `0.0` straight to `1.0` full shuffle.
    pub fn set_swing(&mut self, swing: f32) {
        self.sequencer.set_swing(swing);
    }

    /// The step being played, `0..16`.
    pub fn position(&self) -> usize {
        self.sequencer.position()
    }

    /// Play a note directly, outside the sequencer (auditioning and MIDI keys).
    pub fn note_on(&mut self, hz: f32, accent: bool) {
        self.voice.note_on(hz, accent);
    }

    pub fn note_off(&mut self) {
        self.voice.note_off();
    }

    /// SLIDE: glide to `hz` without retriggering the envelope.
    pub fn glide_to(&mut self, hz: f32) {
        self.voice.glide_to(hz);
    }

    // ---- knobs: set, then read back -------------------------------------------------------------

    /// WAVEFORM switch.
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.knobs.waveform = waveform;
        self.voice.set_waveform(waveform);
    }

    pub fn waveform(&self) -> Waveform {
        self.knobs.waveform
    }

    /// TUNING knob, in knob domain: `0.5` is centre.
    pub fn set_tuning(&mut self, knob: f32) {
        self.knobs.tuning = knob.clamp(0.0, 1.0);
        self.voice
            .set_tuning_semitones(param::tuning_semitones(self.knobs.tuning));
    }

    pub fn tuning(&self) -> f32 {
        self.knobs.tuning
    }

    /// CUT OFF knob.
    pub fn set_cutoff(&mut self, knob: f32) {
        self.knobs.cutoff = knob.clamp(0.0, 1.0);
        self.voice.set_cutoff_knob(self.knobs.cutoff);
    }

    pub fn cutoff(&self) -> f32 {
        self.knobs.cutoff
    }

    /// RESONANCE knob.
    pub fn set_resonance(&mut self, knob: f32) {
        self.knobs.resonance = knob.clamp(0.0, 1.0);
        self.voice.set_resonance_knob(self.knobs.resonance);
    }

    pub fn resonance(&self) -> f32 {
        self.knobs.resonance
    }

    /// ENV MOD knob.
    pub fn set_env_mod(&mut self, knob: f32) {
        self.knobs.env_mod = knob.clamp(0.0, 1.0);
        self.voice.set_env_mod_knob(self.knobs.env_mod);
    }

    pub fn env_mod(&self) -> f32 {
        self.knobs.env_mod
    }

    /// DECAY knob.
    pub fn set_decay(&mut self, knob: f32) {
        self.knobs.decay = knob.clamp(0.0, 1.0);
        self.voice.set_decay_knob(self.knobs.decay);
    }

    pub fn decay(&self) -> f32 {
        self.knobs.decay
    }

    /// ACCENT knob.
    pub fn set_accent(&mut self, knob: f32) {
        self.knobs.accent = knob.clamp(0.0, 1.0);
        self.voice.set_accent_knob(self.knobs.accent);
    }

    pub fn accent(&self) -> f32 {
        self.knobs.accent
    }

    /// VOLUME knob — stored now, applied by the master section in Phase 09.
    pub fn set_volume(&mut self, knob: f32) {
        self.knobs.volume = knob.clamp(0.0, 1.0);
    }

    pub fn volume(&self) -> f32 {
        self.knobs.volume
    }

    /// DRIVE knob: `0.0` is a wire, `1.0` is maximum dirt.
    pub fn set_drive(&mut self, knob: f32) {
        self.knobs.drive = knob.clamp(0.0, 1.0);
        self.drive.set_amount(self.knobs.drive);
    }

    pub fn drive(&self) -> f32 {
        self.knobs.drive
    }
}
