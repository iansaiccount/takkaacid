//! The engine runs the sequencer: the transport starts and stops it, step boundaries land on exact
//! samples, and no block size can change the output.
//!
//! Ruling 25 arithmetic: at 120 BPM a 16th note is 6000 samples, so step 2 begins at sample 12 000 — the
//! plan's 24 000 came from its 2x step-duration error.

use acid_core::engine::Engine;
use acid_core::pattern::{Grid, Pattern, Step};
use acid_core::pitch::Pitch;
use std::sync::Arc;

#[test]
fn a_step_boundary_lands_on_the_exact_expected_sample() {
    let sr = 48_000.0;
    let mut e = Engine::new(sr);
    let mut steps = [Step::rest(); 16];
    steps[0] = Step::note(Pitch::C, 0); // 55 Hz
    steps[2] = Step::note(Pitch::C, 1); // one octave up, at sample 12 000
    e.set_pattern(Arc::new(Pattern::from_steps(steps)));
    e.set_bpm(120.0);
    e.start();
    let mut buf = vec![0.0; 36_000];
    e.render(&mut buf);
    let first_sound = buf.iter().position(|s| s.abs() > 1e-4).unwrap();
    let second_sound = (12_000..).find(|&i| buf[i].abs() > 1e-4).unwrap();

    // `start` is sample-exact by the *same* yardstick a direct note-on gives: whatever the attack and the
    // filter's settling cost before the note is audible, the sequencer adds none of it.
    let mut direct = Engine::new(sr);
    direct.note_on(55.0, false);
    let mut reference = vec![0.0; 1_000];
    direct.render(&mut reference);
    let direct_onset = reference.iter().position(|s| s.abs() > 1e-4).unwrap();
    assert_eq!(
        first_sound, direct_onset,
        "step 0 should sound as promptly as a direct note-on"
    );

    // The rest before step 2 renders exact zeros (the release-tail guarantee from Task 18), so the first
    // non-zero sample after 12 000 *is* step 2's trigger sample — no threshold, no jitter.
    assert!(
        buf[11_500..12_000].iter().all(|s| *s == 0.0),
        "the rest before step 2 should be exact zeros"
    );
    assert_eq!(second_sound, 12_000, "step 2 starts two 16ths after step 0");
}

#[test]
fn block_size_never_changes_the_output() {
    let make = || {
        let mut e = Engine::new(44_100.0);
        let mut steps = [Step::rest(); 16];
        steps[0] = Step::note(Pitch::A, -1);
        steps[4] = Step {
            accent: true,
            ..Step::note(Pitch::E, 0)
        };
        e.set_pattern(Arc::new(Pattern::from_steps(steps)));
        e.set_bpm(137.0);
        e.set_swing(0.4);
        e.start();
        e
    };
    let mut one = make();
    let mut a = vec![0.0; 44_100];
    one.render(&mut a);
    let mut many = make();
    let mut b = Vec::new();
    for n in [1usize, 3, 511, 997, 4096, 38_492] {
        let mut chunk = vec![0.0; n];
        many.render(&mut chunk);
        b.extend_from_slice(&chunk);
    }
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert!((x - y).abs() < 1e-6, "sample {i}: {x} vs {y}");
    }
}

#[test]
fn position_advances_and_stop_silences() {
    let mut e = Engine::new(48_000.0);
    e.set_pattern(Arc::new(Pattern::from_steps(
        [Step::note(Pitch::A, -1); 16],
    )));
    e.set_bpm(240.0);
    e.start();
    e.render(&mut vec![0.0; 24_000]);
    assert!(e.position() > 0);
    e.stop();
    let mut buf = vec![0.0; 4_800];
    e.render(&mut buf);
    assert!(acid_core::analysis::rms(&buf[2_400..]) < 0.001);
    assert!(!e.is_playing());
}

#[test]
fn tempo_reads_back_and_clamps_like_the_panel() {
    let mut e = Engine::new(48_000.0);
    e.set_bpm(97.5);
    assert!((e.bpm() - 97.5).abs() < 1e-6);
    e.set_bpm(1_000.0);
    assert!(
        (e.bpm() - acid_core::timing::MAX_BPM).abs() < 1e-6,
        "the clock's clamp is the one that wins: {}",
        e.bpm()
    );
}

/// A swap into a longer bar keeps the step the sequence was on and takes the finer grid from there: at 120 BPM a sixteenth is
/// 6000 samples and a thirty-second is 3000, so a playhead halfway through step 4 of sixteen hears step 5 of thirty-two half a
/// new step — 1500 samples — after the swap.
///
/// This is a decision, not an accident. The other answer keeps the **fraction of the bar** instead of the step number (4 of 16
/// would become 8 of 32), which holds the bar's absolute position and moves every later step earlier. `Sequencer::set_pattern`
/// says why the step number won; this test is where a change to that choice fails.
#[test]
fn a_swap_into_a_longer_bar_keeps_the_step_it_was_on() {
    let sr = 48_000.0;
    let mut e = Engine::new(sr);
    e.set_bpm(120.0);
    // A silent first bar: the only sound here is the new pattern's step 5, so nothing from the old bar can be mistaken for it.
    e.set_pattern(Arc::new(Pattern::from_steps_in(
        Grid::Sixteenth,
        [Step::rest(); 16],
    )));
    e.start();
    let mut before = vec![0.0; 27_000]; // four steps and a half: the playhead is part-way through step 4
    e.render(&mut before);
    assert!(
        before.iter().all(|s| s.abs() < 1e-4),
        "the first bar is silent, or the marker below proves nothing"
    );

    let mut thirty_two = [Step::rest(); 32];
    thirty_two[5] = Step::note(Pitch::C, 1);
    e.set_pattern(Arc::new(Pattern::from_steps_in(
        Grid::ThirtySecond,
        thirty_two,
    )));
    let mut after = vec![0.0; 12_000];
    e.render(&mut after);
    let first = after.iter().position(|s| s.abs() > 1e-4);
    assert!(
        first.is_some_and(|at| at < 3_600),
        "step 5 of the finer bar arrives within a step of the swap: {first:?} samples in"
    );
}
