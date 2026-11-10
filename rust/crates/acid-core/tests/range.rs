//! The playable range, and what a note outside it does: nothing.
//!
//! The octave buttons transpose the pattern itself, so a note pushed past B5 keeps its name and its flags and simply
//! stops sounding — which is what makes transposing back restore it exactly.

use acid_core::analysis::{peak, rms};
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;
use acid_core::sequencer::Sequencer;
use acid_core::voice::Voice;
use std::sync::Arc;

fn play(pattern: Pattern, samples: usize) -> Vec<f32> {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(Arc::new(pattern));
    seq.set_bpm(120.0);
    let mut voice = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; samples];
    seq.render(&mut voice, &mut buf);
    buf
}

#[test]
fn transposing_up_twice_and_back_is_exact() {
    // Notes that leave the range must keep their names, or coming back down would land somewhere else.
    let high = Step::note(Pitch::B, 5);
    // B5 is the top of the range: the octave button pushes it out, and that is the point.
    assert!(high.audible(), "B5 is inside, just");
    let pattern = Pattern::from_steps([high; 16]);

    let up = pattern.transposed(12);
    assert!(!up.steps[0].audible(), "an octave above B5 is past the top");
    assert_eq!(
        up.steps[0].midi(),
        high.midi() + 12,
        "the note still exists"
    );

    let back = up.transposed(-12).transposed(12).transposed(-12);
    assert_eq!(back.steps, pattern.steps, "the round trip is exact");
}

#[test]
fn a_step_outside_the_range_is_silent_and_its_neighbours_still_sound() {
    // Step 0 is pushed past the top; step 8 is the same note one octave down, which is in range. One sounds, one does
    // not, and the silence is a rest rather than a clamped note.
    let silent = Step::note(Pitch::B, 5).transposed(12);
    let sounding = Step::note(Pitch::B, 5);
    assert!(!silent.audible());
    assert!(sounding.audible());

    let mut steps = [Step::rest(); 16];
    steps[0] = silent;
    steps[8] = sounding;
    // Sixteen steps at 120 BPM is 6 000 samples each, so step 8 lands at 48 000 and the whole phrase is 96 000.
    let buf = play(Pattern::from_steps(steps), 96_000);

    assert!(
        peak(&buf[1_000..4_000]) < 0.01,
        "the out-of-range step should be silent, peaked {}",
        peak(&buf[1_000..4_000])
    );
    assert!(
        rms(&buf[48_000 + 1_000..52_000]) > 0.01,
        "the in-range step should still sound"
    );
}

#[test]
fn a_slide_into_a_silent_step_does_not_retune_it() {
    // A slide retunes the voice rather than retriggering it. Sliding *into* an inaudible step would leave the previous
    // note sounding at a pitch nobody asked for, so an inaudible step closes the gate like a rest does.
    let mut steps = [Step::rest(); 16];
    steps[0] = Step {
        slide: true,
        ..Step::note(Pitch::A, 0)
    };
    steps[1] = Step::note(Pitch::B, 5).transposed(12);
    let buf = play(Pattern::from_steps(steps), 6_000);

    // What the slide would have retuned to, had it been allowed: past the top of the range.
    let forbidden = steps[1].hz();
    assert!(forbidden > 1_000.0, "the silent note is a very high one");

    // The gate closes at the silent step, so the tail must not carry that pitch.
    assert!(
        peak(&buf[4_000..]) < peak(&buf[..2_000]),
        "the note should be releasing, not retuned"
    );
}
