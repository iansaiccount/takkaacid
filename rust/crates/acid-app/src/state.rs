//! State shared between the UI thread and the audio thread.
//!
//! Everything here is either an atomic or a short-lived mutex the audio thread never touches, so the
//! audio callback can never be blocked by the user moving a knob.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

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

/// The seam between the two threads.
#[derive(Debug)]
pub struct SharedState {
    pub playing: AtomicBool,
    pub tempo_bpm: AtomicF32,
    pub volume: AtomicF32,
    pub status: Mutex<AudioStatus>,
}

impl SharedState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            playing: AtomicBool::new(false),
            tempo_bpm: AtomicF32::new(DEFAULT_BPM),
            volume: AtomicF32::new(DEFAULT_VOLUME),
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
