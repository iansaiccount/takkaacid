//! The save file: round-trip, atomicity, and the two conversions between disk and running state.

use acid_app::device::{DeviceKind, MAX_DEVICES, Rack, Scheme};
use acid_app::midi::cc::LEARNABLE;
use acid_app::midi::route::{Channel, InputFilter, Ports, Role};
use acid_app::persist::{
    BankDocument, SaveTarget, bank_kind_notice, bank_path, delete_bank, delete_song, document_name,
    export_song, free_bank_name, free_song_name, import_song, list_banks, list_songs, load_bank,
    load_song, new_song, save_bank, save_song, save_target, session_song_file,
};
use acid_app::persist::{
    ENV_DATA_DIR, Knobs, SAVE_VERSION, SavedDevice, SavedInputs, SavedState, data_dir, load, open,
    rack_to_saved, sanitize, save, saved_to_shared, shared_to_saved, unopened_ports, wanted_kits,
};
use acid_core::bank::PatternBank;
use acid_core::drums::Drum;
use acid_core::pattern::{Grid, Pattern, Step};
use acid_core::pitch::Pitch;

#[test]
fn a_document_name_has_to_be_a_name_and_not_a_path() {
    assert_eq!(document_name("Acid 1").as_deref(), Some("Acid 1"));
    assert_eq!(document_name("  Acid 1  ").as_deref(), Some("Acid 1"));
    assert_eq!(
        document_name(&"x".repeat(48)).as_deref(),
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
        assert_eq!(document_name(bad), None, "{bad:?} should be refused");
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

/// A song file names the file it is in, not the song it came from: the field is what a restart seeds the session's file from, so
/// a copy written under a new name must not leave the copy pointing back at the original.
#[test]
fn a_saved_song_names_the_file_it_lands_in() {
    let dir = tempfile::tempdir().unwrap();
    let saved = SavedState {
        song: Some("Acid 1".to_string()),
        ..SavedState::default()
    };
    save_song(dir.path(), "Acid 2", &saved).unwrap();
    assert_eq!(
        load_song(dir.path(), "Acid 2").unwrap().song.as_deref(),
        Some("Acid 2"),
        "a copy names the file it was written to, not the song it was copied from"
    );
}

/// A session's name is not its file. A demo and a `NEW` song are named with nothing written under the name yet, so a launch
/// that took the name for a file would have the next `SAVE` write `songs/New song.json` — over whatever the player had there.
#[test]
fn a_session_claims_a_song_file_only_when_the_file_is_there() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        session_song_file(dir.path(), None),
        None,
        "a session with no name came from no file"
    );
    assert_eq!(
        session_song_file(dir.path(), Some("New song")),
        None,
        "a demo's name has nothing on disk behind it"
    );
    save_song(dir.path(), "Acid 1", &SavedState::default()).unwrap();
    assert_eq!(
        session_song_file(dir.path(), Some("Acid 1")),
        Some("Acid 1".to_string()),
        "a name with a file behind it is the file the session is on"
    );
    assert_eq!(
        session_song_file(dir.path(), Some("../oops")),
        None,
        "and a name that is not a name names no file, whatever is beside the data directory"
    );
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

/// The free-name rule both save paths share: a taken name lands beside its namesake, and the result is always a name
/// that could be saved — a suffix never pushes it past the limit.
#[test]
fn a_free_name_is_the_wanted_one_or_a_numbered_neighbour() {
    let dir = tempfile::tempdir().unwrap();
    let saved = SavedState::default();
    assert_eq!(free_song_name(dir.path(), "Acid 1"), "Acid 1");

    save_song(dir.path(), "Acid 1", &saved).unwrap();
    assert_eq!(free_song_name(dir.path(), "Acid 1"), "Acid 1 (2)");
    save_song(dir.path(), "Acid 1 (2)", &saved).unwrap();
    assert_eq!(free_song_name(dir.path(), "Acid 1"), "Acid 1 (3)");

    // A name at the limit still leaves room for the suffix, or the "free" name would be one that cannot be saved.
    let long = "y".repeat(48);
    save_song(dir.path(), &long, &saved).unwrap();
    let beside = free_song_name(dir.path(), &long);
    assert_ne!(beside, long);
    assert!(
        document_name(&beside).is_some(),
        "a free name is a saveable name: {beside:?}"
    );
    save_song(dir.path(), &beside, &saved).unwrap();
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

/// The version number is part of the file's contract, not housekeeping: it is what a later build reads to know a document
/// it cannot handle, and what a reader of a diff sees to know a migration was meant. Pinning the number here is what makes
/// an accidental bump fail the suite instead of passing every test that compares a file against `SAVE_VERSION` itself.
/// (Nothing in this build yet refuses a *newer* version — see the wishlist — so the number is a promise, not a guard.)
#[test]
fn the_save_version_is_pinned_at_thirteen() {
    assert_eq!(SAVE_VERSION, 13, "a bump is a migration, not an edit");

    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    assert!(
        text.contains("\"version\": 13"),
        "the document names the version it was written in"
    );
    // `devices` is appended after every field a version-11 build knows, so its prefix — `kit` included — still reads.
    assert!(
        text.find("\"kit\"").unwrap() < text.find("\"devices\"").unwrap(),
        "the device list is the last field, and a session with no kit still names one"
    );
    assert!(
        text.contains("\"devices\": []"),
        "one device is the empty list: the mirror above is the machine, and `sanitize` builds it back from there"
    );
}

/// A pattern's grid travels with it: a thirty-second bar writes thirty-two steps and says which grid they are on.
#[test]
fn a_thirty_second_pattern_writes_thirty_two_steps() {
    let dir = tempfile::tempdir().unwrap();
    let mut written = SavedState::default();
    written
        .bank
        .replace(0, Pattern::empty_in(Grid::ThirtySecond));
    save(dir.path(), &written).unwrap();

    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    let pattern = &document["bank"]["patterns"][0];
    assert_eq!(pattern["grid"], "1/32");
    assert_eq!(
        pattern["steps"].as_array().unwrap().len(),
        32,
        "one step per slot of the bar"
    );

    let loaded = load(dir.path())
        .expect("a file this build wrote is readable")
        .expect("the file is on disk")
        .upgraded();
    assert_eq!(loaded.bank.get(0).grid, Grid::ThirtySecond);
    assert_eq!(loaded.bank.get(0).len(), 32);
}

/// The version-12 shape: a sixteenth pattern whose step list is sixteen entries and which has no `grid` field at all.
/// That is what an older build wrote, and the sentence the round's risk note rests on is that this build still reads it.
#[test]
fn a_version_twelve_pattern_shape_still_reads() {
    let dir = tempfile::tempdir().unwrap();
    let mut written = SavedState::default();
    written
        .bank
        .replace(0, Pattern::from_steps([Step::note(Pitch::A, 1); 16]));
    save(dir.path(), &written).unwrap();

    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    let pattern = document["bank"]["patterns"][0]
        .as_object_mut()
        .expect("a pattern is an object");
    assert_eq!(
        pattern["steps"].as_array().unwrap().len(),
        16,
        "a sixteenth bar writes sixteen steps, not thirty-two"
    );
    assert!(
        pattern.remove("grid").is_some(),
        "this build writes the grid, so an older one would not"
    );
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();

    let loaded = load(dir.path())
        .expect("a version-12 shape is readable")
        .expect("the file is on disk");
    let pattern = loaded.bank.get(0);
    assert_eq!(
        pattern.grid,
        Grid::Sixteenth,
        "no grid means the grid every version-12 file was on"
    );
    assert_eq!(pattern.len(), 16);
    assert!(pattern.steps[0].on, "the sixteen steps came through");
}

/// The kit's name is appended after every field a version-10 file has, so such a file still reads: its prefix is unchanged
/// and the one field it never wrote arrives as `None` — "no kit", which is what a session saved before the drum machine had.
#[test]
fn a_version_ten_file_upgrades_with_no_kit() {
    let dir = tempfile::tempdir().unwrap();
    // A value this build does not default to, so a file that was refused and replaced cannot pass by looking like one.
    let written = SavedState {
        bpm: 138.0,
        ..SavedState::default()
    };
    save(dir.path(), &written).unwrap();

    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    legacy["version"] = serde_json::json!(10);
    assert!(
        legacy.as_object_mut().unwrap().remove("kit").is_some(),
        "the field this build appends is the one a version-10 file never had"
    );
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();

    let loaded = acid_app::persist::load(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.version, 10, "the file says what it is");
    assert_eq!(loaded.kit, None, "a version-10 file names no kit");
    let upgraded = loaded.upgraded();
    assert_eq!(upgraded.version, SAVE_VERSION);
    assert_eq!(upgraded.kit, None, "and upgrading invents nothing");
    assert!(
        (upgraded.bpm - 138.0).abs() < 1e-6,
        "the rest of the document came through"
    );
}

/// The kit's name is the drum machine's one addition to the session: a device that loaded a kit must come back playing it.
#[test]
fn a_saved_session_names_the_kit_its_device_plays() {
    let dir = tempfile::tempdir().unwrap();
    let state = SavedState {
        kit: Some("Acoustic (2)".to_string()),
        ..SavedState::default()
    };
    save(dir.path(), &state).unwrap();
    let loaded = acid_app::persist::load(dir.path()).unwrap().unwrap();
    assert_eq!(
        loaded.kit.as_deref(),
        Some("Acoustic (2)"),
        "the kit a session names is the kit it comes back with"
    );
}

/// A device that has loaded no kit has nothing to name: `None` is the default kit, and writing the default kit's own name
/// would make every restart look for a kit folder called `Init`, which no kit folder is.
#[test]
fn a_session_still_on_the_default_kit_names_no_kit() {
    let shared = acid_app::persist::saved_to_shared(&SavedState::default());
    let saved = shared_to_saved(&shared, PatternBank::factory());
    assert_eq!(
        saved.kit, None,
        "a device on the default kit has no kit to name"
    );

    let mut kit = acid_core::drums::default_kit();
    kit.name = "Acoustic (2)".to_string();
    shared.set_kit(std::sync::Arc::new(kit));
    let saved = shared_to_saved(&shared, PatternBank::factory());
    assert_eq!(
        saved.kit.as_deref(),
        Some("Acoustic (2)"),
        "and a device on a loaded kit names it"
    );
}

/// Remove a step's drum masks from a document, wherever the steps are: this is the shape a version-9 file has, since no
/// step then carried a mask at all. Answers how many keys it took out, so a test cannot pass on a document that never had
/// them.
fn strip_drum_masks(value: &mut serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(fields) => {
            let mut removed = 0;
            // One name per mask a step carries, and a seventeenth voice would be a third: a mask this list does not know
            // would stay in the "version 9" document and the count below would keep passing.
            for key in ["hits", "accents"] {
                if fields.remove(key).is_some() {
                    removed += 1;
                }
            }
            removed + fields.values_mut().map(strip_drum_masks).sum::<usize>()
        }
        serde_json::Value::Array(items) => items.iter_mut().map(strip_drum_masks).sum(),
        _ => 0,
    }
}

#[test]
fn a_version_nine_file_upgrades_with_empty_drum_masks() {
    // Version 9 is the build before the drum machine: not one step names `hits` or `accents`. Such a file has to upgrade
    // rather than be refused, arrive with every mask zeroed — a bit left set would strike a voice the player never
    // struck — and give the 303's phrase back byte for byte.
    let dir = tempfile::tempdir().unwrap();
    // A value this build does not default to, so a file that was refused and replaced by a fresh session cannot pass the
    // round trip below by looking like the one that was written.
    let written = SavedState {
        bpm: 137.0,
        ..SavedState::default()
    };
    save(dir.path(), &written).unwrap();

    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .insert("version".to_string(), serde_json::json!(9));
    let stripped = strip_drum_masks(&mut legacy);
    assert_eq!(
        stripped,
        written.bank.pattern_count() * Grid::Sixteenth.steps_per_bar() * 2,
        "every step of every pattern lost both masks"
    );
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();

    // Read it the way `open` does — `load`, then `upgraded` — and not through `load_or_default`, whose "a broken file means
    // start again" fallback would answer with a fresh session and let this test pass on a file that was never understood.
    let version_nine = acid_app::persist::load(dir.path())
        .expect("a version-9 file is readable")
        .expect("the file is on disk");
    let loaded = version_nine.upgraded();
    assert_eq!(
        loaded.version, SAVE_VERSION,
        "loading stamps it with this build's version"
    );
    assert!(
        (loaded.bpm - 137.0).abs() < 1e-6,
        "the file's own session came back, not the defaults a refused file falls back to"
    );
    for slot in 0..loaded.bank.pattern_count() {
        for step in &loaded.bank.get(slot).steps {
            assert_eq!(
                (step.hits, step.accents),
                (0, 0),
                "pattern {slot} came back carrying a drum"
            );
        }
    }
    assert_eq!(
        serde_json::to_vec(&loaded).unwrap(),
        serde_json::to_vec(&written).unwrap(),
        "the 303's steps come back byte for byte"
    );
}

/// The masks are the drum pattern, so the file has to carry them both ways. Every other test in this file holds zeros, which
/// is exactly the shape a mask that is written and never read would pass — the player would reopen a song to a silent bar.
#[test]
fn a_drum_step_survives_the_save_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut written = SavedState::default();
    let mut steps = [Step::rest(); acid_core::pattern::PATTERN_SLOTS];
    steps[3].hit(Drum::Kick, false);
    steps[3].hit(Drum::Snare, true);
    steps[7].hit(Drum::Cowbell, true);
    written.bank.replace(0, Pattern::from_steps(steps));
    save(dir.path(), &written).unwrap();

    let loaded = acid_app::persist::load(dir.path())
        .expect("a file this build wrote is readable")
        .expect("the file is on disk")
        .upgraded();

    let struck = loaded.bank.get(0).steps[3];
    assert!(struck.on, "the step sounds because voices hit it");
    assert!(struck.is_hit(Drum::Kick));
    assert!(
        !struck.is_accented(Drum::Kick),
        "an unaccented strike stayed plain"
    );
    assert!(struck.is_hit(Drum::Snare));
    assert!(struck.is_accented(Drum::Snare), "an accent survived");
    assert!(!struck.is_hit(Drum::Cowbell), "step 4 is another step's");

    let later = loaded.bank.get(0).steps[7];
    assert!(later.is_hit(Drum::Cowbell) && later.is_accented(Drum::Cowbell));
    assert_eq!(
        serde_json::to_vec(&loaded).unwrap(),
        serde_json::to_vec(&written).unwrap(),
        "the masks come back byte for byte"
    );
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
    let opened = open(dir.path());
    assert!(opened.first_run, "a missing session file is a first run");
    let saved = opened.state;
    assert_eq!(saved.song.as_deref(), Some("New song"));
    assert!(
        saved.bank.get(0).steps.iter().any(|step| step.on),
        "the opening demo plays something"
    );

    save(dir.path(), &saved).unwrap();
    let again = open(dir.path());
    assert!(!again.first_run, "a written session is not a first run");
    assert_eq!(again.state.bank.get(0), saved.bank.get(0));
}

/// A song can leave the library and come back: the export is the library format, so the import is a plain copy in.
#[test]
fn a_song_can_leave_the_library_and_come_back() {
    let dir = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let saved = SavedState {
        bpm: 137.0,
        ..SavedState::default()
    };
    save_song(dir.path(), "Acid 1", &saved).unwrap();

    let exported = out.path().join("Acid 1.json");
    export_song(&exported, &saved).unwrap();
    assert!(exported.exists(), "the export is a file on disk");

    let other = tempfile::tempdir().unwrap();
    let landed = import_song(&exported, other.path()).unwrap();
    assert_eq!(landed, "Acid 1");
    assert_eq!(list_songs(other.path()), vec!["Acid 1".to_string()]);
    assert_eq!(load_song(other.path(), "Acid 1").unwrap().bpm, 137.0);
}

/// The library is not overwritten by an import: a taken name lands beside its namesake.
#[test]
fn an_import_with_a_taken_name_lands_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let saved = SavedState {
        bpm: 120.0,
        ..SavedState::default()
    };
    save_song(dir.path(), "Acid 1", &saved).unwrap();
    let exported = out.path().join("Acid 1.json");
    let louder = SavedState {
        bpm: 90.0,
        ..SavedState::default()
    };
    export_song(&exported, &louder).unwrap();

    let landed = import_song(&exported, dir.path()).unwrap();
    assert_eq!(landed, "Acid 1 (2)");
    assert_eq!(
        list_songs(dir.path()),
        vec!["Acid 1".to_string(), "Acid 1 (2)".to_string()]
    );
    assert_eq!(
        load_song(dir.path(), "Acid 1").unwrap().bpm,
        120.0,
        "the song that was there keeps its tempo"
    );
    assert_eq!(load_song(dir.path(), "Acid 1 (2)").unwrap().bpm, 90.0);
}

/// A file that is not a song is refused before anything touches the library.
#[test]
fn importing_something_that_is_not_a_song_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let junk = out.path().join("notes.json");
    std::fs::write(&junk, b"not a song at all").unwrap();
    assert!(import_song(&junk, dir.path()).is_err());
    assert!(list_songs(dir.path()).is_empty());
    assert!(!dir.path().join("songs").exists());
}

/// The save button writes in place only when the name has not changed; any other name is a new file, so a save never
/// moves a song out from under its own name.
#[test]
fn a_save_writes_in_place_only_when_the_name_has_not_changed() {
    let same = |name: &str| Some(SaveTarget::InPlace(name.to_string()));
    let new = |name: &str| Some(SaveTarget::NewFile(name.to_string()));
    assert_eq!(save_target(Some("Acid 1"), "Acid 1"), same("Acid 1"));
    assert_eq!(save_target(Some("Acid 1"), "  Acid 1  "), same("Acid 1"));
    assert_eq!(save_target(Some("Acid 1"), "Acid 2"), new("Acid 2"));
    assert_eq!(save_target(None, "Acid 1"), new("Acid 1"));
    // A blank field means "keep the song I am on"; with nothing open there is nothing to keep.
    assert_eq!(save_target(Some("Acid 1"), ""), same("Acid 1"));
    assert_eq!(save_target(None, ""), None);
    assert_eq!(save_target(Some("Acid 1"), "a/b"), same("Acid 1"));
}

#[test]
fn a_rack_built_from_a_session_carries_the_session_into_the_mixer() {
    // `main` hands the bus to the audio thread; a rack of one must publish exactly the session it was built from.
    let saved = acid_app::persist::SavedState::default();
    let rack = acid_app::device::Rack::from_saved(&saved);
    rack.publish();
    let held = rack.hold();
    assert_eq!(held.slots.len(), 1);
    assert_eq!(held.slots[0].tempo_bpm.get(), saved.bpm);
}

/// One device for the file tests, with every field set to something a default would not produce, so a field dropped on the way
/// through the file is a failure rather than a coincidence.
fn a_saved_device(number: usize, name: &str, kind: &str) -> SavedDevice {
    SavedDevice {
        number,
        name: name.to_string(),
        kind: kind.to_string(),
        scheme: "VINYL NOIR".to_string(),
        inputs: SavedInputs {
            ports: Ports::Named(vec![format!("Port {number}")]),
            channel: Channel::One(number as u8),
            role: Role::ControlOnly,
        },
        bank: if number == 1 {
            PatternBank::factory()
        } else {
            PatternBank::new_empty()
        },
        knobs: Knobs {
            cutoff: 0.1 * number as f32,
            ..Knobs::default()
        },
        swing: 0.1 * number as f32,
        cc_map: vec![number as u8 + 10; LEARNABLE.len()],
        kit: (kind == "DRUMS").then(|| "808".to_string()),
    }
}

#[test]
fn a_three_device_song_round_trips_every_field() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = SavedState {
        devices: vec![
            a_saved_device(1, "BASS", "ACID"),
            a_saved_device(2, "DRUMS", "DRUMS"),
            a_saved_device(3, "LEAD", "ACID"),
        ],
        ..SavedState::default()
    };
    // The mirror is deliberately **stale**: every flat field disagrees with device 1, so a rewrite that skips one is caught
    // rather than passing because it happened to be right already.
    state.bank = PatternBank::new_empty();
    state.knobs.cutoff = 0.9;
    state.swing = 0.9;
    state.cc_map = vec![1u8; LEARNABLE.len()];
    state.kit = Some("909".to_string());
    save(dir.path(), &state).unwrap();
    let mut loaded = load(dir.path()).unwrap().unwrap();
    sanitize(&mut loaded);
    let loaded = loaded.upgraded();

    assert_eq!(loaded.devices, state.devices, "every field of every device");
    // The flat fields are device 1's mirror, which is the machine a version-11 build can read.
    let first = &loaded.devices[0];
    assert_eq!(loaded.bank, first.bank, "the mirror's bank");
    assert_eq!(loaded.knobs, first.knobs, "the mirror's patch");
    assert_eq!(loaded.swing, first.swing, "the mirror's shuffle");
    assert_eq!(loaded.cc_map, first.cc_map, "the mirror's learned rig");
    assert_eq!(loaded.kit, first.kit, "the mirror's kit");
}

/// Write the file a version-11 build would have written, with `devices` absent and `kit` set to whatever the caller says.
fn write_a_version_eleven_file(dir: &std::path::Path, kit: serde_json::Value) {
    let mut value = serde_json::to_value(SavedState::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("devices");
    object.insert("version".to_string(), serde_json::json!(11));
    object.insert("kit".to_string(), kit);
    std::fs::write(dir.join("state.json"), serde_json::to_vec(&value).unwrap()).unwrap();
}

#[test]
fn a_version_eleven_file_is_one_device_and_its_kit_names_the_kind() {
    // A version-11 file records no kind at all, so the kit it names is the only evidence there is: only a machine that has a
    // kit ever writes one.
    let drums = tempfile::tempdir().unwrap();
    write_a_version_eleven_file(drums.path(), serde_json::json!("808"));
    let opened = open(drums.path());
    assert!(
        !opened.first_run,
        "the file is readable, so this is not a first run"
    );
    let loaded = opened.state;
    assert_eq!(loaded.devices.len(), 1);
    assert_eq!(loaded.devices[0].kind, "DRUMS");
    assert_eq!(loaded.devices[0].kit.as_deref(), Some("808"));

    let acid = tempfile::tempdir().unwrap();
    write_a_version_eleven_file(acid.path(), serde_json::Value::Null);
    let loaded = open(acid.path()).state;
    assert_eq!(loaded.devices.len(), 1);
    assert_eq!(loaded.devices[0].kind, "ACID");
}

/// A save writes the pattern the engine is playing, not the copy the bank last stashed. A step typed into the editor lives in
/// `SharedState` — the audio thread reads it there — and the bank only learns of it on a move, a paste or a load, so a save that
/// read the bank alone would silently drop the player's last edit.
#[test]
fn a_save_carries_the_pattern_being_edited() {
    let mut rack = Rack::from_saved(&SavedState::default());
    // A step the pattern is not playing, so turning it on is an edit the bank cannot already have.
    let index = (0..16)
        .find(|index| !rack.state().pattern().steps[*index].on)
        .expect("a pattern with a resting step");
    let mut step = rack.state().pattern().steps[index];
    step.on = true;
    rack.device_mut().state.set_step(index, step);
    assert!(
        !rack.bank().get(0).steps[index].on,
        "the bank has not been told about the edit, which is why the merge is needed"
    );

    let saved = rack_to_saved(&rack);
    assert!(
        saved.bank.get(0).steps[index].on,
        "the mirror carries the edit"
    );
    assert!(
        saved.devices[0].bank.get(0).steps[index].on,
        "and every device in the list does too"
    );
}
#[test]
fn sanitize_repairs_a_devices_list() {
    let mut state = SavedState {
        devices: vec![
            a_saved_device(1, "BASS", "ACID"),
            a_saved_device(2, "BASS", "ACID"),
            // A duplicate number, a blank name and a kind this build does not have, all on one machine.
            a_saved_device(2, "", "SAMPLER"),
            a_saved_device(3, "LEAD", "DRUMS"),
            // One more than the strip can show.
            a_saved_device(4, "PAD", "ACID"),
        ],
        ..SavedState::default()
    };
    state.devices[0].scheme = "CHROME".to_string();
    state.devices[0].swing = 3.0;
    state.devices[0].cc_map = vec![0u8; 99];
    sanitize(&mut state);

    assert_eq!(state.devices.len(), MAX_DEVICES, "the fifth is dropped");
    let names: Vec<&str> = state.devices.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names[0], "BASS");
    assert_eq!(names[1], "BASS (2)", "a duplicate name is suffixed");
    assert_eq!(
        names[2], "ACID 4",
        "a blank name falls back to the repaired machine's default, built from the repaired number"
    );
    assert_eq!(names[3], "LEAD");
    assert_eq!(state.devices[2].kind, "ACID", "an unknown kind is an acid");
    assert_eq!(
        state.devices[0].scheme, "BLACK",
        "an unknown theme is BLACK"
    );
    assert_eq!(state.devices[0].swing, 1.0, "swing is a 0..=1 position");
    assert_eq!(state.devices[0].cc_map.len(), LEARNABLE.len());
    let numbers: Vec<usize> = state.devices.iter().map(|one| one.number).collect();
    let mut distinct = numbers.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), numbers.len(), "a duplicate number is moved");
}

#[test]
fn a_rack_of_two_devices_snapshots_both_and_mirrors_the_first() {
    let mut rack = Rack::from_saved(&SavedState::default());
    rack.rename(0, "BASS");
    rack.set_scheme(0, Scheme::VinylNoir);
    // A shuffle and a learned controller of its own, so a field dropped on the way out is caught rather than defaulted.
    rack.device_mut().state.swing.set(0.4);
    rack.device_mut().state.cc_map[0].store(74, std::sync::atomic::Ordering::Relaxed);
    let kit = rack.add(DeviceKind::Drums).unwrap();
    rack.rename(kit, "KIT");
    rack.set_scheme(kit, Scheme::FestivalCitrus);
    rack.set_inputs(kit, InputFilter::silent());
    rack.set_tempo_bpm(133.0);

    let saved = rack_to_saved(&rack);
    assert_eq!(saved.devices.len(), 2);
    assert_eq!(saved.devices[0].name, "BASS");
    assert_eq!(saved.devices[0].scheme, "VINYL NOIR");
    assert_eq!(saved.devices[0].number, 1);
    assert_eq!(
        saved.devices[0].swing, 0.4,
        "a machine's shuffle is its own"
    );
    assert_eq!(
        saved.devices[0].cc_map[0], 74,
        "a machine's learned rig is its own"
    );
    assert_eq!(saved.devices[1].name, "KIT");
    assert_eq!(saved.devices[1].kind, "DRUMS");
    assert_eq!(saved.devices[1].scheme, "FESTIVAL CITRUS");
    assert_eq!(saved.devices[1].number, 2);
    assert_eq!(
        saved.devices[1].inputs.ports,
        Ports::Named(Vec::new()),
        "a device that hears nothing says so"
    );
    assert_eq!(saved.bpm, 133.0);
    // The mirror is device 1, so a build that knows version 11 reads the machine this one calls BASS.
    let first = &saved.devices[0];
    assert_eq!(saved.bank, first.bank);
    assert_eq!(saved.knobs, first.knobs);
    assert_eq!(saved.swing, first.swing);
    assert_eq!(saved.cc_map, first.cc_map);
    assert_eq!(saved.kit, first.kit);
}

#[test]
fn a_sessions_kits_are_one_per_drum_machine() {
    let mut nameless = a_saved_device(3, "DRUMS 2", "DRUMS");
    nameless.kit = None;
    let state = SavedState {
        devices: vec![
            a_saved_device(1, "BASS", "ACID"),
            a_saved_device(2, "DRUMS", "DRUMS"),
            nameless,
        ],
        ..SavedState::default()
    };
    assert_eq!(
        wanted_kits(&state),
        vec![
            (1, "808".to_string()),
            // A drum machine that names no kit plays the default one, which is the 808 by name.
            (2, acid_app::kits::default_name().to_string()),
        ],
        "one entry per drum machine, and none for the acid"
    );
}

#[test]
fn a_song_from_another_machine_names_ports_this_one_has_not_got() {
    let mut acid = a_saved_device(1, "BASS", "ACID");
    acid.inputs.ports = Ports::Named(vec!["Launchkey".to_string(), "No Such Port".to_string()]);
    let mut drums = a_saved_device(2, "DRUMS", "DRUMS");
    drums.inputs.ports = Ports::Named(vec!["No Such Port".to_string(), "Keystep".to_string()]);
    let state = SavedState {
        devices: vec![acid, drums],
        ..SavedState::default()
    };
    assert_eq!(
        unopened_ports(&state, &["Launchkey".to_string(), "Keystep".to_string()]),
        vec!["No Such Port".to_string()],
        "named once, in the order it is first met"
    );
    assert!(
        unopened_ports(&SavedState::default(), &[]).is_empty(),
        "a session with no devices names no ports, so nothing is missing"
    );
}

#[test]
fn a_bank_document_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let document = BankDocument::of(&PatternBank::factory(), DeviceKind::Drums);
    save_bank(dir.path(), "Easy", &document).unwrap();

    assert_eq!(list_banks(dir.path()), vec!["Easy".to_string()]);
    assert!(
        bank_path(dir.path(), "Easy").is_file(),
        "a bank is one file, named after itself"
    );
    let loaded = load_bank(dir.path(), "Easy").unwrap();
    assert_eq!(loaded, document, "every field comes back");
    assert_eq!(loaded.kind, "DRUMS");
    assert_eq!(loaded.version, SAVE_VERSION);
}

#[test]
fn a_bank_without_a_usable_name_is_refused_and_a_taken_one_lands_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    let document = BankDocument::of(&PatternBank::factory(), DeviceKind::Acid);
    for bad in ["", "../escape", "a/b", "   "] {
        assert!(
            save_bank(dir.path(), bad, &document).is_err(),
            "{bad:?} is not a file name"
        );
    }
    save_bank(dir.path(), "Easy", &document).unwrap();
    assert_eq!(free_bank_name(dir.path(), "Easy"), "Easy (2)");
    assert_eq!(free_bank_name(dir.path(), "Hard"), "Hard");
}

#[test]
fn deleting_a_bank_removes_its_file_and_refuses_a_bad_name() {
    let dir = tempfile::tempdir().unwrap();
    let document = BankDocument::of(&PatternBank::factory(), DeviceKind::Acid);
    save_bank(dir.path(), "Easy", &document).unwrap();
    delete_bank(dir.path(), "Easy").unwrap();
    assert!(list_banks(dir.path()).is_empty());
    assert!(
        delete_bank(dir.path(), "../escape").is_err(),
        "a stray name can never point out of the banks folder"
    );
}

/// A bank saved under its own name rewrites that one file: the in-place rule, and no `Easy (2)` beside it.
#[test]
fn saving_a_bank_twice_under_its_own_name_rewrites_one_file() {
    let dir = tempfile::tempdir().unwrap();
    let first = BankDocument::of(&PatternBank::factory(), DeviceKind::Acid);
    save_bank(dir.path(), "Easy", &first).unwrap();

    let mut second = first.clone();
    second.bank.select(3);
    let name = match save_target(Some("Easy"), "Easy") {
        Some(SaveTarget::InPlace(name)) => name,
        other => panic!("a bank on its own name writes in place, not {other:?}"),
    };
    save_bank(dir.path(), &name, &second).unwrap();

    assert_eq!(
        list_banks(dir.path()),
        vec!["Easy".to_string()],
        "one file, not a copy beside it"
    );
    assert_eq!(
        load_bank(dir.path(), "Easy").unwrap().bank.current(),
        3,
        "and the file is the second save"
    );
}

/// A bank written on another machine says so once, and names the caption it read — the file travels with the fact, and a
/// mismatch is a warning rather than a refusal because the patterns are all there and simply play nothing.
#[test]
fn a_bank_from_another_kind_is_a_notice_naming_it() {
    assert_eq!(
        bank_kind_notice("ACID", DeviceKind::Acid),
        None,
        "the same machine says nothing"
    );
    let said = bank_kind_notice("DRUMS", DeviceKind::Acid)
        .expect("a drum bank on an acid face is worth a line");
    assert!(
        said.contains("DRUMS") && said.contains("ACID"),
        "the line names both machines: {said}"
    );
    let unknown = bank_kind_notice("GRANDPIANO", DeviceKind::Drums)
        .expect("a caption this build does not know is a notice too");
    assert!(
        unknown.contains("GRANDPIANO"),
        "an unknown caption is kept as read and named: {unknown}"
    );
}

/// A hand-edited bank is repaired on the way in: `load_bank` normalises, so a file holding two patterns still opens with all
/// sixty-four slots and on a pattern that exists. Written after a mutation of `load_bank`'s `normalise` call found nothing that
/// bit — the round trip writes an already-normal bank, so it never asked.
#[test]
fn a_bank_file_with_fewer_patterns_opens_as_sixty_four() {
    let dir = tempfile::tempdir().unwrap();
    let document = BankDocument::of(&PatternBank::factory(), DeviceKind::Drums);
    save_bank(dir.path(), "Easy", &document).unwrap();

    let text = std::fs::read_to_string(bank_path(dir.path(), "Easy")).unwrap();
    let mut hand: serde_json::Value = serde_json::from_str(&text).unwrap();
    hand["bank"]["patterns"].as_array_mut().unwrap().truncate(2);
    hand["bank"]["current"] = serde_json::json!(5);
    std::fs::write(
        bank_path(dir.path(), "Easy"),
        serde_json::to_vec(&hand).unwrap(),
    )
    .unwrap();

    let loaded = load_bank(dir.path(), "Easy").unwrap();
    assert_eq!(
        loaded.bank.pattern_count(),
        64,
        "a bank always holds sixty-four slots"
    );
    assert_eq!(
        loaded.bank.current(),
        1,
        "and it opens on one of the patterns the file actually had"
    );
}

/// A document written by a newer build is **refused**, and the version it named is the answer — not a state relabelled
/// as this build's and quietly rewritten by the next save.
#[test]
fn a_newer_document_is_refused_with_its_version() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();

    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["version"] = serde_json::json!(SAVE_VERSION + 1);
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();

    match load(dir.path()) {
        Err(acid_app::persist::LoadError::Newer { version, .. }) => {
            assert_eq!(
                version,
                SAVE_VERSION + 1,
                "the refusal names the file's version"
            );
        }
        other => panic!("a newer document must be refused, got {other:?}"),
    }
}

/// A document this build's age loads as it always did, and one with no `version` field is version 0.
#[test]
fn a_current_and_an_unversioned_document_still_load() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    assert!(
        load(dir.path()).unwrap().is_some(),
        "this build's own file loads"
    );

    let text = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document.as_object_mut().unwrap().remove("version");
    std::fs::write(
        dir.path().join("state.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let old = load(dir.path())
        .unwrap()
        .expect("an unversioned document loads");
    assert_eq!(old.version, 0, "no field, no version");
}

/// A file that will not parse is a **broken** file, not somebody else's newer one: the refusal names a version only when
/// it could read one.
#[test]
fn a_truncated_document_is_a_broken_file_not_a_newer_one() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("state.json"),
        b"{\"version\": 14, \"bank\": {",
    )
    .unwrap();
    assert!(
        matches!(
            load(dir.path()),
            Err(acid_app::persist::LoadError::Other(_))
        ),
        "a truncated file must not be reported as a newer build's"
    );
}

/// A launch on a newer document refuses it: it answers the version, opens a demo as a first run, and — the part that
/// matters — **leaves the file exactly as it was**.
#[test]
fn a_launch_refuses_a_newer_file_and_does_not_touch_it() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    let path = dir.path().join("state.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["version"] = serde_json::json!(SAVE_VERSION + 1);
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    let before = std::fs::read(&path).unwrap();

    let opened = acid_app::persist::open(dir.path());

    assert_eq!(
        opened.refused,
        Some(SAVE_VERSION + 1),
        "the version it named"
    );
    assert!(opened.first_run, "a refusal opens a first run");
    assert_eq!(
        opened.state,
        acid_app::demo::seed(),
        "and a demo, not the document it could not read"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "the refused file must not be touched"
    );
}

/// A document this build can read opens as it always did: no refusal, not a first run.
#[test]
fn a_launch_on_a_current_file_refuses_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let written = SavedState {
        bpm: 137.0,
        ..SavedState::default()
    };
    save(dir.path(), &written).unwrap();

    let opened = acid_app::persist::open(dir.path());

    assert_eq!(opened.refused, None);
    assert!(!opened.first_run);
    assert!((opened.state.bpm - 137.0).abs() < 1e-6);
}

/// The notice a refused launch shows names the version it found and says the file was left alone — the words a player
/// reads, in one place and testable, because `main` is a binary no test can drive.
#[test]
fn the_refusal_notice_names_the_version_and_the_promise() {
    let said = acid_app::persist::refusal_notice(14);
    assert!(said.contains("14"), "the version it found: {said}");
    assert!(said.contains("13"), "and this build's: {said}");
    assert!(
        said.contains("untouched"),
        "and that the file is still there: {said}"
    );
    assert!(said.is_ascii(), "the face may not print a glyph");
}

/// A song from a newer build is refused by the same rule as the save file, and **names its version** — the dialog shows
/// that, and nothing ever lands in the library.
#[test]
fn a_newer_song_is_refused_by_import_and_by_load() {
    let library = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    save_song(library.path(), "Song", &SavedState::default()).unwrap();

    // The song as this build wrote it, carried out and stamped with the next build's version.
    let text =
        std::fs::read_to_string(acid_app::persist::song_path(library.path(), "Song")).unwrap();
    let mut document: serde_json::Value = serde_json::from_str(&text).unwrap();
    document["version"] = serde_json::json!(SAVE_VERSION + 1);
    let incoming = elsewhere.path().join("Song.json");
    std::fs::write(&incoming, serde_json::to_vec(&document).unwrap()).unwrap();

    let refused = import_song(&incoming, library.path()).expect_err("a newer song is refused");
    assert!(
        refused
            .to_string()
            .contains(&format!("version {}", SAVE_VERSION + 1)),
        "the refusal names the version it found: {refused}"
    );
    assert_eq!(
        list_songs(library.path()),
        vec!["Song".to_string()],
        "nothing landed"
    );

    let loaded = load_song(library.path(), "Song").expect("a song this build can read still loads");
    assert_eq!(loaded.version, SAVE_VERSION, "the song itself is unchanged");
}
