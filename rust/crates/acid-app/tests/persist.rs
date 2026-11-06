//! The save file: round-trip, atomicity, and the two conversions between disk and running state.

use acid_app::persist::{
    ENV_DATA_DIR, SavedState, data_dir, load, save, saved_to_shared, shared_to_saved,
};
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

#[test]
fn a_missing_file_is_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load(dir.path()).unwrap().is_none());
}

#[test]
fn state_round_trips_through_json() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = SavedState {
        bpm: 143.0,
        swing: 0.25,
        ..Default::default()
    };
    saved.knobs.cutoff = 0.42;
    saved.bank.select(5);
    save(dir.path(), &saved).unwrap();

    let back = load(dir.path()).unwrap().expect("state.json should exist");
    assert!((back.bpm - 143.0).abs() < 1e-6);
    assert!((back.swing - 0.25).abs() < 1e-6);
    assert!((back.knobs.cutoff - 0.42).abs() < 1e-6);
    assert_eq!(back.bank.current(), 5);
}

#[test]
fn corrupt_json_reports_an_error_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state.json"), b"{ not json").unwrap();
    let err = load(dir.path()).unwrap_err();
    assert!(
        format!("{err}").to_lowercase().contains("state"),
        "unhelpful error: {err}"
    );
    // A failed load must not overwrite the user's file.
    let untouched = std::fs::read(dir.path().join("state.json")).unwrap();
    assert_eq!(untouched, b"{ not json");
}

#[test]
fn save_is_atomic_and_leaves_no_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    let entries: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(entries.contains(&"state.json".to_string()));
    assert!(
        !entries.iter().any(|n| n.ends_with(".tmp")),
        "left {entries:?}"
    );
}

#[test]
fn only_non_empty_patterns_get_their_own_file() {
    let dir = tempfile::tempdir().unwrap();
    // An explicitly empty bank, so the factory line is not what is being counted.
    let mut saved = SavedState {
        bank: acid_core::bank::PatternBank::new_empty(),
        ..Default::default()
    };
    saved.bank.select(3);
    saved
        .bank
        .replace(3, Pattern::from_steps([Step::note(Pitch::A, 0); 16]));
    save(dir.path(), &saved).unwrap();
    let files: Vec<String> = std::fs::read_dir(dir.path().join("patterns"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        files,
        vec!["A4.json".to_string()],
        "one file for one pattern"
    );
}

#[test]
fn the_shared_state_survives_a_save_and_restore() {
    let mut saved = SavedState {
        bpm: 155.0,
        ..Default::default()
    };
    saved.knobs.cutoff = 0.7;
    saved.knobs.waveform = 1;
    saved
        .bank
        .replace(0, Pattern::from_steps([Step::note(Pitch::C, 0); 16]));

    let shared = saved_to_shared(&saved);
    assert!((shared.tempo_bpm.get() - 155.0).abs() < 1e-6);
    assert!((shared.cutoff.get() - 0.7).abs() < 1e-6);
    assert_eq!(shared.waveform(), acid_core::oscillator::Waveform::Square);
    assert!(shared.pattern().steps[0].on);

    let back = shared_to_saved(&shared, SavedState::default().bank);
    assert!((back.bpm - 155.0).abs() < 1e-6);
    assert!((back.knobs.cutoff - 0.7).abs() < 1e-6);
    assert_eq!(back.knobs.waveform, 1);
    assert!(
        back.bank.get(0).steps[0].on,
        "the current pattern is captured"
    );
}

/// Runs in a child process (see the test below) with `TAKKA_ACID_DATA_DIR` set.
#[test]
fn data_dir_probe() {
    if std::env::var_os(ENV_DATA_DIR).is_some() {
        println!("probe: {}", data_dir().display());
    }
}

#[test]
fn data_dir_honours_the_environment_override() {
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .env(ENV_DATA_DIR, "/tmp/takka-acid-test")
        .args(["--exact", "data_dir_probe", "--nocapture"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("probe: /tmp/takka-acid-test"),
        "the override must win: {stdout}"
    );
}
