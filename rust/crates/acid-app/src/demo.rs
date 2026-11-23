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
use acid_core::drums::Drum;
use acid_core::pattern::{Grid, PATTERN_SLOTS, Pattern, Step};
use acid_core::pitch::Pitch;

use crate::persist::{Knobs, SAVE_VERSION, SavedState};

/// One demo: the name the card shows, the song it loads, and the drum machine's version of it.
pub struct Demo {
    pub name: &'static str,
    pub state: SavedState,
    /// The drum machine's side of this variation: eight one-bar grooves in `A1`..`A8`, the drums this line plays over. The
    /// grooves travel with the name, so a drum bank cannot be listed under a demo that has none, and a groove cannot outlive
    /// the demo it was written for.
    pub drums: [Pattern; 8],
}

/// Every demo, in the order the card lists them.
pub fn songs() -> Vec<Demo> {
    vec![
        slow_acid(),
        hard_techno(),
        blues(),
        rock(),
        funk(),
        acid_jazz(),
        fast_acid(),
    ]
}

/// The drum side of every demo, in the demos' own order, as the bank a drum device loads from `DEMO BANKS`.
///
/// The names are [`songs`]' own — one list, not a second kept beside it — so the song a player hears and the groove it
/// plays over cannot be renamed out from under each other, and the whole-rack document this phase leaves for later pairs
/// them by name without a table to keep in step.
pub fn drum_banks() -> Vec<(&'static str, PatternBank)> {
    songs()
        .into_iter()
        .map(|demo| (demo.name, bank_of(&demo.drums)))
        .collect()
}

/// Eight patterns as the bank a device loads: `A1`..`A8`, and the other seven banks empty.
fn bank_of(drums: &[Pattern; 8]) -> PatternBank {
    let mut bank = PatternBank::new_empty();
    for (slot, pattern) in drums.iter().enumerate() {
        bank.replace(slot, *pattern);
    }
    bank
}

/// Put one of the demos on a drum machine: its eight patterns into the device's bank, and the tempo and shuffle it is
/// played at.
///
/// A bank is patterns only — the kit, the knobs and the session's name are the player's — but a *demo* is not only its
/// patterns, and a groove at another demo's tempo is not the demo it is named after. Answers the demo's name when
/// `index` named one, which is what the status line says.
/// The device has to be a drum machine: a bank of masks played by the 303 is silence, so `main` calls this from its
/// `DeviceKind::Drums` arm and nowhere else.
pub fn load_drum_bank(rack: &mut crate::device::Rack, index: usize) -> Option<&'static str> {
    let demo = songs().into_iter().nth(index)?;
    let device = rack.device_mut();
    device.load_bank(bank_of(&demo.drums));
    // The shuffle is the device's own, like the 303's — a song sets it from the same field — and a drum machine is built
    // at 0.0, so without this the blues demo arrives straight rather than as the shuffle it was written to be.
    device.state.swing.set(demo.state.swing);
    rack.set_tempo_bpm(demo.state.bpm);
    Some(demo.name)
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
fn line(grid: Grid, hits: &[Hit]) -> Pattern {
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    for &(at, pitch, octave, flags) in hits {
        let mut step = Step::note(pitch, octave);
        step.accent = flags.contains('a');
        step.slide = flags.contains('s');
        if flags.contains('g') {
            step.velocity = GHOST;
        }
        steps[at] = step;
    }
    Pattern::from_steps_in(grid, steps)
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
    let mut pattern = pattern;
    let mut sounding = 0;
    // Only the bar: a slot above `len()` is a rest, and the pattern's own grid is what the variation must come back on.
    for step in pattern.iter_mut() {
        if step.on {
            sounding += 1;
            // The downbeat stays, the next note goes, and so on through the bar: half the line, in the same places.
            if sounding % 2 == 0 {
                *step = Step::rest();
            }
        }
    }
    pattern
}

/// One drum voice's strikes in a bar: which steps it hits, and which of those are accented.
///
/// The steps are written as numbers because a bar is sixteen steps and a groove is where its strikes fall; the voices are
/// always from [`Drum`], which is the one catalogue of them.
type Strike = (Drum, &'static [usize], &'static [usize]);

/// A bar of drums from a table of strikes; every step the table does not name is a rest.
///
/// Every strike goes through [`Step::hit`], the only writer of the two masks and of `on`, so a groove cannot draw a cell
/// the machine does not sound — a table written with `Step::note` would be a bar of rests with a grid that looked full.
fn bar(grid: Grid, strikes: &[Strike]) -> Pattern {
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    for &(drum, hits, accents) in strikes {
        for &at in hits {
            steps[at].hit(drum, accents.contains(&at));
        }
    }
    Pattern::from_steps_in(grid, steps)
}

/// The same bar with one voice taken out: the break a groove needs, without a second table to keep in step.
fn without(pattern: Pattern, drum: Drum) -> Pattern {
    let mut pattern = pattern;
    // Only the bar: every slot above `len()` is a rest, and walking the table would touch slots the bar does not hold.
    for step in pattern.iter_mut() {
        step.clear_hit(drum);
    }
    pattern
}

/// The same bar with one voice also struck on more steps: the busier reading of a groove.
fn also(pattern: Pattern, drum: Drum, hits: &[usize], accented: bool) -> Pattern {
    let mut pattern = pattern;
    for &at in hits {
        // A table may name a step the bar does not hold; the grid decides, and a slot above `len()` stays a rest.
        if let Some(step) = pattern.steps.get_mut(at) {
            step.hit(drum, accented);
        }
    }
    pattern
}

/// The drum side of one demo: the groove, six readings of it and the turnaround, so a bank has all eight slots.
///
/// Transposing is not a drum variation — it moves a lattice the masks do not read — so the eight are the ways a section
/// opens, breaks and turns around: the groove, a crash to open on, the hats out, the kick out, a shaker on the offbeats,
/// the turnaround, and two readings of the turnaround. Every variation keeps the downbeat except the kick-out break, whose
/// whole point is to lose it — that is only true because the groove it is taken from has nothing else on its first step —
/// and that each of them sounds at all is the test's to hold.
fn drum_variations(groove: Pattern, turnaround: Pattern) -> [Pattern; 8] {
    // The extra hits are placed **by the bar**, not at fixed sixteenths: "every fourth step from step 4" is a shaker on
    // the offbeats of a sixteen-step bar, of a twelve-step triplet bar and of a thirty-second one alike, and "the last
    // three steps" lifts the snare wherever the bar ends. At `1/16` these are the numbers this table used to spell.
    let len = groove.len();
    let offbeats: Vec<usize> = (3..len).step_by(4).collect();
    let lift: Vec<usize> = (len.saturating_sub(3)..len).collect();
    [
        groove,
        also(groove, Drum::Crash, &[0], true),
        without(groove, Drum::ClosedHat),
        without(groove, Drum::Kick),
        also(groove, Drum::Shaker, &offbeats, false),
        turnaround,
        without(turnaround, Drum::Snare),
        also(turnaround, Drum::Snare, &lift, true),
    ]
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
    let main = line(
        Grid::Sixteenth,
        &[
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
        ],
    );
    let fill = line(
        Grid::Sixteenth,
        &[
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
        ],
    );

    // The drums four to the floor, with the clap on two and four and a closed hat on the offbeats: the slow acid records'
    // own pulse, sparse enough that the slides are still what a player hears.
    let groove = bar(
        Grid::Sixteenth,
        &[
            (Drum::Kick, &[0, 4, 8, 12], &[0]),
            (Drum::Clap, &[4, 12], &[12]),
            (Drum::ClosedHat, &[2, 6, 10, 14], &[6]),
        ],
    );
    let turnaround = bar(
        Grid::Sixteenth,
        &[
            (Drum::Kick, &[0, 8], &[0]),
            (Drum::Snare, &[4, 12], &[]),
            (Drum::ClosedHat, &[2, 6, 10], &[]),
            (Drum::LowTom, &[13, 14], &[14]),
            (Drum::HighTom, &[15], &[15]),
        ],
    );
    Demo {
        name: "Slow Acid",
        state: song(
            118.0,
            0.0,
            patches::patch([patches::SAW, 0.5, 0.35, 0.78, 0.72, 0.46, 0.52, 0.87]),
            main,
            fill,
        ),
        drums: drum_variations(groove, turnaround),
    }
}

/// The same box at the other end: short, bright, and played as a chug with an octave jump every beat.
fn hard_techno() -> Demo {
    let main = line(
        Grid::Sixteenth,
        &[
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
        ],
    );
    let fill = line(
        Grid::Sixteenth,
        &[
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
        ],
    );

    // Sixteenth hats under the kick, the clap on two and four and a rim on the offbeats: the same chug the line plays, at the
    // tempo the demo calls hard techno.
    let groove = bar(
        Grid::Sixteenth,
        &[
            (Drum::Kick, &[0, 4, 8, 12], &[0, 8]),
            (Drum::Clap, &[4, 12], &[12]),
            (
                Drum::ClosedHat,
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                &[2, 6, 10, 14],
            ),
            (Drum::Rim, &[2, 10], &[]),
        ],
    );
    let turnaround = bar(
        Grid::Sixteenth,
        &[
            (Drum::Kick, &[0, 4, 8], &[0]),
            (Drum::Clap, &[4], &[]),
            (Drum::Snare, &[14, 15], &[]),
            (Drum::ClosedHat, &[0, 2, 4, 6, 8, 10, 12], &[2, 10]),
            (Drum::LowTom, &[11], &[]),
        ],
    );
    Demo {
        name: "Hard Techno Acid",
        state: song(
            142.0,
            0.0,
            patches::patch([patches::SAW, 0.5, 0.58, 0.86, 0.88, 0.28, 0.66, 0.95]),
            main,
            fill,
        ),
        drums: drum_variations(groove, turnaround),
    }
}

/// A shuffle comes from the clock, not the notes: the walk is on the eighths and SWING does the rest.
fn blues() -> Demo {
    let main = line(
        Grid::Sixteenth,
        &[
            (0, Pitch::A, 1, "a"),
            (2, Pitch::A, 1, ""),
            (4, Pitch::C, 2, ""),
            (6, Pitch::CSharp, 2, ""),
            (8, Pitch::D, 2, "a"),
            (10, Pitch::DSharp, 2, ""),
            (12, Pitch::E, 2, ""),
            (14, Pitch::D, 2, ""),
        ],
    );
    let fill = line(
        Grid::Sixteenth,
        &[
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
        ],
    );

    // A shuffle is the clock's (this demo swings 0.35), so the ride walks the eighths straight on and the hats sit on the
    // last sixteenth of each beat: the drums lean where the walk leans.
    let groove = bar(
        Grid::Sixteenth,
        &[
            (Drum::Ride, &[0, 2, 4, 6, 8, 10, 12, 14], &[0, 6, 12]),
            (Drum::Kick, &[0, 8], &[0]),
            (Drum::Snare, &[4, 12], &[]),
            (Drum::ClosedHat, &[3, 7, 11, 15], &[]),
        ],
    );
    let turnaround = bar(
        Grid::Sixteenth,
        &[
            (Drum::Ride, &[0, 2, 4, 6], &[0]),
            (Drum::Kick, &[0], &[0]),
            (Drum::Snare, &[4, 12], &[]),
            (Drum::LowTom, &[13], &[]),
            (Drum::MidTom, &[14], &[]),
            (Drum::Crash, &[15], &[15]),
        ],
    );
    Demo {
        name: "Blues",
        state: song(
            92.0,
            0.35,
            patches::patch([patches::SQUARE, 0.5, 0.3, 0.34, 0.36, 0.6, 0.3, 0.47]),
            main,
            fill,
        ),
        drums: drum_variations(groove, turnaround),
    }
}

/// Eighths, root and fifth, and one slide into the downbeat: the bass part of a rock band.
fn rock() -> Demo {
    let main = line(
        Grid::Sixteenth,
        &[
            (0, Pitch::E, 1, "a"),
            (2, Pitch::E, 1, ""),
            (4, Pitch::B, 1, ""),
            (6, Pitch::E, 1, ""),
            (8, Pitch::E, 1, "a"),
            (10, Pitch::B, 1, ""),
            (12, Pitch::E, 2, ""),
            (14, Pitch::E, 1, "s"),
        ],
    );
    let fill = line(
        Grid::Sixteenth,
        &[
            (0, Pitch::E, 1, "a"),
            (2, Pitch::E, 1, ""),
            (4, Pitch::E, 2, ""),
            (6, Pitch::E, 1, ""),
            (8, Pitch::B, 1, "a"),
            (10, Pitch::E, 1, ""),
            (12, Pitch::E, 1, "s"),
            (14, Pitch::E, 2, ""),
        ],
    );

    // A backbeat: the kick on one and the "and" of two, the snare on two and four, eighth hats over it. The rock band's
    // drums, which is what the bass part was written against.
    let groove = bar(
        Grid::Sixteenth,
        &[
            (Drum::Kick, &[0, 6, 8], &[0, 8]),
            (Drum::Snare, &[4, 12], &[12]),
            (Drum::ClosedHat, &[0, 2, 4, 6, 8, 10, 12, 14], &[]),
        ],
    );
    let turnaround = bar(
        Grid::Sixteenth,
        &[
            (Drum::Kick, &[0, 8], &[0]),
            (Drum::Snare, &[4], &[]),
            (Drum::ClosedHat, &[0, 2, 4, 6, 8], &[]),
            (Drum::MidTom, &[12, 13], &[]),
            (Drum::LowTom, &[14], &[]),
        ],
    );
    Demo {
        name: "Rock",
        state: song(
            126.0,
            0.0,
            patches::patch([patches::SQUARE, 0.5, 0.46, 0.26, 0.42, 0.52, 0.4, 0.44]),
            main,
            fill,
        ),
        drums: drum_variations(groove, turnaround),
    }
}

/// The ghosts are the line: a funk bass plays around the beat as much as on it.
fn funk() -> Demo {
    let main = line(
        Grid::Sixteenth,
        &[
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
        ],
    );
    let fill = line(
        Grid::Sixteenth,
        &[
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
        ],
    );

    // The syncopation is the point: the kick around the beat rather than on it, sixteenth hats with the accents on the
    // offbeats, a clave on the last sixteenth of two, and an open hat to close the bar.
    let groove = bar(
        Grid::Sixteenth,
        &[
            (Drum::Kick, &[0, 3, 6, 10], &[0]),
            (Drum::Snare, &[4, 12], &[12]),
            (
                Drum::ClosedHat,
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
                &[2, 6, 10, 14],
            ),
            (Drum::OpenHat, &[15], &[15]),
            (Drum::Clave, &[7], &[]),
        ],
    );
    let turnaround = bar(
        Grid::Sixteenth,
        &[
            (Drum::Kick, &[0, 6], &[0]),
            (Drum::Snare, &[4], &[]),
            (Drum::ClosedHat, &[0, 2, 4, 6, 8, 10], &[2, 10]),
            (Drum::LowConga, &[11], &[]),
            (Drum::HighConga, &[13], &[]),
        ],
    );
    Demo {
        name: "Funk",
        state: song(
            104.0,
            0.12,
            patches::patch([patches::SAW, 0.5, 0.42, 0.58, 0.62, 0.34, 0.56, 0.88]),
            main,
            fill,
        ),
        drums: drum_variations(groove, turnaround),
    }
}

/// The triplet grid *is* the swing: a walking line over a ride on every quarter, with the shuffle knob left at zero.
///
/// Twelve steps to the bar, so a "sixteenth" here is an eighth-note triplet — the line walks in threes and the ride
/// marks the four beats at steps 1, 4, 7 and 10.
fn acid_jazz() -> Demo {
    let main = line(
        Grid::EighthTriplet,
        &[
            (0, Pitch::A, 1, "a"),
            (1, Pitch::C, 2, ""),
            (2, Pitch::E, 2, ""),
            (3, Pitch::D, 2, "s"),
            (4, Pitch::C, 2, ""),
            (5, Pitch::A, 1, ""),
            (6, Pitch::G, 1, "a"),
            (7, Pitch::E, 1, ""),
            (8, Pitch::D, 1, ""),
            (9, Pitch::C, 1, "s"),
            (10, Pitch::A, 1, ""),
            (11, Pitch::G, 1, ""),
        ],
    );
    let fill = line(
        Grid::EighthTriplet,
        &[
            (0, Pitch::A, 1, "a"),
            (2, Pitch::C, 2, ""),
            (3, Pitch::E, 2, "s"),
            (5, Pitch::D, 2, ""),
            (6, Pitch::C, 2, "a"),
            (8, Pitch::A, 1, ""),
            (9, Pitch::G, 1, "s"),
            (11, Pitch::E, 1, ""),
        ],
    );

    // The ride carries the quarter — steps 1, 4, 7 and 10 — with the snare comping late and the kick under the first
    // and the turn into the second half. A hi-hat pedal on two and four, the way a jazz drummer keeps time with the foot,
    // and a brush of shaker through the triplets the line leaves.
    let groove = bar(
        Grid::EighthTriplet,
        &[
            (Drum::Kick, &[0, 7], &[0]),
            (Drum::Snare, &[2, 8], &[8]),
            (Drum::Ride, &[0, 3, 6, 9], &[0]),
            (Drum::ClosedHat, &[3, 9], &[]),
            (Drum::Shaker, &[1, 4, 5, 10], &[]),
        ],
    );
    let turnaround = bar(
        Grid::EighthTriplet,
        &[
            (Drum::Kick, &[0, 9], &[0]),
            (Drum::Snare, &[6, 8, 11], &[11]),
            (Drum::Ride, &[0, 3], &[]),
            (Drum::MidTom, &[10], &[]),
            (Drum::HighTom, &[11], &[]),
        ],
    );
    Demo {
        name: "Acid Jazz",
        state: song(
            140.0,
            0.0,
            patches::patch([patches::SQUARE, 0.6, 0.5, 0.45, 0.55, 0.7, 0.4, 0.9]),
            main,
            fill,
        ),
        drums: drum_variations(groove, turnaround),
    }
}

/// Sixteenth slides broken by thirty-second rolls at 152: the fast one, where the bar is twice as long as it looks.
fn fast_acid() -> Demo {
    let main = line(
        Grid::ThirtySecond,
        &[
            (0, Pitch::E, 1, "a"),
            (2, Pitch::E, 1, ""),
            (4, Pitch::G, 1, "s"),
            (6, Pitch::E, 1, ""),
            (8, Pitch::E, 1, "a"),
            (10, Pitch::C, 2, ""),
            (12, Pitch::E, 2, "s"),
            (14, Pitch::D, 2, ""),
            (16, Pitch::E, 1, "a"),
            (18, Pitch::E, 1, "g"),
            (20, Pitch::G, 1, "s"),
            (22, Pitch::A, 1, ""),
            (24, Pitch::E, 1, "a"),
            (26, Pitch::C, 2, "s"),
            (28, Pitch::B, 1, ""),
            (30, Pitch::E, 1, "s"),
        ],
    );
    // The roll: every thirty-second for the last quarter of the bar, so the fill is the grid showing off.
    let fill = line(
        Grid::ThirtySecond,
        &[
            (0, Pitch::E, 1, "a"),
            (4, Pitch::E, 1, ""),
            (8, Pitch::E, 2, "a"),
            (12, Pitch::G, 1, ""),
            (16, Pitch::E, 1, "s"),
            (20, Pitch::C, 2, ""),
            (24, Pitch::E, 1, "a"),
            (25, Pitch::E, 1, ""),
            (26, Pitch::E, 1, ""),
            (27, Pitch::E, 1, ""),
            (28, Pitch::E, 2, "s"),
            (29, Pitch::E, 2, ""),
            (30, Pitch::E, 2, "s"),
            (31, Pitch::E, 2, ""),
        ],
    );

    // Kick on every eighth — steps 1, 9, 17, 25 — closed hats on the odd thirty-seconds, snare on the second and
    // fourth beats.
    let odd: &[usize] = &[1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31];
    let groove = bar(
        Grid::ThirtySecond,
        &[
            (Drum::Kick, &[0, 8, 16, 24], &[0, 16]),
            (Drum::Snare, &[8, 24], &[24]),
            (Drum::ClosedHat, odd, &[5, 13, 21, 29]),
        ],
    );
    let turnaround = bar(
        Grid::ThirtySecond,
        &[
            (Drum::Kick, &[0, 16], &[0]),
            (Drum::Snare, &[24, 28, 30], &[30]),
            (Drum::ClosedHat, &[8, 12, 20], &[]),
            (Drum::OpenHat, &[31], &[31]),
        ],
    );
    Demo {
        name: "Fast Acid",
        state: song(
            152.0,
            0.0,
            patches::patch([patches::SAW, 0.35, 0.28, 0.82, 0.8, 0.6, 0.58, 0.86]),
            main,
            fill,
        ),
        drums: drum_variations(groove, turnaround),
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
    fn there_are_seven_demos_and_no_two_share_a_name() {
        let demos = songs();
        assert_eq!(demos.len(), 7);
        let mut names: Vec<&str> = demos.iter().map(|demo| demo.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 7, "two demos share a name");
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
