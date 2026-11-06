//! Restore on launch and the save debounce: a broken or hand-edited file must never stop the app opening.

use acid_app::persist::{
    AUTOSAVE_INTERVAL, Autosave, SavedState, load_or_default, sanitize, save,
};
use std::time::Instant;

#[test]
fn an_empty_directory_gives_the_factory_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = load_or_default(dir.path());
    assert_eq!(state, SavedState::default());
}

#[test]
fn a_hand_edited_file_is_clamped_into_range() {
    let dir = tempfile::tempdir().unwrap();
    let mut bad = SavedState {
        bpm: 9_000.0,
        swing: 4.0,
        ..Default::default()
    };
    bad.knobs.cutoff = 3.5;
    bad.knobs.waveform = 7;
    save(dir.path(), &bad).unwrap();
    let mut loaded = load_or_default(dir.path());
    sanitize(&mut loaded);
    assert_eq!(loaded.bpm, acid_core::timing::MAX_BPM);
    assert_eq!(loaded.swing, 1.0);
    assert!((loaded.knobs.cutoff - 1.0).abs() < 1e-6);
    assert!(loaded.knobs.waveform <= 1);
}

#[test]
fn a_corrupt_file_falls_back_to_defaults_instead_of_crashing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state.json"), b"garbage").unwrap();
    assert_eq!(load_or_default(dir.path()), SavedState::default());
}

#[test]
fn autosave_waits_for_the_interval_and_skips_when_clean() {
    let mut a = Autosave::new();
    let t0 = Instant::now();
    assert!(!a.should_save(false, t0));
    assert!(a.should_save(true, t0));
    assert!(!a.should_save(true, t0 + AUTOSAVE_INTERVAL / 2));
    assert!(a.should_save(true, t0 + AUTOSAVE_INTERVAL));
}

/// A save must survive the round trip through the file that `load_or_default` reads back.
#[test]
fn what_the_app_saves_is_what_it_restores() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = SavedState {
        bpm: 121.0,
        ..Default::default()
    };
    saved.knobs.resonance = 0.66;
    save(dir.path(), &saved).unwrap();
    let back = load_or_default(dir.path());
    assert!((back.bpm - 121.0).abs() < 1e-6);
    assert!((back.knobs.resonance - 0.66).abs() < 1e-6);
}
