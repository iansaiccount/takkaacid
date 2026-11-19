//! A kit is a folder: `<data>/kits/<name>/kit.json` plus one WAV per voice.
//!
//! These are the plain path functions — `save_kit`, `load_kit`, `import_kit`, `kit_names` and the
//! compiled-in `factory_kits` — which take a directory and need no display. The browser and the
//! card's row are `tests/kits_live.rs`.

use std::sync::Arc;

use acid_app::kits::{default_name, factory_kits, factory_names, factory_sample_bytes, is_factory};
use acid_app::persist::{
    KitError, free_kit_name, import_kit, kit_name, kit_names, load_kit, save_kit,
};
use acid_core::drums::{Drum, FilterKind, KitVoice, default_kit};

/// The rate these kits are written and read at: a saved WAV records its own rate, so a save and a load have to agree on the
/// one the samples are held at.
const RATE: f32 = 44_100.0;
/// 440 Hz for a tenth of a second, as a WAV file's bytes: the smallest sample a test can hand a
/// kit, written by hand so nothing under test encodes it.
fn sine_wav() -> Vec<u8> {
    let rate = 44_100u32;
    let count = 4_410u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + count * 2).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(count * 2).to_le_bytes());
    for index in 0..count {
        let phase = (index as f32 / rate as f32) * 440.0 * std::f32::consts::TAU;
        let value = (phase.sin() * 0.5 * f32::from(i16::MAX)) as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// The samples in `sine_wav`, so a round trip can be compared without decoding twice.
fn sine_samples() -> Vec<f32> {
    let rate = 44_100f32;
    (0..4_410)
        .map(|index| ((index as f32 / rate) * 440.0 * std::f32::consts::TAU).sin() * 0.5)
        .collect()
}

/// A kit with one voice sampled and the other fifteen silent, and every playable field moved off
/// its default: a round trip that loses a field or a sample cannot pass on the defaults.
fn test_kit() -> acid_core::drums::Kit {
    let mut kit = default_kit();
    kit.name = "Test Kit".to_string();
    let kick = &mut kit.voices[Drum::Kick.index()];
    kick.sample = Arc::from(sine_samples());
    kick.tune = 7.0;
    kick.attack_ms = 1.5;
    kick.decay_ms = 123.0;
    kick.sustain = 0.25;
    kick.release_ms = 45.0;
    kick.filter = FilterKind::Band;
    kick.cutoff = 0.4;
    kick.resonance = 0.7;
    kit.voices[Drum::Snare.index()].mute = true;
    kit
}

fn one_kit(voices: &[KitVoice]) -> acid_core::drums::Kit {
    let mut kit = default_kit();
    kit.name = "Write Me".to_string();
    for (slot, voice) in voices.iter().enumerate() {
        kit.voices[slot] = voice.clone();
    }
    kit
}

#[test]
fn a_kit_round_trips_through_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    let kit = test_kit();
    let written = save_kit(&kit, dir.path(), RATE).unwrap();
    assert_eq!(
        written,
        dir.path().join("Test Kit"),
        "a kit is a folder named after the kit"
    );

    let loaded = load_kit(dir.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(
        loaded.problems.len(),
        0,
        "a kit this build wrote must load without a complaint: {:?}",
        loaded.problems
    );
    assert_eq!(loaded.kit.name, "Test Kit");
    assert_eq!(loaded.kit.voices.len(), Drum::ALL.len());

    let kick = &loaded.kit.voices[Drum::Kick.index()];
    let original = &kit.voices[Drum::Kick.index()];
    assert_eq!(
        kick.sample.len(),
        original.sample.len(),
        "the sample survives whole at the rate it was written for"
    );
    // 16-bit storage is lossy by a hair, so the shape is compared, not the bits.
    let error = kick
        .sample
        .iter()
        .zip(original.sample.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(error < 0.001, "the sample came back {error} off");
    assert_eq!(kick.tune, 7.0);
    assert_eq!(kick.attack_ms, 1.5);
    assert_eq!(kick.decay_ms, 123.0);
    assert_eq!(kick.sustain, 0.25);
    assert_eq!(kick.release_ms, 45.0);
    assert_eq!(
        kick.filter,
        FilterKind::Band,
        "the filter is a choice, not a number"
    );
    assert_eq!(kick.cutoff, 0.4);
    assert_eq!(kick.resonance, 0.7);
    assert!(
        !loaded.kit.voices[Drum::Kick.index()].mute,
        "a voice that was not muted comes back audible"
    );
    assert!(
        loaded.kit.voices[Drum::Snare.index()].mute,
        "a muted voice comes back muted"
    );

    // A voice with no sample is silent, not missing: fifteen of them load without a complaint.
    for drum in Drum::ALL {
        if drum == Drum::Kick {
            continue;
        }
        assert!(
            loaded.kit.voices[drum.index()].sample.is_empty(),
            "{drum:?} was silent in the kit and must not arrive with a sample"
        );
    }
}

#[test]
fn a_kit_is_a_document_and_one_file_per_sampled_voice() {
    let dir = tempfile::tempdir().unwrap();
    save_kit(&test_kit(), dir.path(), RATE).unwrap();
    let folder = dir.path().join("Test Kit");

    let document = std::fs::read_to_string(folder.join("kit.json")).unwrap();
    assert!(
        document.contains("\"name\": \"Test Kit\""),
        "the document names the kit: {document}"
    );
    // The document's field order is what a human reads and what a diff shows, and it mirrors `KitVoice`'s own order with the
    // three provenance fields last: the JSON parser does not care, so this is the only thing that would notice a reorder.
    // Strictly ascending, so swapping two neighbours fails here and not in a screenshot of a hand-edited file.
    let mut at = document.find("\"tune\"").unwrap();
    for field in [
        "\"attack_ms\"",
        "\"decay_ms\"",
        "\"sustain\"",
        "\"release_ms\"",
        "\"filter\"",
        "\"cutoff\"",
        "\"resonance\"",
        "\"mute\"",
        "\"file\"",
        "\"source\"",
        "\"credit\"",
    ] {
        let next = document
            .find(field)
            .unwrap_or_else(|| panic!("no {field} in the document"));
        assert!(next > at, "{field} must come after the field before it");
        at = next;
    }

    assert!(
        folder.join("KICK.wav").is_file(),
        "a sampled voice has a file"
    );
    assert!(
        !folder.join("SNR.wav").exists(),
        "a silent voice gets no file: there is nothing to write"
    );
}

#[test]
fn a_save_keeps_the_provenance_the_document_already_had() {
    // A kit that was imported carries where its samples came from. Saving it again must not wipe
    // that: the player's own edits are not a reason to forget a licence.
    let dir = tempfile::tempdir().unwrap();
    save_kit(&test_kit(), dir.path(), RATE).unwrap();
    let folder = dir.path().join("Test Kit");
    let path = folder.join("kit.json");
    let mut document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for voice in document["voices"].as_array_mut().unwrap() {
        if voice["file"] == "KICK.wav" {
            voice["source"] = serde_json::json!("https://example.invalid/kick");
            voice["credit"] = serde_json::json!("CC0 by a test");
        }
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();

    // Now save the same kit again, as an edit of one dial would.
    let mut kit = test_kit();
    kit.voices[Drum::Kick.index()].decay_ms = 99.0;
    save_kit(&kit, dir.path(), RATE).unwrap();

    let after = std::fs::read_to_string(&path).unwrap();
    assert!(
        after.contains("https://example.invalid/kick"),
        "a save forgot where the sample came from: {after}"
    );
    assert!(after.contains("CC0 by a test"));
    assert!(after.contains("99.0"), "and the edit is written");
}

#[test]
fn an_import_lands_beside_a_taken_name_and_never_overwrites() {
    let library = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let kit = test_kit();
    let source = save_kit(&kit, elsewhere.path(), RATE).unwrap();

    let first = import_kit(&source, "Test Kit", library.path(), 44_100.0).unwrap();
    assert_eq!(first.folder, library.path().join("Test Kit"));
    assert_eq!(first.name, "Test Kit");
    let second = import_kit(&source, "Test Kit", library.path(), 44_100.0).unwrap();
    assert_eq!(
        second.folder,
        library.path().join("Test Kit (2)"),
        "a second import of the same name lands beside the first"
    );
    assert!(library.path().join("Test Kit").is_dir());
    assert!(library.path().join("Test Kit (2)").is_dir());

    // The imported kit is a whole kit, sample and all.
    let loaded = load_kit(library.path(), "Test Kit (2)", 44_100.0).unwrap();
    assert_eq!(loaded.problems.len(), 0);
    assert!(!loaded.kit.voices[Drum::Kick.index()].sample.is_empty());
}

#[test]
fn an_import_lands_a_kit_with_a_missing_wav_and_names_the_file_to_replace() {
    // An import is the moment a player is holding a folder from somewhere else, so a kit with one broken voice lands anyway:
    // fifteen good voices and a file name to fix are worth more than a refusal and nothing on disk. The copy's document keeps
    // the name the source gave (`KICK.wav`), so dropping the right file in is the whole repair — and the next load says so.
    let library = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let source = save_kit(&test_kit(), elsewhere.path(), RATE).unwrap();
    std::fs::remove_file(source.join("KICK.wav")).unwrap();

    let imported = import_kit(&source, "Test Kit", library.path(), 44_100.0).unwrap();
    assert_eq!(imported.name, "Test Kit");
    assert_eq!(
        imported.problems,
        vec![KitError::Missing {
            voice: Drum::Kick,
            file: "KICK.wav".to_string()
        }],
        "the import says which voice's file is not there, so the notice can name it"
    );
    assert!(
        imported.folder.join("kit.json").is_file(),
        "the kit landed: a copy a player can fix"
    );

    let loaded = load_kit(library.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(
        loaded.problems.len(),
        1,
        "and the copy still names the hole: {:?}",
        loaded.problems
    );
    assert_eq!(
        loaded.kit.voices[Drum::Kick.index()].sample.len(),
        0,
        "the voice is silent until the file is put back"
    );

    // The repair: drop the right WAV in, under the name the document already carries, and the kit loads whole.
    std::fs::write(imported.folder.join("KICK.wav"), sine_wav()).unwrap();
    let repaired = load_kit(library.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(repaired.problems.len(), 0, "{:?}", repaired.problems);
    assert_eq!(repaired.kit.voices[Drum::Kick.index()].sample.len(), 4_410);
}

#[test]
fn a_save_removes_a_wav_for_a_voice_that_lost_its_sample() {
    // A kit folder should hold the samples its document names and nothing else: a stale WAV is a voice the player deleted
    // coming back if the name ever returns. Only the names this module writes are removed, so a player's own note in the
    // folder survives.
    let dir = tempfile::tempdir().unwrap();
    let mut kit = test_kit();
    save_kit(&kit, dir.path(), RATE).unwrap();
    let folder = dir.path().join("Test Kit");
    assert!(folder.join("KICK.wav").is_file());
    std::fs::write(folder.join("notes.txt"), b"mine").unwrap();

    kit.voices[Drum::Kick.index()].sample = Arc::default();
    save_kit(&kit, dir.path(), RATE).unwrap();
    assert!(
        !folder.join("KICK.wav").exists(),
        "the sample is gone from the kit, so its file goes too"
    );
    assert!(
        folder.join("notes.txt").is_file(),
        "and nothing else in the folder is touched"
    );
    let loaded = load_kit(dir.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(loaded.problems.len(), 0, "a silent voice is not a hole");
}

#[test]
fn a_save_keeps_a_wav_that_will_not_decode() {
    // The other side of the deletion: a file the app cannot *play* is not a voice the player deleted, it is the hole the
    // notice names. A kit handed over with one ADPCM or half-downloaded WAV must survive a save with its bytes and its name —
    // deleting it would destroy the player's only copy of the sample the status line just told them to replace.
    let dir = tempfile::tempdir().unwrap();
    save_kit(&test_kit(), dir.path(), RATE).unwrap();
    let folder = dir.path().join("Test Kit");
    std::fs::write(folder.join("KICK.wav"), b"not a wav at all").unwrap();
    let loaded = load_kit(dir.path(), "Test Kit", RATE).unwrap();
    assert_eq!(loaded.problems.len(), 1);

    let mut edited = loaded.kit.clone();
    edited.voices[Drum::Kick.index()].decay_ms = 111.0;
    save_kit(&edited, dir.path(), RATE).unwrap();

    assert!(
        folder.join("KICK.wav").is_file(),
        "a save destroyed the file the notice told the player to replace"
    );
    assert_eq!(
        load_kit(dir.path(), "Test Kit", RATE)
            .unwrap()
            .problems
            .len(),
        1,
        "and the hole is still the repair"
    );
}

#[test]
fn an_import_refuses_a_document_that_names_a_path() {
    // The one problem an import cannot turn into a repairable kit: there is no file to put in the folder, and the name cannot
    // be written into the copy, so the voice would vanish from the copy without a word. A broken document is refused whole.
    let library = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let source = save_kit(&test_kit(), elsewhere.path(), RATE).unwrap();
    let path = source.join("kit.json");
    let mut document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    document["voices"][0]["file"] = serde_json::json!("../../etc/passwd");
    std::fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();

    let error = import_kit(&source, "Test Kit", library.path(), RATE).unwrap_err();
    assert!(
        matches!(error, KitError::NotAName { .. }),
        "{error:?} should name the document's entry"
    );
    assert!(
        !library.path().join("Test Kit").exists(),
        "a refused import leaves nothing behind"
    );
}

#[test]
fn a_save_keeps_the_name_of_a_file_that_is_not_there() {
    // The other half of the import repair: a save must not turn a kit *with a hole* into a kit that never had the voice, or
    // the file name the player was told to fill in would be forgotten the first time they touched a dial.
    let dir = tempfile::tempdir().unwrap();
    let kit = test_kit();
    save_kit(&kit, dir.path(), RATE).unwrap();
    std::fs::remove_file(dir.path().join("Test Kit").join("KICK.wav")).unwrap();
    let loaded = load_kit(dir.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(loaded.problems.len(), 1);

    // Save what was loaded, exactly as an edit of one dial would.
    let mut edited = loaded.kit.clone();
    edited.voices[Drum::Kick.index()].decay_ms = 111.0;
    save_kit(&edited, dir.path(), RATE).unwrap();

    let after = load_kit(dir.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(
        after.problems,
        vec![KitError::Missing {
            voice: Drum::Kick,
            file: "KICK.wav".to_string()
        }],
        "the hole is still a hole, with the name the player can fill"
    );
}

#[test]
fn a_kit_with_a_missing_wav_loads_every_other_voice_and_names_the_one() {
    // Review Focus 2: a missing, renamed or unreadable file must not cost the player the kit.
    let dir = tempfile::tempdir().unwrap();
    let mut kit = test_kit();
    kit.voices[Drum::Snare.index()].sample = Arc::from(sine_samples());
    save_kit(&kit, dir.path(), RATE).unwrap();
    std::fs::remove_file(dir.path().join("Test Kit").join("SNR.wav")).unwrap();

    let loaded = load_kit(dir.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(loaded.problems.len(), 1, "{:?}", loaded.problems);
    assert_eq!(
        loaded.problems[0],
        KitError::Missing {
            voice: Drum::Snare,
            file: "SNR.wav".to_string()
        }
    );
    assert!(
        !loaded.kit.voices[Drum::Kick.index()].sample.is_empty(),
        "the voice whose file is there still loads"
    );
    assert!(
        loaded.kit.voices[Drum::Snare.index()].sample.is_empty(),
        "the missing voice is silent, not the last file that happened to be loaded"
    );
    // The notice sentence names the voice and the file, in the words a player reads — pinned whole, because a sentence with
    // both in it is the one thing this variant exists for.
    assert_eq!(loaded.problems[0].to_string(), "SNR: SNR.wav is missing");
}

#[test]
fn a_wav_that_will_not_decode_is_a_problem_the_decoder_words() {
    let dir = tempfile::tempdir().unwrap();
    save_kit(&test_kit(), dir.path(), RATE).unwrap();
    let folder = dir.path().join("Test Kit");
    std::fs::write(folder.join("KICK.wav"), b"not a wav at all").unwrap();

    let loaded = load_kit(dir.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(loaded.problems.len(), 1);
    let KitError::Undecodable { voice, file, error } = loaded.problems[0].clone() else {
        panic!("a broken file is Undecodable, not {:?}", loaded.problems[0]);
    };
    assert_eq!(voice, Drum::Kick);
    assert_eq!(file, "KICK.wav");
    assert_eq!(
        error,
        acid_app::wav::WavError::NotRiff,
        "the decoder's own words, not a second vocabulary for the same bytes"
    );
    assert!(loaded.problems[0].to_string().contains("not a RIFF file"));
}

#[test]
fn a_kit_written_by_hand_loads_its_own_wav() {
    // The player's door into kits: a folder made by hand, with a document and a WAV nothing in this build wrote. The document
    // is written out in full because a kit document is refused **whole** when a field is missing — `KitVoiceDoc` has no
    // `#[serde(default)]`, so a hand-edit that drops one gets "the kit's document will not read" rather than a voice at zero.
    // That is the deliberate half of "kits are data": omission is an error, not a silence.
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("My Own");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("KICK.wav"), sine_wav()).unwrap();
    let mut voices = Vec::new();
    for slot in 0..Drum::ALL.len() {
        let file = if slot == Drum::Kick.index() {
            "KICK.wav"
        } else {
            ""
        };
        voices.push(serde_json::json!({
            "tune": 0.0,
            "attack_ms": 2.0,
            "decay_ms": 300.0,
            "sustain": 0.0,
            "release_ms": 20.0,
            "filter": 0,
            "cutoff": 1.0,
            "resonance": 0.0,
            "mute": false,
            "file": file,
            "source": "my own recording",
            "credit": "none needed",
        }));
    }
    std::fs::write(
        folder.join("kit.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "name": "My Own",
            "voices": voices,
        }))
        .unwrap(),
    )
    .unwrap();

    let loaded = load_kit(dir.path(), "My Own", 44_100.0).unwrap();
    assert_eq!(loaded.problems.len(), 0, "{:?}", loaded.problems);
    let kick = &loaded.kit.voices[Drum::Kick.index()];
    assert_eq!(
        kick.sample.len(),
        4_410,
        "the file's own samples arrive at the engine's rate"
    );
    let peak = kick
        .sample
        .iter()
        .copied()
        .fold(0.0f32, |peak, value| peak.max(value.abs()));
    assert!(
        (peak - 0.5).abs() < 0.01,
        "the hand-written sine is 0.5 peak"
    );
}

#[test]
fn a_kit_that_is_not_there_says_so_and_a_document_that_is_not_a_kit_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        matches!(
            load_kit(dir.path(), "Nowhere", 44_100.0),
            Err(KitError::NoDocument)
        ),
        "a kit that is not there says so"
    );

    let folder = dir.path().join("Hollow");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("kit.json"), b"{ not json").unwrap();
    assert!(
        matches!(
            load_kit(dir.path(), "Hollow", 44_100.0),
            Err(KitError::UnreadableDocument)
        ),
        "a document that is not a kit is refused"
    );
}

#[test]
fn a_name_that_is_a_path_is_refused_by_the_same_rule_a_song_name_is() {
    let dir = tempfile::tempdir().unwrap();
    for bad in ["../escape", "/absolute", "a/b", "a\\b", "", "  ", ".hidden"] {
        assert_eq!(kit_name(bad), None, "{bad:?} is not a kit name");
        assert!(
            load_kit(dir.path(), bad, 44_100.0).is_err(),
            "{bad:?} must not be read from"
        );
        let mut kit = test_kit();
        kit.name = bad.to_string();
        assert!(
            matches!(save_kit(&kit, dir.path(), RATE), Err(KitError::BadName)),
            "{bad:?} must not be written to"
        );
    }
    assert_eq!(kit_name("  Acoustic 2  ").as_deref(), Some("Acoustic 2"));
}

#[test]
fn a_kit_may_not_take_a_factory_kits_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut kit = test_kit();
    for name in factory_names() {
        assert!(is_factory(name), "{name} must be reserved");
        assert_eq!(kit_name(name).as_deref(), Some(name), "it is a valid name");
        kit.name = name.to_string();
        assert!(
            matches!(save_kit(&kit, dir.path(), RATE), Err(KitError::BadName)),
            "a player kit may not be written over the factory kit {name}"
        );
        assert_eq!(
            free_kit_name(dir.path(), name),
            format!("{name} (2)"),
            "an edit of a factory kit lands beside its name"
        );
    }
    // And no directory was made for any of them.
    for name in factory_names() {
        assert!(
            !dir.path().join(name).exists(),
            "{name} was written to disk"
        );
    }

    // And the default kit's own word, which is not a factory name but is just as unavailable: `SavedState::kit: None` means
    // "the default kit", so a folder called `Init` would load, play, and be forgotten on the next save.
    let default = acid_core::drums::DEFAULT_KIT_NAME;
    assert_eq!(default, "Init");
    assert!(!is_factory(default), "it is not one of the three");
    let mut kit = test_kit();
    kit.name = default.to_string();
    assert!(
        matches!(save_kit(&kit, dir.path(), RATE), Err(KitError::BadName)),
        "a player's kit may not be called {default}"
    );
    assert_eq!(
        free_kit_name(dir.path(), default),
        "Init (2)",
        "and a copy of one lands beside it"
    );
}

#[test]
fn kit_names_lists_the_factory_kits_then_the_players_own() {
    // A first run has no `kits/` directory at all, and that is not an error.
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("kits");
    let mut expected: Vec<String> = factory_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    assert_eq!(
        kit_names(&absent),
        expected,
        "a fresh path offers the factory kits"
    );

    let mut kit = test_kit();
    save_kit(&kit, &absent, RATE).unwrap();
    kit.name = "Zebra".to_string();
    save_kit(&kit, &absent, RATE).unwrap();
    kit.name = "Aardvark".to_string();
    save_kit(&kit, &absent, RATE).unwrap();
    // A stray file is not a kit: the folder is the kit.
    std::fs::write(absent.join("notes.txt"), b"hello").unwrap();

    expected.push("Aardvark".to_string());
    expected.push("Test Kit".to_string());
    expected.push("Zebra".to_string());
    assert_eq!(kit_names(&absent), expected);
    // What the list is made of, asked of the function rather than of the list this test built: two blocks, each sorted, the
    // factory three first, and no name twice — a player's folder that shadowed a factory name would be a duplicate here.
    let names = kit_names(&absent);
    let (factory_block, player_block) = names.split_at(factory_names().len());
    let mut sorted_factory = factory_block.to_vec();
    sorted_factory.sort();
    assert_eq!(
        factory_block,
        sorted_factory.as_slice(),
        "the kits that ship come first, sorted"
    );
    let mut sorted_player = player_block.to_vec();
    sorted_player.sort();
    assert_eq!(
        player_block,
        sorted_player.as_slice(),
        "then the player's own, sorted"
    );
    let mut unique = names.clone();
    unique.dedup();
    assert_eq!(unique, names, "no name is listed twice");
}

#[test]
fn factory_kits_are_three_kits_of_sixteen_audible_voices() {
    let rate = 48_000.0;
    let kits = factory_kits(rate);
    assert_eq!(kits.len(), 3, "three kits ship");
    // The kit a session with no kit name plays, pinned: `SavedState::kit: None` means this one, so reordering `FACTORY`
    // would move the default for every new session and for every file written before the drum machine.
    assert_eq!(
        default_name(),
        "808",
        "the default kit is the first that ships, and it is called 808"
    );
    assert_eq!(
        kits.iter().map(|kit| kit.name.clone()).collect::<Vec<_>>(),
        factory_names()
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>(),
        "the compiled-in kits are the reserved names, in the catalogue's order"
    );

    for kit in &kits {
        assert_eq!(kit.voices.len(), Drum::ALL.len());
        for drum in Drum::ALL {
            let voice = &kit.voices[drum.index()];
            assert!(
                !voice.sample.is_empty(),
                "{}: {drum:?} has no sample, so its row in the grid would be silent",
                kit.name
            );
            let peak = voice
                .sample
                .iter()
                .copied()
                .fold(0.0f32, |peak, value| peak.max(value.abs()));
            assert!(
                peak > 0.05,
                "{}: {drum:?} is inaudible (peak {peak})",
                kit.name
            );
            assert!(
                voice.sample.len() <= (0.6 * rate) as usize + 1,
                "{}: {drum:?} is {} samples, longer than the spec's 0.6 s",
                kit.name,
                voice.sample.len()
            );
            assert!(
                voice.sample.iter().all(|value| value.is_finite()),
                "{}: {drum:?} carries a sample that is not a number",
                kit.name
            );
        }
        // A kit whose voices are all the same number is a bug the checks above cannot see.
        let mutes = kit.mute_mask();
        assert_ne!(mutes, u16::MAX, "{}: every voice starts muted", kit.name);
    }
}

#[test]
fn the_factory_kits_decode_at_the_rate_they_are_handed() {
    // Trap 1: the rate is a parameter, never a default. A kit decoded at the wrong rate plays every
    // voice at the wrong pitch, and nothing in the face would say why.
    let at_48 = &factory_kits(48_000.0)[0].voices[Drum::Kick.index()].sample;
    let at_44 = &factory_kits(44_100.0)[0].voices[Drum::Kick.index()].sample;
    let ratio = at_48.len() as f32 / at_44.len() as f32;
    let expected = 48_000.0 / 44_100.0;
    assert!(
        (ratio - expected).abs() < 0.01,
        "48 kHz gave {} samples and 44.1 kHz gave {}; the rate is not being used",
        at_48.len(),
        at_44.len()
    );
}

#[test]
fn the_three_kits_fit_in_the_binary_the_spec_budgeted() {
    // `include_bytes!` puts every WAV in the app, so the spec's "about 2 MB for three kits" is a budget a build can break
    // silently. This is the number the binary grows by, not the decoded size: forty-eight voices at 0.6 s and 48 kHz decode
    // to about 5 MB of `f32` while the app runs, which is the price of playing them without resampling per block.
    let bytes = factory_sample_bytes();
    assert!(
        bytes <= 5 * 1024 * 1024 / 2,
        "the three kits ship {bytes} bytes of WAV, past the spec's ~2 MB"
    );
    assert!(
        bytes > 1024 * 1024,
        "the three kits ship only {bytes} bytes, which is not forty-eight samples"
    );

    // And every file is the shape the spec asked for — 16-bit mono at its own rate, 44.1 kHz. A header is 44 bytes and a
    // sample is two, so the bytes on disk and the samples decoded at 44.1 kHz are the same fact told twice; a stereo or
    // 24-bit file would break this line and nothing else would notice, because the byte *total* happens to match.
    let decoded: usize = factory_kits(44_100.0)
        .iter()
        .flat_map(|kit| kit.voices.iter())
        .map(|voice| voice.sample.len())
        .sum();
    assert_eq!(
        bytes,
        44 * 48 + decoded * 2,
        "{decoded} samples at 16-bit mono is {} bytes of PCM; the kits ship {bytes}",
        decoded * 2
    );
}

#[test]
fn the_kit_folder_is_under_the_data_directory() {
    let dir = acid_app::persist::kits_dir();
    assert_eq!(
        dir.file_name().and_then(|name| name.to_str()),
        Some("kits"),
        "kits live in <data>/kits, one folder each"
    );
    // Under the data directory, so `TAKKA_ACID_DATA_DIR` takes the kits with the sessions — which is how the guide's
    // screenshots and the demo run are taken on a scratch path.
    assert!(
        dir.starts_with(acid_app::persist::data_dir()),
        "{dir:?} is not under the data directory"
    );
}

#[test]
fn a_silent_kit_writes_no_files_and_loads_as_sixteen_silent_voices() {
    let dir = tempfile::tempdir().unwrap();
    let kit = one_kit(&[]);
    save_kit(&kit, dir.path(), RATE).unwrap();
    let folder = dir.path().join("Write Me");
    assert!(folder.join("kit.json").is_file());
    let entries = std::fs::read_dir(&folder).unwrap().count();
    assert_eq!(entries, 1, "one document and not one WAV");

    let loaded = load_kit(dir.path(), "Write Me", 44_100.0).unwrap();
    assert_eq!(loaded.problems.len(), 0);
    assert!(
        loaded
            .kit
            .voices
            .iter()
            .all(|voice| voice.sample.is_empty())
    );
}

#[test]
fn a_document_that_names_a_path_is_refused_rather_than_read() {
    // A kit document is data a player can edit, so a `file` in it is a name and never a path.
    let dir = tempfile::tempdir().unwrap();
    save_kit(&test_kit(), dir.path(), RATE).unwrap();
    let folder = dir.path().join("Test Kit");
    let path = folder.join("kit.json");
    let mut document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    document["voices"][0]["file"] = serde_json::json!("../../etc/passwd");
    std::fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();

    let loaded = load_kit(dir.path(), "Test Kit", 44_100.0).unwrap();
    assert_eq!(loaded.problems.len(), 1, "{:?}", loaded.problems);
    assert!(
        matches!(loaded.problems[0], KitError::NotAName { .. }),
        "{:?}",
        loaded.problems[0]
    );
}

#[test]
fn a_rate_that_is_not_a_rate_is_refused_and_not_a_panic() {
    let dir = tempfile::tempdir().unwrap();
    save_kit(&test_kit(), dir.path(), RATE).unwrap();
    for rate in [0.0, -48_000.0, f32::NAN] {
        let loaded = load_kit(dir.path(), "Test Kit", rate).unwrap();
        assert_eq!(
            loaded.problems.len(),
            1,
            "a nonsense rate is a problem with the sample, not a panic ({rate})"
        );
    }
    // The file itself was never the problem: only the decode was refused, so a later load at a real rate still finds it.
    assert!(
        dir.path().join("Test Kit").join("KICK.wav").is_file(),
        "a refused decode must not touch the file on disk"
    );
}
