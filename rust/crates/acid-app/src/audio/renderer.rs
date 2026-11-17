//! The audio callback's brain: one block, every parameter, no allocation.
//!
//! `apply` is the only place a parameter reaches the engine, so the mapping is unit-testable without a
//! sound card. `AudioRenderer` reads the shared state once per block, swaps the sequencer's pattern only
//! when the UI actually published a new one, and copies the mono result to every channel.
//!
//! **Known exception to "no allocation here":** retiring a pattern drops the previous `Arc<Pattern>` on the audio thread
//! (`self.pattern = latest`). It normally frees nothing, because the UI's bank holds the same pattern, but it is a freed
//! `Arc` in principle and the allocator is not async-signal-safe. The clean fix is to hand the retired `Arc` back to the
//! UI to drop, or to stop holding an `Arc` here at all and copy the sixteen steps; both are larger changes than the risk
//! warrants today, so the exception is named rather than hidden.

use std::sync::Arc;

use acid_core::engine::Engine;
use acid_core::pattern::Pattern;

use crate::state::{Parameters, SharedState};

/// Push a parameter snapshot into the engine's knobs and transport.
pub fn apply(params: &Parameters, engine: &mut Engine) {
    engine.set_waveform(params.waveform);
    engine.set_tuning(params.tuning);
    engine.set_cutoff(params.cutoff);
    engine.set_resonance(params.resonance);
    engine.set_env_mod(params.env_mod);
    engine.set_decay(params.decay);
    engine.set_accent(params.accent);
    engine.set_volume(params.volume);
    engine.set_bpm(params.tempo_bpm);
    engine.set_swing(params.swing);
}

impl AudioRenderer {
    /// Take the transport where the rack put it, and tell the engine what that means.
    ///
    /// The three states are not three engine calls in every direction: a resume out of a pause keeps the step, a start out of
    /// a stop returns to the bar, and both silences are the same call to the engine. Which one a change is depends on where it
    /// came from, and that is the one thing this struct knows and the engine does not.
    fn settle_transport(&mut self, transport: crate::device::Transport) {
        if transport == self.transport {
            return;
        }
        match transport {
            crate::device::Transport::Playing => match self.transport {
                // Out of a pause: carry on from the step the pause kept.
                crate::device::Transport::Paused => self.engine.resume(),
                // Out of a stop (or asked to play while already playing, which cannot reach here): the bar again.
                crate::device::Transport::Stopped | crate::device::Transport::Playing => {
                    self.engine.start()
                }
            },
            crate::device::Transport::Paused => self.engine.pause(),
            crate::device::Transport::Stopped => self.engine.stop(),
        }
        self.transport = transport;
    }
}

/// Owns the engine for the audio thread's lifetime and keeps it in step with the UI.
pub struct AudioRenderer {
    engine: Engine,
    /// The transport the engine was last told about. The engine cannot tell a stop from a pause — both silence it and keep its
    /// step — so what a *change* means is decided here, where the previous state is still known.
    transport: crate::device::Transport,
    /// What the engine was last told about the two performers, so a block that changes nothing costs nothing.
    reversed: bool,
    transpose: i32,
    state: Arc<SharedState>,
    /// Interleaving scratch, sized once at open so a block never allocates.
    mono: Vec<f32>,
    /// The pattern last handed to the engine, so an unchanged one costs a refcount compare.
    pattern: Arc<Pattern>,
    channels: usize,
    max_block: usize,
    /// Whether a MIDI note was held at the last block, so only changes trigger a note.
    midi_gate_on: bool,
    /// A start this device should make on the next block: `(step, samples of silence first)`. A device joining a running
    /// rack waits for the bar line, which can fall inside a block, and begins its first step on that sample.
    pending_start: Option<(usize, usize)>,
}

impl AudioRenderer {
    pub fn new(
        sample_rate: f32,
        channels: usize,
        state: Arc<SharedState>,
        max_block: usize,
    ) -> Self {
        let pattern = state.pattern();
        let mut engine = Engine::new(sample_rate);
        engine.set_pattern(Arc::clone(&pattern));
        Self {
            engine,
            transport: crate::device::Transport::Stopped,
            reversed: false,
            transpose: 0,
            state,
            mono: vec![0.0; max_block.max(1)],
            pattern,
            channels: channels.max(1),
            max_block: max_block.max(1),
            midi_gate_on: false,
            pending_start: None,
        }
    }

    /// Start this device's clock on `step` at the next block, instead of on the downbeat.
    ///
    /// This is the **paused** rack's path: the mixer hands it the step the rack is holding, so the next play starts it with
    /// everyone else rather than at zero. A device joining a rack that is *playing* waits for the bar line instead — see
    /// [`AudioRenderer::start_later`]. Harmless while stopped: a start that lands before the transport begins is dropped.
    pub fn seek(&mut self, step: usize) {
        self.start_later(step, 0);
    }

    /// Start on `step` after `silence` samples of the next block: the bar line's own sample, not the block's.
    pub fn start_later(&mut self, step: usize, silence: usize) {
        self.pending_start = Some((step, silence));
    }

    /// Where step 0 began in the last block, in samples from its start. This device's bar line is the rack's, so it is what
    /// the mixer releases every device waiting to join on.
    pub fn bar_line(&self) -> Option<usize> {
        self.engine.bar_line()
    }

    /// Whether a start is still waiting for its block. A start issued for a device whose own transport has just stopped is
    /// put back rather than dropped, and the mixer re-marks that slot `waiting` when it sees this.
    pub fn has_pending_start(&self) -> bool {
        self.pending_start.is_some()
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Render one mono block: parameters first, then the pattern if it changed, then the engine.
    pub fn render_mono(&mut self, out: &mut [f32]) {
        let params = self.state.snapshot();
        apply(&params, &mut self.engine);
        self.settle_transport(params.transport);
        // The performers: a direction the sequencer applies at the bar line, and a transpose the voice hears at once.
        if params.reversed != self.reversed {
            self.engine.set_reverse(params.reversed);
            self.reversed = params.reversed;
        }
        if params.transpose != self.transpose {
            self.engine.set_transpose(params.transpose);
            self.transpose = params.transpose;
        }

        // A device that joined a running rack starts where it was told to. After `apply`, because `apply` is what starts an
        // engine whose transport has just begun.
        if let Some((step, silence)) = self.pending_start.take() {
            if params.transport == crate::device::Transport::Stopped {
                // A release that arrives as the rack stops is not dropped: it goes back, so the mixer can keep the slot
                // waiting and it joins on the next bar line. Dropped, the device would play from step 0 under a rack that
                // resumed mid-bar — the out-of-phase bug this whole path exists to prevent.
                self.pending_start = Some((step, silence));
            } else {
                // A seek arms the clock, which is what a joining device wants on a *running* rack and exactly what it must
                // not do on a paused one: it would run while the rack — and its own mirror — said paused. Take the step,
                // then put the silence back.
                self.engine.start_later(step, silence);
                if params.transport != crate::device::Transport::Playing {
                    self.engine.pause();
                }
            }
        }

        // MIDI: a rising gate starts a note, a falling one releases it. Only edges act, so a held note is
        // not retriggered every block. The sequencer keeps running underneath — this plays over the pattern.
        // `Acquire` pairs with the `Release` store on the gate, so the frequency, accent and velocity written before it
        // are visible here. A `Relaxed` load could start the previous note's pitch for one block.
        let gate = self
            .state
            .midi_gate
            .load(std::sync::atomic::Ordering::Acquire);
        if gate != self.midi_gate_on {
            if gate {
                // The audition plays at the velocity the key was struck, so what the player hears is what the step will
                // carry — a soft touch used to sound at full level and then write itself 12 dB quieter.
                let velocity = self
                    .state
                    .midi_velocity
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .clamp(0, 127) as u8;
                self.engine.note_at(
                    self.state.midi_hz.get(),
                    self.state
                        .midi_accent
                        .load(std::sync::atomic::Ordering::Relaxed),
                    velocity,
                );
            } else {
                self.engine.note_off();
            }
            self.midi_gate_on = gate;
        }

        let latest = self.state.pattern();
        if !Arc::ptr_eq(&latest, &self.pattern) {
            self.pattern = latest;
            self.engine.set_pattern(Arc::clone(&self.pattern));
        }

        let mut offset = 0;
        while offset < out.len() {
            let take = (out.len() - offset).min(self.max_block);
            self.engine.render(&mut out[offset..offset + take]);
            offset += take;
        }
        // Publish the step so the panel's LEDs can chase the audio rather than the wall clock.
        self.state
            .position
            .store(self.engine.position(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Render one interleaved block, duplicating each mono frame to every channel.
    pub fn render_into(&mut self, out: &mut [f32]) {
        let frames = out.len() / self.channels;
        if frames == 0 {
            return;
        }
        // `mono` is borrowed out of `self` so the loop can call `&mut self` methods while it writes here.
        let mut mono = std::mem::take(&mut self.mono);
        let mut done = 0;
        while done < frames {
            let take = (frames - done).min(self.max_block);
            let buf = &mut mono[..take];
            self.render_mono(buf);
            for (frame, sample) in buf.iter().enumerate() {
                let base = (done + frame) * self.channels;
                out[base..base + self.channels].fill(*sample);
            }
            done += take;
        }
        self.mono = mono;
    }
}
