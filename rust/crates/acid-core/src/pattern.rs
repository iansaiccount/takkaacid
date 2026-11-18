//! The 16-step pattern — the model the step keys, the sequencer and the file format all share.
//!
//! This is `acid-core`'s only serde type. Its shape is deliberately flat and `Copy` so the audio thread
//! can hold a snapshot and the UI can edit a clone without coordination.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::drums::Drum;
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

/// One step of a pattern: a note or a rest, with its two performance flags, the velocity it was struck at, and the drum
/// voices it strikes.
///
/// A step serves both machines. The 303 reads `pitch`, `octave_offset`, `velocity`, `accent` and `slide` and ignores the
/// masks; a drum pattern writes the masks and ignores the note. `on` is the one field they share: the step sounds because
/// one of the two says so.
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
    /// Which of the sixteen drum voices this step strikes: one bit per [`Drum`], in catalogue order. A pattern written
    /// before the drum machine existed has none, which is why the field defaults to silence rather than refusing.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hits: u16,
    /// Which of those strikes are accented. A bit without its `hits` bit means nothing, so the two are written together.
    #[cfg_attr(feature = "serde", serde(default))]
    pub accents: u16,
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
            hits: 0,
            accents: 0,
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
            hits: 0,
            accents: 0,
        }
    }

    pub fn rest() -> Self {
        Self::default()
    }

    /// Frequency this step asks the voice for.
    pub fn hz(&self) -> f32 {
        step_hz(self.pitch, self.octave_offset)
    }

    /// The MIDI note this step names, with nothing clamped.
    pub fn midi(&self) -> i32 {
        crate::pitch::step_midi(self.pitch, self.octave_offset)
    }

    /// Whether this step's note is inside the instrument's range. An inaudible step plays as a rest.
    pub fn audible(&self) -> bool {
        (crate::pitch::NOTE_LOW..=crate::pitch::NOTE_HIGH).contains(&self.midi())
    }

    /// Strike `drum` on this step: the only writer of the two masks, and it writes `on` with them, so a step can never
    /// be struck and silent at once.
    ///
    /// `accented` is written rather than added: striking a voice again without the accent takes the accent off with it,
    /// so a bit in `accents` can never outlive the hit it belonged to.
    pub fn hit(&mut self, drum: Drum, accented: bool) {
        let bit = drum_bit(drum);
        self.hits |= bit;
        if accented {
            self.accents |= bit;
        } else {
            self.accents &= !bit;
        }
        self.on = true;
    }

    /// Clear `drum` from this step: both of its bits, and `on` when that was the last voice left — a drum step with no
    /// hit is a rest.
    ///
    /// Clearing a voice the step does not strike changes nothing at all, so a stray click on an empty cell cannot rest a
    /// step the 303 put a note on.
    pub fn clear_hit(&mut self, drum: Drum) {
        let bit = drum_bit(drum);
        let struck = self.hits & bit != 0;
        self.hits &= !bit;
        self.accents &= !bit;
        if self.hits == 0 && struck {
            self.on = false;
        }
    }

    /// Whether this step strikes `drum`.
    pub fn is_hit(&self, drum: Drum) -> bool {
        self.hits & drum_bit(drum) != 0
    }

    /// Whether this step's strike on `drum` is accented. False for a voice the step does not strike, and false for an
    /// accent bit with no hit under it — nothing here writes one, but a hand-edited document can.
    pub fn is_accented(&self, drum: Drum) -> bool {
        self.hits & self.accents & drum_bit(drum) != 0
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

/// The bit a voice owns in [`Step::hits`] and [`Step::accents`].
///
/// Derived from the catalogue's own `index` rather than written out again: sixteen voices is exactly the width of the
/// mask, and a voice that moved in [`Drum::ALL`] moves here with it.
fn drum_bit(drum: Drum) -> u16 {
    1u16 << drum.index() as u16
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drums::Drum;

    #[test]
    fn a_hit_sets_the_mask_the_accent_and_on() {
        let mut step = Step::rest();
        assert!(!step.on, "a rest does not sound");

        step.hit(Drum::Kick, false);
        assert!(step.on, "a drum step is on because a voice hits it");
        assert!(step.is_hit(Drum::Kick));
        assert!(!step.is_accented(Drum::Kick));
        assert_eq!(step.hits, drum_bit(Drum::Kick));
        for drum in Drum::ALL {
            if drum != Drum::Kick {
                assert!(!step.is_hit(drum), "{drum:?} was struck by a kick");
            }
        }

        step.hit(Drum::Snare, true);
        assert_eq!(step.hits, drum_bit(Drum::Kick) | drum_bit(Drum::Snare));
        assert!(step.is_accented(Drum::Snare));
        assert!(
            !step.is_accented(Drum::Kick),
            "one voice's accent is its own"
        );

        // The same voice struck again without the accent drops it. A bit in `accents` with no `hits` bit under it means
        // nothing, so the two masks are written together or the accent outlives the accent the pointer has moved off.
        step.hit(Drum::Snare, false);
        assert!(step.is_hit(Drum::Snare));
        assert!(!step.is_accented(Drum::Snare));
        assert_eq!(step.hits, drum_bit(Drum::Kick) | drum_bit(Drum::Snare));

        // The last voice in the catalogue is the top bit, and it is a bit like any other.
        step.hit(Drum::Shaker, true);
        assert_eq!(
            step.hits,
            drum_bit(Drum::Kick) | drum_bit(Drum::Snare) | drum_bit(Drum::Shaker)
        );
        assert!(step.is_accented(Drum::Shaker));

        // A transposition moves the note and leaves the drums where they were: the lattice migration rewrites every step
        // of an old bank, and it must not rewrite which voices a step strikes.
        let moved = step.transposed(12);
        assert_eq!((moved.hits, moved.accents), (step.hits, step.accents));
        assert_ne!(moved.octave_offset, step.octave_offset);
    }

    #[test]
    fn clearing_the_last_hit_rests_the_step() {
        let mut step = Step::rest();
        step.hit(Drum::ClosedHat, true);
        step.hit(Drum::OpenHat, false);

        step.clear_hit(Drum::ClosedHat);
        assert!(step.on, "another voice still hits this step");
        assert!(!step.is_hit(Drum::ClosedHat));
        assert!(
            !step.is_accented(Drum::ClosedHat),
            "the accent goes with it"
        );
        assert!(step.is_hit(Drum::OpenHat));

        step.clear_hit(Drum::OpenHat);
        assert!(!step.on, "no voice left, so the step rests");
        assert_eq!((step.hits, step.accents), (0, 0));

        // Clearing a voice that never hit is a no-op, not a wipe: a stray click on an empty cell must not rest a step
        // another voice is holding — nor one the 303 put a note on, which is the case the grid cannot reach today but a
        // hand-built step can.
        let mut held = Step::rest();
        held.hit(Drum::Kick, true);
        held.clear_hit(Drum::Ride);
        assert!(held.on);
        assert!(held.is_hit(Drum::Kick) && held.is_accented(Drum::Kick));

        let mut note = Step::note(Pitch::C, 1);
        note.clear_hit(Drum::Kick);
        assert!(note.on, "a clear that removed nothing cannot rest a note");
        assert_eq!((note.hits, note.accents), (0, 0));
    }

    #[test]
    fn an_accent_with_no_hit_under_it_is_not_an_accent() {
        // Nothing in the four methods can write an accent bit without its hit, but a save file is JSON a human may edit.
        // The accent belongs to a strike, so a bit with no strike under it answers as nothing rather than as an accent a
        // later bank would play.
        let disagreeing = Step {
            accents: drum_bit(Drum::Ride),
            ..Step::rest()
        };
        assert!(!disagreeing.is_hit(Drum::Ride));
        assert!(!disagreeing.is_accented(Drum::Ride));
        assert!(!disagreeing.on, "and the step is still a rest");
    }

    #[test]
    fn a_drum_edit_leaves_the_303_fields_alone() {
        // A step is both worlds' type, so a drum edit must not reach into the 303's note. `on` is the one field they
        // share, by design: the step sounds because one of the two machines says so.
        let mut step = Step {
            accent: true,
            slide: true,
            velocity: 91,
            ..Step::note(Pitch::G, 2)
        };
        let note = (
            step.pitch,
            step.octave_offset,
            step.accent,
            step.slide,
            step.velocity,
            step.on,
        );

        step.hit(Drum::Clap, true);
        step.hit(Drum::Shaker, true);
        step.hit(Drum::Clap, false);
        step.clear_hit(Drum::Shaker);

        assert_eq!(
            (
                step.pitch,
                step.octave_offset,
                step.accent,
                step.slide,
                step.velocity,
                step.on,
            ),
            note,
            "a drum edit moved the 303's note"
        );
    }

    #[test]
    fn a_pattern_without_masks_is_a_303_pattern() {
        // The masks are additive: the factory's starter phrase was written before the drum machine existed and still
        // strikes no drum, and its `on` flags say exactly what they always said.
        let phrase = crate::factory::starter();
        assert!(
            phrase
                .steps
                .iter()
                .all(|step| step.hits == 0 && step.accents == 0),
            "the 303's phrase strikes a drum"
        );
        assert!(
            phrase.steps.iter().any(|step| step.on),
            "and it still sounds"
        );

        // Every constructor leaves the masks zeroed too, so a step built today is the step an old file holds.
        let note = Step::note(Pitch::A, 1);
        assert_eq!((note.hits, note.accents), (0, 0));
        assert!(note.on);
        let rest = Step::rest();
        assert_eq!((rest.hits, rest.accents), (0, 0));
        assert!(!rest.on);
    }
}
