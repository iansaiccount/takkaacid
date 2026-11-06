//! The drive stage: the last thing that can add dirt before the limiter.
//!
//! `soft_clip` is `x / (1 + |x|)` — odd-symmetric, monotonic, and bounded by 1, so no input can push it
//! past full scale. Feeding it a hotter signal and normalising by `soft_clip(gain)` keeps a full-scale input
//! near full scale while the quiet parts bend towards the harmonics.
//!
//! The stage blends **dry → clipped** by the knob, not by normalising alone: normalising the clip is not
//! transparent at zero (0.9 in would come out at 0.947), and a Drive at 0 has to be a wire.

use crate::param::Smooth;

/// Gain at full drive.
pub const MAX_DRIVE_GAIN: f32 = 8.0;
/// Time constant for a drive move, so turning the knob never clicks.
const SMOOTH_MS: f32 = 10.0;

/// Odd-symmetric soft clip, `|y| < 1` for every finite `x`.
pub fn soft_clip(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

/// The drive knob, smoothed, with the clip applied per sample.
#[derive(Copy, Clone, Debug)]
pub struct Drive {
    /// The knob in `0.0..=1.0`, smoothed. Gain and blend are both derived from it.
    amount: Smooth,
    /// Where the smoothing is heading.
    target: f32,
}

impl Drive {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            amount: Smooth::new(0.0, SMOOTH_MS, sample_rate),
            target: 0.0,
        }
    }

    /// `0.0` is a wire, `1.0` is maximum dirt. Smoothed, so a jump in the knob never clicks.
    pub fn set_amount(&mut self, knob: f32) {
        self.target = knob.clamp(0.0, 1.0);
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let amount = self.amount.process(self.target);
        let gain = 1.0 + (MAX_DRIVE_GAIN - 1.0) * amount * amount;
        let clipped = soft_clip(x * gain) / soft_clip(gain);
        x * (1.0 - amount) + clipped * amount
    }
}
