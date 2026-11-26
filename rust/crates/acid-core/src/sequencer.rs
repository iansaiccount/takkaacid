//! The sequencer: it owns the step clock and turns pattern steps into performer events.
//!
//! `render` splits every block at the next step boundary, so a note starts on the exact sample the clock
//! names whatever block size the audio device asks for. `Sequencer`'s own `start` and `stop` take no performer — they leave
//! a pending action that the next `render` applies, which keeps *this* half of the transport on the audio thread. A machine
//! that has to silence itself (the drum machine's audition has no transport to close its gate) does it in its own `stop`, and
//! that call belongs on the thread that renders it.
//!
//! What a step *sounds like* is not the clock's question. The bar, `REVERSE`, swing, pause, the bar line and the
//! sample-accurate start are one implementation for every machine in the rack; "make this step sound" is a `Performer`,
//! and the 303's voice and the drum machine are two of them.

use std::sync::Arc;

use crate::clock::Clock;
use crate::pattern::{Pattern, Step};

/// What a machine does when the clock hands it a step.
///
/// The clock renders *through* the performer: every path in [`Sequencer::render`] — the wait before a late start, the tail
/// after a stop, the block loop — goes through these four methods, which is why `render` is one of them. A machine cannot
/// keep a second clock and drift, because it is never asked for samples the clock did not place.
pub trait Performer: Sized {
    /// Make `step` sound. A step is both worlds' type, so each machine reads its own half: the 303 reads the note, the
    /// drums read the masks.
    fn step(&mut self, step: &Step);

    /// Close the gate. A stop, or the end of a note: the tail folds away through the release rather than being cut.
    fn release(&mut self);

    /// Render the next `out.len()` samples.
    fn render(&mut self, out: &mut [f32]);

    /// Clear the per-run state a start must not inherit — the 303's slide history. Called on the step a start begins with,
    /// never mid-run.
    fn arm(&mut self);
}

/// Which step of the pattern sounds at clock `position`.
///
/// **The one place this arithmetic exists.** [`Sequencer::deliver`] plays through it and a panel draws its playhead with it, so
/// a marker cannot point at a step that is not the one sounding: `REVERSE` reads the bar from its other end while the clock keeps
/// walking forward, and a playhead left on the clock would be on the one step that is silent.
///
/// `position` wraps, so a caller may hand a long-running count or a value read from a cell a block behind the audio.
pub fn sounding_step(position: usize, reversed: bool, steps: usize) -> usize {
    // A pattern always has steps, so this is a guard against a caller's arithmetic rather than a case the app reaches.
    let steps = steps.max(1);
    let position = position % steps;
    if reversed {
        steps - 1 - position
    } else {
        position
    }
}

/// Plays a `Pattern` through a [`Performer`], one step at a time.
pub struct Sequencer {
    clock: Clock,
    pattern: Arc<Pattern>,
    /// Current step, always `0..pattern.len()`.
    position: usize,
    /// Read the pattern from its other end. A direction, not a different pattern: nothing is stored, copied or saved. It only
    /// ever changes on a bar line — see [`Sequencer::set_reverse`].
    reverse: bool,
    /// A direction asked for while the bar is running. It waits for the wrap: flipping under the line would put the two halves
    /// of a bar out of phase, and a bar is the unit the whole instrument agrees on.
    reverse_pending: Option<bool>,
    running: bool,
    /// `start` triggered step 0 but had no voice: the next `render` sends it.
    trigger_pending: bool,
    /// `stop` closed the transport but had no voice: the next `render` closes the gate.
    gate_close_pending: bool,
    /// Where step 0 began inside the last `render`, in samples from its start. The rack's bar line is this device's — every
    /// playing device is in step with every other — and a device waiting to join is released on it.
    bar_line: Option<usize>,
    /// A start that waits out a number of samples first: `(step, samples still to wait)`. Set by
    /// [`Sequencer::start_later`], and the only way to begin a step part-way through a block.
    pending_later: Option<(usize, usize)>,
}

impl Sequencer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            clock: Clock::new(sample_rate),
            pattern: Arc::new(Pattern::empty()),
            position: 0,
            reverse: false,
            reverse_pending: None,
            running: false,
            trigger_pending: false,
            gate_close_pending: false,
            bar_line: None,
            pending_later: None,
        }
    }

    /// Swap the playing pattern.
    ///
    /// The step **number** is kept and the grid comes from the new pattern: a swap into a longer bar carries on counting steps,
    /// so the sequence is heard from the step it was on, with finer steps around it. The position wraps into the new bar, so a
    /// swap into a shorter one begins at a step that exists rather than one that has gone.
    ///
    /// **The alternative, and why it lost.** Keeping the *fraction of the bar* instead (4 of 16 would become 8 of 32) holds the
    /// bar's absolute position and moves every later step earlier. The step number won because `GRID` subdivides the bar the
    /// player is already counting, and a step is what the face shows and the hand touches. Cost if wrong: a grid change lands
    /// its bar line up to one bar away from where absolute-position arithmetic would put it.
    /// `tests/engine_transport.rs::a_swap_into_a_longer_bar_keeps_the_step_it_was_on` pins the choice.
    pub fn set_pattern(&mut self, pattern: Arc<Pattern>) {
        // The grid is the pattern's and the clock divides the bar by it: a grid change rescales the step in flight, as a tempo
        // change does, so the current step is never restarted. The wrap is the shorter bar's safety.
        self.clock.set_steps_per_bar(pattern.grid.steps_per_bar());
        self.position %= pattern.len();
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
        self.start_at(0);
    }

    /// Start on `step` rather than on the downbeat.
    ///
    /// A device joining a **paused** rack lands on the step the resume will use, so the next play starts it with everyone
    /// else. One joining a **playing** rack is held silent until the bar line and starts with [`Sequencer::start_later`]
    /// instead: started here, it would play from the rack's current step, a step out of phase until the bar wrapped. The
    /// clock is put back on a boundary and the step it lands on sounds, so a seek is never a silent restart. `step` wraps:
    /// callers may hand a long-running count, not an index.
    pub fn start_at(&mut self, step: usize) {
        self.position = step % self.pattern.len();
        self.arm();
    }

    /// Start on `step` once `silence` samples of the current block have gone by.
    ///
    /// This is how a device joins a rack that is already running. It has to wait for the rack's bar line, and a bar line can
    /// fall inside a block: waiting for the whole block would begin its first step a few milliseconds late against the rest of
    /// the rack, and starting at the top of the block would begin it early. `silence` is the bar line's own offset.
    ///
    /// It overrides a start already issued this block — `running` goes back down and the pending trigger with it — because the
    /// caller has decided where inside the block the first step belongs.
    pub fn start_later(&mut self, step: usize, silence: usize) {
        if silence == 0 {
            self.start_at(step);
            return;
        }
        self.running = false;
        self.trigger_pending = false;
        self.gate_close_pending = false;
        self.pending_later = Some((step % self.pattern.len(), silence));
    }

    /// Where step 0 began in the last [`Sequencer::render`], in samples from its start, if it began in that block at all.
    pub fn bar_line(&self) -> Option<usize> {
        self.bar_line
    }

    /// Render up to the sample a delayed start was told to begin on, and arm it there. Answers the offset the rest of the
    /// block begins at: zero when no start is pending, and `out.len()` when the start is still to come in a later block.
    ///
    /// Nothing sounds and the clock does not move while it waits. The performer still renders, so a release tail from a
    /// previous run fades rather than being cut.
    fn wait_out_start<P: Performer>(&mut self, performer: &mut P, out: &mut [f32]) -> usize {
        let Some((step, left)) = self.pending_later else {
            return 0;
        };
        let waiting = left.min(out.len());
        performer.render(&mut out[..waiting]);
        if waiting < left {
            self.pending_later = Some((step, left - waiting));
            return waiting;
        }
        self.pending_later = None;
        self.start_at(step);
        waiting
    }

    /// Arm the clock and the gate for a step to sound: **the one place** that says what starting a step means.
    ///
    /// The clock kept counting through a silence, so it is put back on a boundary: otherwise the first step after a start is
    /// whatever fraction of a step the previous run happened to leave, and with swing it can even be the short half of the
    /// pair.
    ///
    /// One thing a start means is **not** here: the performer's own `arm`. That is called where the pending step fires, because
    /// a machine is handed the step it arms for.
    fn arm(&mut self) {
        self.clock.restart();
        self.running = true;
        self.trigger_pending = true;
        self.gate_close_pending = false;
    }

    /// Stop the transport. The gate closes on the next render, which then fades the release tail out
    /// rather than cutting it.
    ///
    /// A **stop** and a **pause** are the same silence on this side of the seam (see `Engine`), so this says nothing about a
    /// direction that is waiting for the bar: `Engine::stop` follows it with [`Sequencer::settle_direction`], and a pause does
    /// not, because the bar the direction was asked in is still coming when the line resumes.
    pub fn stop(&mut self) {
        // A start that was waiting out part of a block goes with the stop: `start_later` has already put `running` down, so
        // the guard below would leave it to fire into the next render.
        self.pending_later = None;
        if self.running {
            self.running = false;
            self.trigger_pending = false;
            self.gate_close_pending = true;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Take a direction that was waiting for the bar, without waiting for it.
    ///
    /// This is a **stop**'s move, and only a stop's: the next start after a stop returns to the top of the bar, so a direction
    /// asked for mid-bar would otherwise be lost, and the new bar would run the old way under the key that asked for the new
    /// one. A **pause** must not take it: the bar it was asked in is still going to arrive when the line resumes, and both
    /// halves of that bar belong to the direction it began in.
    pub fn settle_direction(&mut self) {
        if let Some(next) = self.reverse_pending.take() {
            self.reverse = next;
        }
    }

    /// Ask for a direction. A stopped transport takes it at once; a running one takes it on the next bar line, so the change
    /// lands where every other device's bar also lands. The step the clock is on does not move either way.
    pub fn set_reverse(&mut self, reverse: bool) {
        if self.running {
            self.reverse_pending = Some(reverse);
        } else {
            self.reverse = reverse;
            self.reverse_pending = None;
        }
    }

    /// Whether a direction is waiting for the bar.
    pub fn reverse_is_pending(&self) -> bool {
        self.reverse_pending.is_some()
    }

    pub fn is_reverse(&self) -> bool {
        self.reverse
    }

    /// Carry on from the step a silence kept, arming the clock and the gate like any other start.
    ///
    /// The position is **not** moved — that is the whole of the difference from [`Sequencer::start_at`], which is handed the
    /// step to start on. The sequencer cannot tell a stop from a pause, because on this side of the seam they are the same
    /// thing: silent, with the step kept. Which one the player asked for is [`acid_core::engine::Engine`]'s question, and it
    /// answers it by calling `resume` or `start` — never by reading a flag from here.
    pub fn resume(&mut self) {
        self.arm();
    }

    /// The step being played, `0..pattern.len()`.
    pub fn position(&self) -> usize {
        self.position
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Hand one step to the performer.
    ///
    /// Backwards reads the pattern from its other end while the clock keeps walking forward: the position, the bar and the
    /// step count all still mean what they meant, and only the step that sounds is mirrored. What makes a step sound is the
    /// performer's: a rest, and a drum step striking no voice, are both silence there — the clock does not decide it.
    fn deliver<P: Performer>(&mut self, performer: &mut P, position: usize) {
        let step = self.pattern.steps[sounding_step(position, self.reverse, self.pattern.len())];
        performer.step(&step);
    }

    /// Render the next `out.len()` samples, starting whatever step is due inside that block.
    ///
    /// When stopped, this still renders the performer so a stop fades the release tail instead of cutting it.
    pub fn render<P: Performer>(&mut self, performer: &mut P, out: &mut [f32]) {
        self.bar_line = None;
        let head = self.wait_out_start(performer, out);
        let out = &mut out[head..];
        if self.gate_close_pending {
            performer.release();
            self.gate_close_pending = false;
        }
        if self.trigger_pending {
            self.trigger_pending = false;
            // The performer is armed on the step a start begins with: what a machine keeps for a run — the 303's slide
            // history — is cleared here, once, rather than in every way of starting a line.
            performer.arm();
            self.deliver(performer, self.position);
            // A start on the downbeat *is* a bar line, and the rack goes by what its devices announce: this is the one that
            // releases a device waiting to join after a stop.
            if self.position == 0 {
                self.bar_line = Some(head);
            }
        }
        if !self.running {
            performer.render(out);
            return;
        }

        let mut offset = 0;
        while offset < out.len() {
            let left = out.len() - offset;
            // Render up to the boundary and no further. The `max(1.0)` keeps a sub-sample remainder
            // from stalling the loop: a block never shrinks to zero, it just overshoots by under a sample.
            let take = left.min(self.clock.samples_until_next_step().max(1.0) as usize);
            performer.render(&mut out[offset..offset + take]);
            offset += take;

            let crossed = self.clock.advance_by(take);
            if crossed > 0 {
                let before = self.position;
                self.position = (self.position + crossed) % self.pattern.len();
                // The bar line: where a direction may change, and where a device waiting to join is let in. Both halves of
                // the old bar played in the direction that bar began in.
                if self.position == 0 {
                    self.bar_line = Some(head + offset);
                }
                if (self.position < before || (self.position == 0 && crossed > 0))
                    && let Some(next) = self.reverse_pending.take()
                {
                    self.reverse = next;
                }
                self.deliver(performer, self.position);
            }
        }
    }
}
