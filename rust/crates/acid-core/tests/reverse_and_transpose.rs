//! The bottom row's two engine behaviours: a mirrored read, and a transpose that is not the saved knob.

use std::sync::Arc;

use acid_core::engine::Engine;
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

const SR: f32 = 48_000.0;

/// Step 0 silent, step 15 a note: played forwards the first step is a rest, played backwards it is the note.
fn last_step_only() -> Arc<Pattern> {
    let mut steps = [Step::note(Pitch::A, -1); 16];
    steps[0] = Step::rest();
    Arc::new(Pattern::from_steps(steps))
}

#[test]
fn a_reverse_read_sounds_the_other_end_of_the_pattern_on_the_same_step() {
    let mut forward = Engine::new(SR);
    forward.set_pattern(last_step_only());
    forward.start_at(0);
    let mut buf = vec![0.0; 2_048];
    forward.render(&mut buf);
    assert!(
        !forward.is_playing_something(),
        "forwards, step 0 is the rest it is written as"
    );

    let mut backward = Engine::new(SR);
    backward.set_pattern(last_step_only());
    backward.set_reverse(true);
    backward.start_at(0);
    let mut buf = vec![0.0; 2_048];
    backward.render(&mut buf);
    assert!(
        backward.is_playing_something(),
        "backwards, step 0 reads step 15, which is a note"
    );
}

#[test]
fn reverse_is_a_direction_and_leaves_the_clock_alone() {
    let mut engine = Engine::new(SR);
    engine.set_pattern(last_step_only());
    engine.start_at(9);
    engine.set_reverse(true);
    assert_eq!(
        engine.position(),
        9,
        "flipping the direction does not move the step the clock is on"
    );
}

#[test]
fn the_transpose_sits_on_top_of_the_knob_and_clamps_to_an_octave() {
    let mut engine = Engine::new(SR);
    engine.set_tuning(0.5);
    let knob_only = engine.tuning();

    engine.set_transpose(7);
    assert_eq!(engine.transpose(), 7);
    assert!(
        (engine.tuning() - knob_only).abs() < 1e-6,
        "the knob is the patch: the transpose does not rewrite it"
    );

    engine.set_transpose(99);
    assert_eq!(engine.transpose(), 12, "an octave up is the ceiling");
    engine.set_transpose(-99);
    assert_eq!(engine.transpose(), -12, "and an octave down the floor");
    engine.set_transpose(0);
    assert_eq!(engine.transpose(), 0);
}
