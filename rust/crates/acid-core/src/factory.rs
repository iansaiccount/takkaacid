//! The factory line: what plays the first time the app is opened.
//!
//! `A1` is the launcher — a real acid phrase, not a scale: a driving 16th rhythm, four accents, and two
//! `A1` is the launcher — a real acid phrase, not a scale: a driving 16th rhythm, four accents, and two
//! slides that can be heard: G slurring down into E flat at step 7, and G into the accented downbeat. `A2`–`A8`
//! are that phrase displaced within the bar, so they share its pitch material (C, E♭, G, B♭) exactly; they are
//! starting points to edit, not finished variations.

use crate::pattern::{Pattern, STEPS_PER_PATTERN, Step};
use crate::pitch::Pitch;

/// The launcher phrase, note for note.
pub fn launcher() -> Pattern {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    steps[0] = Step {
        accent: true,
        ..Step::note(Pitch::C, 0)
    };
    steps[1] = Step::note(Pitch::C, 0);
    steps[3] = Step::note(Pitch::DSharp, 0);
    // The slides sit where they can be heard: a slide glides into the *note* on the next step, so a slide
    // before a rest closes the gate and does nothing.
    steps[5] = Step {
        accent: true,
        slide: true,
        ..Step::note(Pitch::G, 0)
    };
    steps[6] = Step::note(Pitch::DSharp, 0);
    steps[8] = Step {
        accent: true,
        ..Step::note(Pitch::C, 0)
    };
    steps[10] = Step::note(Pitch::ASharp, 0);
    steps[12] = Step::note(Pitch::C, 0);
    steps[13] = Step {
        accent: true,
        ..Step::note(Pitch::C, 0)
    };
    steps[15] = Step {
        slide: true,
        ..Step::note(Pitch::G, 0)
    };
    Pattern::from_steps(steps)
}

/// A phrase moved `by` sixteenths later within the bar.
fn displaced(base: &Pattern, by: usize) -> Pattern {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    for (index, slot) in steps.iter_mut().enumerate() {
        *slot = base.steps[(index + by) % STEPS_PER_PATTERN];
    }
    Pattern::from_steps(steps)
}

/// Bank A: the launcher plus seven displacements of it.
pub fn factory_patterns() -> [Pattern; 8] {
    let base = launcher();
    [
        base,
        displaced(&base, 2),
        displaced(&base, 4),
        displaced(&base, 6),
        displaced(&base, 8),
        displaced(&base, 10),
        displaced(&base, 12),
        displaced(&base, 14),
    ]
}
