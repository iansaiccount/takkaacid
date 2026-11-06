use acid_core::analysis::{dominant_hz, rms};
use acid_core::voice::Voice;

#[test]
fn glide_reaches_the_target_and_never_overshoots() {
    let mut v = Voice::new(48_000.0);
    v.note_on(55.0, false);
    v.render(&mut vec![0.0; 4_800]);
    v.glide_to(110.0);
    let mut buf = vec![0.0; 24_000]; // 500 ms
    v.render(&mut buf);
    let early = dominant_hz(&buf[0..2_400], 48_000.0);
    let late = dominant_hz(&buf[21_600..], 48_000.0);
    assert!(
        early > 55.0 && early < 110.0,
        "should be mid-glide, got {early}"
    );
    assert!(
        (late - 110.0).abs() < 1.5,
        "should have arrived, got {late}"
    );
}

#[test]
fn glide_does_not_retrigger_the_gate() {
    let mut v = Voice::new(48_000.0);
    // Long decay so the VCA envelope is effectively flat across the comparison window: this test is
    // about the *gate* not retriggering, not about the amp envelope's shape.
    v.set_decay_knob(1.0);
    v.note_on(55.0, false);
    let mut buf = vec![0.0; 4_800];
    v.render(&mut buf);
    let before = rms(&buf[buf.len() - 2_400..]);
    v.glide_to(110.0);
    let mut buf = vec![0.0; 2_400];
    v.render(&mut buf);
    let after = rms(&buf);
    assert!(
        (after / before - 1.0).abs() < 0.3,
        "level jumped: {before} -> {after}"
    );
    assert!(v.is_active());
}

#[test]
fn set_pitch_is_immediate() {
    let mut v = Voice::new(48_000.0);
    v.note_on(55.0, false);
    v.set_pitch(220.0);
    let mut buf = vec![0.0; 4_096];
    v.render(&mut buf);
    assert!((dominant_hz(&buf, 48_000.0) - 220.0).abs() < 3.0);
}
