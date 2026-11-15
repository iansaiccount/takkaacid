//! Pause is not stop: the clock stops but the step is kept, and a resume carries on from it.
//!
//! The engine cannot tell the two states apart — both silence the sequencer and keep its step — so what is tested here is the
//! pair of answers: `start` returns to the top of the bar and `resume` does not. Which one a player gets is the app's
//! `Transport`, and this file is the contract it leans on.

use std::sync::Arc;

use acid_core::analysis::rms;
use acid_core::engine::Engine;
use acid_core::factory::starter;
use acid_core::timing;

/// 48 kHz, so a sixteenth at the launch tempo is 5_538 samples.
const SR: f32 = 48_000.0;

fn engine_at(step: usize) -> Engine {
    let mut engine = Engine::new(SR);
    engine.set_pattern(Arc::new(starter()));
    engine.start_at(step);
    engine
}

/// Half a step at the engine's own tempo, from the one place that owns the formula.
fn half_step(engine: &Engine) -> usize {
    (timing::step_duration_samples(0, engine.bpm(), SR, 0.0) / 2.0).round() as usize
}

#[test]
fn a_pause_keeps_the_step_and_a_resume_carries_on_from_it() {
    let mut engine = engine_at(5);
    engine.pause();
    assert!(!engine.is_playing(), "a pause is silent");
    assert_eq!(engine.position(), 5, "but the step is kept");

    engine.resume();
    assert!(engine.is_playing());
    assert_eq!(
        engine.position(),
        5,
        "a resume carries on, it does not return to the top of the bar"
    );
}

/// The difference the round exists for: stop is "the bar again", pause is "from here".
#[test]
fn a_stop_returns_to_the_top_and_a_pause_does_not() {
    let stopped = {
        let mut engine = engine_at(5);
        engine.stop();
        engine.start();
        engine.position()
    };
    let paused = {
        let mut engine = engine_at(5);
        engine.pause();
        engine.resume();
        engine.position()
    };
    assert_eq!(stopped, 0, "RUN starts the bar again");
    assert_eq!(paused, 5, "a resumed pause is not a restart");
}

/// A pause is silent at the audio level, not merely on the transport flag: the tail is faded out and the buffer is quiet.
#[test]
fn a_pause_silences_the_output() {
    let mut engine = engine_at(0);
    let mut before = vec![0.0; half_step(&engine)];
    engine.render(&mut before);
    assert!(rms(&before) > 0.0, "the step was sounding before the pause");

    engine.pause();
    let mut after = vec![0.0; 4_800];
    engine.render(&mut after);
    assert!(
        rms(&after[2_400..]) < 0.001,
        "a paused transport is silent: {}",
        rms(&after[2_400..])
    );
}

/// Half a step, a pause, a resume, half a step again: the step has not moved, and one whole step after the resume it has.
///
/// The half-render *before* the pause is the point. Without it the clock has never been part-stepped, so the test would pass
/// with `resume` doing no clock work at all — which is exactly how the first version of this test failed to fail.
#[test]
fn a_resume_restarts_the_clock_on_the_step_it_kept() {
    let mut engine = engine_at(7);
    let half = half_step(&engine);
    let mut part = vec![0.0; half];
    engine.render(&mut part);
    assert_eq!(engine.position(), 7, "half a step is not a whole one");

    engine.pause();
    engine.resume();
    // **More** than half: a clock that had not restarted would still be holding the half it had when the pause came, and this
    // buffer would cross the boundary and take the step. That is the difference the assertion is about — equal halves do not
    // cross, so they cannot tell a restarted clock from a stopped one.
    let mut again = vec![0.0; half + 64];
    engine.render(&mut again);
    assert_eq!(
        engine.position(),
        7,
        "the clock restarted, so half a step and a little after the resume is still the step it resumed on"
    );

    let whole = (half as f64 * 2.0) as usize;
    let mut rest = vec![0.0; whole];
    engine.render(&mut rest);
    assert_eq!(engine.position(), 8, "and a whole step after it advances");
}

/// A resume re-strikes the step it kept: the note sounds again from the top of that step, not as a continuation of the gate.
///
/// A deliberate deck semantic — pressing play on a paused deck starts the sound again — and the reason `resume` arms rather
/// than merely un-pausing the clock. Judged by ear in task 4; pinned here so it cannot change silently.
#[test]
fn a_resume_strikes_the_step_again() {
    let mut engine = engine_at(3);
    let half = half_step(&engine);
    let mut part = vec![0.0; half];
    engine.render(&mut part);

    engine.pause();
    let mut quiet = vec![0.0; 2_400];
    engine.render(&mut quiet);
    assert!(!engine.is_playing_something(), "a pause closes the gate");

    engine.resume();
    let mut struck = vec![0.0; 512];
    engine.render(&mut struck);
    assert!(
        engine.is_playing_something() && rms(&struck) > 0.0,
        "a resume sounds the step it kept"
    );
}
