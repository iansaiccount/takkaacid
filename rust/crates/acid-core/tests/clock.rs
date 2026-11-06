//! The step clock: it answers "how many samples until the next 16th?" and "which step are we on?".
//!
//! It counts steps forever and never wraps — wrapping to 0–15 is the sequencer's job. Every duration is
//! `f64` samples, so a long run split into arbitrary block sizes still lands on the right step count.
//!
//! Two notes on the arithmetic here:
//! - At 120 BPM a 16th note is 125 ms (6000 samples at 48 kHz), so one second crosses **eight** steps,
//!   not four. The plan said four; that was corrected in Task 21.
//! - Advances are deliberately *not* exact multiples of a step (50 000, not 48 000 samples). A duration
//!   computed as `sr * 60 / (bpm * 4)` lands within a nanosecond of the boundary rather than on it, so
//!   asserting a count at an exact boundary would be a coin flip. Margins of hundreds of samples keep
//!   every count here deterministic.

use acid_core::clock::Clock;

#[test]
fn eight_steps_fit_in_one_second_at_120_bpm() {
    let mut c = Clock::new(48_000.0);
    c.set_bpm(120.0);
    assert_eq!(c.step_index(), 0);
    assert!(
        (c.samples_until_next_step() - 6_000.0).abs() < 1e-6,
        "the first step starts immediately, at the tempo just set: {}",
        c.samples_until_next_step()
    );

    // 50 000 samples = eight 16ths plus 2000 samples: comfortably off the boundary in both directions.
    let crossed = c.advance_by(50_000);

    assert_eq!(crossed, 8);
    assert_eq!(c.step_index(), 8);
    assert!((c.samples_until_next_step() - 4_000.0).abs() < 1e-6);
}

#[test]
fn odd_block_sizes_do_not_accumulate_drift() {
    let mut c = Clock::new(44_100.0);
    c.set_bpm(137.0);
    let total = 44_100 * 10;
    let mut consumed = 0;
    while consumed < total {
        let n = (total - consumed).min(337);
        c.advance_by(n);
        consumed += n;
    }
    let expected_steps =
        (total as f64 / acid_core::timing::samples_per_step(137.0, 44_100.0)) as usize;
    assert!(
        (c.step_index() as i64 - expected_steps as i64).abs() <= 1,
        "{} vs {expected_steps}",
        c.step_index()
    );
}

#[test]
fn the_clock_counts_steps_without_wrapping() {
    // Wrapping to 0–15 is the sequencer's job (Task 23); the clock counts forever.
    let mut c = Clock::new(48_000.0);
    c.set_bpm(300.0);
    let crossed = c.advance_by(48_000 * 4);
    assert!(
        crossed > 16,
        "four seconds at 300 bpm is more than 16 steps, got {crossed}"
    );
    assert_eq!(c.step_index(), crossed);
    assert_eq!(Clock::new(48_000.0).step_index(), 0);
}

#[test]
fn tempo_changes_apply_to_the_step_in_flight() {
    let mut c = Clock::new(48_000.0);
    c.set_bpm(120.0); // 6000 samples per step
    assert_eq!(c.advance_by(3_000), 0, "half a step crosses nothing");
    c.set_bpm(240.0); // 3000 samples per step; the half-elapsed step scales with it
    assert!(
        (c.samples_until_next_step() - 1_500.0).abs() < 1e-6,
        "the remaining half-step halves with the tempo: {}",
        c.samples_until_next_step()
    );
    assert_eq!(c.advance_by(1_000), 0, "1000 samples is under the 1500 left");
    assert_eq!(c.advance_by(1_000), 1, "2000 samples clears the boundary at the new tempo");
}

#[test]
fn swing_trades_time_between_even_and_odd_steps() {
    let mut c = Clock::new(48_000.0);
    c.set_bpm(120.0); // 6000 samples per step
    c.set_swing(1.0); // even steps 1.5x, odd steps 0.5x; step 0 rescales to 9000
    assert_eq!(c.swing(), 1.0);
    assert!(
        (c.samples_until_next_step() - 9_000.0).abs() < 1e-6,
        "a full-swing even step is 1.5 steps long: {}",
        c.samples_until_next_step()
    );
    // Cross into step 1, keeping 100 samples of slack on either side of each boundary so float
    // error in `sr * 60 / (bpm * 4)` cannot flip a crossing.
    assert_eq!(c.advance_by(8_900), 0);
    assert_eq!(c.advance_by(200), 1);
    assert!(
        (c.samples_until_next_step() - 2_900.0).abs() < 1e-6,
        "step 1 is odd: 3000 long, minus the 100 carried over: {}",
        c.samples_until_next_step()
    );

    assert_eq!(c.advance_by(2_800), 0);
    assert_eq!(c.advance_by(200), 1);
    assert!(
        (c.samples_until_next_step() - 8_900.0).abs() < 1e-6,
        "step 2 is even again: 9000 long, minus 100: {}",
        c.samples_until_next_step()
    );
}
