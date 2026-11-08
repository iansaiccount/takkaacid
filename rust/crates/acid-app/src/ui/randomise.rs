//! Randomising a pattern's notes while keeping its shape.
//!
//! The rhythm is the part a player spends time on, so it survives: a step that is on stays on and keeps its accent, its
//! slide and its velocity. Only the note changes. A rest keeps its note too, so turning a rest into a note later still
//! finds a pitch there.

use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::{NOTE_HIGH, NOTE_LOW, Pitch, step_midi};

/// A pattern with new notes on the steps that already sound.
///
/// Seeded so a result can be reproduced: the panel seeds it from the clock, a test from a number.
pub fn randomised(pattern: Pattern, seed: u64) -> Pattern {
    let mut rng = Rng::new(seed);
    let mut steps = pattern.steps;
    for step in &mut steps {
        if !step.on {
            continue;
        }
        let pitch = Pitch::ALL[rng.below(Pitch::ALL.len() as u64) as usize];
        *step = Step {
            pitch,
            octave_offset: nearest_octave(pitch, step.octave_offset),
            ..*step
        };
    }
    Pattern::from_steps(steps)
}

/// The octave closest to `wanted` that keeps `pitch` inside the instrument's range.
///
/// A randomiser that produced silent steps would look broken: the note would be drawn, nothing would sound, and the
/// player would blame the engine.
fn nearest_octave(pitch: Pitch, wanted: i8) -> i8 {
    let fits = |octave: i8| {
        let midi = step_midi(pitch, octave);
        (NOTE_LOW..=NOTE_HIGH).contains(&midi)
    };
    let mut octave = wanted;
    // Walk towards the range one octave at a time; at most a few steps, and never a loop that cannot end.
    for _ in 0..8 {
        if fits(octave) || octave == i8::MAX || octave == i8::MIN {
            break;
        }
        octave += if step_midi(pitch, octave) < NOTE_LOW {
            1
        } else {
            -1
        };
    }
    octave
}

/// xorshift64: nothing to depend on, nothing to seed, and good enough to shuffle twelve pitch classes.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        // Zero is a fixed point of xorshift, so a zero seed would randomise nothing at all.
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern() -> Pattern {
        let mut steps = [Step::rest(); 16];
        for (index, pitch) in [Pitch::C, Pitch::D, Pitch::F].into_iter().enumerate() {
            steps[index * 2] = Step {
                accent: index == 1,
                slide: index == 2,
                velocity: 40 + index as u8,
                ..Step::note(pitch, 0)
            };
        }
        Pattern::from_steps(steps)
    }

    #[test]
    fn the_rhythm_survives_and_the_notes_change() {
        let before = pattern();
        let after = randomised(before, 7);

        for index in 0..16 {
            assert_eq!(
                after.steps[index].on, before.steps[index].on,
                "step {index} changed its gate"
            );
            assert_eq!(after.steps[index].accent, before.steps[index].accent);
            assert_eq!(after.steps[index].slide, before.steps[index].slide);
            assert_eq!(after.steps[index].velocity, before.steps[index].velocity);
        }
        assert!(
            (0..16).any(|index| after.steps[index].pitch != before.steps[index].pitch),
            "nothing was randomised"
        );
    }

    #[test]
    fn every_random_note_is_inside_the_range() {
        // Both registers matter: a pattern low in the range can take any pitch class, one at the top cannot — the octave
        // clamp is what stops a randomise from writing notes that would be drawn but never heard.
        let mut top = [Step::rest(); 16];
        for step in &mut top {
            *step = Step::note(Pitch::A, 5);
        }
        for source in [pattern(), Pattern::from_steps(top)] {
            for seed in 0..64 {
                let after = randomised(source, seed);
                for step in after.steps.iter().filter(|step| step.on) {
                    assert!(step.audible(), "seed {seed} wrote a silent note: {step:?}");
                }
            }
        }
    }

    #[test]
    fn rests_keep_their_note_so_turning_one_on_finds_a_pitch() {
        let before = pattern();
        let after = randomised(before, 3);
        for index in 0..16 {
            if !before.steps[index].on {
                assert_eq!(after.steps[index].pitch, before.steps[index].pitch);
            }
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_pattern_and_zero_still_randomises() {
        assert_eq!(randomised(pattern(), 42), randomised(pattern(), 42));
        assert_ne!(randomised(pattern(), 42), randomised(pattern(), 43));
        // A zero seed must not collapse xorshift to a constant: it stands in for the clock, which always moves.
        let zero = randomised(pattern(), 0);
        assert_ne!(zero, pattern(), "a zero seed randomised nothing");
        assert_ne!(zero, randomised(pattern(), 1));
        assert!(randomised(pattern(), 0).steps.iter().any(|step| step.on));
    }

    #[test]
    fn an_empty_pattern_stays_empty() {
        let empty = Pattern::empty();
        assert_eq!(randomised(empty, 11), empty);
    }

    #[test]
    fn a_note_at_the_top_of_the_range_is_pulled_down_rather_than_silenced() {
        let mut steps = [Step::rest(); 16];
        steps[0] = Step::note(Pitch::B, 5); // the highest note there is
        let after = randomised(Pattern::from_steps(steps), 5);
        assert!(after.steps[0].audible());
    }
}
