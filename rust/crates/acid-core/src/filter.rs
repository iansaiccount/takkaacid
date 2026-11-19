//! Two filters. The 303's: four-pole resonant lowpass, zero-delay-feedback (TPT) ladder. The drum machine's: a
//! state-variable filter with three faces (`MultimodeFilter`, below), which is the same integrators solved once for low, band
//! and high at the same time.
//!
//! Four one-pole TPT integrators in series with a feedback path from the last stage. Cutoff is logarithmic in the UI but
//! linear here. The coefficients are rebuilt whenever the cutoff is set — and the voice sets it **every sample**, because the
//! filter envelope is a per-sample signal. A rebuild is one `tan`, and that is what a per-sample cutoff costs.

use crate::drums::FilterKind;

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

/// One state-variable filter with three faces: `LP`, `BP` and `HP`, from the same two integrators.
///
/// A drum voice's filter is deliberately **not** the 303's ladder. The ladder's feedback loop is what gives the 303 its
/// scream, and it answers on the low side only; a drum needs a hat that reads as a hat, which is the high side of the same
/// signal. This is the topology-preserving state-variable filter — the same trapezoidal integrators the ladder uses, solved
/// once — so all three answers come from one pair of states and switching `LP | BP | HP` on the face restarts nothing.
///
/// Cutoff and resonance are knob positions mapped exactly as the 303's are, by [`param::cutoff_hz`] and
/// [`param::resonance`], so a dial half way round opens both machines' filters to the same place.
pub struct MultimodeFilter {
    sample_rate: f32,
    kind: FilterKind,
    cutoff_hz: f32,
    resonance: f32,
    /// `g = tan(pi * fc / sr)`.
    g: f32,
    /// The damping, `2` at no resonance: `k = 1/Q`.
    k: f32,
    /// The two integrator states. All three faces read them; none of them writes a second copy.
    ic1: f32,
    ic2: f32,
}

impl MultimodeFilter {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            sample_rate,
            kind: FilterKind::Low,
            cutoff_hz: 1_000.0,
            resonance: 0.0,
            g: 0.0,
            k: 0.0,
            ic1: 0.0,
            ic2: 0.0,
        };
        filter.set_cutoff_hz(1_000.0);
        filter.set_resonance(0.0);
        filter
    }

    /// Which side of the filter the output is taken from.
    pub fn set_kind(&mut self, kind: FilterKind) {
        self.kind = kind;
    }

    /// Cutoff in hertz, clamped to `MIN_CUTOFF_HZ .. 0.45 * sample_rate`.
    pub fn set_cutoff_hz(&mut self, hz: f32) {
        let max = 0.45 * self.sample_rate;
        let hz = hz.clamp(MIN_CUTOFF_HZ, max.max(MIN_CUTOFF_HZ));
        self.cutoff_hz = hz;
        self.g = (core::f32::consts::PI * hz / self.sample_rate).tan();
    }

    pub fn cutoff_hz(&self) -> f32 {
        self.cutoff_hz
    }

    /// Resonance in `0.0..=1.0`. The damping falls from `2` — a plain pair of poles, flat — to 2% of that at the top,
    /// which is a resonance worth hearing and still a filter that cannot ring up for ever on a sample's own noise.
    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
        self.k = 2.0 * (1.0 - 0.98 * self.resonance);
    }

    pub fn resonance(&self) -> f32 {
        self.resonance
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        let a2 = self.g * a1;
        let a3 = self.g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        match self.kind {
            FilterKind::Low => v2,
            FilterKind::Band => v1,
            FilterKind::High => x - self.k * v1 - v2,
        }
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

    /// A one-sample step through one face, after the filter has settled: what each face says a constant input is.
    fn settled(kind: FilterKind) -> f32 {
        let mut f = MultimodeFilter::new(48_000.0);
        f.set_kind(kind);
        f.set_cutoff_hz(1_000.0);
        f.set_resonance(0.0);
        let mut last = 0.0;
        for _ in 0..9_600 {
            last = f.process(0.25);
        }
        last
    }

    #[test]
    fn the_low_face_passes_dc_and_the_others_do_not() {
        assert!((settled(FilterKind::Low) - 0.25).abs() < 0.01);
        assert!(settled(FilterKind::High).abs() < 1e-3);
        assert!(settled(FilterKind::Band).abs() < 1e-3);
    }

    #[test]
    fn the_three_faces_take_different_sides_of_one_signal() {
        // A signal that alternates every sample sits at Nyquist, so the low face has to throw it away and the high face has
        // to pass it. One pair of integrator states answers all three.
        let through = |kind: FilterKind| {
            let mut f = MultimodeFilter::new(48_000.0);
            f.set_kind(kind);
            f.set_cutoff_hz(1_000.0);
            f.set_resonance(0.0);
            let out: Vec<f32> = (0..2_000)
                .map(|i| f.process(if i % 2 == 0 { 1.0 } else { -1.0 }))
                .collect();
            crate::analysis::rms(&out[100..])
        };
        let (low, band, high) = (
            through(FilterKind::Low),
            through(FilterKind::Band),
            through(FilterKind::High),
        );
        assert!(high > 0.9, "the high face passes Nyquist: {high}");
        assert!(low < 0.02, "the low face does not: {low}");
        assert!(
            band < 0.1,
            "and the band face passes one band, not the edge: {band}"
        );
    }

    #[test]
    fn resonance_lifts_the_band_it_sits_on() {
        // At the cutoff the band face's gain is `1/k`, so the resonance is what makes it a peak rather than a slope.
        let gain = |resonance: f32| {
            let mut f = MultimodeFilter::new(48_000.0);
            f.set_kind(FilterKind::Band);
            f.set_cutoff_hz(1_000.0);
            f.set_resonance(resonance);
            let out: Vec<f32> = (0..9_600)
                .map(|i| f.process((core::f32::consts::TAU * 1_000.0 * i as f32 / 48_000.0).sin()))
                .collect();
            crate::analysis::rms(&out[4_800..])
        };
        let flat = gain(0.0);
        let peak = gain(0.9);
        assert!(
            peak > flat * 3.0,
            "resonance should lift the band: {flat} -> {peak}"
        );
    }

    #[test]
    fn the_cutoff_and_the_resonance_are_clamped_to_their_ranges() {
        let mut f = MultimodeFilter::new(48_000.0);
        f.set_cutoff_hz(0.0);
        assert_eq!(f.cutoff_hz(), MIN_CUTOFF_HZ);
        f.set_cutoff_hz(1_000_000.0);
        assert!((f.cutoff_hz() - 0.45 * 48_000.0).abs() < 0.01);
        f.set_resonance(9.0);
        assert_eq!(f.resonance(), 1.0);
        f.set_resonance(-1.0);
        assert_eq!(f.resonance(), 0.0);
    }

    #[test]
    fn reset_clears_the_tail() {
        let mut f = MultimodeFilter::new(48_000.0);
        f.set_cutoff_hz(2_000.0);
        for _ in 0..1_000 {
            f.process(0.5);
        }
        f.reset();
        assert_eq!(f.process(0.0), 0.0);
    }
}
