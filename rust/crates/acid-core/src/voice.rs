//! One mono synth voice.
//!
//! The oscillator is real from Phase 03; the filter and VCA arrive in Phase 04. The public API does not
//! change shape when they do.

use crate::oscillator::{Oscillator, Waveform};

/// Gate and accent as plain floats, so the sequencer and the UI can carry them without enums.
#[derive(Copy, Clone, Debug, Default)]
#[allow(dead_code)] // fields are read once the sequencer drives the voice in Phase 05
pub struct VoiceParams {
    pub gate: f32,
    pub accent: f32,
}

const ATTACK_SECONDS: f32 = 0.005;
const RELEASE_SECONDS: f32 = 0.02;
const AMPLITUDE: f32 = 0.3;
const ACCENT_BOOST: f32 = 0.4;

pub struct Voice {
    oscillator: Oscillator,
    sample_rate: f32,
    /// The note's frequency before TUNING, in hertz.
    base_hz: f32,
    /// TUNING in semitones, `-12.0..=12.0`.
    tuning_semitones: f32,
    gate: bool,
    accent: bool,
    /// 0.0..=1.0 gain envelope edge, so notes start and stop without a click.
    level: f32,
    active: bool,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        let mut voice = Self {
            oscillator: Oscillator::new(sample_rate),
            sample_rate,
            base_hz: 55.0,
            tuning_semitones: 0.0,
            gate: false,
            accent: false,
            level: 0.0,
            active: false,
        };
        voice.apply_pitch();
        voice
    }

    pub fn note_on(&mut self, hz: f32, accent: bool) {
        self.base_hz = hz;
        self.accent = accent;
        self.gate = true;
        self.active = true;
        self.apply_pitch();
    }

    pub fn set_pitch(&mut self, hz: f32) {
        self.base_hz = hz;
        self.apply_pitch();
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

    pub fn note_off(&mut self) {
        self.gate = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    fn apply_pitch(&mut self) {
        let hz = self.base_hz * 2f32.powf(self.tuning_semitones / 12.0);
        self.oscillator.set_frequency(hz);
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let attack_step = 1.0 / (ATTACK_SECONDS * self.oscillator_sample_rate());
        let release_step = 1.0 / (RELEASE_SECONDS * self.oscillator_sample_rate());
        let amp = AMPLITUDE * if self.accent { 1.0 + ACCENT_BOOST } else { 1.0 };

        for slot in out.iter_mut() {
            if self.gate {
                self.level = (self.level + attack_step).min(1.0);
            } else {
                self.level = (self.level - release_step).max(0.0);
                if self.level == 0.0 {
                    self.active = false;
                }
            }
            let sample = self.oscillator.next_sample() * amp * self.level;
            *slot = sample.clamp(-0.999, 0.999);
        }
    }

    fn oscillator_sample_rate(&self) -> f32 {
        self.sample_rate
    }
}
