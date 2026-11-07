//! The oscillator: one phase accumulator, four waveshapes.
//!
//! Deliberately naive — no band-limiting. Anti-aliasing is Phase 12; at bass frequencies through a
//! 4-pole lowpass the fold-back is largely masked, and sine and triangle barely alias at all.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The panel's WAVEFORM switch.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Waveform {
    #[default]
    Saw,
    Square,
    Sine,
    Triangle,
}

impl Waveform {
    /// Every wave the panel offers, in the order the control shows them.
    pub const ALL: [Waveform; 4] = [
        Waveform::Saw,
        Waveform::Square,
        Waveform::Sine,
        Waveform::Triangle,
    ];

    /// Where this wave sits in [`Self::ALL`]; the save file stores this number.
    pub fn index(self) -> usize {
        match self {
            Waveform::Saw => 0,
            Waveform::Square => 1,
            Waveform::Sine => 2,
            Waveform::Triangle => 3,
        }
    }

    /// The wave at `index`, clamped to the catalogue: a file from a newer build names a wave this one does not have, and
    /// clipping to the last entry is kinder than refusing to load.
    pub fn from_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }

    /// The panel's ASCII caption for this wave.
    pub fn name(self) -> &'static str {
        match self {
            Waveform::Saw => "SAW",
            Waveform::Square => "SQR",
            Waveform::Sine => "SINE",
            Waveform::Triangle => "TRI",
        }
    }
}

/// Lowest frequency the oscillator will play.
pub const MIN_HZ: f32 = 16.0;
/// Highest frequency the oscillator will play.
pub const MAX_HZ: f32 = 4_000.0;

pub struct Oscillator {
    sample_rate: f32,
    /// `0.0..1.0`, advanced by `inc` per sample.
    phase: f64,
    inc: f64,
    waveform: Waveform,
}

impl Oscillator {
    pub fn new(sample_rate: f32) -> Self {
        let mut osc = Self {
            sample_rate,
            phase: 0.0,
            inc: 0.0,
            waveform: Waveform::Saw,
        };
        osc.set_frequency(55.0);
        osc
    }

    pub fn set_frequency(&mut self, hz: f32) {
        let hz = hz.clamp(MIN_HZ, MAX_HZ);
        self.inc = hz as f64 / self.sample_rate as f64;
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn waveform(&self) -> Waveform {
        self.waveform
    }

    /// Frequency implied by the current increment, for tests and readback.
    pub fn frequency(&self) -> f32 {
        (self.inc * self.sample_rate as f64) as f32
    }

    /// The value at the current phase; both waves read the same accumulator, so switching the
    /// waveform cannot jump the pitch.
    pub fn next_sample(&mut self) -> f32 {
        let value = match self.waveform {
            Waveform::Saw => 2.0 * self.phase as f32 - 1.0,
            Waveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            // Phase 0 is the bottom of every wave, so switching between them cannot move the pitch or the level.
            Waveform::Sine => (core::f64::consts::TAU * self.phase).sin() as f32,
            Waveform::Triangle => 1.0 - 4.0 * (self.phase as f32 - 0.5).abs(),
        };
        self.phase += self.inc;
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }
        value
    }

    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }
}
