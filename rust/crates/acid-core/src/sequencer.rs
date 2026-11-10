//! The sequencer: it owns the step clock and turns pattern steps into voice events.
//!
//! `render` splits every block at the next step boundary, so a note starts on the exact sample the clock
//! names whatever block size the audio device asks for. `start` and `stop` take no voice — they leave a
//! pending action that the next `render` applies, which keeps the audio callback the only place that
//! touches the voice's gate.

use std::sync::Arc;

use crate::clock::Clock;
use crate::pattern::{Pattern, STEPS_PER_PATTERN};
use crate::voice::Voice;

/// Plays a `Pattern` through a `Voice`, one step at a time.
pub struct Sequencer {
    clock: Clock,
    pattern: Arc<Pattern>,
    /// Current step, always `0..STEPS_PER_PATTERN`.
    position: usize,
    /// Whether the step that was just played carried a slide, so the next note glides into place.
    prev_slide: bool,
    running: bool,
    /// `start` triggered step 0 but had no voice: the next `render` sends it.
    trigger_pending: bool,
    /// `stop` closed the transport but had no voice: the next `render` closes the gate.
    gate_close_pending: bool,
}

impl Sequencer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            clock: Clock::new(sample_rate),
            pattern: Arc::new(Pattern::empty()),
            position: 0,
            prev_slide: false,
            running: false,
            trigger_pending: false,
            gate_close_pending: false,
        }
    }

    /// Swap the playing pattern. Position, slide history and the clock are untouched, so a switch at
    /// speed does not stutter the beat.
    pub fn set_pattern(&mut self, pattern: Arc<Pattern>) {
        self.pattern = pattern;
    }

    pub fn pattern(&self) -> &Arc<Pattern> {
        &self.pattern
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.clock.set_bpm(bpm);
    }

    pub fn set_swing(&mut self, swing: f32) {
        self.clock.set_swing(swing);
    }

    /// Start at step 0. The note sounds on the first sample rendered afterwards.
    pub fn start(&mut self) {
        self.running = true;
        // The clock kept counting through the stop, so it has to be put back on the downbeat: otherwise the first step
        // after RUN is whatever fraction of a step the previous run happened to leave, and with swing it can even be the
        // short half of the pair.
        self.clock.restart();
        self.position = 0;
        self.prev_slide = false;
        self.trigger_pending = true;
        self.gate_close_pending = false;
    }

    /// Stop the transport. The gate closes on the next render, which then fades the release tail out
    /// rather than cutting it.
    pub fn stop(&mut self) {
        if self.running {
            self.running = false;
            self.trigger_pending = false;
            self.gate_close_pending = true;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// The step being played, `0..16`.
    pub fn position(&self) -> usize {
        self.position
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Apply the trigger rule for one step and remember whether it slides.
    fn trigger(&mut self, voice: &mut Voice, position: usize) {
        let step = self.pattern.steps[position];
        // A rest, and a note transposed outside the instrument's range, are the same thing to the voice: silence. The
        // step keeps its flags and its note, so transposing back brings it in again.
        if !step.on || !step.audible() {
            voice.note_off();
            // A rest cannot glide out of anything. A rest keeps its flags, so it may carry `slide`; letting
            // that through would retune the next note with the gate still shut and it would never sound.
            self.prev_slide = false;
            return;
        }
        if self.prev_slide {
            // The previous step asked to slide into this one: retune without retriggering the envelopes.
            voice.glide_to(step.hz());
        } else {
            // A step carries its own velocity; `note_on` would play it at full scale.
            voice.note_at(step.hz(), step.accent, step.velocity);
        }
        self.prev_slide = step.slide;
    }

    /// Render the next `out.len()` samples, starting whatever step is due inside that block.
    ///
    /// When stopped, this still renders the voice so a stop fades the release tail instead of cutting it.
    pub fn render(&mut self, voice: &mut Voice, out: &mut [f32]) {
        if self.gate_close_pending {
            voice.note_off();
            self.gate_close_pending = false;
        }
        if self.trigger_pending {
            self.trigger(voice, self.position);
            self.trigger_pending = false;
        }
        if !self.running {
            voice.render(out);
            return;
        }

        let mut offset = 0;
        while offset < out.len() {
            let left = out.len() - offset;
            // Render up to the boundary and no further. The `max(1.0)` keeps a sub-sample remainder
            // from stalling the loop: a block never shrinks to zero, it just overshoots by under a sample.
            let take = left.min(self.clock.samples_until_next_step().max(1.0) as usize);
            voice.render(&mut out[offset..offset + take]);
            offset += take;

            let crossed = self.clock.advance_by(take);
            if crossed > 0 {
                self.position = (self.position + crossed) % STEPS_PER_PATTERN;
                self.trigger(voice, self.position);
            }
        }
    }
}
