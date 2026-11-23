//! The demo drum banks: a groove for every demo variation, and what loading one does to a device.
//!
//! The acid demos and their drum sides are **one list** (`demo::songs`), so nothing here holds two hand-kept lists
//! together: each `Demo` carries its own eight grooves, and `demo::drum_banks` is the same five songs read from the drum
//! side. What these tests pin is the part a pairing cannot state for itself — the five names the book prints, that every
//! pattern a player can load strikes something, and that a demo arrives on a drum machine as patterns, a tempo and a
//! shuffle, with the kit its own.
//!
//! Not a `_live` test: nothing here drives `draw_panel`. The card's `DEMO BANKS` rows are the same five rows on either
//! face, so the widget's behaviour does not differ and `tests/songs_live.rs` already pins the row's request.

use std::sync::Arc;

use acid_app::demo;
use acid_app::device::{DeviceKind, Rack};
use acid_app::persist;
use acid_app::ui::PanelState;
use acid_app::ui::pattern_swap;
use acid_core::bank::{PATTERNS_PER_BANK, PatternBank, TOTAL_PATTERNS};
use acid_core::drums::{Drum, Kit, default_kit};
use acid_core::pattern::{Grid, Pattern, Step};

/// The seven variations the card lists, page 12 names and a later song document will pair by.
const NAMES: [&str; 7] = [
    "Slow Acid",
    "Hard Techno Acid",
    "Blues",
    "Rock",
    "Funk",
    "Acid Jazz",
    "Fast Acid",
];

/// How many voices this step strikes, counted through the voice catalogue rather than a mask constant.
fn struck(step: &Step) -> usize {
    Drum::ALL.iter().filter(|drum| step.is_hit(**drum)).count()
}

/// How many strikes one bar holds.
fn strikes(pattern: &Pattern) -> usize {
    pattern.steps.iter().map(struck).sum()
}

#[test]
fn every_demo_variation_has_a_drum_bank_of_eight_patterns() {
    let banks = demo::drum_banks();
    assert_eq!(banks.len(), NAMES.len(), "one bank per demo variation");

    for (name, bank) in &banks {
        assert!(NAMES.contains(name), "{name} is not a demo variation");
        assert_eq!(bank.pattern_count(), TOTAL_PATTERNS);
        assert_eq!(bank.current(), 0, "{name} does not open on its first");
        // A demo bank fills `A1`..`A8` and leaves the other seven banks empty: it is eight patterns, not sixty-four.
        for slot in 0..PATTERNS_PER_BANK {
            assert!(
                strikes(bank.get(slot)) > 0,
                "{name} A{} is a silent pattern",
                slot + 1
            );
        }
        // Eight variations a player can reach for, not eight slots with a bar written twice: a repeat is a dead row on
        // the grid the same way a silent one is.
        for slot in 0..PATTERNS_PER_BANK {
            for other in (slot + 1)..PATTERNS_PER_BANK {
                assert_ne!(
                    bank.get(slot),
                    bank.get(other),
                    "{name} A{} and A{} are the same bar",
                    slot + 1,
                    other + 1
                );
            }
        }
        for slot in PATTERNS_PER_BANK..TOTAL_PATTERNS {
            assert_eq!(
                *bank.get(slot),
                Pattern::empty(),
                "{name} has something in B{slot}",
                slot = slot + 1 - PATTERNS_PER_BANK
            );
        }
    }
}

#[test]
fn the_demo_names_are_the_acid_names() {
    let songs = demo::songs();
    let banks = demo::drum_banks();

    // The five names, in the order the card lists and the book names them. This is the assertion that bites: the two
    // accessors read one list, so a rename has to be a decision about the book too.
    let names: Vec<&str> = banks.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, NAMES);

    // ...and it is one list, so each demo's groove is reached by that demo's own name, in the demos' order.
    let demo_names: Vec<&str> = songs.iter().map(|demo| demo.name).collect();
    assert_eq!(
        demo_names, names,
        "a drum bank is not under its demo's name"
    );

    // Five names with four grooves would be five rows that play the same thing on two of them.
    for (a, first) in banks.iter().enumerate() {
        for second in banks.iter().skip(a + 1) {
            assert_ne!(
                first.1.get(0),
                second.1.get(0),
                "{} and {} open on the same groove",
                first.0,
                second.0
            );
        }
    }
}

#[test]
fn every_pattern_of_every_bank_strikes_something() {
    for (name, bank) in demo::drum_banks() {
        for slot in 0..PATTERNS_PER_BANK {
            let pattern = bank.get(slot);
            assert!(
                strikes(pattern) > 0,
                "{name} A{} has no hit in its bar",
                slot + 1
            );
            for (at, step) in pattern.steps.iter().enumerate() {
                // `Step::hit` is the only writer of the masks and of `on`, so a step that sounds without striking was
                // written with `Step::note` — which a drum machine plays as silence, whatever the grid draws.
                assert_eq!(
                    step.on,
                    struck(step) > 0,
                    "{name} A{} step {} sounds nothing",
                    slot + 1,
                    at + 1
                );
            }
        }
    }
}

#[test]
fn a_demo_arrives_on_a_drum_machine_as_patterns_a_tempo_and_a_shuffle() {
    // A rack with a drum machine on it, playing a kit of the player's own: the groove that arrives must not bring a
    // sound with it.
    let mut rack = Rack::from_saved(&persist::SavedState::default());
    rack.add(DeviceKind::Drums).expect("the rack has room");
    rack.select(1);
    rack.device().state.adopt_kit(Kit {
        name: "Mine".to_string(),
        ..default_kit()
    });
    let kit = rack.device().state.kit();
    let knobs = rack.device().state.snapshot();

    let (name, bank) = demo::drum_banks()
        .into_iter()
        .nth(4)
        .expect("the fifth demo");
    assert_eq!(name, "Funk");
    let funk = demo::songs().into_iter().nth(4).expect("the fifth demo");

    assert_eq!(demo::load_drum_bank(&mut rack, 4), Some("Funk"));

    // The whole bank landed, and the banks beyond `A` are the empty ones a demo bank brings.
    assert_eq!(rack.device().bank.pattern_count(), TOTAL_PATTERNS);
    for slot in 0..PATTERNS_PER_BANK {
        assert_eq!(rack.device().bank.get(slot), bank.get(slot));
    }
    assert_eq!(rack.device().bank.current(), 0);
    // ...and the device's own state carries it: the renderer reads the state, not the bank.
    assert_eq!(*rack.device().state.pattern(), *bank.get(0));
    assert_eq!(rack.device().state.pattern_index(), 0);
    // A demo is its tempo and its shuffle too: a drum machine is built straight and at the rack's tempo, so a funk groove
    // left at a slow acid 118 would not be the demo the row is named after.
    assert_eq!(rack.tempo_bpm(), funk.state.bpm);
    assert_eq!(rack.device().state.snapshot().swing, funk.state.swing);
    // A bank is patterns only: the kit and the knobs a hand set are still there.
    assert!(Arc::ptr_eq(&kit, &rack.device().state.kit()));
    assert_eq!(rack.device().state.kit().name, "Mine");
    assert_eq!(rack.device().state.snapshot().cutoff, knobs.cutoff);

    // What the panel does with it: the grid shows the bank's first pattern, from `A1`.
    let mut panel = PanelState::default();
    pattern_swap::adopt(&mut panel, rack.state(), rack.bank());
    assert_eq!(*panel.pattern, *bank.get(0));
    assert_eq!(panel.pattern_id, PatternBank::id_of(0));
    assert_eq!(panel.selected, 0);

    // A seventh demo does not exist — seven names, indexes 0..6 — and asking for one leaves the machine where it was.
    assert_eq!(demo::load_drum_bank(&mut rack, 7), None);
    assert_eq!(rack.device().bank.get(0), bank.get(0));
    assert_eq!(rack.tempo_bpm(), funk.state.bpm);
}

/// The two demos that put the other grids on the card: the jazz line is twelve eighth-triplets at 140, the fast one
/// thirty-two thirty-seconds at 152, and each arrives with its eight drums on the same grid — the drum machine's own
/// showcase of what a grid is for.
#[test]
fn the_two_new_demos_are_on_the_other_two_grids() {
    let songs = demo::songs();
    let jazz = songs.get(5).expect("the sixth demo");
    assert_eq!(jazz.name, "Acid Jazz");
    assert_eq!(jazz.state.bpm, 140.0);
    assert_eq!(jazz.state.bank.get(0).grid, Grid::EighthTriplet);
    assert_eq!(jazz.state.bank.get(0).len(), 12);
    for slot in 0..PATTERNS_PER_BANK {
        assert_eq!(
            jazz.state.bank.get(slot).grid,
            Grid::EighthTriplet,
            "Acid Jazz A{} is not a triplet bar",
            slot + 1
        );
        assert_eq!(
            jazz.drums[slot].grid,
            Grid::EighthTriplet,
            "Acid Jazz's drum A{} is not on the line's grid",
            slot + 1
        );
        assert!(
            strikes(&jazz.drums[slot]) > 0,
            "Acid Jazz A{} is silent",
            slot + 1
        );
    }

    let fast = songs.get(6).expect("the seventh demo");
    assert_eq!(fast.name, "Fast Acid");
    assert_eq!(fast.state.bpm, 152.0);
    assert_eq!(fast.state.bank.get(0).grid, Grid::ThirtySecond);
    assert_eq!(fast.state.bank.get(0).len(), 32);
    for slot in 0..PATTERNS_PER_BANK {
        assert_eq!(
            fast.state.bank.get(slot).grid,
            Grid::ThirtySecond,
            "Fast Acid A{} is not a thirty-second bar",
            slot + 1
        );
        assert_eq!(fast.drums[slot].grid, Grid::ThirtySecond);
        assert!(
            strikes(&fast.drums[slot]) > 0,
            "Fast Acid A{} is silent",
            slot + 1
        );
    }
}
