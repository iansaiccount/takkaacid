//! The step clock: a countdown to the next step boundary that never drifts.
//!
//! The clock counts steps forever and never wraps — the sequencer maps `step_index()` onto the pattern's own count.
//! Durations are `f64` samples, recomputed at every boundary from the *current* tempo, swing and grid, so a long run split
//! into arbitrary block sizes still lands on the same step count.
//!
//! How long a step is comes from the pattern's grid (`pattern::Grid::steps_per_bar`), so a bar is a bar at every grid and
//! only its division changes. A grid change is rescaled like a tempo change: the step in flight keeps its elapsed fraction.

use crate::pattern::Grid;
use crate::timing::{DEFAULT_BPM, clamp_bpm, step_duration_samples};

/// Counts down to step boundaries, carrying the leftover samples forward.
#[derive(Debug, Clone)]
pub struct Clock {
    sample_rate: f32,
    bpm: f32,
    swing: f32,
    /// Steps to the bar: the grid of the pattern the clock is playing, so a bar is divided as its pattern says. Sixteenths
    /// until a pattern says otherwise.
    steps_per_bar: usize,
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
        let steps_per_bar = Grid::Sixteenth.steps_per_bar();
        Self {
            sample_rate,
            bpm,
            swing,
            steps_per_bar,
            step_index: 0,
            remaining: step_duration_samples(0, bpm, sample_rate, swing, steps_per_bar),
        }
    }

    /// Set the tempo, clamped to the panel's range.
    ///
    /// The step in flight is rescaled rather than restarted, so a knob sweep is heard immediately and
    /// the beat never jumps backwards.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.rescale(clamp_bpm(bpm), self.swing, self.steps_per_bar);
    }

    /// Set the swing amount, clamped to `0.0..=1.0`. Applies from the step in flight, like the tempo.
    pub fn set_swing(&mut self, swing: f32) {
        self.rescale(self.bpm, swing.clamp(0.0, 1.0), self.steps_per_bar);
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

    /// Put the clock back on step 0 with a whole step in hand.
    ///
    /// Stopping the transport does not stop the clock — it only stops the sequencer from rendering, so `remaining` is
    /// frozen wherever the last run left it. A restart that skipped this would give the downbeat whatever fraction of a
    /// step was left over: press RUN 100 ms into a 115 ms step and the first note is a 15 ms blip. It also resets the
    /// index, because swing derives a step's length from the clock's own index and a restart at position 0 on an odd
    /// index would start the bar off-swing.
    pub fn restart(&mut self) {
        self.step_index = 0;
        self.remaining = step_duration_samples(
            0,
            self.bpm,
            self.sample_rate,
            self.swing,
            self.steps_per_bar,
        );
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
            self.remaining = step_duration_samples(
                self.step_index,
                self.bpm,
                self.sample_rate,
                self.swing,
                self.steps_per_bar,
            );
        }
        self.remaining -= left;
        crossed
    }

    /// Steps to the bar, as the pattern on the air divides it.
    pub fn steps_per_bar(&self) -> usize {
        self.steps_per_bar
    }

    /// Divide the bar into `steps_per_bar` from here on.
    ///
    /// The step in flight is **rescaled**, exactly as a tempo change is: half a step elapsed stays half a step elapsed, so
    /// a grid change is heard from the step it was made in and the clock never jumps backwards. The same advice is a
    /// no-op, because `Sequencer::set_pattern` publishes a grid on every pattern change and a rescale per block would
    /// drift.
    pub fn set_steps_per_bar(&mut self, steps_per_bar: usize) {
        let steps_per_bar = steps_per_bar.max(1);
        if steps_per_bar != self.steps_per_bar {
            self.rescale(self.bpm, self.swing, steps_per_bar);
        }
    }

    /// Recompute the in-flight step for a new tempo, swing or grid, keeping its elapsed fraction.
    ///
    /// Half a step elapsed stays half a step elapsed whatever the new duration is — rescaling instead
    /// of restarting is what makes a tempo, swing or grid change take effect on the step it was made in.
    fn rescale(&mut self, bpm: f32, swing: f32, steps_per_bar: usize) {
        let old = step_duration_samples(
            self.step_index,
            self.bpm,
            self.sample_rate,
            self.swing,
            self.steps_per_bar,
        );
        let new =
            step_duration_samples(self.step_index, bpm, self.sample_rate, swing, steps_per_bar);
        self.remaining = self.remaining * new / old;
        self.bpm = bpm;
        self.swing = swing;
        self.steps_per_bar = steps_per_bar;
    }
}
