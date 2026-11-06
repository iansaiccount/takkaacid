use acid_core::pattern::{Pattern, PatternId, Step, STEPS_PER_PATTERN};
use acid_core::pitch::Pitch;

#[test]
fn a_new_pattern_is_sixteen_rests() {
    let p = Pattern::empty();
    assert_eq!(p.steps.len(), STEPS_PER_PATTERN);
    assert!(p.steps.iter().all(|s| !s.on && !s.accent && !s.slide));
}

#[test]
fn a_note_step_reports_its_frequency() {
    let s = Step::note(Pitch::CHigh, 0);
    assert!(s.on);
    assert!((s.hz() - 110.0).abs() < 0.1);
    let up = Step::note(Pitch::CHigh, 1);
    assert!((up.hz() - 220.0).abs() < 0.2);
}

#[test]
fn patterns_round_trip_through_json() {
    let mut p = Pattern::empty();
    p.steps[0] = Step {
        accent: true,
        ..Step::note(Pitch::E, 0)
    };
    p.steps[3] = Step {
        slide: true,
        ..Step::note(Pitch::DSharp, 0)
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: Pattern = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn pattern_ids_read_like_the_hardware() {
    assert_eq!(
        PatternId {
            bank: 0,
            number: 0
        }
        .to_string(),
        "A1"
    );
    assert_eq!(
        PatternId {
            bank: 2,
            number: 4
        }
        .to_string(),
        "C5"
    );
    assert_eq!(
        PatternId {
            bank: 7,
            number: 7
        }
        .to_string(),
        "H8"
    );
}
