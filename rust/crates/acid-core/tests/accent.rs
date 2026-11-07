//! ACCENT, the way a TB-303 does it: an accented step is louder *and* has a fast filter kick at its attack, and the
//! ACCENT knob decides how much of both there is. At zero the accent must be inaudible; at full it is the character.

use acid_core::analysis::{brightness, rms};
use acid_core::voice::Voice;

const SR: f32 = 48_000.0;

/// One 4 Hz-free A1 note, rendered for `samples` samples, with the accent character set by `knob`.
fn render(accent: bool, knob: f32, samples: usize, env_mod: f32) -> Vec<f32> {
    let mut voice = Voice::new(SR);
    voice.set_cutoff_knob(0.25);
    voice.set_env_mod_knob(env_mod);
    voice.set_accent_knob(knob);
    voice.note_at(55.0, accent, 127);
    let mut buffer = vec![0.0; samples];
    voice.render(&mut buffer);
    buffer
}

#[test]
fn the_accent_knob_at_zero_makes_an_accent_inaudible() {
    // Nothing about an accented note may differ from a plain one when the knob is down: no level, no filter kick.
    let plain = render(false, 0.0, 9_600, 0.5);
    let accented = render(true, 0.0, 9_600, 0.5);
    let (plain, accented) = (&plain[2_400..], &accented[2_400..]);

    assert!(
        (rms(plain) - rms(accented)).abs() / rms(plain) < 0.02,
        "level differs: {} vs {}",
        rms(plain),
        rms(accented)
    );
    assert!(
        (brightness(plain) - brightness(accented)).abs() / brightness(plain) < 0.05,
        "tone differs: {} vs {}",
        brightness(plain),
        brightness(accented)
    );
}

#[test]
fn the_accent_knob_at_full_lifts_the_level_like_the_hardware() {
    // A 303 accent is a good few dB louder; 1.5x is about +3.5 dB and is the floor of what should be audible.
    let plain = render(false, 1.0, 9_600, 0.0);
    let accented = render(true, 1.0, 9_600, 0.0);
    let ratio = rms(&accented[4_800..]) / rms(&plain[4_800..]);
    assert!(
        ratio > 1.4,
        "an accent should be clearly louder, not {ratio:.2}x"
    );
}

#[test]
fn an_accent_is_a_fast_filter_kick_at_the_attack() {
    // The accent's filter kick is a pulse: brightest at the attack, back to the plain tone by the end of the note. With
    // the filter envelope switched off, a plain note holds a steady tone.
    let samples = 9_600;
    let plain = render(false, 1.0, samples, 0.0);
    let accented = render(true, 1.0, samples, 0.0);

    let plain_early = brightness(&plain[0..2_400]);
    let plain_late = brightness(&plain[7_200..samples]);
    let accented_early = brightness(&accented[0..2_400]);
    let accented_late = brightness(&accented[7_200..samples]);

    assert!(
        (plain_early / plain_late - 1.0).abs() < 0.25,
        "a plain note should hold its tone: {:.2}x",
        plain_early / plain_late
    );
    assert!(
        accented_early > plain_early * 1.5,
        "the accent must kick at the attack: {accented_early:.4} vs {plain_early:.4}"
    );
    assert!(
        accented_late < plain_late * 1.15,
        "and the kick must decay, not leave the note brighter: {accented_late:.4} vs {plain_late:.4}"
    );
}

#[test]
fn the_kick_is_proportional_to_the_knob() {
    // Half the knob, half the character: neither silent nor full.
    let quiet = brightness(&render(true, 0.25, 2_400, 0.0)[0..1_200]);
    let loud = brightness(&render(true, 1.0, 2_400, 0.0)[0..1_200]);
    assert!(loud > quiet * 1.2, "quiet {quiet}, loud {loud}");

    // And the knob is monotonic in the level it adds.
    let level = |knob: f32| rms(&render(true, knob, 9_600, 0.0)[4_800..]);
    assert!(level(1.0) > level(0.5) && level(0.5) > level(0.0));
}
