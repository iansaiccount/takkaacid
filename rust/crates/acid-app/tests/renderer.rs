//! The renderer: one block of audio, driven by whatever the UI has published.
//!
//! `apply` is the single place parameters reach the engine; `AudioRenderer` reads a snapshot once per block,
//! swaps the pattern only when it actually changed, and copies the mono result to every channel.

use acid_app::audio::renderer::{AudioRenderer, apply};
use acid_app::state::SharedState;
use acid_core::analysis::peak;
use acid_core::oscillator::Waveform;

#[test]
fn applying_parameters_reaches_the_engine_knobs() {
    let s = SharedState::new();
    s.set_waveform(Waveform::Square);
    s.tuning.set(0.75);
    s.cutoff.set(0.9);
    let mut e = acid_core::engine::Engine::new(48_000.0);
    apply(&s.snapshot(), &mut e);
    // The engine exposes readback for every knob it stores:
    assert_eq!(e.waveform(), Waveform::Square);
    assert!((e.tuning() - 0.75).abs() < 1e-6);
    assert!((e.cutoff() - 0.9).abs() < 1e-6);
}

#[test]
fn stopped_transport_renders_silence_into_every_channel() {
    let s = SharedState::new();
    let mut r = AudioRenderer::new(48_000.0, 2, s, 2_048);
    let mut out = vec![1.0f32; 4_096]; // stereo
    r.render_into(&mut out);
    assert_eq!(peak(&out), 0.0);
}

#[test]
fn playing_duplicates_the_mono_signal_to_both_channels() {
    let s = SharedState::new();
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);
    s.tempo_bpm.set(120.0);
    s.set_pattern(std::sync::Arc::new(
        acid_core::pattern::Pattern::from_steps(
            [acid_core::pattern::Step::note(acid_core::pitch::Pitch::A, -1); 16],
        ),
    ));
    let mut r = AudioRenderer::new(48_000.0, 2, s, 2_048);
    let mut out = vec![0.0f32; 8_192];
    r.render_into(&mut out);
    for i in (0..8_192).step_by(2) {
        assert_eq!(out[i], out[i + 1], "channels differ at {i}");
    }
    assert!(peak(&out) > 0.01);
}

/// Nothing about a block depends on its size: the same second of audio in one call or many is identical.
#[test]
fn block_size_never_changes_what_the_renderer_produces() {
    let make = || {
        let s = SharedState::new();
        s.playing.store(true, std::sync::atomic::Ordering::Relaxed);
        s.tempo_bpm.set(137.0);
        s.set_pattern(std::sync::Arc::new(
            acid_core::pattern::Pattern::from_steps(
                [acid_core::pattern::Step::note(acid_core::pitch::Pitch::E, 0); 16],
            ),
        ));
        AudioRenderer::new(44_100.0, 1, s, 4_096)
    };

    let mut one = make();
    let mut a = vec![0.0f32; 44_100];
    one.render_into(&mut a);

    let mut many = make();
    let mut b = Vec::with_capacity(44_100);
    for n in [1usize, 7, 511, 997, 4_096, 38_488] {
        let mut chunk = vec![0.0f32; n];
        many.render_into(&mut chunk);
        b.extend_from_slice(&chunk);
    }
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert!((x - y).abs() < 1e-6, "sample {i}: {x} vs {y}");
    }
}

/// A keyboard note has to be audible even with the transport stopped and no pattern.
#[test]
fn a_midi_note_sounds_and_releases_over_a_silent_pattern() {
    let s = SharedState::new();
    s.midi_hz.set(220.0);
    s.midi_gate
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let mut r = AudioRenderer::new(48_000.0, 1, s.clone(), 2_048);

    let mut buf = vec![0.0f32; 24_000];
    r.render_into(&mut buf);
    assert!(peak(&buf) > 0.01, "the held MIDI note never sounded");

    s.midi_gate
        .store(false, std::sync::atomic::Ordering::Relaxed);
    let mut tail = vec![0.0f32; 24_000];
    r.render_into(&mut tail);
    assert!(
        peak(&tail[4_800..]) < 0.001,
        "the note should have released"
    );
}

#[test]
fn a_soft_midi_note_auditions_at_the_velocity_it_will_be_written_with() {
    // What the player hears has to match the step that gets written. A key struck softly used to audition at full level
    // and then save itself 12 dB quieter, which is exactly the mismatch the pitch change had already fixed.
    let level = |velocity: u32| {
        let s = SharedState::new();
        s.midi_hz.set(220.0);
        s.midi_velocity
            .store(velocity, std::sync::atomic::Ordering::Relaxed);
        s.midi_gate
            .store(true, std::sync::atomic::Ordering::Release);
        let mut r = AudioRenderer::new(48_000.0, 1, s, 2_048);
        let mut buf = vec![0.0f32; 12_000];
        r.render_into(&mut buf);
        peak(&buf[1_000..])
    };

    let soft = level(30);
    let hard = level(127);
    assert!(soft > 0.0, "a soft note still has to sound");
    assert!(
        soft < hard * 0.5,
        "velocity 30 ({soft}) should be clearly quieter than 127 ({hard})"
    );
}
