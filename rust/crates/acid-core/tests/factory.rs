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
