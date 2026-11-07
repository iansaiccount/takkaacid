//! The factory line: a phrase worth pressing RUN for, and seven variations that stay in its register.

use acid_core::bank::PatternBank;
use acid_core::factory::factory_patterns;
use acid_core::pitch::Pitch;

#[test]
fn there_are_eight_factory_patterns_and_the_first_is_a_line() {
    let patterns = factory_patterns();
    assert_eq!(patterns.len(), 8);
    let a1 = &patterns[0];
    let notes: Vec<usize> = a1
        .steps
        .iter()
        .enumerate()
        .filter(|(_, s)| s.on)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(notes, vec![0, 1, 3, 5, 6, 8, 10, 12, 13, 15], "A1's rhythm");
    assert!(
        a1.steps.iter().filter(|s| s.accent).count() >= 3,
        "needs accents"
    );
    assert!(
        a1.steps.iter().filter(|s| s.slide).count() >= 2,
        "needs slides"
    );
}

#[test]
fn pitches_stay_in_the_acids_home_register() {
    for p in factory_patterns() {
        for s in p.steps {
            assert!(s.pitch.semitones() <= 12);
            assert!(
                (-1..=1).contains(&s.octave_offset),
                "octave {}",
                s.octave_offset
            );
        }
    }
    assert_eq!(factory_patterns()[0].steps[6].pitch, Pitch::DSharp);
}

/// A slide is a promise to the next *note*, not to the next step: the sequencer only glides when the step it
/// glides into sounds. A slide followed by a rest closes the gate and does nothing at all — which is exactly
/// where the launcher shipped its two slides, so the factory line's slides were silent.
#[test]
fn every_slide_leads_into_a_sounding_step() {
    for (pattern_index, pattern) in factory_patterns().iter().enumerate() {
        for (index, step) in pattern.steps.iter().enumerate() {
            if step.slide {
                let next = pattern.steps[(index + 1) % acid_core::pattern::STEPS_PER_PATTERN];
                assert!(
                    next.on,
                    "pattern {pattern_index} step {index} slides into a rest, so nothing glides"
                );
            }
        }
    }
}

/// The two slides must be *audible* slides: the launcher slides down to E flat and slur into the accented
/// downbeat, both into a note that sounds.
#[test]
fn the_launcher_slides_into_a_note_and_into_the_downbeat() {
    let a1 = factory_patterns()[0];
    assert!(a1.steps[5].slide, "G slides down to E flat at step 7");
    assert_eq!(a1.steps[6].pitch, Pitch::DSharp);
    assert!(a1.steps[15].slide, "G slurs into the downbeat");
    assert_eq!(a1.steps[0].pitch, Pitch::C);
}

/// End to end on the shipped line: step 6 slides G down into E flat, so just after the boundary the voice is
/// still near G and by the end of the step it has dropped to E flat. This is what "the slide does something"
/// means, measured on `A1` rather than on a synthetic pattern.
#[test]
fn the_launcher_slide_actually_glides() {
    use acid_core::analysis::dominant_hz;
    use acid_core::sequencer::Sequencer;
    use acid_core::voice::Voice;

    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(std::sync::Arc::new(factory_patterns()[0]));
    seq.set_bpm(120.0); // one 16th is 6000 samples
    let mut voice = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; 42_000];
    seq.render(&mut voice, &mut buf);

    // Step 5 (G, sliding) starts at 30 000, step 6 (E flat) at 36 000, step 7 is a rest at 42 000.
    let early = dominant_hz(&buf[36_200..37_800], sr);
    let late = dominant_hz(&buf[40_400..41_900], sr);
    assert!(
        early > 74.0,
        "the glide should start near G (82 Hz), not jump to E flat: {early} Hz"
    );
    assert!(
        late < 72.0,
        "the glide should reach E flat (65 Hz) by the end of the step: {late} Hz"
    );
}

#[test]
fn the_factory_bank_fills_bank_a_and_leaves_the_rest_empty() {
    let bank = PatternBank::factory();
    let a = bank.get(0);
    assert!(a.steps[0].on);
    assert_eq!(bank.get(8).steps.iter().filter(|s| s.on).count(), 0);
    assert_eq!(bank.current(), 0);
}

#[test]
fn every_factory_pattern_renders_without_clipping() {
    for (i, p) in factory_patterns().iter().enumerate() {
        let mut e = acid_core::engine::Engine::new(48_000.0);
        e.set_pattern(std::sync::Arc::new(*p));
        e.start();
        let mut buf = vec![0.0; 48_000 * 2];
        e.render(&mut buf);
        assert!(
            buf.iter().all(|s| s.is_finite()),
            "pattern {i} produced NaN"
        );
        assert!(
            acid_core::analysis::peak(&buf) > 0.01,
            "pattern {i} is silent"
        );
        assert!(
            acid_core::analysis::peak(&buf) <= 0.999,
            "pattern {i} clipped"
        );
    }
}

#[test]
fn the_variations_keep_the_launchers_pitch_material() {
    let material = |p: &acid_core::pattern::Pattern| {
        let mut pitches: Vec<Pitch> = p.steps.iter().filter(|s| s.on).map(|s| s.pitch).collect();
        pitches.sort_by_key(|p| p.semitones());
        pitches.dedup();
        pitches
    };
    let launcher = material(&factory_patterns()[0]);
    for (i, p) in factory_patterns().iter().enumerate() {
        assert!(!p.steps.iter().all(|s| !s.on), "pattern {i} is empty");
        let pitches = material(p);
        assert!(
            pitches.iter().all(|pitch| launcher.contains(pitch)),
            "pattern {i} leaves the launcher's pitch set: {pitches:?}"
        );
    }
}
