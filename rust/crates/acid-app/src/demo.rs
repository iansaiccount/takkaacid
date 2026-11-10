//! The demo songs: five patches and their lines, built into the binary.
//!
//! A song here is a bank of up to sixty-four one-bar patterns plus a patch, so a demo is one bank of eight variations of a
//! genre's line and the knobs that make it sound like that genre. Nothing here touches the disk: the demos cannot be
//! overwritten, and the card never lists them among the player's own files.
//!
//! Two of the acid lines are transcriptions of the published one-bar TB-303 references at
//! `acidvoice.com/midi_pattern_tb_303_x0xb0x.htm` (patterns 07 and 19), moved into this pitch model; the blues, rock and
//! funk lines are written here. They are in the spirit of those records, not copies of them.

use acid_core::bank::PatternBank;
use acid_core::pattern::{Pattern, STEPS_PER_PATTERN, Step};
use acid_core::pitch::Pitch;

use crate::persist::{Knobs, SAVE_VERSION, SavedState};

/// One demo: the name the card shows, and the song it loads.
pub struct Demo {
    pub name: &'static str,
    pub state: SavedState,
}

/// Every demo, in the order the card lists them.
pub fn songs() -> Vec<Demo> {
    vec![slow_acid(), hard_techno(), blues(), rock(), funk()]
}

/// What a first run opens on: the slow acid demo, already carrying the name a new song starts with.
pub fn seed() -> SavedState {
    let mut state = slow_acid().state;
    state.song = Some(NEW_SONG.to_string());
    state
}

/// The name a demo carries once it has been adopted: it is not saved anywhere until the player renames it.
pub const NEW_SONG: &str = "New song";

/// One authored step: where it sits in the bar, what it plays, how many octaves up, and its flags.
///
/// The flags are a little language so the tables below read like a sequencer grid: `a` accent, `s` slide, `g` ghost note
/// (a soft one, the breath between the loud ones a funk line lives on).
type Hit = (usize, Pitch, i8, &'static str);

/// A phrase from a table of hits; every step not named is a rest.
fn line(hits: &[Hit]) -> Pattern {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    for &(at, pitch, octave, flags) in hits {
        let mut step = Step::note(pitch, octave);
        step.accent = flags.contains('a');
        step.slide = flags.contains('s');
        if flags.contains('g') {
            step.velocity = GHOST;
        }
        steps[at] = step;
    }
    Pattern::from_steps(steps)
}

/// What a ghost note plays at: audible, never the point.
const GHOST: u8 = 60;

/// A song made of one bank: eight variations of the genre's main line and its fill, plus the patch.
fn song(bpm: f32, swing: f32, knobs: Knobs, main: Pattern, fill: Pattern) -> SavedState {
    let mut bank = PatternBank::new_empty();
    for (slot, pattern) in variations(main, fill).iter().enumerate() {
        bank.replace(slot, *pattern);
    }
    SavedState {
        version: SAVE_VERSION,
        bank,
        knobs,
        bpm,
        swing,
        song: None,
        ..SavedState::default()
    }
}

/// Eight variations from two authored lines: each line as written, the same an octave either way, and a sparser reading
/// of each — the one to build a break on. The pattern knob has somewhere to go without eight phrases to maintain.
fn variations(main: Pattern, fill: Pattern) -> [Pattern; 8] {
    [
        main,
        main.transposed(-12),
        main.transposed(12),
        sparser(main),
        fill,
        fill.transposed(-12),
        fill.transposed(12),
        sparser(fill),
    ]
}

/// Every other sounding step rests, so the line breathes: the variation to build a break on.
fn sparser(pattern: Pattern) -> Pattern {
    let mut steps = pattern.steps;
    let mut sounding = 0;
    for step in steps.iter_mut() {
        if step.on {
            sounding += 1;
            // The downbeat stays, the next note goes, and so on through the bar: half the line, in the same places.
            if sounding % 2 == 0 {
                *step = Step::rest();
            }
        }
    }
    Pattern::from_steps(steps)
}

/// The patch each demo plays through. "Saw" and "square" are the wave catalogue's indices, so a demo cannot name a wave
/// the oscillator does not have.
mod patches {
    use crate::persist::Knobs;

    /// `waveform`, `tuning`, `cutoff`, `resonance`, `env_mod`, `decay`, `accent`, `volume` — in that order, so each demo
    /// reads as the six or seven numbers that make it sound like itself.
    pub fn patch(numbers: [f32; 8]) -> Knobs {
        Knobs {
            waveform: numbers[0] as u8,
            tuning: numbers[1],
            cutoff: numbers[2],
            resonance: numbers[3],
            env_mod: numbers[4],
            decay: numbers[5],
            accent: numbers[6],
            volume: numbers[7],
        }
    }

    pub const SAW: f32 = 0.0;
    pub const SQUARE: f32 = 1.0;
}

/// The original: a saw, the filter open enough to hear the slides, and a slow line that leans on them.
fn slow_acid() -> Demo {
    let main = line(&[
        (0, Pitch::A, 1, "a"),
        (2, Pitch::A, 1, ""),
        (3, Pitch::C, 2, "s"),
        (4, Pitch::E, 1, ""),
        (6, Pitch::A, 1, ""),
        (7, Pitch::C, 2, "s"),
        (8, Pitch::A, 1, "a"),
        (10, Pitch::G, 1, ""),
        (11, Pitch::E, 1, "s"),
        (12, Pitch::A, 1, ""),
        (14, Pitch::C, 2, ""),
        (15, Pitch::E, 1, "s"),
    ]);
    let fill = line(&[
        (0, Pitch::A, 1, "a"),
        (2, Pitch::C, 2, ""),
        (4, Pitch::A, 1, ""),
        (6, Pitch::E, 2, "s"),
        (8, Pitch::A, 1, "a"),
        (9, Pitch::A, 1, "g"),
        (10, Pitch::C, 2, ""),
        (11, Pitch::A, 1, "s"),
        (12, Pitch::G, 1, ""),
        (13, Pitch::E, 1, "g"),
        (14, Pitch::A, 1, ""),
        (15, Pitch::C, 2, "s"),
    ]);
    Demo {
        name: "Slow Acid",
        state: song(
            118.0,
            0.0,
            patches::patch([patches::SAW, 0.5, 0.35, 0.78, 0.72, 0.46, 0.52, 0.87]),
            main,
            fill,
        ),
    }
}

/// The same box at the other end: short, bright, and played as a chug with an octave jump every beat.
fn hard_techno() -> Demo {
    let main = line(&[
        (0, Pitch::E, 1, "a"),
        (1, Pitch::E, 1, ""),
        (2, Pitch::E, 1, ""),
        (3, Pitch::E, 1, "s"),
        (4, Pitch::E, 2, "a"),
        (5, Pitch::E, 1, ""),
        (6, Pitch::E, 1, ""),
        (7, Pitch::E, 1, "s"),
        (8, Pitch::E, 1, "a"),
        (9, Pitch::E, 1, ""),
        (10, Pitch::G, 1, ""),
        (11, Pitch::E, 1, "s"),
        (12, Pitch::E, 2, "a"),
        (13, Pitch::E, 1, ""),
        (14, Pitch::E, 1, ""),
        (15, Pitch::E, 1, "s"),
    ]);
    let fill = line(&[
        (0, Pitch::E, 1, "a"),
        (1, Pitch::E, 1, "g"),
        (2, Pitch::E, 2, "s"),
        (3, Pitch::E, 1, ""),
        (4, Pitch::E, 1, "a"),
        (5, Pitch::E, 1, "g"),
        (6, Pitch::E, 1, "s"),
        (7, Pitch::E, 1, ""),
        (8, Pitch::E, 2, "a"),
        (9, Pitch::E, 1, ""),
        (10, Pitch::E, 1, "g"),
        (11, Pitch::E, 1, "s"),
        (12, Pitch::E, 1, "a"),
        (13, Pitch::E, 1, ""),
        (14, Pitch::E, 1, "g"),
        (15, Pitch::E, 1, "s"),
    ]);
    Demo {
        name: "Hard Techno Acid",
        state: song(
            142.0,
            0.0,
            patches::patch([patches::SAW, 0.5, 0.58, 0.86, 0.88, 0.28, 0.66, 0.95]),
            main,
            fill,
        ),
    }
}

/// A shuffle comes from the clock, not the notes: the walk is on the eighths and SWING does the rest.
fn blues() -> Demo {
    let main = line(&[
        (0, Pitch::A, 1, "a"),
        (2, Pitch::A, 1, ""),
        (4, Pitch::C, 2, ""),
        (6, Pitch::CSharp, 2, ""),
        (8, Pitch::D, 2, "a"),
        (10, Pitch::DSharp, 2, ""),
        (12, Pitch::E, 2, ""),
        (14, Pitch::D, 2, ""),
    ]);
    let fill = line(&[
        (0, Pitch::A, 1, "a"),
        (2, Pitch::A, 1, ""),
        (3, Pitch::A, 1, "g"),
        (4, Pitch::C, 2, ""),
        (6, Pitch::CSharp, 2, ""),
        (8, Pitch::D, 2, "a"),
        (10, Pitch::DSharp, 2, ""),
        (12, Pitch::E, 2, ""),
        (13, Pitch::E, 2, "g"),
        (14, Pitch::D, 2, ""),
        (15, Pitch::D, 2, "s"),
    ]);
    Demo {
        name: "Blues",
        state: song(
            92.0,
            0.35,
            patches::patch([patches::SQUARE, 0.5, 0.3, 0.34, 0.36, 0.6, 0.3, 0.47]),
            main,
            fill,
        ),
    }
}

/// Eighths, root and fifth, and one slide into the downbeat: the bass part of a rock band.
fn rock() -> Demo {
    let main = line(&[
        (0, Pitch::E, 1, "a"),
        (2, Pitch::E, 1, ""),
        (4, Pitch::B, 1, ""),
        (6, Pitch::E, 1, ""),
        (8, Pitch::E, 1, "a"),
        (10, Pitch::B, 1, ""),
        (12, Pitch::E, 2, ""),
        (14, Pitch::E, 1, "s"),
    ]);
    let fill = line(&[
        (0, Pitch::E, 1, "a"),
        (2, Pitch::E, 1, ""),
        (4, Pitch::E, 2, ""),
        (6, Pitch::E, 1, ""),
        (8, Pitch::B, 1, "a"),
        (10, Pitch::E, 1, ""),
        (12, Pitch::E, 1, "s"),
        (14, Pitch::E, 2, ""),
    ]);
    Demo {
        name: "Rock",
        state: song(
            126.0,
            0.0,
            patches::patch([patches::SQUARE, 0.5, 0.46, 0.26, 0.42, 0.52, 0.4, 0.44]),
            main,
            fill,
        ),
    }
}

/// The ghosts are the line: a funk bass plays around the beat as much as on it.
fn funk() -> Demo {
    let main = line(&[
        (0, Pitch::E, 1, "a"),
        (1, Pitch::E, 1, "g"),
        (2, Pitch::E, 1, ""),
        (4, Pitch::E, 1, ""),
        (6, Pitch::G, 1, "g"),
        (7, Pitch::E, 1, "s"),
        (8, Pitch::E, 1, "a"),
        (9, Pitch::E, 1, "g"),
        (10, Pitch::E, 1, ""),
        (12, Pitch::B, 1, ""),
        (13, Pitch::E, 1, "g"),
        (14, Pitch::E, 2, ""),
        (15, Pitch::E, 2, "s"),
    ]);
    let fill = line(&[
        (0, Pitch::E, 1, "a"),
        (2, Pitch::E, 1, "g"),
        (3, Pitch::E, 1, ""),
        (4, Pitch::E, 1, "s"),
        (6, Pitch::E, 1, "g"),
        (8, Pitch::E, 2, "a"),
        (10, Pitch::E, 1, "g"),
        (11, Pitch::E, 1, ""),
        (12, Pitch::G, 1, ""),
        (13, Pitch::E, 1, "g"),
        (14, Pitch::E, 1, ""),
        (15, Pitch::B, 1, "s"),
    ]);
    Demo {
        name: "Funk",
        state: song(
            104.0,
            0.12,
            patches::patch([patches::SAW, 0.5, 0.42, 0.58, 0.62, 0.34, 0.56, 0.88]),
            main,
            fill,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use acid_core::pitch::{NOTE_HIGH, NOTE_LOW, step_midi};
    use acid_core::timing::{MAX_BPM, MIN_BPM};

    /// Every demo's eight patterns, by slot.
    fn bank(demo: &Demo) -> Vec<Pattern> {
        (0..8).map(|slot| *demo.state.bank.get(slot)).collect()
    }

    #[test]
    fn there_are_five_demos_and_no_two_share_a_name() {
        let demos = songs();
        assert_eq!(demos.len(), 5);
        let mut names: Vec<&str> = demos.iter().map(|demo| demo.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 5, "two demos share a name");
    }

    #[test]
    fn every_demo_plays_something_the_engine_can_reach() {
        for demo in songs() {
            let patterns = bank(&demo);
            for (slot, pattern) in patterns.iter().enumerate() {
                let sounding = pattern.steps.iter().filter(|step| step.on).count();
                assert!(sounding > 0, "{} A{} is empty", demo.name, slot + 1);
                for step in pattern.steps.iter().filter(|step| step.on) {
                    let midi = step_midi(step.pitch, step.octave_offset);
                    assert!(
                        (NOTE_LOW..=NOTE_HIGH).contains(&midi),
                        "{} A{} plays MIDI {midi}, outside the instrument",
                        demo.name,
                        slot + 1
                    );
                }
            }
            // The genre lives in the performance as much as the notes: a bank with no accent sounds like a test tone.
            let accented = patterns
                .iter()
                .flat_map(|pattern| pattern.steps)
                .any(|step| step.on && step.accent);
            assert!(accented, "{} has no accent anywhere", demo.name);
        }
    }

    #[test]
    fn a_demos_eight_variations_are_all_different() {
        for demo in songs() {
            let patterns = bank(&demo);
            for (a, first) in patterns.iter().enumerate() {
                for second in patterns.iter().skip(a + 1) {
                    assert_ne!(
                        first, second,
                        "{} has two identical patterns in its bank",
                        demo.name
                    );
                }
            }
        }
    }

    #[test]
    fn the_five_demos_are_five_lines() {
        let firsts: Vec<Pattern> = songs().iter().map(|demo| *demo.state.bank.get(0)).collect();
        for (a, first) in firsts.iter().enumerate() {
            for second in firsts.iter().skip(a + 1) {
                assert_ne!(first, second, "two demos start from the same phrase");
            }
        }
    }

    #[test]
    fn every_demo_patch_is_inside_the_engine_and_its_own() {
        let demos = songs();
        for demo in &demos {
            let state = &demo.state;
            assert!(
                (MIN_BPM..=MAX_BPM).contains(&state.bpm),
                "{} is out of tempo",
                demo.name
            );
            assert!(
                (0.0..=1.0).contains(&state.swing),
                "{} swings out of range",
                demo.name
            );
            assert!(
                state.knobs.waveform <= 3,
                "{} names a wave that is not there",
                demo.name
            );
            assert!((0.0..=1.0).contains(&state.knobs.resonance));
            assert_eq!(state.song, None, "{} claims a file", demo.name);
        }
        // Two demos on the same patch would make the genre list a lie.
        for (a, first) in demos.iter().enumerate() {
            for second in demos.iter().skip(a + 1) {
                assert_ne!(
                    (first.state.knobs, first.state.bpm),
                    (second.state.knobs, second.state.bpm),
                    "{} and {} play the same patch",
                    first.name,
                    second.name
                );
            }
        }
    }

    #[test]
    fn a_first_run_opens_on_the_slow_acid_line_under_a_new_name() {
        let seeded = seed();
        assert_eq!(seeded.song.as_deref(), Some(NEW_SONG));
        assert_eq!(
            seeded.bank.get(0),
            songs()[0].state.bank.get(0),
            "the seed is the first demo"
        );
        assert_eq!(songs()[0].name, "Slow Acid");
    }
}
