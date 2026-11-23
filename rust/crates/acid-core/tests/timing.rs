//! The bar is the tempo's; the grid only divides it. Every count here comes from `Grid::steps_per_bar`, so a grid added
//! to the catalogue is measured here without a second hand-kept number.
use acid_core::pattern::Grid;
use acid_core::timing::*;

#[test]
fn a_16th_at_120_bpm_is_6000_samples_at_48k() {
    let steps = Grid::Sixteenth.steps_per_bar();
    assert!((samples_per_step(120.0, 48_000.0, steps) - 6_000.0).abs() < 1e-6);
}

#[test]
fn a_16th_at_120_bpm_is_5512_5_samples_at_44k1() {
    let steps = Grid::Sixteenth.steps_per_bar();
    assert!((samples_per_step(120.0, 44_100.0, steps) - 5_512.5).abs() < 1e-6);
}

#[test]
fn tempo_is_clamped_to_the_panel_range() {
    assert_eq!(clamp_bpm(10.0), MIN_BPM);
    assert_eq!(clamp_bpm(999.0), MAX_BPM);
    assert_eq!(clamp_bpm(140.0), 140.0);
}

#[test]
fn swing_lengthens_even_steps_and_shortens_odd_ones() {
    let steps = Grid::Sixteenth.steps_per_bar();
    let step = samples_per_step(120.0, 48_000.0, steps);
    let even = step_duration_samples(0, 120.0, 48_000.0, 1.0, steps);
    let odd = step_duration_samples(1, 120.0, 48_000.0, 1.0, steps);
    assert!((even - step * 1.5).abs() < 1e-6, "even {even}");
    assert!((odd - step * 0.5).abs() < 1e-6, "odd {odd}");
}

#[test]
fn a_pair_of_steps_always_lasts_two_steps() {
    let steps = Grid::Sixteenth.steps_per_bar();
    for swing in [0.0, 0.25, 0.5, 1.0] {
        let pair = step_duration_samples(0, 137.0, 44_100.0, swing, steps)
            + step_duration_samples(1, 137.0, 44_100.0, swing, steps);
        let expected = 2.0 * samples_per_step(137.0, 44_100.0, steps);
        assert!(
            (pair - expected).abs() < 1e-6,
            "swing {swing}: {pair} vs {expected}"
        );
    }
}

#[test]
fn a_bar_is_the_same_length_at_every_grid() {
    let bar = samples_per_bar(137.0, 44_100.0);
    assert!(
        (bar - 44_100.0 * 240.0 / 137.0).abs() < 1e-6,
        "four beats at 137"
    );
    for grid in Grid::ALL {
        let steps = grid.steps_per_bar();
        let stepped = samples_per_step(137.0, 44_100.0, steps) * steps as f64;
        assert!(
            (stepped - bar).abs() < 1e-6,
            "{} does not fill a bar",
            grid.name()
        );
    }
}

#[test]
fn swing_keeps_a_bar_of_each_grid_the_same_length() {
    for grid in Grid::ALL {
        for swing in [0.0, 0.25, 1.0] {
            let steps = grid.steps_per_bar();
            let total: f64 = (0..steps)
                .map(|i| step_duration_samples(i, 137.0, 44_100.0, swing, steps))
                .sum();
            assert!(
                (total - samples_per_bar(137.0, 44_100.0)).abs() < 1e-6,
                "{} at swing {swing} changed the bar",
                grid.name()
            );
        }
    }
}
