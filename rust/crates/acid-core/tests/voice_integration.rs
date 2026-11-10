//! Integration test for the whole voice as the Engine exposes it: does the launch patch sound like a
//! decaying resonant bass, and does every knob read back what was written to it.
//!
//! These assertions are deliberately about *audible* properties (attack level, decay, brightness falling)
//! plus the knob contract — not about internals. If a change makes the voice stop decaying or the filter
//! stop closing, this file fails before the ear test would.

use acid_core::analysis::{brightness, peak, rms};
use acid_core::engine::Engine;
use acid_core::oscillator::Waveform;

#[test]
fn a_default_patch_produces_a_decaying_resonant_bass_note() {
    let mut e = Engine::new(48_000.0);
    e.set_waveform(Waveform::Saw);
    e.set_tuning(0.5);
    e.set_cutoff(0.35);
    e.set_resonance(0.6);
    e.set_env_mod(0.5);
    e.set_decay(0.35);
    e.set_accent(0.8);
    // Volume at unity: this test measures the patch's envelope, not the master stage behind it.
    e.set_volume(1.0);
    e.note_on(55.0, false);

    let mut buf = vec![0.0; 48_000];
    e.render(&mut buf);

    let attack = &buf[0..2_400];
    let body = &buf[12_000..14_400];
    assert!(rms(attack) > 0.05, "no attack");
    assert!(rms(body) < rms(attack), "should decay");
    assert!(
        brightness(attack) > brightness(body),
        "filter should close as it decays"
    );
    assert!(peak(&buf) <= 0.999);
}

#[test]
fn every_knob_reads_back_what_was_set() {
    let mut e = Engine::new(48_000.0);
    e.set_waveform(Waveform::Square);
    e.set_tuning(0.75);
    e.set_cutoff(0.2);
    e.set_resonance(0.9);
    e.set_env_mod(0.1);
    e.set_decay(0.6);
    e.set_accent(0.4);
    e.set_volume(0.55);

    assert_eq!(e.waveform(), Waveform::Square);
    assert!((e.tuning() - 0.75).abs() < 1e-6);
    assert!((e.cutoff() - 0.2).abs() < 1e-6);
    assert!((e.resonance() - 0.9).abs() < 1e-6);
    assert!((e.env_mod() - 0.1).abs() < 1e-6);
    assert!((e.decay() - 0.6).abs() < 1e-6);
    assert!((e.accent() - 0.4).abs() < 1e-6);
    assert!((e.volume() - 0.55).abs() < 1e-6);
}

#[test]
fn knobs_clamp_instead_of_storing_nonsense() {
    let mut e = Engine::new(48_000.0);
    e.set_cutoff(9.0);
    e.set_resonance(-3.0);
    e.set_tuning(2.5);
    e.set_volume(1.4);

    assert!((e.cutoff() - 1.0).abs() < 1e-6, "cutoff {}", e.cutoff());
    assert!((e.resonance() - 0.0).abs() < 1e-6);
    assert!((e.tuning() - 1.0).abs() < 1e-6);
    assert!((e.volume() - 1.0).abs() < 1e-6);
}

#[test]
fn a_fresh_engine_starts_on_the_launch_patch() {
    let e = Engine::new(48_000.0);
    assert_eq!(e.waveform(), Waveform::Saw);
    assert!((e.tuning() - 0.5).abs() < 1e-6);
    assert!((e.cutoff() - acid_core::voice::DEFAULT_CUTOFF_KNOB).abs() < 1e-6);
    assert!((e.resonance() - acid_core::voice::DEFAULT_RESONANCE_KNOB).abs() < 1e-6);
    assert!((e.env_mod() - acid_core::voice::DEFAULT_ENV_MOD_KNOB).abs() < 1e-6);
    assert!((e.decay() - acid_core::voice::DEFAULT_DECAY_KNOB).abs() < 1e-6);
    assert!((e.accent() - acid_core::voice::DEFAULT_ACCENT_KNOB).abs() < 1e-6);
    assert!((e.volume() - acid_core::engine::DEFAULT_VOLUME_KNOB).abs() < 1e-6);
}
