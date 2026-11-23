//! The sequencer: it owns the step clock and turns pattern steps into voice events.
//!
//! Sample-accuracy is the point. `render` splits a block at every step boundary, so a note starts on the
//! exact sample the clock says it should, whatever block size the audio device asks for.

use acid_core::pattern::{Grid, Pattern, Step};
use acid_core::pitch::Pitch;
use acid_core::sequencer::Sequencer;
use acid_core::voice::Voice;
use std::sync::Arc;

fn pattern_with(indices: &[(usize, Step)]) -> Arc<Pattern> {
    let mut p = Pattern::empty();
    for (i, s) in indices {
        p.steps[*i] = *s;
    }
    Arc::new(p)
}

#[test]
fn a_single_note_reaches_the_voice_on_step_zero() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(pattern_with(&[(0, Step::note(Pitch::A, -1))]));
    seq.set_bpm(120.0);
    let mut v = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; 4_800];
    seq.render(&mut v, &mut buf);
    assert!(acid_core::analysis::rms(&buf[1_000..]) > 0.01);
}

#[test]
fn rests_close_the_gate_for_exactly_their_step() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(pattern_with(&[(0, Step::note(Pitch::A, 0))])); // steps 1..15 are rests
    seq.set_bpm(120.0);
    let mut v = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; 24_000]; // two steps
    seq.render(&mut v, &mut buf);
    let first = acid_core::analysis::rms(&buf[2_000..10_000]);
    let second = acid_core::analysis::rms(&buf[13_000..23_000]);
    assert!(first > 0.01, "first step silent");
    assert!(second < first * 0.1, "second step should be a rest");
}

#[test]
fn position_wraps_at_sixteen_and_stop_closes_the_gate() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(Arc::new(Pattern::empty()));
    seq.set_bpm(300.0);
    seq.start();
    let mut v = Voice::new(sr);
    seq.render(&mut v, &mut vec![0.0; 4_800]);
    assert!(seq.position() < 16);
    seq.stop();
    assert!(!seq.is_running());
    let mut buf = vec![0.0; 4_800];
    seq.render(&mut v, &mut buf);
    assert!(acid_core::analysis::rms(&buf) < 0.001);
}

#[test]
fn a_twelve_step_pattern_wraps_at_twelve() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(Arc::new(Pattern::from_steps_in(
        Grid::EighthTriplet,
        [Step::note(Pitch::A, 0); 12],
    )));
    seq.set_bpm(120.0);
    seq.start();
    let mut v = Voice::new(sr);
    // One bar at 120 BPM is 96 000 samples, and a twelfth of it is a whole 8000: no float ambiguity.
    let step = acid_core::timing::samples_per_bar(120.0, sr) / 12.0;
    seq.render(&mut v, &mut vec![0.0; 11 * step as usize + 1]);
    assert_eq!(seq.position(), 11, "eleven steps into a twelve-step bar");
    seq.render(&mut v, &mut vec![0.0; step as usize + 1]);
    assert_eq!(
        seq.position(),
        0,
        "the twelfth wraps to the top of the bar, not to a thirteenth step"
    );
    assert_eq!(seq.clock().step_index(), 12, "one bar is twelve steps");
}

#[test]
fn a_bar_line_arrives_after_one_bar_at_every_grid() {
    let sr = 48_000.0;
    let bar = acid_core::timing::samples_per_bar(120.0, sr) as usize;
    for grid in Grid::ALL {
        let mut seq = Sequencer::new(sr);
        seq.set_pattern(Arc::new(Pattern::empty_in(grid)));
        seq.set_bpm(120.0);
        seq.start();
        let mut v = Voice::new(sr);
        seq.render(&mut v, &mut vec![0.0; bar]);
        assert_eq!(
            seq.bar_line(),
            Some(bar),
            "{}: a bar is a bar, so its line lands at the same sample",
            grid.name()
        );
    }
}

#[test]
fn sounding_step_mirrors_within_the_count() {
    use acid_core::sequencer::sounding_step;
    assert_eq!(sounding_step(0, false, 12), 0);
    assert_eq!(sounding_step(11, false, 12), 11);
    assert_eq!(
        sounding_step(0, true, 12),
        11,
        "a twelve-step bar reads from 11"
    );
    assert_eq!(sounding_step(11, true, 12), 0);
    assert_eq!(sounding_step(0, true, 32), 31);
    assert_eq!(sounding_step(31, true, 32), 0);
}
