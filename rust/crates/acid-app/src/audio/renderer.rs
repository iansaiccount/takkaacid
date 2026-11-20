//! The audio callback's brain: one block, every parameter, no allocation.
//!
//! `apply` is the only place a parameter reaches the 303's engine, so the mapping is unit-testable without a sound card.
//! `AudioRenderer` owns one [`Machine`] — the 303's engine or the drum machine, chosen once from the device's kind — reads
//! the shared state once per block, swaps the sequencer's pattern only when the UI actually published a new one, and copies
//! the mono result to every channel.
//!
//! **Known exception to "no allocation here":** retiring a pattern drops the previous `Arc<Pattern>` on the audio thread
//! (`self.pattern = latest`). It normally frees nothing, because the UI's bank holds the same pattern, but it is a freed
//! `Arc` in principle and the allocator is not async-signal-safe. The clean fix is to hand the retired `Arc` back to the
//! UI to drop, or to stop holding an `Arc` here at all and copy the sixteen steps; both are larger changes than the risk
//! warrants today, so the exception is named rather than hidden.
//!
//! A kit swapped mid-block is the same shape and a worse case: the panel publishes a kit on every frame a hand is on a dial,
//! so the retired `Arc<Kit>` really is freed on the audio thread (`Machine::Drums`'s `kit`), and `tests/no_alloc.rs` cannot
//! see it — that counter counts allocations, and a free is not one.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use acid_core::drums::{Drums, Kit};
use acid_core::engine::Engine;
use acid_core::param::{self, Smooth};
use acid_core::pattern::Pattern;

use crate::device::DeviceKind;
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

/// Push what a drum machine takes from the same snapshot.
///
/// The seven 303 knobs and the master volume are deliberately absent. On a drum device the seven are the selected voice's and
/// live in the kit — task 4's ruling, and the face writes them there — so pushing them into the machine would be a second
/// source of truth for one dial. The master volume has no engine to sit in (a drum machine has none), so it is the renderer's
/// gain: [`apply_volume`], below. What is left is the rack's and the device's — tempo, swing, the transport, the direction and
/// the device's own transpose — and those a drum machine takes exactly as the 303 does.
pub fn apply_drums(params: &Parameters, drums: &mut Drums) {
    drums.set_bpm(params.tempo_bpm);
    drums.set_swing(params.swing);
}

/// The 303's master VOLUME knob, applied to a drum block.
///
/// The same gain, the same ramp and the same exact silence at zero that `Engine::render` gives a 303, because the knob is drawn
/// in the top strip of **every** face. Two differences, named rather than glossed:
///
/// - **Where it sits in the chain.** An engine has its own limiter before its volume; a drum machine has none — the spec puts
///   the ceiling in the mixer's — so this gain comes after the sixteen voices are summed and before the rack's ceiling. A drum
///   device quietened here is a drum device the limiter has less to hold.
/// - **No idle snap.** `Engine::set_volume` jumps its smoother when nothing is playing, so a knob moved on a stopped 303 takes
///   effect at once. `Drums` answers no such question, so a knob moved on a stopped drum device ramps over its 10 ms — which a
///   `HIT` pressed straight after it hears as a fade-in. Closing that gap wants a "nothing is sounding" answer from the
///   machine, which is a wishlist line rather than this task's.
///
/// The ramp itself is `param::VOLUME_MS`, the engine's own constant, so the two cannot drift apart into a click.
fn apply_volume(volume: &mut Smooth, knob: f32, out: &mut [f32]) {
    let target = param::master_volume(knob);
    for sample in out.iter_mut() {
        // Volume 0 is a mute, so it must be exact silence rather than an asymptote.
        let gain = if target == 0.0 {
            volume.snap(0.0);
            volume.value()
        } else {
            volume.process(target)
        };
        *sample *= gain;
    }
}

/// The machine one device runs.
///
/// A device is one machine for its lifetime — the kind is chosen when it is made and never changed — so this is decided once,
/// at open, from the state's own `kind`. Every decision that differs between the two is written out below, twice, and a third
/// kind would fail to compile at each of them rather than silently render the bass.
///
/// **Both arms are boxed** so a `Machine` is a pointer or two rather than a kilobyte and a half: the drum machine is sixteen
/// voices and a kit, and an enum is as large as its largest arm, so leaving it inline would put all of that inside every value
/// of a type whose only job is to say which machine this is. Each is built once, at open, where the mixer already allocates the
/// slot that holds it.
enum Machine {
    Acid(Box<Engine>),
    /// The drum machine, the kit last handed to it, and the master gain.
    ///
    /// `kit` is the last `Arc` given to `drums`: an unchanged kit costs one refcount compare per block instead of a `set_kit`
    /// that would drop an `Arc` on the audio thread every block. `Drums` holds its own clone, so a kit the panel swaps out
    /// mid-block stays alive for the block that is still playing it.
    Drums {
        drums: Box<Drums>,
        kit: Arc<Kit>,
        volume: Smooth,
    },
}

impl Machine {
    /// The machine this device runs, at `sample_rate`, on `pattern`.
    ///
    /// The kind comes from the state and nowhere else, so this function cannot be handed a machine that disagrees with the
    /// device it is built for.
    fn of(sample_rate: f32, state: &SharedState, pattern: Arc<Pattern>) -> Self {
        match state.kind {
            DeviceKind::Acid => {
                let mut engine = Engine::new(sample_rate);
                engine.set_pattern(pattern);
                Machine::Acid(Box::new(engine))
            }
            DeviceKind::Drums => {
                let kit = state.kit();
                let mut drums = Drums::new(sample_rate, Arc::clone(&kit));
                drums.set_pattern(pattern);
                // The gain starts where the knob is, so a device opened with the volume down makes no sound at its first
                // block rather than ramping down into silence for ten milliseconds.
                let volume = Smooth::new(
                    param::master_volume(state.volume.get()),
                    param::VOLUME_MS,
                    sample_rate,
                );
                Machine::Drums {
                    drums: Box::new(drums),
                    kit,
                    volume,
                }
            }
        }
    }

    fn set_pattern(&mut self, pattern: Arc<Pattern>) {
        match self {
            Machine::Acid(engine) => engine.set_pattern(pattern),
            Machine::Drums { drums, .. } => drums.set_pattern(pattern),
        }
    }

    /// A direction the clock applies at the bar line.
    fn set_reverse(&mut self, reversed: bool) {
        match self {
            Machine::Acid(engine) => engine.set_reverse(reversed),
            Machine::Drums { drums, .. } => drums.set_reverse(reversed),
        }
    }

    /// The device's performance transpose, heard at once.
    fn set_transpose(&mut self, semitones: i32) {
        match self {
            Machine::Acid(engine) => engine.set_transpose(semitones),
            Machine::Drums { drums, .. } => drums.set_transpose(semitones),
        }
    }

    /// The bar again, from step 0.
    fn start(&mut self) {
        match self {
            Machine::Acid(engine) => engine.start(),
            Machine::Drums { drums, .. } => drums.start(),
        }
    }

    /// On `step` after `silence` samples of the next block.
    fn start_later(&mut self, step: usize, silence: usize) {
        match self {
            Machine::Acid(engine) => engine.start_later(step, silence),
            Machine::Drums { drums, .. } => drums.start_later(step, silence),
        }
    }

    /// Silence from here, and the step kept for [`Machine::resume`].
    fn pause(&mut self) {
        match self {
            Machine::Acid(engine) => engine.pause(),
            Machine::Drums { drums, .. } => drums.pause(),
        }
    }

    /// Carry on from the step a pause kept.
    fn resume(&mut self) {
        match self {
            Machine::Acid(engine) => engine.resume(),
            Machine::Drums { drums, .. } => drums.resume(),
        }
    }

    /// Silence now, back to the bar next time.
    fn stop(&mut self) {
        match self {
            Machine::Acid(engine) => engine.stop(),
            Machine::Drums { drums, .. } => drums.stop(),
        }
    }

    fn position(&self) -> usize {
        match self {
            Machine::Acid(engine) => engine.position(),
            Machine::Drums { drums, .. } => drums.position(),
        }
    }

    fn bar_line(&self) -> Option<usize> {
        match self {
            Machine::Acid(engine) => engine.bar_line(),
            Machine::Drums { drums, .. } => drums.bar_line(),
        }
    }
}

/// Owns a device's machine for the audio thread's lifetime and keeps it in step with the UI.
pub struct AudioRenderer {
    /// The device's machine, chosen at open from [`SharedState::kind`].
    machine: Machine,
    /// The transport the machine was last told about. The machine cannot tell a stop from a pause — both silence it and keep
    /// its step — so what a *change* means is decided here, where the previous state is still known.
    transport: crate::device::Transport,
    /// What the machine was last told about the two performers, so a block that changes nothing costs nothing.
    reversed: bool,
    transpose: i32,
    state: Arc<SharedState>,
    /// Interleaving scratch, sized once at open so a block never allocates.
    mono: Vec<f32>,
    /// The pattern last handed to the machine, so an unchanged one costs a refcount compare.
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
        let machine = Machine::of(sample_rate, &state, Arc::clone(&pattern));
        Self {
            machine,
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
        self.machine.bar_line()
    }

    /// Whether a start is still waiting for its block. A start issued for a device whose own transport has just stopped is
    /// put back rather than dropped, and the mixer re-marks that slot `waiting` when it sees this.
    pub fn has_pending_start(&self) -> bool {
        self.pending_start.is_some()
    }

    /// Render one mono block: parameters first, then the pattern if it changed, then the machine.
    pub fn render_mono(&mut self, out: &mut [f32]) {
        let params = self.state.snapshot();
        self.push(&params);
        self.settle_transport(params.transport);
        // The performers: a direction the sequencer applies at the bar line, and a transpose the voice hears at once.
        if params.reversed != self.reversed {
            self.machine.set_reverse(params.reversed);
            self.reversed = params.reversed;
        }
        if params.transpose != self.transpose {
            self.machine.set_transpose(params.transpose);
            self.transpose = params.transpose;
        }
        // The drum machine's own wiring, once a block: the kit the panel swapped in, the mask it holds, and a `HIT` waiting.
        self.service_drums();

        // A device that joined a running rack starts where it was told to. After the parameters, because those are what start
        // a machine whose transport has just begun.
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
                self.machine.start_later(step, silence);
                if params.transport != crate::device::Transport::Playing {
                    self.machine.pause();
                }
            }
        }

        self.service_note();

        let latest = self.state.pattern();
        if !Arc::ptr_eq(&latest, &self.pattern) {
            self.pattern = latest;
            self.machine.set_pattern(Arc::clone(&self.pattern));
        }

        let mut offset = 0;
        while offset < out.len() {
            let take = (out.len() - offset).min(self.max_block);
            let chunk = &mut out[offset..offset + take];
            match &mut self.machine {
                Machine::Acid(engine) => engine.render(chunk),
                Machine::Drums { drums, volume, .. } => {
                    drums.render(chunk);
                    apply_volume(volume, params.volume, chunk);
                }
            }
            offset += take;
        }
        // Publish the step so the panel's LEDs can chase the audio rather than the wall clock.
        self.state
            .position
            .store(self.machine.position(), Ordering::Relaxed);
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

    /// Take the transport where the rack put it, and tell the machine what that means.
    ///
    /// The three states are not three calls in every direction: a resume out of a pause keeps the step, a start out of a
    /// stop returns to the bar, and both silences are the same call to the machine. Which one a change is depends on where it
    /// came from, and that is the one thing this struct knows and the machine does not.
    fn settle_transport(&mut self, transport: crate::device::Transport) {
        if transport == self.transport {
            return;
        }
        match transport {
            crate::device::Transport::Playing => match self.transport {
                // Out of a pause: carry on from the step the pause kept.
                crate::device::Transport::Paused => self.machine.resume(),
                // Out of a stop (or asked to play while already playing, which cannot reach here): the bar again.
                crate::device::Transport::Stopped | crate::device::Transport::Playing => {
                    self.machine.start()
                }
            },
            crate::device::Transport::Paused => self.machine.pause(),
            crate::device::Transport::Stopped => self.machine.stop(),
        }
        self.transport = transport;
    }

    /// Hand the parameters a machine takes to that machine.
    fn push(&mut self, params: &Parameters) {
        match &mut self.machine {
            Machine::Acid(engine) => apply(params, engine),
            Machine::Drums { drums, .. } => apply_drums(params, drums),
        }
    }

    /// The drum machine's own state, read once per block: the kit the panel swapped in, the mask it holds, and any `HIT`
    /// waiting to be heard.
    ///
    /// One `Arc` compare, one `u32` load and one swap — nothing that allocates, and nothing held across the block: `Drums`
    /// keeps its own clone of the kit, which is what keeps a kit swapped out mid-block alive. A 303 device pays nothing.
    fn service_drums(&mut self) {
        // Written as an exhaustive match rather than a `let … else`, like every other decision on a machine here: a third
        // kind has to say what its per-block wiring is instead of falling into nothing.
        let (drums, kit) = match &mut self.machine {
            Machine::Acid(_) => return,
            Machine::Drums { drums, kit, .. } => (drums, kit),
        };
        let latest = self.state.kit();
        if !Arc::ptr_eq(&latest, kit) {
            drums.set_kit(Arc::clone(&latest));
            *kit = latest;
        }
        // The live mask is the state's, not the kit's flags: the kit's are the starting mask a load seeds
        // (`SharedState::adopt_kit`), and the panel's shift-clicks write the state's. Sixteen voices fit a `u16`.
        drums.set_mute_mask(self.state.drum_mute.load(Ordering::Relaxed) as u16);
        if let Some(drum) = self.state.take_audition() {
            drums.audition(drum);
        }
    }

    /// MIDI: a rising gate starts a note, a falling one releases it. Only edges act, so a held note is not retriggered every
    /// block. The sequencer keeps running underneath — this plays over the pattern.
    ///
    /// A drum device's notes are a later round (the spec's non-goal: no MIDI pads this phase), and the gate is the 303's, so
    /// a drum device takes no note at all — the early return here is where that decision is written.
    fn service_note(&mut self) {
        // Exhaustive, as `service_drums` is: a third kind has to answer for the note gate too.
        let engine = match &mut self.machine {
            Machine::Acid(engine) => engine,
            Machine::Drums { .. } => return,
        };
        // `Acquire` pairs with the `Release` store on the gate, so the frequency, accent and velocity written before it are
        // visible here. A `Relaxed` load could start the previous note's pitch for one block.
        let gate = self.state.midi_gate.load(Ordering::Acquire);
        if gate == self.midi_gate_on {
            return;
        }
        if gate {
            // The audition plays at the velocity the key was struck, so what the player hears is what the step will carry —
            // a soft touch used to sound at full level and then write itself 12 dB quieter.
            let velocity = self
                .state
                .midi_velocity
                .load(Ordering::Relaxed)
                .clamp(0, 127) as u8;
            engine.note_at(
                self.state.midi_hz.get(),
                self.state.midi_accent.load(Ordering::Relaxed),
                velocity,
            );
        } else {
            engine.note_off();
        }
        self.midi_gate_on = gate;
    }
}
