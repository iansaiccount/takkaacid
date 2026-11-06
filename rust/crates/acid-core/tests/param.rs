use acid_core::param::*;

#[test]
fn endpoints_are_exact() {
    assert_eq!(tuning_semitones(0.0), -12.0);
    assert_eq!(tuning_semitones(0.5), 0.0);
    assert_eq!(tuning_semitones(1.0), 12.0);
    assert!((cutoff_hz(0.0) - 80.0).abs() < 0.01);
    assert!((cutoff_hz(1.0) - 8000.0).abs() < 0.01);
    assert!((decay_seconds(0.0) - 0.03).abs() < 1e-6);
    assert!((decay_seconds(1.0) - 2.0).abs() < 1e-6);
}

#[test]
fn cutoff_midpoint_is_geometric() {
    // sqrt(80 * 8000) = 800 Hz
    assert!((cutoff_hz(0.5) - 800.0).abs() < 0.5);
}

#[test]
fn knob_positions_are_clamped() {
    assert_eq!(tuning_semitones(-2.0), -12.0);
    assert!((cutoff_hz(9.0) - 8000.0).abs() < 0.01);
    assert!((master_volume(2.0) - 1.0).abs() < 1e-6);
}

#[test]
fn smoothing_approaches_its_target_without_overshoot() {
    let mut s = Smooth::new(0.0, 20.0, 48_000.0);
    let mut last = 0.0;
    for _ in 0..(48_000 / 2) {
        last = s.process(1.0);
        assert!(last <= 1.0 + 1e-6, "overshoot: {last}");
    }
    assert!(last > 0.99);
}
