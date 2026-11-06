//! Gain staging: the factory line should be a sensible listening level before anyone touches a knob.

use acid_core::analysis::{peak, rms};

/// The window the factory pattern has to land in at launch settings (volume 1.0, drive 0.0).
const WINDOW_DB: (f32, f32) = (-18.0, -10.0);

fn factory_engine() -> acid_core::engine::Engine {
    let mut e = acid_core::engine::Engine::new(48_000.0);
    e.set_pattern(std::sync::Arc::new(
        acid_core::factory::factory_patterns()[0],
    ));
    e.set_volume(1.0);
    e.start();
    e
}

#[test]
fn the_factory_pattern_lands_between_minus_eighteen_and_ten_db() {
    let mut e = factory_engine();
    let mut buf = vec![0.0; 48_000 * 4];
    e.render(&mut buf);
    // Skip the first second: the attack of the first note is not the loop's level.
    let level = rms(&buf[48_000..]);
    let db = 20.0 * level.log10();
    assert!(
        (WINDOW_DB.0..=WINDOW_DB.1).contains(&db),
        "factory pattern sits at {db:.2} dBFS, wanted {WINDOW_DB:?}"
    );
    assert!(peak(&buf[48_000..]) < 0.9, "peaks too hot");
    // Recorded next to the voice's amplitude constant: this is the number it was tuned to.
    let target = acid_core::engine::TARGET_RMS_DB;
    assert!(
        (db - target).abs() < 4.0,
        "staged for {target} dB but measures {db:.2} dBFS"
    );
}

#[test]
fn drive_and_resonance_at_maximum_stay_below_the_ceiling() {
    let mut e = acid_core::engine::Engine::new(48_000.0);
    e.set_pattern(std::sync::Arc::new(
        acid_core::factory::factory_patterns()[0],
    ));
    e.set_drive(1.0);
    e.set_resonance(1.0);
    e.set_env_mod(1.0);
    e.set_volume(1.0);
    e.start();
    let mut buf = vec![0.0; 48_000 * 4];
    e.render(&mut buf);
    assert!(peak(&buf) <= acid_core::limiter::CEILING + 1e-6);
    assert!(buf.iter().all(|s| s.is_finite()));
}
