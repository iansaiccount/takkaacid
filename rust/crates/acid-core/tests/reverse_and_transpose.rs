//! The bottom row's two engine behaviours: a mirrored read, and a transpose that is not the saved knob.

use std::sync::Arc;

use acid_core::engine::Engine;
use acid_core::pattern::{Pattern, STEPS_PER_PATTERN, Step};
use acid_core::pitch::Pitch;
use acid_core::timing;

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

/// Step 0 a note, everything else a rest: forwards the bar opens on a note, backwards it opens on a rest.
fn first_step_only() -> Arc<Pattern> {
    let mut steps = [Step::rest(); 16];
    steps[0] = Step::note(Pitch::A, -1);
    Arc::new(Pattern::from_steps(steps))
}

/// A whole bar at the engine's tempo, in samples, plus a step to land on the next downbeat.
fn bar_and_a_step(engine: &Engine) -> usize {
    let step = timing::step_duration_samples(0, engine.bpm(), SR, 0.0);
    (step * (STEPS_PER_PATTERN + 1) as f64).round() as usize
}

/// The player's requirement: a direction asked for mid-bar waits for the bar line, so the bar it was asked in plays in the
/// direction it began in and the next one plays the other way.
#[test]
fn a_direction_asked_for_mid_bar_waits_for_the_bar_line() {
    let mut engine = Engine::new(SR);
    engine.set_pattern(first_step_only());
    engine.start_at(0);

    // Two steps of the bar, then ask to go backwards.
    let step = timing::step_duration_samples(0, engine.bpm(), SR, 0.0) as usize;
    let mut two_steps = vec![0.0; step * 2];
    engine.render(&mut two_steps);
    engine.set_reverse(true);
    assert!(
        engine.reverse_is_pending(),
        "a running transport takes the direction at the bar line, not at once"
    );

    // Finish the bar and step onto the next downbeat: forwards, that downbeat is the note at step 0.
    let rest = bar_and_a_step(&engine) - step * 2;
    let mut out = vec![0.0; rest];
    engine.render(&mut out);
    assert!(
        !engine.is_playing_something(),
        "the new bar opens backwards, so step 0 reads the rest at the far end"
    );
    assert!(
        !engine.reverse_is_pending(),
        "and the direction is no longer waiting"
    );
}

/// And the same bar, played without the flip, opens on its note — the control for the test above.
#[test]
fn the_same_bar_forwards_opens_on_its_note() {
    let mut engine = Engine::new(SR);
    engine.set_pattern(first_step_only());
    engine.start_at(0);
    let step = timing::step_duration_samples(0, engine.bpm(), SR, 0.0) as usize;
    let mut two_steps = vec![0.0; step * 2];
    engine.render(&mut two_steps);

    // Onto the downbeat and a little past it: a whole step further would render past the note's release and the control
    // would be asserting silence for the wrong reason.
    let step_len = timing::step_duration_samples(0, engine.bpm(), SR, 0.0);
    let to_downbeat = (step_len * (STEPS_PER_PATTERN as f64 - 2.0)).round() as usize + 64;
    let mut out = vec![0.0; to_downbeat];
    engine.render(&mut out);
    assert_eq!(engine.position(), 0, "the next downbeat");
    assert!(
        engine.is_playing_something(),
        "forwards, the downbeat is the note at step 0"
    );
}
