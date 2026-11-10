//! The limiter's only real promise: nothing leaves above the ceiling, ever, and the gain comes back.

use acid_core::limiter::{CEILING, Limiter, RELEASE_MS};

#[test]
fn a_pathological_input_never_escapes_the_ceiling() {
    let sr = 48_000.0;
    let mut l = Limiter::new(sr);
    for i in 0..(sr as usize * 5) {
        let x = if i % 4_800 < 100 { 12.0 } else { -0.9 };
        let y = l.process(x);
        assert!(y.is_finite(), "NaN at {i}");
        assert!(y.abs() <= CEILING + 1e-6, "leaked {y} at {i}");
    }
}

#[test]
fn gain_recovers_after_a_burst() {
    let sr = 48_000.0;
    let mut l = Limiter::new(sr);
    for _ in 0..4_800 {
        l.process(4.0);
    }
    assert!(l.gain() < 0.5, "did not duck: {}", l.gain());
    // 5 release times of quiet input
    for _ in 0..(RELEASE_MS * 0.001 * sr * 5.0) as usize {
        l.process(0.1);
    }
    assert!(l.gain() > 0.95, "gain stuck at {}", l.gain());
}

#[test]
fn quiet_material_passes_with_unity_gain() {
    let mut l = Limiter::new(48_000.0);
    for _ in 0..48_000 {
        assert!((l.process(0.3) - 0.3).abs() < 1e-6);
    }
    assert!((l.gain() - 1.0).abs() < 1e-6);
}

/// A single loud sample must already be limited — the guarantee comes from instant attack, not lookahead.
#[test]
fn one_isolated_peak_is_caught_on_its_own_sample() {
    let mut l = Limiter::new(48_000.0);
    for _ in 0..48_000 {
        l.process(0.1);
    }
    let y = l.process(20.0);
    assert!(y.abs() <= CEILING + 1e-6, "leaked {y}");
    // and the material right after it stays inside the ceiling too
    for _ in 0..4_800 {
        assert!(l.process(0.5).abs() <= CEILING + 1e-6);
    }
}

/// The chain, not just the stage: with drive at maximum the engine's output still never passes the ceiling.
#[test]
fn the_engine_never_leaves_the_ceiling_with_drive_at_maximum() {
    let mut e = acid_core::engine::Engine::new(48_000.0);
    e.set_pattern(std::sync::Arc::new(
        acid_core::pattern::Pattern::from_steps(
            [acid_core::pattern::Step::note(acid_core::pitch::Pitch::A, -1); 16],
        ),
    ));
    e.set_resonance(1.0);
    e.set_cutoff(1.0);
    e.set_drive(1.0);
    e.start();
    let mut buf = vec![0.0; 48_000];
    for _ in 0..4 {
        e.render(&mut buf);
        assert!(
            buf.iter()
                .all(|s| s.is_finite() && s.abs() <= CEILING + 1e-6),
            "the master bus escaped the ceiling"
        );
    }
}
