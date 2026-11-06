//! The 16-step pattern — the model the step keys, the sequencer and the file format all share.
//!
//! This is `acid-core`'s only serde type. Its shape is deliberately flat and `Copy` so the audio thread
//! can hold a snapshot and the UI can edit a clone without coordination.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::pitch::{Pitch, step_hz};

/// Steps in one pattern.
pub const STEPS_PER_PATTERN: usize = 16;

/// One step of a pattern: a note or a rest, with its two performance flags.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Step {
    /// A resting step still remembers its pitch and flags, so re-enabling restores them.
    pub on: bool,
    pub pitch: Pitch,
    pub octave_offset: i8,
    pub accent: bool,
    pub slide: bool,
}

impl Step {
    /// A sounding step at `pitch`, transposed by whole octaves.
    pub fn note(pitch: Pitch, octave_offset: i8) -> Self {
        Self {
            on: true,
            pitch,
            octave_offset,
            accent: false,
            slide: false,
        }
    }

    pub fn rest() -> Self {
        Self::default()
    }

    /// Frequency this step asks the voice for.
    pub fn hz(&self) -> f32 {
        step_hz(self.pitch, self.octave_offset)
    }
}

/// Sixteen steps — one bar at 16th notes.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Pattern {
    pub steps: [Step; STEPS_PER_PATTERN],
}

impl Pattern {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_steps(steps: [Step; STEPS_PER_PATTERN]) -> Self {
        Self { steps }
    }
}

/// Where a pattern lives: bank `0..=7` (A–H) and number `0..=7` (displayed 1–8).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PatternId {
    pub bank: u8,
    pub number: u8,
}

impl std::fmt::Display for PatternId {
    /// Renders the hardware's label: `A1` .. `H8`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let letter = (b'A' + self.bank.min(7)) as char;
        write!(f, "{letter}{}", self.number.min(7) + 1)
    }
}
