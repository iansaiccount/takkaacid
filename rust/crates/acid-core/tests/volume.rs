//! Master volume: applied after the limiter, so it can only ever attenuate, and exact at zero.

use acid_core::engine::Engine;
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

/// A pattern that definitely makes sound.
fn playing_engine() -> Engine {
    let mut e = Engine::new(48_000.0);
    e.set_pattern(std::sync::Arc::new(Pattern::from_steps(
        [Step::note(Pitch::A, -1); 16],
    )));
    e
}

#[test]
fn volume_zero_is_exact_silence() {
    let mut e = playing_engine();
    e.set_volume(0.0);
    e.start();
    let mut buf = vec![0.0; 24_000];
    e.render(&mut buf);
    assert!(buf.iter().all(|s| *s == 0.0));
}

#[test]
fn half_volume_is_about_twelve_db_down() {
    let rms_at = |knob: f32| {
        let mut e = playing_engine();
        e.set_volume(knob);
        e.start();
        let mut buf = vec![0.0; 48_000];
        e.render(&mut buf);
        acid_core::analysis::rms(&buf[24_000..])
    };
    let full = rms_at(1.0);
    let half = rms_at(0.5);
    let db = 20.0 * (half / full).log10();
    assert!((db + 12.04).abs() < 0.5, "half knob was {db} dB");
}

#[test]
fn volume_cannot_push_past_the_ceiling() {
    let mut e = Engine::new(48_000.0);
    e.set_resonance(1.0);
    e.set_drive(1.0);
    e.set_volume(1.0);
    e.note_on(27.5, true);
    let mut buf = vec![0.0; 96_000];
    e.render(&mut buf);
    assert!(acid_core::analysis::peak(&buf) <= acid_core::limiter::CEILING + 1e-6);
}

/// The knob is smoothed, so a jump from silence to full must not step the output.
#[test]
fn a_volume_jump_does_not_click() {
    let mut e = playing_engine();
    e.set_volume(0.0);
    e.start();
    let mut warm = vec![0.0; 4_800];
    e.render(&mut warm);
    e.set_volume(1.0);
    let mut buf = vec![0.0; 4_800];
    e.render(&mut buf);
    let first = acid_core::analysis::peak(&buf[..96]);
    let later = acid_core::analysis::peak(&buf[2_400..]);
    assert!(
        first < later * 0.5,
        "the first samples after the jump should ramp, not leap: {first} vs {later}"
    );
}
