//! The drive stage: bounded, monotonic clipping that is also a wire at zero.
//!
//! `amount_zero_is_transparent` is the one that matters — a Drive at 0 must be a wire, which is why the
//! stage blends dry into clipped rather than normalising the clip alone.

use acid_core::drive::{Drive, MAX_DRIVE_GAIN, soft_clip};

#[test]
fn soft_clip_is_monotonic_and_bounded() {
    let mut last = soft_clip(-50.0);
    for i in -1_000..=1_000 {
        let y = soft_clip(i as f32 * 0.05);
        assert!(y >= last - 1e-6, "not monotonic at {i}");
        assert!(y.abs() < 1.0, "escaped the bound at {i}: {y}");
        last = y;
    }
    assert_eq!(soft_clip(0.0), 0.0);
    assert!((soft_clip(0.01) - 0.01).abs() < 1e-3);
}

#[test]
fn amount_zero_is_transparent() {
    let mut d = Drive::new(48_000.0);
    d.set_amount(0.0);
    for _ in 0..480 {
        d.process(0.0);
    }
    for x in [-0.9, -0.5, 0.0, 0.25, 0.9] {
        assert!((d.process(x) - x).abs() < 0.02, "input {x}");
    }
}

#[test]
fn more_drive_means_more_harmonics_on_the_same_sine() {
    let brightness_after = |amount: f32| {
        let sr = 48_000.0;
        let mut d = Drive::new(sr);
        d.set_amount(amount);
        for _ in 0..4_800 {
            d.process(0.0);
        }
        let buf: Vec<f32> = (0..48_000)
            .map(|i| d.process((core::f32::consts::TAU * 110.0 * i as f32 / sr).sin() * 0.8))
            .collect();
        acid_core::analysis::brightness(&buf[24_000..])
    };
    let clean = brightness_after(0.0);
    let dirty = brightness_after(1.0);
    assert!(dirty > clean * 1.5, "clean {clean} dirty {dirty}");
}

#[test]
fn maximum_gain_never_exceeds_unity_output() {
    let mut d = Drive::new(44_100.0);
    d.set_amount(1.0);
    for _ in 0..4_410 {
        d.process(0.0);
    }
    let peak = (0..44_100)
        .map(|_| d.process(0.99).abs())
        .fold(0.0f32, f32::max);
    assert!(peak <= 0.999, "peak {peak}");
    assert!((MAX_DRIVE_GAIN - 8.0).abs() < 1e-6);
}

/// The knob is smoothed: a jump from clean to dirty must not step the output.
#[test]
fn a_sudden_drive_jump_does_not_click() {
    let sr = 48_000.0;
    let mut d = Drive::new(sr);
    d.set_amount(0.0);
    for _ in 0..1_000 {
        d.process(0.5);
    }
    let before = d.process(0.5);
    d.set_amount(1.0);
    let after = d.process(0.5);
    assert!(
        (after - before).abs() < 0.05,
        "the first sample after the jump should barely move: {before} -> {after}"
    );
}
