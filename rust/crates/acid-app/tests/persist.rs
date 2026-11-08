//! The save file: round-trip, atomicity, and the two conversions between disk and running state.

use acid_app::persist::{
    ENV_DATA_DIR, Knobs, SAVE_VERSION, SavedState, data_dir, load, open, save, saved_to_shared,
    shared_to_saved,
};
use acid_app::persist::{
    delete_song, list_songs, load_song, new_song, rename_song, save_song, song_name,
};
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
fn a_new_song_starts_from_the_starter_phrase_and_nothing_else() {
    let song = new_song();
    assert_eq!(
        song.version, SAVE_VERSION,
        "a new song is written in today's format"
    );
    assert_eq!(*song.bank.get(0), acid_core::factory::starter());
    assert!(
        song.bank.get(1).steps.iter().all(|step| !step.on),
        "only the first pattern carries notes"
    );
    assert!((song.bpm - acid_core::timing::DEFAULT_BPM).abs() < 1e-6);
    assert_eq!(song.song, None, "a new song has no name until it is saved");
}

#[test]
fn deleting_a_song_removes_its_file_and_refuses_a_bad_name() {
    let dir = tempfile::tempdir().unwrap();
    let saved = SavedState::default();
    save_song(dir.path(), "Doomed", &saved).unwrap();
    assert_eq!(list_songs(dir.path()), vec!["Doomed".to_string()]);

    delete_song(dir.path(), "Doomed").unwrap();
    assert!(list_songs(dir.path()).is_empty());
    assert!(
        delete_song(dir.path(), "Doomed").is_err(),
        "a second delete fails"
    );
    assert!(
        delete_song(dir.path(), "../oops").is_err(),
        "a path is not a name"
    );
}

#[test]
fn renaming_a_song_moves_its_file_and_refuses_a_taken_name() {
    let dir = tempfile::tempdir().unwrap();
    let saved = SavedState::default();
    save_song(dir.path(), "Old", &saved).unwrap();
    save_song(dir.path(), "Taken", &saved).unwrap();

    rename_song(dir.path(), "Old", "New").unwrap();
    assert_eq!(
        list_songs(dir.path()),
        vec!["New".to_string(), "Taken".to_string()],
        "the file moved, and the list says so"
    );

    // Retyping the name you see is not an error.
    assert!(rename_song(dir.path(), "New", "New").is_ok());
    // Renaming onto another song would destroy it.
    assert!(rename_song(dir.path(), "New", "Taken").is_err());
    // A missing source and a path-shaped name are errors.
    assert!(rename_song(dir.path(), "Gone", "Other").is_err());
    assert!(rename_song(dir.path(), "New", "../oops").is_err());
}

#[test]
fn the_learned_midi_mapping_round_trips_and_an_old_file_gets_the_factory_one() {
    let dir = tempfile::tempdir().unwrap();
    let saved = SavedState {
        cc_map: vec![10, 11, 12, 13, 14, 15, 16, 17],
        ..Default::default()
    };
    save(dir.path(), &saved).unwrap();
    assert_eq!(
        load(dir.path()).unwrap().unwrap().cc_map,
        vec![10, 11, 12, 13, 14, 15, 16, 17],
        "a learned rig survives a restart"
    );

    // A file from before the field: the factory pairing, not silence.
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    legacy.as_object_mut().unwrap().remove("cc_map");
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();
    assert_eq!(
        load(dir.path()).unwrap().unwrap().cc_map,
        acid_app::midi::cc::DEFAULT_CC.to_vec()
    );
}

#[test]
fn a_map_from_before_the_waveform_knob_keeps_every_fader_it_had() {
    // Seven entries is a map written when the switch could not be taught. The waveform's controller is prefixed and
    // nothing else moves: a fader that taught CUT OFF must still be CUT OFF after the upgrade.
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.insert("version".to_string(), serde_json::json!(4));
    object.insert(
        "cc_map".to_string(),
        serde_json::json!([10, 11, 12, 13, 14, 15, 16]),
    );
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();

    // `load_or_default` is what the app runs at startup, and it is where the migration happens.
    let loaded = acid_app::persist::load_or_default(dir.path());
    assert_eq!(
        loaded.cc_map,
        vec![
            acid_app::midi::cc::DEFAULT_CC[0],
            10,
            11,
            12,
            13,
            14,
            15,
            16,
            acid_app::midi::cc::DEFAULT_CC[8],
            acid_app::midi::cc::DEFAULT_CC[9],
            acid_app::midi::cc::DEFAULT_CC[10]
        ],
        "the switch gains its controller; every learned fader keeps its slot"
    );
    assert_eq!(loaded.version, SAVE_VERSION);
}

#[test]
fn a_map_from_before_the_pattern_selector_keeps_every_learned_fader() {
    // Eight entries is a map from the build before the pattern selector and the learnable tempo: both are appended, so
    // nothing that already had a slot may move.
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.insert("version".to_string(), serde_json::json!(5));
    object.insert(
        "cc_map".to_string(),
        serde_json::json!([30, 31, 32, 33, 34, 35, 36, 37]),
    );
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();

    let loaded = acid_app::persist::load_or_default(dir.path());
    assert_eq!(
        loaded.cc_map,
        vec![30, 31, 32, 33, 34, 35, 36, 37, 72, 21, 73],
        "the eight learned faders stay put; tempo, pattern and swing are appended"
    );
    assert_eq!(loaded.version, SAVE_VERSION);
}

#[test]
fn a_map_from_before_the_shuffle_knob_keeps_every_fader_it_had() {
    // Ten entries is a map from the build before the shuffle could be taught. Swing is appended at slot ten, so every
    // existing controller keeps its knob and a rig that already drove the shuffle keeps driving it.
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.insert("version".to_string(), serde_json::json!(7));
    object.insert(
        "cc_map".to_string(),
        serde_json::json!([40, 41, 42, 43, 44, 45, 46, 47, 48, 49]),
    );
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();

    let loaded = acid_app::persist::load_or_default(dir.path());
    assert_eq!(
        loaded.cc_map,
        vec![40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 73],
        "the shuffle gains its controller; every learned fader keeps its slot"
    );
    assert_eq!(loaded.version, SAVE_VERSION);
}

#[test]
fn every_wave_survives_a_round_trip() {
    // The switch used to be clipped to two faces on load, which turned SINE and TRIANGLE into SQUARE.
    let dir = tempfile::tempdir().unwrap();
    for wave in acid_core::oscillator::Waveform::ALL {
        let saved = SavedState {
            knobs: Knobs {
                waveform: wave.index() as u8,
                ..Knobs::default()
            },
            ..Default::default()
        };
        save(dir.path(), &saved).unwrap();
        assert_eq!(
            load(dir.path()).unwrap().unwrap().knobs.waveform,
            wave.index() as u8,
            "{wave:?} came back as another wave"
        );
    }
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
fn the_hover_help_switch_round_trips_and_an_old_file_gets_it_on() {
    let dir = tempfile::tempdir().unwrap();
    save(
        dir.path(),
        &SavedState {
            tooltips: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !load(dir.path()).unwrap().unwrap().tooltips,
        "off stays off"
    );

    // A file written before the field existed opens with the hints on, because help should not have to be found.
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    legacy.as_object_mut().unwrap().remove("tooltips");
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();
    assert!(load(dir.path()).unwrap().unwrap().tooltips);
}

#[test]
fn the_randomise_dials_round_trip_and_an_old_file_gets_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    save(
        dir.path(),
        &SavedState {
            density: 0.25,
            scale: 3,
            ..Default::default()
        },
    )
    .unwrap();
    let loaded = load(dir.path()).unwrap().unwrap();
    assert!((loaded.density - 0.25).abs() < 1e-6, "density survives");
    assert_eq!(loaded.scale, 3, "the scale survives");

    // A file from before the dials existed gets a half-full chromatic roll, which is where the button started.
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    let object = legacy.as_object_mut().unwrap();
    object.remove("density");
    object.remove("scale");
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();

    let loaded = acid_app::persist::load_or_default(dir.path());
    assert!((loaded.density - acid_app::ui::randomise::DEFAULT_DENSITY).abs() < 1e-6);
    assert_eq!(loaded.scale, 0, "chromatic");
}

#[test]
fn a_hand_edited_density_and_scale_are_clamped_into_range() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut wild: serde_json::Value = serde_json::from_str(&text).unwrap();
    let object = wild.as_object_mut().unwrap();
    object.insert("density".to_string(), serde_json::json!(7.5));
    object.insert("scale".to_string(), serde_json::json!(200));
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&wild).unwrap(),
    )
    .unwrap();

    let loaded = acid_app::persist::load_or_default(dir.path());
    assert_eq!(loaded.density, 1.0);
    assert_eq!(
        loaded.scale as usize,
        acid_app::ui::scale::Scale::ALL.len() - 1
    );
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
fn a_file_from_before_the_lattice_moved_keeps_its_sound() {
    // Version 2 and older were written when the panel's `C0` was A1: the same note is spelled 21 semitones higher now.
    // Loading one must shift every step, or every saved pattern would drop by a twelfth and a half.
    let dir = tempfile::tempdir().unwrap();
    // A file an older build would have written: version 2, before the lattice moved.
    let mut saved = SavedState {
        version: 2,
        ..SavedState::default()
    };
    let mut steps = [Step::rest(); 16];
    // The launcher's old spelling: D#0, which sounded 65.4 Hz.
    steps[0] = Step::note(Pitch::DSharp, 0);
    saved.bank.replace(0, Pattern::from_steps(steps));
    save(dir.path(), &saved).unwrap();

    // `load_or_default` is what the app runs at startup, and it is where the migration happens.
    let loaded = acid_app::persist::load_or_default(dir.path());
    let step = loaded.bank.get(0).steps[0];

    assert_eq!(
        loaded.version, SAVE_VERSION,
        "stamped with this build's version"
    );
    assert!(
        (step.hz() - 65.41).abs() < 0.5,
        "the note kept its frequency: {} Hz",
        step.hz()
    );
    assert_eq!(
        (step.pitch, step.octave_offset),
        (Pitch::C, 2),
        "and is spelled on the new lattice"
    );
}

#[test]
fn a_file_from_this_build_is_not_shifted_again() {
    // The migration must be a one-time step: a version-3 file loads exactly as it was written.
    let dir = tempfile::tempdir().unwrap();
    let mut saved = SavedState::default();
    let mut steps = [Step::rest(); 16];
    steps[0] = Step::note(Pitch::C, 3);
    saved.bank.replace(0, Pattern::from_steps(steps));
    save(dir.path(), &saved).unwrap();

    let loaded = load(dir.path()).unwrap().unwrap();
    let step = loaded.bank.get(0).steps[0];
    assert_eq!((step.pitch, step.octave_offset), (Pitch::C, 3));
    assert!((step.hz() - 130.81).abs() < 0.5, "C3, untouched");
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

/// The knob row's order is the file's order: `positions()` reads the seven tone knobs in `Knob::ALL` order and
/// `from_positions()` puts them back, so a knob cannot be saved into another knob's field.
#[test]
fn the_knob_order_the_file_saves_is_the_panel_order() {
    use acid_app::ui::waveform_position;
    use acid_core::oscillator::Waveform;

    // Every knob at a different position, so a swap of two fields cannot go unnoticed. The waveform is a switch, so its
    // slot has to start on one of its four faces.
    let positions: [f32; 7] = [
        waveform_position(Waveform::Sine),
        0.11,
        0.22,
        0.33,
        0.44,
        0.55,
        0.66,
    ];
    let knobs = Knobs::from_positions(&positions);
    assert_eq!(knobs.positions(), positions, "the file reordered the knobs");
}

/// The rewrite must not move a byte of the file. The struct on the left is built through the catalogue; the one on the
/// right is the hand-written literal this build used before, and is what an older file on disk looks like.
#[test]
fn a_file_written_by_this_build_is_the_file_the_old_build_wrote() {
    use acid_app::ui::waveform_position;
    use acid_core::oscillator::Waveform;

    let old = Knobs {
        waveform: 2,
        tuning: 0.1,
        cutoff: 0.2,
        resonance: 0.3,
        env_mod: 0.4,
        decay: 0.5,
        accent: 0.6,
        volume: 0.7,
    };
    let new = Knobs {
        volume: 0.7,
        ..Knobs::from_positions(&[
            waveform_position(Waveform::Sine),
            0.1,
            0.2,
            0.3,
            0.4,
            0.5,
            0.6,
        ])
    };

    assert_eq!(new, old, "the catalogue puts a knob in the wrong field");
    assert_eq!(
        serde_json::to_vec(&new).unwrap(),
        serde_json::to_vec(&old).unwrap(),
        "the field names or their order changed"
    );
}

/// The file and the running app agree knob for knob: what the panel reads is what the file stores, and loading it back
/// moves the same atomics. This is the pair of conversions `main` and a song load both go through.
#[test]
fn the_file_and_the_shared_state_agree_knob_for_knob() {
    use acid_app::knobs::Knob;
    use acid_app::state::SharedState;
    use acid_app::ui::waveform_position;
    use acid_core::oscillator::Waveform;

    let shared = SharedState::new();
    let positions: [f32; 7] = [
        waveform_position(Waveform::Triangle),
        0.15,
        0.25,
        0.35,
        0.45,
        0.65,
        0.75,
    ];
    for (knob, position) in Knob::ALL.iter().zip(positions) {
        knob.publish(&shared, position);
    }
    assert_eq!(
        Knob::ALL.map(|knob| knob.read(&shared)),
        positions,
        "the shared state did not take the positions"
    );

    let saved = shared_to_saved(&shared, SavedState::default().bank);
    assert_eq!(
        saved.knobs.positions(),
        positions,
        "the file lost a knob or moved it"
    );

    let back = saved_to_shared(&saved);
    for (knob, position) in Knob::ALL.iter().zip(saved.knobs.positions()) {
        assert!(
            (knob.read(&back) - position).abs() < 1e-6,
            "{knob:?} came back as {}",
            knob.read(&back)
        );
    }
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

/// Saving is manual unless the player turned autosave on; that choice has to survive a restart, and a file written before
/// the switch existed must come back with it off.
#[test]
fn the_autosave_switch_round_trips_and_an_old_file_gets_it_off() {
    let dir = tempfile::tempdir().unwrap();
    let saved = SavedState {
        autosave: true,
        ..Default::default()
    };
    save(dir.path(), &saved).unwrap();
    assert!(
        load(dir.path()).unwrap().unwrap().autosave,
        "the switch was lost"
    );

    // A file from before the field: take the key out and it must load as manual saving.
    let mut value: serde_json::Value = serde_json::to_value(&saved).unwrap();
    value.as_object_mut().unwrap().remove("autosave");
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let old = load(dir.path()).unwrap().expect("an old file still loads");
    assert!(
        !old.autosave,
        "an old file must not arrive with autosave on"
    );
}

/// A first run is not an empty machine any more: it opens on a demo, under the name a new song starts with, and marks the
/// session unsaved because nothing of it is on disk.
#[test]
fn a_first_run_opens_on_a_demo_under_a_new_name() {
    let dir = tempfile::tempdir().unwrap();
    let (opened, first_run) = open(dir.path());
    assert!(first_run, "a missing session file is a first run");
    assert_eq!(opened.song.as_deref(), Some("New song"));
    assert!(
        opened.bank.get(0).steps.iter().any(|step| step.on),
        "the opening demo plays something"
    );

    save(dir.path(), &opened).unwrap();
    let (again, first_run) = open(dir.path());
    assert!(!first_run, "a written session is not a first run");
    assert_eq!(again.bank.get(0), opened.bank.get(0));
}
