//! One mono synth voice.
//!
//! Signal order: glide → oscillator → (filter, Phase 04 Task 17) → VCA. The VCA envelope is the amp
//! envelope; accent multiplies its output.

use crate::envelope::Envelope;
use crate::oscillator::{Oscillator, Waveform};
use crate::param;

/// Gate and accent as plain floats, so the sequencer and the UI can carry them without enums.
#[derive(Copy, Clone, Debug, Default)]
#[allow(dead_code)] // fields are read once the sequencer drives the voice in Phase 05
pub struct VoiceParams {
    pub gate: f32,
    pub accent: f32,
}

/// How much louder an accented step is, at a fully clockwise ACCENT knob.
pub const ACCENT_GAIN: f32 = 1.4;
/// SLIDE time constant: a step glides into the next note over roughly this long.
pub const GLIDE_TC_MS: f32 = 60.0;
/// DECAY knob position at launch.
pub const DEFAULT_DECAY_KNOB: f32 = 0.35;
/// ACCENT knob position at launch.
pub const DEFAULT_ACCENT_KNOB: f32 = 0.8;

const ATTACK_MS: f32 = 2.0;
const SUSTAIN: f32 = 0.35;
const RELEASE_MS: f32 = 20.0;
const AMPLITUDE: f32 = 0.3;

pub struct Voice {
    oscillator: Oscillator,
    sample_rate: f32,
    amp_env: Envelope,
    /// Current oscillator frequency in hertz; the glide moves this toward `glide_target_hz`.
    current_hz: f32,
    glide_target_hz: f32,
    glide_coeff: f32,
    /// The note's frequency before TUNING, in hertz.
    base_hz: f32,
    /// TUNING in semitones, `-12.0..=12.0`.
    tuning_semitones: f32,
    accent: bool,
    accent_amount: f32,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        let mut amp_env = Envelope::new(sample_rate);
        amp_env.set_attack_ms(ATTACK_MS);
        amp_env.set_sustain(SUSTAIN);
        amp_env.set_release_ms(RELEASE_MS);
        amp_env.set_decay_seconds(param::decay_seconds(DEFAULT_DECAY_KNOB));

        let mut voice = Self {
            oscillator: Oscillator::new(sample_rate),
            sample_rate,
            amp_env,
            current_hz: 55.0,
            glide_target_hz: 55.0,
            glide_coeff: 1.0 - (-1.0 / (GLIDE_TC_MS * 0.001 * sample_rate)).exp(),
            base_hz: 55.0,
            tuning_semitones: 0.0,
            accent: false,
            accent_amount: DEFAULT_ACCENT_KNOB,
        };
        voice.apply_pitch();
        voice
    }

    pub fn note_on(&mut self, hz: f32, accent: bool) {
        self.base_hz = hz;
        self.accent = accent;
        self.apply_pitch();
        self.amp_env.gate_on();
    }

    pub fn set_pitch(&mut self, hz: f32) {
        self.base_hz = hz;
        self.apply_pitch();
    }

    /// SLIDE: move to `hz` without retriggering the envelope. Exponential, `GLIDE_TC_MS` time constant.
    pub fn glide_to(&mut self, hz: f32) {
        self.base_hz = hz;
        self.glide_target_hz = hz * 2f32.powf(self.tuning_semitones / 12.0);
    }

    /// Where the glide is heading, in hertz (for tests and UI readback).
    pub fn glide_target_hz(&self) -> f32 {
        self.glide_target_hz
    }

    pub fn set_tuning_semitones(&mut self, semitones: f32) {
        self.tuning_semitones = semitones;
        self.apply_pitch();
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.oscillator.set_waveform(waveform);
    }

    pub fn waveform(&self) -> Waveform {
        self.oscillator.waveform()
    }

    /// DECAY knob in `0.0..=1.0`, mapped by [`param::decay_seconds`].
    pub fn set_decay_knob(&mut self, knob: f32) {
        self.amp_env.set_decay_seconds(param::decay_seconds(knob));
    }

    /// ACCENT knob in `0.0..=1.0`: how much an accented note pushes.
    pub fn set_accent_knob(&mut self, knob: f32) {
        self.accent_amount = param::accent_amount(knob);
    }

    pub fn note_off(&mut self) {
        self.amp_env.gate_off();
    }

    pub fn is_active(&self) -> bool {
        !self.amp_env.is_idle()
    }

    fn apply_pitch(&mut self) {
        let hz = self.base_hz * 2f32.powf(self.tuning_semitones / 12.0);
        self.current_hz = hz;
        self.glide_target_hz = hz;
        self.oscillator.set_frequency(hz);
    }

    fn accent_gain(&self) -> f32 {
        if self.accent {
            1.0 + self.accent_amount * (ACCENT_GAIN - 1.0)
        } else {
            1.0
        }
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let amp = AMPLITUDE * self.accent_gain();

        for slot in out.iter_mut() {
            // Glide first: the oscillator must be at the right frequency for this very sample.
            if (self.glide_target_hz - self.current_hz).abs() < 0.01 {
                self.current_hz = self.glide_target_hz;
            } else {
                self.current_hz += (self.glide_target_hz - self.current_hz) * self.glide_coeff;
            }
            self.oscillator.set_frequency(self.current_hz);

            let env = self.amp_env.next();
            let sample = self.oscillator.next_sample() * env * amp;
            *slot = sample.clamp(-0.999, 0.999);
        }
    }

    /// Kept for symmetry with the filter/VCA stages that follow; the rate never changes after
    /// construction.
    #[allow(dead_code)]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}
