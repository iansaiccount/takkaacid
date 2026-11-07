//! State shared between the UI thread and the audio thread.
//!
//! Everything here is either an atomic or a short-lived mutex the audio thread never touches, so the
//! audio callback can never be blocked by the user moving a knob.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
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
    pub playing: bool,
}

/// The seam between the two threads.
///
/// Every knob is an atomic read once per audio block; the pattern lives behind an `ArcSwap`, so an edit on
/// the UI thread publishes a new `Arc` and the audio thread's `load` never waits. `status` is a mutex, but
/// only the UI thread touches it.
#[derive(Debug)]
pub struct SharedState {
    pub playing: AtomicBool,
    /// Step the transport is on, `0..16`, published by the audio thread for the LEDs.
    pub position: AtomicUsize,
    pub tempo_bpm: AtomicF32,
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
    /// True while a MIDI clock is driving the tempo.
    pub external_clock: AtomicBool,
    /// The computer keyboard's octave offset, `-2..=2`.
    pub octave: AtomicI32,
    /// The controller each learnable knob listens to, in `midi::cc::LEARNABLE` order; `-1` means unmapped.
    pub cc_map: [AtomicI32; 7],
    /// True while the panel is waiting for a controller to learn.
    pub learn_active: AtomicBool,
    /// The controller the MIDI thread saw while learning; `-1` until one arrives.
    pub learn_cc: AtomicI32,
    /// The MIDI notes the panel has not placed yet, oldest first. A queue rather than one slot: two notes can land
    /// inside a single frame, and the later one must not overwrite the earlier one's pitch or velocity.
    pub notes: NoteQueue,
    pub pattern: ArcSwap<Pattern>,
    pub status: Mutex<AudioStatus>,
}

impl SharedState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            playing: AtomicBool::new(false),
            position: AtomicUsize::new(0),
            tempo_bpm: AtomicF32::new(DEFAULT_BPM),
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
            external_clock: AtomicBool::new(false),
            octave: AtomicI32::new(0),
            cc_map: std::array::from_fn(|slot| {
                AtomicI32::new(i32::from(crate::midi::cc::DEFAULT_CC[slot]))
            }),
            learn_active: AtomicBool::new(false),
            learn_cc: AtomicI32::new(-1),
            notes: NoteQueue::new(),
            status: Mutex::new(AudioStatus::Starting),
        })
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
    }

    pub fn toggle_playing(&self) -> bool {
        let next = !self.is_playing();
        self.set_playing(next);
        next
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
            playing: self.is_playing(),
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
    fn playing_toggles_and_reports() {
        let s = SharedState::new();
        assert!(!s.is_playing());
        assert!(s.toggle_playing());
        assert!(s.is_playing());
        assert!(!s.toggle_playing());
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
