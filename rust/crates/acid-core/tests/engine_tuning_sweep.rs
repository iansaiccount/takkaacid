use acid_core::analysis::dominant_hz;
use acid_core::engine::Engine;
use acid_core::oscillator::Waveform;

#[test]
fn knobs_can_be_moved_while_a_note_is_sounding() {
    let mut e = Engine::new(48_000.0);
    e.note_on(110.0, false);
    e.render(&mut vec![0.0; 4_800]);

    e.set_tuning(0.75); // +6 semitones
    let mut buf = vec![0.0; 9_600];
    e.render(&mut buf);
    let hz = dominant_hz(&buf[4_800..], 48_000.0);
    assert!((hz - 155.5).abs() < 3.0, "expected ~155.6 Hz, got {hz}");

    e.set_waveform(Waveform::Square);
    let mut buf = vec![0.0; 4_800];
    e.render(&mut buf);
    assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 1.001));
    assert!(e.is_playing_something(), "a live note must survive a knob move");
}
