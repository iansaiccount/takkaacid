//! The step clock: a countdown to the next 16th-note boundary that never drifts.
//!
//! The clock counts steps forever and never wraps — the sequencer maps `step_index() % 16` onto the
//! pattern. Durations are `f64` samples, recomputed at every boundary from the *current* tempo and
//! swing, so a long run split into arbitrary block sizes still lands on the same step count.

use crate::timing::{DEFAULT_BPM, clamp_bpm, step_duration_samples};

/// Counts down to step boundaries, carrying the leftover samples forward.
#[derive(Debug, Clone)]
pub struct Clock {
    sample_rate: f32,
    bpm: f32,
    swing: f32,
    /// Unbounded step count since the clock was created; callers wrap it onto the pattern.
    step_index: usize,
    /// Samples left in the current step. `f64` so a run of any length stays drift-free.
    remaining: f64,
}

impl Clock {
    /// A clock at the launch tempo, with the first step starting immediately.
    pub fn new(sample_rate: f32) -> Self {
        let bpm = DEFAULT_BPM;
        let swing = 0.0;
        Self {
            sample_rate,
            bpm,
            swing,
            step_index: 0,
            remaining: step_duration_samples(0, bpm, sample_rate, swing),
        }
    }

    /// Set the tempo, clamped to the panel's range.
    ///
    /// The step in flight is rescaled rather than restarted, so a knob sweep is heard immediately and
    /// the beat never jumps backwards.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.rescale(clamp_bpm(bpm), self.swing);
    }

    /// Set the swing amount, clamped to `0.0..=1.0`. Applies from the step in flight, like the tempo.
    pub fn set_swing(&mut self, swing: f32) {
        self.rescale(self.bpm, swing.clamp(0.0, 1.0));
    }

    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    pub fn swing(&self) -> f32 {
        self.swing
    }

    /// Steps completed since the clock started, unbounded.
    pub fn step_index(&self) -> usize {
        self.step_index
    }

    /// Samples left before the next boundary — how far the caller may render before it must
    /// reconsider the pattern.
    pub fn samples_until_next_step(&self) -> f64 {
        self.remaining
    }

    /// Advance by `samples`, returning how many step boundaries were crossed.
    ///
    /// Every crossing recomputes the next step's duration from the current tempo and swing, so the
    /// leftovers never accumulate into drift.
    pub fn advance_by(&mut self, samples: usize) -> usize {
        let mut left = samples as f64;
        let mut crossed = 0;
        while left >= self.remaining {
            left -= self.remaining;
            self.step_index += 1;
            crossed += 1;
            self.remaining =
                step_duration_samples(self.step_index, self.bpm, self.sample_rate, self.swing);
        }
        self.remaining -= left;
        crossed
    }

    /// Recompute the in-flight step for a new tempo or swing, keeping its elapsed fraction.
    ///
    /// Half a step elapsed stays half a step elapsed whatever the new duration is — rescaling instead
    /// of restarting is what makes a tempo or swing change take effect on the step it was made in.
    fn rescale(&mut self, bpm: f32, swing: f32) {
        let old = step_duration_samples(self.step_index, self.bpm, self.sample_rate, self.swing);
        let new = step_duration_samples(self.step_index, bpm, self.sample_rate, swing);
        self.remaining = self.remaining * new / old;
        self.bpm = bpm;
        self.swing = swing;
    }
}
