//! The single place a knob position becomes a physical unit.
//!
//! Every user-facing tuning control is a knob in `0.0..=1.0`; these functions turn that into hertz,
//! seconds or semitones. The UI never duplicates a mapping, so a change here changes the panel too.

/// TUNING: one octave either side of centre, linear.
pub fn tuning_semitones(k: f32) -> f32 {
    -12.0 + 24.0 * k.clamp(0.0, 1.0)
}

/// CUT OFF: logarithmic, 80 Hz to 8 kHz.
pub fn cutoff_hz(k: f32) -> f32 {
    log_map(k, 80.0, 8_000.0)
}

/// DECAY: logarithmic, 30 ms to 2 s.
pub fn decay_seconds(k: f32) -> f32 {
    log_map(k, 0.03, 2.0)
}

/// ENV MOD: how far the filter envelope sweeps, `0.0..=1.0`.
pub fn env_mod(k: f32) -> f32 {
    k.clamp(0.0, 1.0)
}

/// RESONANCE: tapered so the first third stays tame and the last third screams.
pub fn resonance(k: f32) -> f32 {
    k.clamp(0.0, 1.0).powf(1.5)
}

/// ACCENT: how much an accented step pushes, `0.0..=1.0`.
pub fn accent_amount(k: f32) -> f32 {
    k.clamp(0.0, 1.0)
}

/// VOLUME: perceptual taper, quiet half of the range.
pub fn master_volume(k: f32) -> f32 {
    k.clamp(0.0, 1.0).powf(2.0)
}

/// Logarithmic interpolation between two positive bounds.
fn log_map(k: f32, min: f32, max: f32) -> f32 {
    min * (max / min).powf(k.clamp(0.0, 1.0))
}

/// One-pole smoother for parameter changes, so a knob move never clicks.
#[derive(Copy, Clone, Debug)]
pub struct Smooth {
    value: f32,
    coeff: f32,
}

impl Smooth {
    /// `ms` is the time constant; `sample_rate` is the engine's rate.
    pub fn new(value: f32, ms: f32, sample_rate: f32) -> Self {
        let samples = (ms * 0.001 * sample_rate).max(1.0);
        Self {
            value,
            coeff: 1.0 - (-1.0 / samples).exp(),
        }
    }

    /// Move one sample towards `target` and return the current value.
    pub fn process(&mut self, target: f32) -> f32 {
        self.value += (target - self.value) * self.coeff;
        self.value
    }

    /// Jump straight to `target` (used when a note starts from silence).
    pub fn snap(&mut self, target: f32) {
        self.value = target;
    }

    /// The current smoothed value.
    pub fn value(&self) -> f32 {
        self.value
    }
}
