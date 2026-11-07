//! The save file: round-trip, atomicity, and the two conversions between disk and running state.

use acid_app::persist::{
    ENV_DATA_DIR, SAVE_VERSION, SavedState, data_dir, load, save, saved_to_shared, shared_to_saved,
};
use acid_app::persist::{list_songs, load_song, save_song, song_name};
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

#[test]
fn a_song_name_has_to_be_a_name_and_not_a_path() {
    assert_eq!(song_name("Acid 1").as_deref(), Some("Acid 1"));
    assert_eq!(song_name("  Acid 1  ").as_deref(), Some("Acid 1"));
    assert_eq!(
        song_name(&"x".repeat(48)).as_deref(),
        Some("x".repeat(48).as_str())
    );
    for bad in [
        "",
        "   ",
        "../escape",
        "a/b",
        "a\\b",
        ".hidden",
        "a:b",
        "a?b",
        &"x".repeat(49),
    ] {
        assert_eq!(song_name(bad), None, "{bad:?} should be refused");
    }
}

#[test]
fn songs_round_trip_and_are_listed() {
    let dir = tempfile::tempdir().unwrap();
    let saved = SavedState {
        bpm: 137.0,
        song: Some("Acid 1".to_string()),
        ..Default::default()
    };
    save_song(dir.path(), "Acid 1", &saved).unwrap();
    save_song(dir.path(), "Another", &saved).unwrap();

    assert_eq!(
        list_songs(dir.path()),
        vec!["Acid 1".to_string(), "Another".to_string()],
        "listed by name, sorted"
    );
    let back = load_song(dir.path(), "Acid 1").unwrap();
    assert!(
        (back.bpm - 137.0).abs() < 1e-6,
        "the song is the whole session"
    );
    assert_eq!(back.song.as_deref(), Some("Acid 1"));
    assert_eq!(
        back.version, SAVE_VERSION,
        "songs are upgraded like the session file"
    );

    // A name that cannot be a file is refused rather than written somewhere surprising.
    assert!(save_song(dir.path(), "../oops", &saved).is_err());
    assert_eq!(list_songs(dir.path()).len(), 2, "and nothing was written");
}

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
fn the_chosen_midi_port_round_trips_and_an_old_file_still_loads() {
    let dir = tempfile::tempdir().unwrap();
    let saved = SavedState {
        midi_input: Some("Launchkey MIDI".to_string()),
        ..Default::default()
    };
    save(dir.path(), &saved).unwrap();
    assert_eq!(
        load(dir.path()).unwrap().unwrap().midi_input.as_deref(),
        Some("Launchkey MIDI")
    );

    // A file written before the field existed must still load, with no preference — and nothing may panic.
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    legacy.as_object_mut().unwrap().remove("midi_input");
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();
    assert_eq!(load(dir.path()).unwrap().unwrap().midi_input, None);
}

#[test]
fn a_saved_file_names_its_version_and_an_old_one_upgrades_to_it() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    assert_eq!(load(dir.path()).unwrap().unwrap().version, SAVE_VERSION);

    // A file written before the field existed: it loads, and comes back versioned.
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    legacy.as_object_mut().unwrap().remove("version");
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();
    let old = load(dir.path()).unwrap().unwrap();
    assert_eq!(old.version, 0, "no field, no version");
    let upgraded = acid_app::persist::load_or_default(dir.path());
    assert_eq!(upgraded.version, SAVE_VERSION, "loading upgrades it");
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
fn a_save_writes_one_file_and_leaves_no_pattern_directory() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = SavedState {
        bank: acid_core::bank::PatternBank::new_empty(),
        ..Default::default()
    };
    saved.bank.select(3);
    saved
        .bank
        .replace(3, Pattern::from_steps([Step::note(Pitch::A, 0); 16]));
    save(dir.path(), &saved).unwrap();

    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["state.json".to_string()],
        "one file, and it is the one load reads"
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
