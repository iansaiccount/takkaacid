//! The 16-step pattern — the model the step keys, the sequencer and the file format all share.
//!
//! This is `acid-core`'s only serde type. Its shape is deliberately flat and `Copy` so the audio thread
//! can hold a snapshot and the UI can edit a clone without coordination.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::drums::Drum;
use crate::pitch::{Pitch, step_hz};

/// The most slots a pattern can hold: the longest bar any grid needs ([`Grid::ThirtySecond`]'s thirty-two).
///
/// A pattern is a fixed table of these with a `len()` from its grid, because `Pattern` is `Copy` on purpose — the audio
/// thread holds an `Arc` snapshot and every step edit copies the whole pattern.
pub const PATTERN_SLOTS: usize = 32;

/// The grid a pattern's bar is divided into.
///
/// A pattern is always exactly one 4/4 bar; the grid says how many steps that bar has. `1/16` is the sixteenths every
/// pattern born before this build is on, `1/32` divides the same bar twice as finely, and `1/8T` divides it into
/// eighth-note triplets, which sit *off* the sixteenth grid rather than inside it.
///
/// A catalogue, in this repo's sense: `ALL` is the order the face's key cycles, `name`/`from_name` are the mapping the
/// file uses, and every `match` over it is written out so a fourth grid fails to compile where a decision is owed.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Grid {
    /// Sixteen steps to the bar: the grid every file written before this build is on, and the one a new pattern takes.
    #[default]
    Sixteenth,
    /// Thirty-two steps to the bar: finer than a sixteenth.
    ThirtySecond,
    /// Twelve steps to the bar — eighth-note triplets.
    EighthTriplet,
}

impl Grid {
    /// Every grid, in the order the transport row's key cycles them.
    pub const ALL: [Grid; 3] = [Grid::Sixteenth, Grid::ThirtySecond, Grid::EighthTriplet];

    /// What the face and the file call it.
    pub fn name(self) -> &'static str {
        match self {
            Grid::Sixteenth => "1/16",
            Grid::ThirtySecond => "1/32",
            Grid::EighthTriplet => "1/8T",
        }
    }

    /// The grid a caption names, or `None` for a word this build does not know.
    pub fn from_name(name: &str) -> Option<Grid> {
        match name {
            "1/16" => Some(Grid::Sixteenth),
            "1/32" => Some(Grid::ThirtySecond),
            "1/8T" => Some(Grid::EighthTriplet),
            _ => None,
        }
    }

    /// Steps in one 4/4 bar of this grid.
    ///
    /// `const` because a test that builds a bar of steps sizes its array from it.
    pub const fn steps_per_bar(self) -> usize {
        match self {
            Grid::Sixteenth => 16,
            Grid::ThirtySecond => 32,
            Grid::EighthTriplet => 12,
        }
    }

    /// Whether `index` falls on a beat: every fourth step, every eighth, every third.
    ///
    /// Not `const`: the count comes from `beats_every`, and the modulo is `is_multiple_of` so the workspace's clippy
    /// gate does not read `% n == 0` as a hand-rolled spelling of it.
    pub fn is_beat(self, index: usize) -> bool {
        index.is_multiple_of(self.beats_every())
    }

    /// Steps to a beat: a quarter note, which every grid divides evenly.
    const fn beats_every(self) -> usize {
        match self {
            Grid::Sixteenth => 4,
            Grid::ThirtySecond => 8,
            Grid::EighthTriplet => 3,
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for Grid {
    /// By caption, so a reader of the file can see which grid a pattern is on, and a caption this build does not know
    /// reads as sixteenths rather than refusing the song.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Grid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(Grid::from_name(&name).unwrap_or_default())
    }
}

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
        let bit = drum.bit();
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
        let bit = drum.bit();
        let struck = self.hits & bit != 0;
        self.hits &= !bit;
        self.accents &= !bit;
        if self.hits == 0 && struck {
            self.on = false;
        }
    }

    /// Whether this step strikes `drum`.
    pub fn is_hit(&self, drum: Drum) -> bool {
        self.hits & drum.bit() != 0
    }

    /// Whether this step's strike on `drum` is accented. False for a voice the step does not strike, and false for an
    /// accent bit with no hit under it — nothing here writes one, but a hand-edited document can.
    pub fn is_accented(&self, drum: Drum) -> bool {
        self.hits & self.accents & drum.bit() != 0
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

/// One bar of a pattern: the grid it is divided by, and its slots.
///
/// A pattern is always **exactly one 4/4 bar**, so `len()` is the grid's own count and the slots above it are rests. That
/// invariant is what makes "the row is the pattern" and "the file is the row" the same sentence: `from_steps_in` rests
/// what it is not given, `with_grid` rests what a narrower bar drops, and the file writes only the steps that play.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Pattern {
    /// How this bar is divided. A pattern is the grid's, not the device's, so a bank can hold all three.
    pub grid: Grid,
    /// The bar's slots. **Every slot at or above `len()` is a rest, always** — see the type's own note.
    pub steps: [Step; PATTERN_SLOTS],
}

impl Pattern {
    /// An empty sixteenth bar: what a pattern that has never said otherwise is.
    pub fn empty() -> Self {
        Self::default()
    }

    /// An empty bar of `grid`: what `CLEAR` leaves behind, so clearing a pattern never shortens its bar.
    pub fn empty_in(grid: Grid) -> Self {
        Self {
            grid,
            ..Self::default()
        }
    }

    /// Steps in this bar: the grid's count, and the only count a player of this pattern asks.
    #[expect(
        clippy::len_without_is_empty,
        reason = "a bar always has steps; there is no empty pattern to name"
    )]
    pub fn len(&self) -> usize {
        self.grid.steps_per_bar()
    }

    /// A sixteenth pattern from its sixteen steps. Extra steps are dropped and missing ones are rests: a pattern is
    /// exactly one bar, and the grid says how long that is. Use [`Pattern::from_steps_in`] for any other grid.
    pub fn from_steps(steps: impl AsRef<[Step]>) -> Self {
        Self::from_steps_in(Grid::Sixteenth, steps)
    }

    /// A pattern of `grid` from up to `grid.steps_per_bar()` steps: the rest of the bar is rests, and anything past its
    /// end is dropped.
    pub fn from_steps_in(grid: Grid, steps: impl AsRef<[Step]>) -> Self {
        let steps = steps.as_ref();
        let mut pattern = Self::empty_in(grid);
        let taken = steps.len().min(pattern.len());
        pattern.steps[..taken].copy_from_slice(&steps[..taken]);
        pattern
    }

    /// The same music on another grid. The steps the new bar can hold are kept and the ones it cannot are **rested** — the
    /// one place a pattern loses sound, and the panel says so on the status line when it does.
    pub fn with_grid(self, grid: Grid) -> Self {
        let mut pattern = Self { grid, ..self };
        let len = pattern.len();
        for step in &mut pattern.steps[len..] {
            *step = Step::rest();
        }
        pattern
    }

    /// The steps that play, in step order.
    pub fn iter(&self) -> impl Iterator<Item = &Step> {
        self.steps[..self.len()].iter()
    }

    /// The steps that play, for an edit to walk.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Step> {
        let len = self.len();
        self.steps[..len].iter_mut()
    }

    /// Every step moved by whole semitones: the same sound, spelled on a different lattice.
    ///
    /// This exists for one migration: the lattice root moved from MIDI 33 to MIDI 12, so a pattern written before that
    /// has to be rewritten 21 semitones up to keep sounding the same.
    pub fn transposed(self, semitones: i8) -> Self {
        Self {
            steps: self.steps.map(|step| step.transposed(semitones)),
            ..self
        }
    }
}

/// `{ "grid": "1/32", "steps": [ … the steps that play … ] }`.
///
/// The shadow exists because the file writes a *list of the steps that play* while the pattern holds a fixed slot table:
/// a sixteenth pattern's step list is sixteen entries, exactly as a version-12 build wrote it, which is what lets an older
/// build read a sixteenth song this build saved.
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
struct PatternFile {
    /// Absent in every version-12 file, and in a file naming a grid this build does not know: both read as sixteenths.
    #[serde(default)]
    grid: Grid,
    steps: Vec<Step>,
}

#[cfg(feature = "serde")]
impl Serialize for Pattern {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PatternFile {
            grid: self.grid,
            steps: self.iter().copied().collect(),
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // `from_steps_in` pads a short list with rests and drops a long one, so a hand-edited file lands on the invariant
        // rather than beside it.
        let file = PatternFile::deserialize(deserializer)?;
        Ok(Pattern::from_steps_in(file.grid, &file.steps))
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
        assert_eq!(step.hits, Drum::Kick.bit());
        for drum in Drum::ALL {
            if drum != Drum::Kick {
                assert!(!step.is_hit(drum), "{drum:?} was struck by a kick");
            }
        }

        step.hit(Drum::Snare, true);
        assert_eq!(step.hits, Drum::Kick.bit() | Drum::Snare.bit());
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
        assert_eq!(step.hits, Drum::Kick.bit() | Drum::Snare.bit());

        // The last voice in the catalogue is the top bit, and it is a bit like any other.
        step.hit(Drum::Shaker, true);
        assert_eq!(
            step.hits,
            Drum::Kick.bit() | Drum::Snare.bit() | Drum::Shaker.bit()
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
            accents: Drum::Ride.bit(),
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
