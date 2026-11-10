//! A step's velocity: the level it was struck at, its full-scale default, and what it does to the voice.

use acid_core::pattern::{DEFAULT_VELOCITY, Step};
use acid_core::pitch::Pitch;
use acid_core::voice::{Voice, velocity_to_gain};

/// The loudest sample of a note at `velocity`, after the filter and envelope have had their say.
fn peak(velocity: u8) -> f32 {
    let mut voice = Voice::new(44_100.0);
    voice.note_at(110.0, false, velocity);
    let mut out = vec![0.0; 4410];
    voice.render(&mut out);
    out.iter().fold(0.0f32, |loudest, s| loudest.max(s.abs()))
}

#[test]
fn velocity_scales_the_note_level() {
    let full = peak(DEFAULT_VELOCITY);
    let half = peak(64);
    assert!(full > 0.01, "the full-scale note makes a sound: {full}");

    // The voice is linear in its level, so half the velocity is half the output — within the filter's rounding.
    let ratio = half / full;
    let wanted = 64.0 / 127.0;
    assert!(
        (ratio - wanted).abs() < 0.02,
        "velocity 64 should be {wanted:.3} of full scale, not {ratio:.3}"
    );
}

#[test]
fn silence_is_a_velocity_too() {
    assert_eq!(peak(0), 0.0, "a zero-velocity note is silent");
    assert!((velocity_to_gain(0) - 0.0).abs() < 1e-6);
    assert!((velocity_to_gain(255) - 1.0).abs() < 1e-6, "and is clamped");
}

#[test]
fn a_step_nothing_struck_plays_at_full_scale() {
    // The default has to be unity, or every pattern written before velocity existed would change level.
    assert_eq!(Step::default().velocity, DEFAULT_VELOCITY);
    assert_eq!(Step::note(Pitch::C, 0).velocity, DEFAULT_VELOCITY);
    assert_eq!(Step::rest().velocity, DEFAULT_VELOCITY);
    assert!((velocity_to_gain(DEFAULT_VELOCITY) - 1.0).abs() < 1e-6);
}

#[test]
fn a_step_saved_before_velocity_loads_at_full_scale() {
    let mut saved = serde_json::to_value(Step::note(Pitch::C, 0)).expect("a step is serializable");
    saved
        .as_object_mut()
        .expect("a step is an object")
        .remove("velocity")
        .expect("it had one to remove");

    let step: Step = serde_json::from_value(saved).expect("an older step still loads");
    assert_eq!(
        step.velocity, DEFAULT_VELOCITY,
        "a file written before velocity carries the full-scale default"
    );
}

#[test]
fn velocity_survives_a_round_trip() {
    let step = Step {
        velocity: 42,
        ..Step::note(Pitch::E, 1)
    };
    let json = serde_json::to_string(&step).expect("serializable");
    let back: Step = serde_json::from_str(&json).expect("deserializable");
    assert_eq!(back, step);
}
