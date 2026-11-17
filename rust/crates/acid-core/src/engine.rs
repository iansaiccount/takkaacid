//! The audio engine — the only type the audio thread touches.
//!
//! Created with the device's sample rate and asked to fill buffers. Everything it needs is preallocated
//! at construction, so `render` never allocates and never locks. The engine also *stores* every knob
//! position: the UI writes knobs here, the panel reads them back, and Phase 08 persists them.

use std::sync::Arc;

use crate::drive::Drive;
use crate::limiter::Limiter;
use crate::oscillator::Waveform;
use crate::param;
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

/// What the factory pattern should measure at launch settings, in dBFS. The voice's amplitude is staged for
/// this; `tests/gain_staging.rs` holds it to a ±4 dB window.
pub const TARGET_RMS_DB: f32 = -14.0;

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
    limiter: Limiter,
    /// The VOLUME knob's taper, smoothed so a sweep does not step.
    volume: param::Smooth,
    playing: bool,
    /// Semitones of performance transpose, on top of the saved TUNING knob. Not part of the patch.
    transpose: i32,
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
            limiter: Limiter::new(sample_rate),
            volume: param::Smooth::new(param::master_volume(knobs.volume), 10.0, sample_rate),
            playing: false,
            transpose: 0,
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
        // Master chain, in this order: voice/sequencer → drive → limiter → VOLUME. The limiter sits before
        // the volume so the ceiling holds whatever the volume knob is doing.
        let target = param::master_volume(self.knobs.volume);
        for sample in out.iter_mut() {
            // Volume 0 is a mute, so it must be exact silence rather than an asymptote.
            let volume = if target == 0.0 {
                self.volume.snap(0.0);
                self.volume.value()
            } else {
                self.volume.process(target)
            };
            *sample = self.limiter.process(self.drive.process(*sample)) * volume;
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
        self.start_at(0);
    }

    /// Start the transport on `step`: the **paused** rack's path, where a joining device lands on the step the resume will
    /// use. A device joining a *playing* rack is held for its bar line and starts with [`Engine::start_later`].
    pub fn start_at(&mut self, step: usize) {
        self.playing = true;
        self.sequencer.start_at(step);
    }

    /// Start the transport on `step` after `silence` samples of the next block: a device joining a rack mid-block begins on
    /// the bar line's own sample rather than at the top of the block.
    pub fn start_later(&mut self, step: usize, silence: usize) {
        self.playing = true;
        self.sequencer.start_later(step, silence);
    }

    /// Where step 0 began in the last block, in samples from its start, if it began at all.
    ///
    /// The rack's bar line is its devices' — every playing device is in step with every other — so this is what a device
    /// waiting to join waits for.
    pub fn bar_line(&self) -> Option<usize> {
        self.sequencer.bar_line()
    }

    /// Stop the transport and close the gate. Closing it here as well as in the sequencer means a stop
    /// silences immediately even if no render follows.
    /// A direction waiting for the bar is taken with the stop: the next start returns to the top of the bar, so that bar is
    /// the first chance the direction will get. A pause does not take it — the bar it was asked in arrives on the resume.
    pub fn stop(&mut self) {
        self.playing = false;
        self.sequencer.stop();
        self.sequencer.settle_direction();
    }

    /// Stop the clock and keep the step, so [`Engine::resume`] carries on from it.
    ///
    /// The same silence as [`Engine::stop`], because below this line the two are the same thing. The third state exists
    /// because of what follows: a stop is answered with `start`, which returns to the top of the bar, and a pause with
    /// `resume`, which does not.
    pub fn pause(&mut self) {
        self.playing = false;
        self.sequencer.stop();
    }

    /// Carry on from the step a pause kept. Unlike [`Engine::start`] this does not return to the top of the bar.
    ///
    /// **Not for coming out of a stop.** The engine keeps its step through a stop as well, so it cannot tell the two states
    /// apart: a caller that resumes from `Stopped` gets the stale step rather than the bar's first. Play from `Stopped` is
    /// [`Engine::start`]; the app's `Rack::transport` is what knows which of the two the player pressed.
    pub fn resume(&mut self) {
        self.playing = true;
        self.sequencer.resume();
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

    /// Play a note directly, outside the sequencer, at full velocity (the on-screen keyboard has no velocity to offer).
    pub fn note_on(&mut self, hz: f32, accent: bool) {
        self.note_at(hz, accent, crate::pattern::DEFAULT_VELOCITY);
    }

    /// Play a note directly at a given velocity: a MIDI keyboard knows how hard the key was struck, and what the player
    /// hears has to match the step it writes.
    pub fn note_at(&mut self, hz: f32, accent: bool, velocity: u8) {
        self.voice.note_at(hz, accent, velocity);
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
        self.apply_tuning();
    }

    /// The performance transpose: semitones added to whatever the TUNING knob already says, and deliberately never saved.
    /// The knob is the patch; this is the rehearsal.
    pub fn set_transpose(&mut self, semitones: i32) {
        self.transpose = semitones.clamp(-12, 12);
        self.apply_tuning();
    }

    pub fn transpose(&self) -> i32 {
        self.transpose
    }

    /// Run the sequencer's line backwards: a direction on the clock, not a different pattern.
    pub fn set_reverse(&mut self, reverse: bool) {
        self.sequencer.set_reverse(reverse);
    }

    /// Whether the direction asked for is waiting for the next bar line.
    pub fn reverse_is_pending(&self) -> bool {
        self.sequencer.reverse_is_pending()
    }

    /// The direction the line is playing in now.
    pub fn is_reverse(&self) -> bool {
        self.sequencer.is_reverse()
    }

    /// The one place that turns the saved knob and the unsaved transpose into one semitone offset at the voice.
    fn apply_tuning(&mut self) {
        let knob = param::tuning_semitones(self.knobs.tuning);
        self.voice
            .set_tuning_semitones(knob + self.transpose as f32);
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

    /// VOLUME knob, applied after the limiter so it can only attenuate.
    ///
    /// A change made while nothing is playing takes effect at once; a change during playback ramps, so a
    /// sweep never steps.
    pub fn set_volume(&mut self, knob: f32) {
        self.knobs.volume = knob.clamp(0.0, 1.0);
        if !self.playing && !self.voice.is_active() {
            self.volume.snap(param::master_volume(self.knobs.volume));
        }
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
