//! Pause is not stop: the clock stops but the step is kept, and a resume carries on from it.

use std::sync::Arc;

use acid_core::engine::Engine;
use acid_core::factory::starter;

/// 48 kHz, so a sixteenth at the launch tempo is 5_538 samples.
const SR: f32 = 48_000.0;

fn engine_at(step: usize) -> Engine {
    let mut engine = Engine::new(SR);
    engine.set_pattern(Arc::new(starter()));
    engine.start_at(step);
    engine
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

/// The clock restarts on a resume, so the first step after a pause is a whole step and not the tail of the one it left.
#[test]
fn a_resume_plays_a_whole_step_before_it_advances() {
    let mut engine = engine_at(7);
    engine.pause();
    engine.resume();
    // A whole sixteenth would advance the step; half of one must not.
    let half = ((SR * 60.0 / (130.0 * 4.0)) / 2.0).round() as usize;
    let mut buf = vec![0.0; half];
    engine.render(&mut buf);
    assert_eq!(
        engine.position(),
        7,
        "half a step into a resumed bar, the step is still the one it resumed on"
    );
}
