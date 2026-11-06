use acid_core::analysis::{dominant_hz, rms};
use acid_core::voice::Voice;

#[test]
fn a_held_note_sounds_at_the_requested_pitch() {
    let mut v = Voice::new(48_000.0);
    v.note_on(440.0, false);
    let mut buf = vec![0.0; 9_600];
    v.render(&mut buf);
    let hz = dominant_hz(&buf[4_800..], 48_000.0);
    assert!((hz - 440.0).abs() < 8.0, "got {hz}");
    assert!(rms(&buf) > 0.05);
}

#[test]
fn note_off_reaches_silence() {
    let mut v = Voice::new(48_000.0);
    v.note_on(220.0, false);
    v.render(&mut vec![0.0; 2_400]);
    v.note_off();
    let mut tail = vec![0.0; 4_800];
    v.render(&mut tail);
    assert!(rms(&tail[tail.len() / 2..]) < 0.001);
    assert!(!v.is_active());
}

#[test]
fn output_never_clips_or_contains_nan() {
    let mut v = Voice::new(44_100.0);
    v.note_on(55.0, true);
    let mut buf = vec![0.0; 44_100];
    v.render(&mut buf);
    assert!(
        buf.iter().all(|s| s.is_finite() && s.abs() <= 0.999),
        "bad sample"
    );
}
