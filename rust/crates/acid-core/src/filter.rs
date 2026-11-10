//! Four-pole resonant lowpass, zero-delay-feedback (TPT) ladder.
//!
//! Four one-pole TPT integrators in series with a feedback path from the last stage. Cutoff is logarithmic in the UI but
//! linear here. The coefficients are rebuilt whenever the cutoff changes — and the voice changes it **every sample**,
//! because the filter envelope is a per-sample signal; what the filter never does is rebuild them for an unchanged
//! cutoff.

/// Lowest cutoff the filter will accept.
pub const MIN_CUTOFF_HZ: f32 = 20.0;

/// Feedback at `resonance = 1.0`. Just under 4.0: right at the edge of self-oscillation, but still
/// numerically stable for every input the voice can produce.
const MAX_FEEDBACK: f32 = 3.95;

pub struct LadderFilter {
    sample_rate: f32,
    cutoff_hz: f32,
    resonance: f32,
    /// Integrator state, one per pole.
    s: [f32; 4],
    /// `G = g / (1 + g)` where `g = tan(pi * fc / sr)`.
    g_over: f32,
}

impl LadderFilter {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            sample_rate,
            cutoff_hz: 1_000.0,
            resonance: 0.0,
            s: [0.0; 4],
            g_over: 0.0,
        };
        filter.set_cutoff_hz(1_000.0);
        filter
    }

    /// Cutoff in hertz, clamped to `MIN_CUTOFF_HZ .. 0.45 * sample_rate`.
    pub fn set_cutoff_hz(&mut self, hz: f32) {
        let max = 0.45 * self.sample_rate;
        let hz = hz.clamp(MIN_CUTOFF_HZ, max.max(MIN_CUTOFF_HZ));
        self.cutoff_hz = hz;
        let g = (core::f32::consts::PI * hz / self.sample_rate).tan();
        self.g_over = g / (1.0 + g);
    }

    pub fn cutoff_hz(&self) -> f32 {
        self.cutoff_hz
    }

    /// Resonance in `0.0..=1.0`; the top of the range sits at the edge of self-oscillation.
    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
    }

    pub fn resonance(&self) -> f32 {
        self.resonance
    }

    pub fn reset(&mut self) {
        self.s = [0.0; 4];
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let k = MAX_FEEDBACK * self.resonance;
        let g = self.g_over;
        let one_minus_g = 1.0 - g;
        let g2 = g * g;
        let g4 = g2 * g2;

        // Each stage contributes `S_i = (1 - G) * s_i` to its own output when the input is zero, so the
        // ladder's output is `G^4 * u + S` for the *same current* sample's input `u`. Solving the
        // feedback loop `u = x - k * y` in closed form is what makes this a ZDF ladder instead of a
        // one-sample-delayed (and, near the self-oscillation edge, unstable) feedback path.
        let s = self.s;
        let s1 = one_minus_g * s[0];
        let s2 = one_minus_g * s[1];
        let s3 = one_minus_g * s[2];
        let s4 = one_minus_g * s[3];
        let state_sum = g2 * g * s1 + g2 * s2 + g * s3 + s4;
        let y4 = (g4 * x + state_sum) / (1.0 + k * g4);
        let mut v = x - k * y4;

        for state in &mut self.s {
            let vn = (v - *state) * g;
            let lp = vn + *state;
            *state = lp + vn;
            v = lp;
        }
        y4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoff_is_clamped_to_the_usable_range() {
        let mut f = LadderFilter::new(48_000.0);
        f.set_cutoff_hz(0.0);
        assert_eq!(f.cutoff_hz(), MIN_CUTOFF_HZ);
        f.set_cutoff_hz(1_000_000.0);
        assert!((f.cutoff_hz() - 0.45 * 48_000.0).abs() < 0.01);
    }

    #[test]
    fn dc_passes_through_unchanged_and_reset_clears_the_tail() {
        let mut f = LadderFilter::new(48_000.0);
        f.set_cutoff_hz(2_000.0);
        let mut last = 0.0;
        for _ in 0..20_000 {
            last = f.process(0.25);
        }
        assert!((last - 0.25).abs() < 0.01, "dc gain was {last}");
        f.reset();
        assert_eq!(f.process(0.0), 0.0);
    }

    #[test]
    fn resonance_is_clamped_to_the_knob_range() {
        let mut f = LadderFilter::new(48_000.0);
        f.set_resonance(9.0);
        assert_eq!(f.resonance(), 1.0);
        f.set_resonance(-1.0);
        assert_eq!(f.resonance(), 0.0);
    }
}
