//! The live instrument surface: every knob, the pattern and the transport flag, shared without a lock the
//! audio thread could ever wait on.

use acid_app::state::SharedState;
use acid_core::oscillator::Waveform;
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

#[test]
fn every_knob_survives_a_snapshot() {
    let s = SharedState::new();
    s.tuning.set(0.25);
    s.cutoff.set(0.75);
    s.resonance.set(0.6);
    s.env_mod.set(0.4);
    s.decay.set(0.33);
    s.accent.set(0.9);
    s.volume.set(0.7);
    s.tempo_bpm.set(142.0);
    s.swing.set(0.5);
    s.set_waveform(Waveform::Square);
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);

    let p = s.snapshot();
    assert_eq!(p.waveform, Waveform::Square);
    assert!((p.tuning - 0.25).abs() < 1e-6);
    assert!((p.cutoff - 0.75).abs() < 1e-6);
    assert!((p.tempo_bpm - 142.0).abs() < 1e-6);
    assert!(p.playing);
}

#[test]
fn editing_one_step_keeps_the_other_fifteen() {
    let s = SharedState::new();
    let mut p = Pattern::empty();
    p.steps[2] = Step::note(Pitch::E, 0);
    s.set_pattern(std::sync::Arc::new(p));
    s.set_step(
        2,
        Step {
            accent: true,
            ..Step::note(Pitch::G, 0)
        },
    );
    let read = s.pattern();
    assert!(read.steps[2].accent);
    assert_eq!(read.steps[2].pitch, Pitch::G);
    assert_eq!(read.steps.len(), 16);
    assert!(read.steps.iter().enumerate().all(|(i, st)| i == 2 || !st.on));
}

#[test]
fn switching_waveform_round_trips_both_ways() {
    let s = SharedState::new();
    s.set_waveform(Waveform::Square);
    assert_eq!(s.waveform(), Waveform::Square);
    s.set_waveform(Waveform::Saw);
    assert_eq!(s.waveform(), Waveform::Saw);
}

#[test]
fn a_stray_step_index_is_ignored_rather_than_panicking() {
    let s = SharedState::new();
    s.set_pattern(std::sync::Arc::new(Pattern::from_steps(
        [Step::note(Pitch::C, 0); 16],
    )));
    s.set_step(99, Step::rest());
    let read = s.pattern();
    assert!(read.steps.iter().all(|step| step.on), "nothing may change");
}
