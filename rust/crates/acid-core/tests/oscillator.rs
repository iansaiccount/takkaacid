use acid_core::analysis::{dominant_hz, rms};
use acid_core::oscillator::{Oscillator, Waveform};

fn render(wave: Waveform, hz: f32, sr: f32, n: usize) -> Vec<f32> {
    let mut o = Oscillator::new(sr);
    o.set_frequency(hz);
    o.set_waveform(wave);
    (0..n).map(|_| o.next_sample()).collect()
}

#[test]
fn saw_holds_the_requested_pitch_at_44k1_and_48k() {
    for sr in [44_100.0, 48_000.0] {
        let buf = render(Waveform::Saw, 110.0, sr, 65_536);
        let hz = dominant_hz(&buf, sr);
        assert!((hz - 110.0).abs() < 1.1, "sr {sr} got {hz}");
    }
}

#[test]
fn square_holds_the_same_pitch_as_saw_without_switching() {
    let sr = 48_000.0;
    let saw = render(Waveform::Saw, 220.0, sr, 65_536);
    let sq = render(Waveform::Square, 220.0, sr, 65_536);
    assert!((dominant_hz(&saw, sr) - dominant_hz(&sq, sr)).abs() < 2.0);
}

#[test]
fn both_waveforms_are_bipolar_and_dc_free() {
    let sr = 48_000.0;
    for w in [Waveform::Saw, Waveform::Square] {
        let buf = render(w, 55.0, sr, 48_000);
        let mean: f32 = buf.iter().sum::<f32>() / buf.len() as f32;
        assert!(mean.abs() < 0.02, "{w:?} mean {mean}");
        assert!(rms(&buf) > 0.4, "{w:?} too quiet");
        assert!(buf.iter().all(|s| s.abs() <= 1.001));
    }
}

#[test]
fn frequency_is_clamped_and_never_produces_nan() {
    let mut o = Oscillator::new(48_000.0);
    o.set_frequency(0.0);
    let buf: Vec<f32> = (0..1_000).map(|_| o.next_sample()).collect();
    assert!(buf.iter().all(|s| s.is_finite()));
    o.set_frequency(1.0e9);
    let buf: Vec<f32> = (0..1_000).map(|_| o.next_sample()).collect();
    assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 1.001));
}
