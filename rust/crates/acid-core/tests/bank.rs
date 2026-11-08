//! The pattern bank: 64 patterns addressed like the hardware's eight banks of eight, and live switching that
//! never restarts the loop.

use acid_core::bank::{PATTERNS_PER_BANK, PatternBank};
use acid_core::engine::Engine;
use acid_core::pattern::{Pattern, PatternId, Step};
use acid_core::pitch::Pitch;
use std::sync::Arc;

#[test]
fn sixty_four_patterns_are_addressed_as_a1_to_h8() {
    assert_eq!(PatternBank::id_of(0).to_string(), "A1");
    assert_eq!(PatternBank::id_of(63).to_string(), "H8");
    assert_eq!(PatternBank::id_of(PATTERNS_PER_BANK).to_string(), "B1");
    assert_eq!(PatternBank::index_of(PatternId { bank: 7, number: 7 }), 63);
}

#[test]
fn selecting_keeps_the_engine_at_its_current_step() {
    let sr = 48_000.0;
    let mut e = Engine::new(sr);
    let a = Pattern::from_steps([Step::note(Pitch::A, -1); 16]);
    let b = Pattern::from_steps([Step::note(Pitch::C, 1); 16]);
    e.set_pattern(Arc::new(a));
    e.set_bpm(120.0);
    e.start();
    e.render(&mut vec![0.0; 24_000]); // four steps in at 120 bpm
    let position = e.position();
    assert!(position >= 4, "should have advanced, at {position}");
    e.set_pattern(Arc::new(b));
    assert_eq!(
        e.position(),
        position,
        "switching must not restart the loop"
    );
    let mut buf = vec![0.0; 12_000];
    e.render(&mut buf);
    assert!(
        acid_core::analysis::rms(&buf[6_000..]) > 0.01,
        "new pattern must keep sounding"
    );
}

#[test]
fn the_bank_round_trips_through_json() {
    let mut bank = PatternBank::new_empty();
    bank.replace(9, Pattern::from_steps([Step::note(Pitch::D, 0); 16]));
    bank.select(9);
    let json = serde_json::to_string(&bank).unwrap();
    let back: PatternBank = serde_json::from_str(&json).unwrap();
    assert_eq!(back.current(), 9);
    assert!(back.get(9).steps[0].on);
}

#[test]
fn selecting_out_of_range_clamps_instead_of_losing_the_bank() {
    let mut bank = PatternBank::new_empty();
    bank.select(999);
    assert_eq!(bank.current(), 63);
    bank.select(0);
    assert_eq!(bank.current(), 0);
    assert_eq!(
        bank.get(999).steps.len(),
        16,
        "a stray index reads the last pattern"
    );
}

#[test]
fn a_truncated_bank_is_repaired_rather_than_panicking() {
    // A save file is JSON a human may edit, and a shortened `patterns` array used to index out of bounds on the very
    // first read — taking the app down at startup, before anything could be shown or repaired.
    let mut short: PatternBank =
        serde_json::from_value(short_bank_json(1, 5)).expect("a short bank still parses");
    assert_eq!(short.pattern_count(), 1, "the file really is short");

    // Reading before the repair must not panic either: `get` is on the audio path.
    assert_eq!(short.get(5).steps.len(), 16);

    short.normalise();
    assert_eq!(short.pattern_count(), acid_core::bank::TOTAL_PATTERNS);
    assert_eq!(
        short.current(),
        0,
        "the selection is clamped into what exists"
    );
}

#[test]
fn a_bank_with_no_patterns_at_all_reads_silence_instead_of_panicking() {
    let empty: PatternBank =
        serde_json::from_value(short_bank_json(0, 7)).expect("an empty bank parses");
    assert_eq!(empty.get(7).steps.len(), 16);
    assert!(!empty.get(7).steps.iter().any(|step| step.on));
}

/// A real bank as JSON with its `patterns` array cut down to `keep` entries and its selection set to `current` — the shape
/// a truncated or hand-edited save file arrives in.
fn short_bank_json(keep: usize, current: usize) -> serde_json::Value {
    let mut value = serde_json::to_value(PatternBank::new_empty()).expect("a bank serialises");
    let patterns = value["patterns"].as_array().expect("patterns is an array");
    let kept = patterns[..keep].to_vec();
    value["patterns"] = serde_json::Value::Array(kept);
    value["current"] = serde_json::json!(current);
    value
}
