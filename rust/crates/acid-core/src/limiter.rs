//! The limiter: the guarantee that nothing leaves the engine louder than the ceiling.
//!
//! The attack is instantaneous — the gain is computed from this sample's peak before the sample is scaled —
//! so the ceiling holds without lookahead. The release is a one-pole return to unity, so a loud burst ducks
//! the output and then recovers instead of pumping forever.

/// Nothing leaves above this.
pub const CEILING: f32 = 0.98;
/// Time constant for the gain coming back up after a burst.
pub const RELEASE_MS: f32 = 200.0;

/// Instant-attack, one-pole-release peak limiter.
#[derive(Copy, Clone, Debug)]
pub struct Limiter {
    ceiling: f32,
    release_coeff: f32,
    gain: f32,
    env: f32,
}

impl Limiter {
    pub fn new(sample_rate: f32) -> Self {
        let samples = (RELEASE_MS * 0.001 * sample_rate).max(1.0);
        Self {
            ceiling: CEILING,
            release_coeff: 1.0 - (-1.0 / samples).exp(),
            gain: 1.0,
            env: 0.0,
        }
    }

    /// The gain currently applied, for tests and for a future gain-reduction meter.
    pub fn gain(&self) -> f32 {
        self.gain
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let peak = x.abs();
        // Instant attack on the envelope, one-pole release between peaks.
        if peak > self.env {
            self.env = peak;
        } else {
            self.env *= 1.0 - self.release_coeff;
        }
        let target = if self.env > self.ceiling {
            self.ceiling / self.env
        } else {
            1.0
        };
        if target < self.gain {
            // Instant attack: this sample is already scaled by the gain it needs.
            self.gain = target;
        } else {
            // Release upward only, one time constant at a time.
            self.gain += (target - self.gain) * self.release_coeff;
        }
        (x * self.gain).clamp(-self.ceiling, self.ceiling)
    }
}
