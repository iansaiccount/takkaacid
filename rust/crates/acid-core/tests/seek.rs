//! Seeking the sequencer, and starting it part-way through a block: a device joining a **paused** rack lands on the step the
//! resume will use, and a device joining a **playing** one is held silent until the bar line and begins on that sample.

use std::sync::Arc;

use acid_core::engine::Engine;
use acid_core::factory::starter;
use acid_core::pattern::Grid;

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
    e.start_at(Grid::Sixteenth.steps_per_bar() + 3);
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

/// A device joining a rack mid-block is told to begin on the bar line's own sample, so its first step and the rack's are
/// the same sample rather than a block apart.
#[test]
fn a_start_delayed_inside_a_block_begins_on_the_sample_it_was_given() {
    let mut e = Engine::new(SR);
    e.set_pattern(Arc::new(starter()));
    let mut block = vec![0.0; 1_024];
    // 100 samples of silence first: the start cannot land at sample 0, and nothing may begin before the block's head.
    e.start_later(0, 100);
    e.render(&mut block);
    assert_eq!(e.position(), 0);
    assert!(e.is_playing());
    // ...and the step it was given sounds from the hundredth sample on: the start happened, and at the sample it named.
    assert!(
        block[..100].iter().all(|sample| *sample == 0.0),
        "nothing sounds before the sample the start was given"
    );
    assert!(
        block[100..].iter().any(|sample| *sample != 0.0),
        "the step must sound from the sample the start was given, not later and not never"
    );
}

/// A start given a delay longer than the block waits for the next one rather than beginning early.
#[test]
fn a_delayed_start_that_does_not_fit_waits_for_the_next_block() {
    let mut e = Engine::new(SR);
    e.set_pattern(Arc::new(starter()));
    let mut block = vec![0.0; 256];
    e.start_later(0, 600);
    e.render(&mut block);
    assert!(
        block.iter().all(|sample| *sample == 0.0),
        "600 samples of wait, 256 rendered: the first block is silence"
    );
    // 344 samples of the wait are left, so step 0 sounds inside this block.
    let mut rest = vec![0.0; 512];
    e.render(&mut rest);
    assert!(
        rest.iter().any(|sample| *sample != 0.0),
        "the step begins once the wait is over"
    );
    assert_eq!(e.position(), 0);
}

/// The mixer releases a waiting device on the bar line, so the sequencer has to say where in the block it fell.
#[test]
fn a_bar_line_is_reported_at_the_sample_it_fell_on() {
    let mut e = Engine::new(SR);
    e.set_pattern(Arc::new(starter()));
    let step_len = (SR * 60.0 / (130.0 * 4.0)).round() as usize;
    // A whole bar in blocks of `step_len`, so the wrap lands exactly on the last block's end.
    assert_eq!(
        e.bar_line(),
        None,
        "a bar line is announced by the block it is rendered in, not before"
    );
    e.start();
    let mut block = vec![0.0; step_len];
    // Starting on the downbeat is a bar line too: that is the one a device waiting to join after a stop is released on.
    e.render(&mut block);
    assert_eq!(
        e.bar_line(),
        Some(0),
        "a start on step 0 is a bar line at the block's head"
    );
    for _ in 1..Grid::Sixteenth.steps_per_bar() {
        e.render(&mut block);
        assert_eq!(
            e.bar_line(),
            None,
            "no bar line inside the first fifteen steps"
        );
    }
    // The sixteenth block carries the wrap, and the wrap lands where the step's sample count leaves it: a sixteenth at 130
    // BPM is 5 538.46 samples, so sixteen blocks of 5 538 run 7.4 samples short of a bar and the line falls on the eighth.
    e.render(&mut block);
    let at = e.bar_line().expect("the wrap to step 0 is a bar line");
    assert_eq!(
        at, 8,
        "the wrap falls eight samples into the sixteenth block: {at}"
    );
}
