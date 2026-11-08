//! Randomising a pattern's notes while keeping its shape.
//!
//! The rhythm is the part a player spends time on, so it survives: a step that is on stays on and keeps its accent, its
//! slide and its velocity. Only the note changes. A rest keeps its note too, so turning a rest into a note later still
//! finds a pitch there.

use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::{NOTE_HIGH, NOTE_LOW, Pitch, step_midi};

/// A whole new pattern: which steps sound, and what they play.
///
/// This is a jump start, not a variation of what is already there, so the gates are rerolled with the notes. What a step
/// keeps is its character — accent, slide, velocity — so a rerolled pattern still has the accent touches and glides the
/// player put there, now carrying new notes.
///
/// Seeded so a result can be reproduced: the panel seeds it from the clock, a test from a number.
pub fn randomised(pattern: Pattern, seed: u64) -> Pattern {
    let mut rng = Rng::new(seed);
    let mut steps = pattern.steps;
    for step in &mut steps {
        // One step in two sounds: dense enough to be a pattern, open enough to hear the shuffle.
        let on = rng.below(2) == 0;
        let pitch = Pitch::ALL[rng.below(Pitch::ALL.len() as u64) as usize];
        *step = Step {
            on,
            pitch,
            octave_offset: nearest_octave(pitch, step.octave_offset),
            ..*step
        };
    }
    // A jump start that lands on silence is not a jump start.
    if !steps.iter().any(|step| step.on) {
        steps[0].on = true;
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
    fn the_gates_and_the_notes_are_both_rerolled() {
        let before = pattern();
        let after = randomised(before, 7);

        assert_ne!(after, before, "nothing was randomised");
        assert!(
            (0..16).any(|index| after.steps[index].on != before.steps[index].on),
            "the gates should move too: it is a jump start, not a variation"
        );
        assert!(
            (0..16).any(|index| after.steps[index].pitch != before.steps[index].pitch),
            "no note changed"
        );
        assert!(
            after.steps.iter().any(|step| step.on),
            "a jump start that lands on silence is not a jump start"
        );
    }

    #[test]
    fn a_steps_own_character_survives_the_reroll() {
        // Accent, slide and velocity are the player's touch; only the gate and the note are rerolled.
        let before = pattern();
        let after = randomised(before, 9);
        for index in 0..16 {
            assert_eq!(after.steps[index].accent, before.steps[index].accent);
            assert_eq!(after.steps[index].slide, before.steps[index].slide);
            assert_eq!(after.steps[index].velocity, before.steps[index].velocity);
        }
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
    fn an_empty_pattern_comes_back_as_a_pattern() {
        // A jump start from silence: the whole point is that something comes out.
        let empty = Pattern::empty();
        let after = randomised(empty, 11);
        assert!(after.steps.iter().any(|step| step.on), "still silent");
        assert!(
            after
                .steps
                .iter()
                .filter(|step| step.on)
                .all(|step| step.audible())
        );
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
    fn a_note_at_the_top_of_the_range_is_pulled_down_rather_than_silenced() {
        // Every step sits at the top of the range, so a pitch class that would overshoot must be moved down an octave —
        // for every seed, not just the one that happens to be lucky.
        let mut steps = [Step::rest(); 16];
        for step in &mut steps {
            *step = Step::note(Pitch::B, 5); // the highest note there is
        }
        for seed in 0..64 {
            let after = randomised(Pattern::from_steps(steps), seed);
            for step in after.steps.iter().filter(|step| step.on) {
                assert!(step.audible(), "seed {seed} left a silent note: {step:?}");
            }
        }
    }
}
