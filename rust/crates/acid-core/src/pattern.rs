//! The 16-step pattern — the model the step keys, the sequencer and the file format all share.
//!
//! This is `acid-core`'s only serde type. Its shape is deliberately flat and `Copy` so the audio thread
//! can hold a snapshot and the UI can edit a clone without coordination.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::pitch::{Pitch, step_hz};

/// Steps in one pattern.
pub const STEPS_PER_PATTERN: usize = 16;

/// The velocity a step plays at when nothing struck it: MIDI's full scale, so an untouched pattern sounds exactly as
/// loud as it did before velocity existed.
pub const DEFAULT_VELOCITY: u8 = 127;

/// Serde's stand-in for a file written before steps carried a velocity.
#[cfg(feature = "serde")]
fn default_velocity() -> u8 {
    DEFAULT_VELOCITY
}

/// One step of a pattern: a note or a rest, with its two performance flags and the velocity it was struck at.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Step {
    /// A resting step still remembers its pitch and flags, so re-enabling restores them.
    pub on: bool,
    pub pitch: Pitch,
    pub octave_offset: i8,
    pub accent: bool,
    pub slide: bool,
    /// MIDI velocity, `0..=127`: the step's level. ACCENT is a separate boost on top of it, so a soft accented note is
    /// still louder than a soft plain one.
    #[cfg_attr(feature = "serde", serde(default = "default_velocity"))]
    pub velocity: u8,
}

impl Default for Step {
    /// A rest at full velocity: `on: false` keeps it silent, and a rest that is switched on again must not be silent
    /// too, which is what a zero velocity would make it.
    fn default() -> Self {
        Self {
            on: false,
            pitch: Pitch::default(),
            octave_offset: 0,
            accent: false,
            slide: false,
            velocity: DEFAULT_VELOCITY,
        }
    }
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
            velocity: DEFAULT_VELOCITY,
        }
    }

    pub fn rest() -> Self {
        Self::default()
    }

    /// Frequency this step asks the voice for.
    pub fn hz(&self) -> f32 {
        step_hz(self.pitch, self.octave_offset)
    }

    /// This step moved by whole semitones, rolling into the octave: the same sound on a shifted lattice.
    pub fn transposed(self, semitones: i8) -> Self {
        let absolute = i32::from(self.pitch.semitones())
            + 12 * i32::from(self.octave_offset)
            + i32::from(semitones);
        Self {
            pitch: Pitch::ALL[absolute.rem_euclid(12) as usize],
            octave_offset: absolute
                .div_euclid(12)
                .clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8,
            ..self
        }
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

    /// Every step moved by whole semitones: the same sound, spelled on a different lattice.
    ///
    /// This exists for one migration: the lattice root moved from MIDI 33 to MIDI 12, so a pattern written before that
    /// has to be rewritten 21 semitones up to keep sounding the same.
    pub fn transposed(self, semitones: i8) -> Self {
        Self {
            steps: self.steps.map(|step| step.transposed(semitones)),
        }
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
