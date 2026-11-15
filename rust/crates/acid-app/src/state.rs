//! State shared between the UI thread and the audio thread.
//!
//! Everything here is either an atomic or a short-lived mutex the audio thread never touches, so the
//! audio callback can never be blocked by the user moving a knob.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::notes::NoteQueue;
use acid_core::engine::DEFAULT_TUNING_KNOB;
use acid_core::oscillator::Waveform;
use acid_core::pattern::{Pattern, STEPS_PER_PATTERN, Step};
use acid_core::voice::{
    DEFAULT_ACCENT_KNOB, DEFAULT_CUTOFF_KNOB, DEFAULT_DECAY_KNOB, DEFAULT_ENV_MOD_KNOB,
    DEFAULT_RESONANCE_KNOB,
};
use arc_swap::ArcSwap;
/// Tempo used until the sequencer owns one (Phase 05's `timing` module takes this over).
pub const DEFAULT_BPM: f32 = 130.0;

/// Default master volume knob position.
pub const DEFAULT_VOLUME: f32 = 0.8;

/// An `f32` behind an atomic, for knobs read by the audio thread once per block.
#[derive(Debug, Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self(AtomicU32::new(value.to_bits()))
    }

    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn set(&self, value: f32) {
        self.0.store(value.to_bits(), Ordering::Relaxed);
    }
}

/// Controller take-over state for one learnable control: the last value a controller sent, and whether it owns the knob.
///
/// Two atomics rather than one packed word: the flag would have to hide inside the value's bits, and that trick costs
/// more in reading than the four bytes it saves.
#[derive(Debug)]
pub struct Pickup {
    /// The last value the controller sent, as f32 bits; `u32::MAX` until it sends one, because `0.0` is a real value.
    last: AtomicU32,
    /// True once the controller has reached the knob and may turn it.
    engaged: AtomicBool,
}

impl Pickup {
    pub const fn new() -> Self {
        Self {
            last: AtomicU32::new(u32::MAX),
            engaged: AtomicBool::new(false),
        }
    }

    pub fn last(&self) -> Option<f32> {
        match self.last.load(Ordering::Relaxed) {
            u32::MAX => None,
            bits => Some(f32::from_bits(bits)),
        }
    }

    pub fn engaged(&self) -> bool {
        self.engaged.load(Ordering::Relaxed)
    }

    /// Remember what the controller just sent, and whether that took the knob.
    pub fn record(&self, sent: f32, engaged: bool) {
        self.last.store(sent.to_bits(), Ordering::Relaxed);
        self.engaged.store(engaged, Ordering::Relaxed);
    }
}

impl Default for Pickup {
    fn default() -> Self {
        Self::new()
    }
}

/// What the audio stream is doing, for the panel's status line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AudioStatus {
    Starting,
    Running { device: String, sample_rate: u32 },
    Failed { reason: String },
}
/// Every control in one `Copy` value: what the audio callback reads once per block.
#[derive(Copy, Clone, Debug)]
pub struct Parameters {
    pub waveform: Waveform,
    pub tuning: f32,
    pub cutoff: f32,
    pub resonance: f32,
    pub env_mod: f32,
    pub decay: f32,
    pub accent: f32,
    pub volume: f32,
    pub tempo_bpm: f32,
    pub swing: f32,
    pub transport: crate::device::Transport,
}

/// The seam between the two threads.
///
/// Every knob is an atomic read once per audio block; the pattern lives behind an `ArcSwap`, so an edit on
/// the UI thread publishes a new `Arc` and the audio thread's `load` never waits. `status` is a mutex, but
/// only the UI thread touches it.
#[derive(Debug)]
pub struct SharedState {
    /// The state this device is in. A device in a rack mirrors the rack's — `Rack::sync_transport` is its only writer — and
    /// a device on its own (every test, and the audio crate's own harness) owns its transport and moves it itself.
    pub transport: AtomicU8,
    /// True when the player has muted this device. A gain, not a stop: the audio thread reads it once per block and the
    /// sequencer keeps running underneath, so unmuting rejoins on the beat instead of restarting the pattern.
    pub mute: AtomicBool,
    /// Step the transport is on, `0..16`, published by the audio thread for the LEDs.
    pub position: AtomicUsize,
    /// The tempo this state's engine follows. In a rack it is the rack's own cell — the same `Arc` in every device — so the
    /// panel, the MIDI thread and the fan-out-free engines all read and write one atomic. A state built on its own owns one.
    pub tempo_bpm: Arc<AtomicF32>,
    pub swing: AtomicF32,
    /// `0` = saw, `1` = square.
    pub waveform: AtomicUsize,
    pub tuning: AtomicF32,
    pub cutoff: AtomicF32,
    pub resonance: AtomicF32,
    pub env_mod: AtomicF32,
    pub decay: AtomicF32,
    pub accent: AtomicF32,
    pub volume: AtomicF32,
    /// Frequency of the note a MIDI keyboard is holding, in Hz.
    pub midi_hz: AtomicF32,
    /// True while a MIDI note is held.
    pub midi_gate: AtomicBool,
    /// True when the held note was struck hard enough to accent.
    pub midi_accent: AtomicBool,
    /// How hard the held note was struck, `0..=127`. The audition plays at this velocity, so what the player hears is
    /// what the step will carry.
    pub midi_velocity: AtomicU32,
    /// The computer keyboard's octave offset, `-2..=2`.
    pub octave: AtomicI32,
    /// The controller each learnable knob listens to, in `midi::cc::LEARNABLE` order; `-1` means unmapped.
    pub cc_map: [AtomicI32; crate::midi::cc::LEARNABLE.len()],
    /// True while the panel is waiting for a controller to learn.
    pub learn_active: AtomicBool,
    /// The controller the MIDI thread saw while learning; `-1` until one arrives.
    pub learn_cc: AtomicI32,
    /// Which of the 64 patterns is playing, `0..64`. `main` owns the bank, so it keeps this in step; the MIDI thread
    /// only reads it, to know which bank a pattern message is pointing into.
    pub pattern_index: AtomicI32,
    /// The pattern a learned controller has selected, `0..64`, or `-1` when the selection follows the sound. A pending
    /// swap lives here for one bar, so the pickup can measure against what the fader is holding.
    pub pattern_selected: AtomicI32,
    /// A pattern a controller asked for, as an absolute number; `-1` when there is nothing to do. The MIDI thread may
    /// not touch the bank, so it leaves this and `main` drains it.
    pub pattern_request: AtomicI32,
    /// Take-over state, one per learnable control, in `cc_map` slot order: a controller has to reach a knob before it
    /// may turn it, so a fader parked somewhere else cannot slam the panel when a song loads.
    pub pickup: [Pickup; crate::midi::cc::LEARNABLE.len()],
    /// The MIDI notes the panel has not placed yet, oldest first. A queue rather than one slot: two notes can land
    /// inside a single frame, and the later one must not overwrite the earlier one's pitch or velocity.
    pub notes: NoteQueue,
    pub pattern: ArcSwap<Pattern>,
    pub status: Mutex<AudioStatus>,
}

impl SharedState {
    /// A state with a tempo cell of its own: a device on its own, or a test.
    pub fn new() -> Arc<Self> {
        Self::with_tempo(Arc::new(AtomicF32::new(DEFAULT_BPM)))
    }

    /// A state sharing `tempo`: how a device joins a rack, where there is one tempo for every engine. The cell is the
    /// rack's, so a tempo the panel or the MIDI thread writes is there before the next block reads it, and nothing has to be
    /// fanned out into a copy that a frame could turn stale.
    pub fn with_tempo(tempo: Arc<AtomicF32>) -> Arc<Self> {
        Arc::new(Self {
            transport: AtomicU8::new(crate::device::Transport::Stopped.byte()),
            mute: AtomicBool::new(false),
            position: AtomicUsize::new(0),
            tempo_bpm: tempo,
            swing: AtomicF32::new(0.0),
            waveform: AtomicUsize::new(0),
            // The launch patch is the engine's, so the panel and the engine cannot disagree about it.
            tuning: AtomicF32::new(DEFAULT_TUNING_KNOB),
            cutoff: AtomicF32::new(DEFAULT_CUTOFF_KNOB),
            resonance: AtomicF32::new(DEFAULT_RESONANCE_KNOB),
            env_mod: AtomicF32::new(DEFAULT_ENV_MOD_KNOB),
            decay: AtomicF32::new(DEFAULT_DECAY_KNOB),
            accent: AtomicF32::new(DEFAULT_ACCENT_KNOB),
            volume: AtomicF32::new(DEFAULT_VOLUME),
            pattern: ArcSwap::from_pointee(Pattern::empty()),
            midi_hz: AtomicF32::new(0.0),
            midi_gate: AtomicBool::new(false),
            midi_accent: AtomicBool::new(false),
            midi_velocity: AtomicU32::new(u32::from(acid_core::pattern::DEFAULT_VELOCITY)),
            octave: AtomicI32::new(0),
            cc_map: std::array::from_fn(|slot| {
                AtomicI32::new(i32::from(crate::midi::cc::DEFAULT_CC[slot]))
            }),
            learn_active: AtomicBool::new(false),
            learn_cc: AtomicI32::new(-1),
            pattern_index: AtomicI32::new(0),
            // Nothing is waiting for the bar, so the selection is the sound.
            pattern_selected: AtomicI32::new(-1),
            pattern_request: AtomicI32::new(-1),
            pickup: std::array::from_fn(|_| Pickup::new()),
            notes: NoteQueue::new(),
            status: Mutex::new(AudioStatus::Starting),
        })
    }

    /// The state the rack is in, as this device last heard it.
    pub fn transport(&self) -> crate::device::Transport {
        crate::device::Transport::from_mirror(self.transport.load(Ordering::Relaxed))
    }

    pub fn set_transport(&self, transport: crate::device::Transport) {
        self.transport.store(transport.byte(), Ordering::Relaxed);
    }

    /// Whether the clock is running. A pause is not playing.
    pub fn is_playing(&self) -> bool {
        self.transport().is_playing()
    }

    pub fn is_muted(&self) -> bool {
        self.mute.load(Ordering::Relaxed)
    }

    pub fn set_mute(&self, muted: bool) {
        self.mute.store(muted, Ordering::Relaxed);
    }

    /// Every control at once, as the audio callback wants them: eleven relaxed loads, no allocation.
    pub fn snapshot(&self) -> Parameters {
        Parameters {
            waveform: self.waveform(),
            tuning: self.tuning.get(),
            cutoff: self.cutoff.get(),
            resonance: self.resonance.get(),
            env_mod: self.env_mod.get(),
            decay: self.decay.get(),
            accent: self.accent.get(),
            volume: self.volume.get(),
            tempo_bpm: self.tempo_bpm.get(),
            swing: self.swing.get(),
            transport: self.transport(),
        }
    }

    pub fn set_waveform(&self, waveform: Waveform) {
        // The catalogue owns the numbering, so adding a wave cannot leave a stale `match` behind.
        self.waveform.store(waveform.index(), Ordering::Relaxed);
    }

    pub fn waveform(&self) -> Waveform {
        Waveform::from_index(self.waveform.load(Ordering::Relaxed))
    }

    /// Publish a whole pattern. The audio thread sees either the old or the new one, never a half-built one.
    pub fn set_pattern(&self, pattern: Arc<Pattern>) {
        self.pattern.store(pattern);
    }
    pub fn pattern(&self) -> Arc<Pattern> {
        self.pattern.load_full()
    }

    /// Which pattern is playing, `0..64`; `main` keeps this in step with the bank.
    pub fn pattern_index(&self) -> usize {
        self.pattern_index.load(Ordering::Relaxed).max(0) as usize
    }

    pub fn set_pattern_index(&self, index: usize) {
        self.pattern_index.store(index as i32, Ordering::Relaxed);
    }

    /// Take the pattern a controller asked for, if any. Drained by `main`, which owns the bank.
    pub fn take_pattern_request(&self) -> Option<usize> {
        let requested = self.pattern_request.swap(-1, Ordering::Relaxed);
        (requested >= 0).then_some(requested as usize)
    }

    /// The pattern a learned controller has selected but not yet heard.
    ///
    /// The swap waits for the bar, so between messages the sounding pattern (`pattern_index`) is still the old one while
    /// the fader has already moved on. A controller's pickup has to measure against what it is holding — the selection —
    /// or the second message of a sweep would look like a hand on the knob and be dropped.
    pub fn pattern_selected(&self) -> usize {
        let selected = self.pattern_selected.load(Ordering::Relaxed);
        if selected >= 0 {
            selected as usize
        } else {
            self.pattern_index()
        }
    }

    /// Record the pattern a controller has selected. `main` sets this when it takes a request and when it adopts a
    /// pattern, so the selection and the sound never disagree for longer than a bar.
    pub fn set_pattern_selected(&self, index: usize) {
        self.pattern_selected.store(index as i32, Ordering::Relaxed);
    }

    /// Follow the sounding pattern again: nothing is waiting for the bar.
    pub fn clear_pattern_selected(&self) {
        self.pattern_selected.store(-1, Ordering::Relaxed);
    }

    /// Clone-on-write one step. Stray indices are ignored: this is reachable from the UI.
    pub fn set_step(&self, index: usize, step: Step) {
        if index >= STEPS_PER_PATTERN {
            return;
        }
        self.pattern.rcu(|current| {
            let mut next = **current;
            next.steps[index] = step;
            Arc::new(next)
        });
    }

    pub fn status(&self) -> AudioStatus {
        match self.status.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn set_status(&self, status: AudioStatus) {
        match self.status.lock() {
            Ok(mut guard) => *guard = status,
            Err(poisoned) => *poisoned.into_inner() = status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_starts_unmuted_and_remembers_being_muted() {
        let s = SharedState::new();
        assert!(!s.is_muted(), "a fresh device is audible");
        s.set_mute(true);
        assert!(s.is_muted());
        s.set_mute(false);
        assert!(!s.is_muted());
    }

    /// The mirror is a state, not a bool: a paused device is silent and is not playing, and the byte survives the round trip
    /// that `Rack::sync_transport` performs on it every time the rack moves.
    #[test]
    fn the_transport_mirror_carries_all_three_states() {
        use crate::device::Transport;

        let s = SharedState::new();
        assert_eq!(s.transport(), Transport::Stopped);
        assert!(!s.is_playing());

        for state in Transport::ALL {
            s.set_transport(state);
            assert_eq!(s.transport(), state, "{state:?} must survive the mirror");
            assert_eq!(s.is_playing(), state == Transport::Playing, "{state:?}");
        }

        // A byte nobody wrote reads as Stopped rather than as some third state: a transport is not a thing to guess at.
        s.set_transport(Transport::Paused);
        s.transport.store(9, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(s.transport(), Transport::Stopped);
    }

    #[test]
    fn status_round_trips() {
        let s = SharedState::new();
        assert_eq!(s.status(), AudioStatus::Starting);
        s.set_status(AudioStatus::Running {
            device: "PipeWire".to_string(),
            sample_rate: 48_000,
        });
        assert_eq!(
            s.status(),
            AudioStatus::Running {
                device: "PipeWire".to_string(),
                sample_rate: 48_000
            }
        );
    }

    #[test]
    fn defaults_sound_like_a_patch_not_a_silence() {
        let s = SharedState::new();
        assert!((s.tempo_bpm.get() - DEFAULT_BPM).abs() < 1e-6);
        assert!(s.volume.get() > 0.5);
    }
}
