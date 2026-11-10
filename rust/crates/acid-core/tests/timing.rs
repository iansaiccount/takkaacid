use acid_core::timing::*;

#[test]
fn a_16th_at_120_bpm_is_6000_samples_at_48k() {
    assert!((samples_per_step(120.0, 48_000.0) - 6_000.0).abs() < 1e-6);
}

#[test]
fn a_16th_at_120_bpm_is_5512_5_samples_at_44k1() {
    assert!((samples_per_step(120.0, 44_100.0) - 5_512.5).abs() < 1e-6);
}

#[test]
fn tempo_is_clamped_to_the_panel_range() {
    assert_eq!(clamp_bpm(10.0), MIN_BPM);
    assert_eq!(clamp_bpm(999.0), MAX_BPM);
    assert_eq!(clamp_bpm(140.0), 140.0);
}

#[test]
fn swing_lengthens_even_steps_and_shortens_odd_ones() {
    let step = samples_per_step(120.0, 48_000.0);
    let even = step_duration_samples(0, 120.0, 48_000.0, 1.0);
    let odd = step_duration_samples(1, 120.0, 48_000.0, 1.0);
    assert!((even - step * 1.5).abs() < 1e-6, "even {even}");
    assert!((odd - step * 0.5).abs() < 1e-6, "odd {odd}");
}

#[test]
fn a_pair_of_steps_always_lasts_two_steps() {
    for swing in [0.0, 0.25, 0.5, 1.0] {
        let pair = step_duration_samples(0, 137.0, 44_100.0, swing)
            + step_duration_samples(1, 137.0, 44_100.0, swing);
        let expected = 2.0 * samples_per_step(137.0, 44_100.0);
        assert!(
            (pair - expected).abs() < 1e-6,
            "swing {swing}: {pair} vs {expected}"
        );
    }
}
