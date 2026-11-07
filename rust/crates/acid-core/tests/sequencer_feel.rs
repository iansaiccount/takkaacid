//! Playback feel: an accented step pushes harder, and a slide step glides into the next note instead of
//! retriggering the envelopes.
//!
//! Window arithmetic: the plan's offsets assumed 12 000-sample steps, i.e. the 2x step-duration error
//! corrected in Task 21 (Ruling 25). Every offset here is half the plan's — the step-0/step-1 boundary is
//! at sample 6000, not 12 000 — and the pitch is read inside step 1, before the rest closes the gate.

use acid_core::analysis::{dominant_hz, peak, rms};
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;
use acid_core::sequencer::Sequencer;
use acid_core::voice::Voice;
use std::sync::Arc;

#[test]
fn an_accented_step_peaks_higher_than_a_plain_one() {
    let sr = 48_000.0;
    let peak_of = |accent: bool| {
        let mut seq = Sequencer::new(sr);
        let step = Step {
            accent,
            ..Step::note(Pitch::A, 0)
        };
        seq.set_pattern(Arc::new(Pattern::from_steps([step; 16])));
        seq.set_bpm(120.0);
        let mut v = Voice::new(sr);
        seq.start();
        let mut buf = vec![0.0; 12_000];
        seq.render(&mut v, &mut buf);
        peak(&buf[1_000..])
    };
    assert!(
        peak_of(true) > peak_of(false) * 1.2,
        "accented {} should beat plain {} by 20%",
        peak_of(true),
        peak_of(false)
    );
}

/// Resting a step keeps its flags, so a rest can carry `slide`. If that flag leaks into the trigger for the next
/// note, the note retunes with the gate still shut and never sounds at all.
#[test]
fn a_slide_flag_on_a_rest_does_not_silence_the_next_note() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    let mut steps = [Step::rest(); 16];
    steps[0] = Step::note(Pitch::A, 0);
    steps[1] = Step {
        slide: true,
        ..Step::rest()
    };
    steps[2] = Step::note(Pitch::A, 0);
    seq.set_pattern(Arc::new(Pattern::from_steps(steps)));
    seq.set_bpm(120.0); // one 16th is 6000 samples: step 2 starts at 12 000
    let mut v = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; 18_000];
    seq.render(&mut v, &mut buf);

    let played = rms(&buf[12_400..13_800]);
    assert!(
        played > 0.01,
        "the note after a rest carrying a slide flag was silent: {played}"
    );
}

#[test]
fn a_slide_step_does_not_restart_the_envelope() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    let mut first = Step::note(Pitch::A, -1);
    first.slide = true;
    let second = Step::note(Pitch::A, 0);
    let mut steps = [Step::rest(); 16];
    steps[0] = first;
    steps[1] = second;
    seq.set_pattern(Arc::new(Pattern::from_steps(steps)));
    seq.set_bpm(120.0);
    let mut v = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; 24_000]; // four steps: two notes, then rests
    seq.render(&mut v, &mut buf);

    // The gate stays open across the boundary at 6000: a retrigger would restart the 2 ms attack from
    // zero and dip the level there.
    let at_boundary = rms(&buf[5_800..6_200]);
    let before_boundary = rms(&buf[4_000..5_500]);
    assert!(
        at_boundary > before_boundary * 0.5,
        "level collapsed: {before_boundary} -> {at_boundary}"
    );

    // Just after the boundary the pitch is still near the old note — a glide, not a jump to the octave.
    let early = dominant_hz(&buf[6_200..8_000], sr);
    assert!(early < 70.0, "pitch jumped instead of gliding: {early} Hz");

    // By the end of step 1 it has climbed most of the way to the octave above. The glide's 60 ms time
    // constant means 125 ms of step gets it to ~87 Hz, not 92.5 — hence the threshold sits below the
    // target, and the trend is what is being asserted here.
    let late = dominant_hz(&buf[9_500..11_900], sr);
    assert!(
        late > 78.0,
        "should have glided up toward the octave: {late} Hz"
    );
}
