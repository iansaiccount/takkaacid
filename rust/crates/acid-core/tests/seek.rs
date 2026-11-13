//! Seeking the sequencer: a device added while the transport runs joins it in phase instead of restarting at step 0.

use std::sync::Arc;

use acid_core::engine::Engine;
use acid_core::factory::starter;
use acid_core::pattern::STEPS_PER_PATTERN;

/// 48 kHz, so a sixteenth at the launch tempo is 5_538 samples.
const SR: f32 = 48_000.0;

#[test]
fn start_is_start_at_zero() {
    let mut a = Engine::new(SR);
    let mut b = Engine::new(SR);
    a.set_pattern(Arc::new(starter()));
    b.set_pattern(Arc::new(starter()));
    a.start();
    b.start_at(0);
    assert_eq!(a.position(), b.position());
    assert_eq!(a.position(), 0);
}

#[test]
fn start_at_puts_the_sequencer_on_the_step_it_was_given() {
    let mut e = Engine::new(SR);
    e.set_pattern(Arc::new(starter()));
    e.start_at(9);
    assert_eq!(e.position(), 9);
    // A step index past the pattern wraps; the rack may hand it a long count, not an index.
    e.start_at(STEPS_PER_PATTERN + 3);
    assert_eq!(e.position(), 3);
}

#[test]
fn a_seeked_engine_runs_from_there() {
    let mut e = Engine::new(SR);
    e.set_pattern(Arc::new(starter()));
    e.start_at(15);
    // One sixteenth, plus a little: the wrap from 15 to 0 must have happened.
    let step_len = (SR * 60.0 / (130.0 * 4.0)).round() as usize;
    let mut buf = vec![0.0; step_len + 64];
    e.render(&mut buf);
    assert_eq!(e.position(), 0, "step 15 was the last of the bar");
}
