//! The clipboard gestures: what a copy puts on the clipboard and a paste does with it.
//!
//! Both arrive as egui *events*, not key presses — `egui-winit` turns `Ctrl+C`/`Ctrl+V` into `Event::Copy` and
//! `Event::Paste(text)` and never delivers the key — so this is one function beside `escape`, tested with events and a real
//! bank rather than with frames. `main` is what hands it the frame's events and takes the text back for `ctx.copy_text`.

use acid_app::clipboard::{self, Clip};
use acid_app::state::SharedState;
use acid_app::ui::{PanelState, dispatch};
use acid_core::bank::PatternBank;
use acid_core::pattern::{Pattern, STEPS_PER_PATTERN, Step};
use acid_core::pitch::Pitch;
use eframe::egui::Event;

/// Two patterns, neither empty: the one in the slot and the one on the clipboard, so a test can tell them apart by content.
fn left() -> Pattern {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    steps[0] = Step::note(Pitch::C, -1);
    steps[4] = Step {
        accent: true,
        ..Step::note(Pitch::G, 0)
    };
    Pattern::from_steps(steps)
}

fn right() -> Pattern {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    steps[2] = Step {
        slide: true,
        velocity: 40,
        ..Step::note(Pitch::A, 1)
    };
    steps[3] = Step::note(Pitch::DSharp, 0);
    Pattern::from_steps(steps)
}

/// A third pattern, so "the neighbour was not touched" means something: the paste, the slot it lands in and the slot beside
/// it are three different things.
fn neighbour() -> Pattern {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    steps[0] = Step {
        accent: true,
        slide: true,
        velocity: 90,
        ..Step::note(Pitch::E, 2)
    };
    steps[14] = Step::note(Pitch::F, -1);
    Pattern::from_steps(steps)
}

/// A rig with the panel on the bank's current slot, holding the pattern `left()` there, with the neighbours filled with
/// patterns of their own so a paste can be shown to leave them alone.
fn rig() -> (std::sync::Arc<SharedState>, PatternBank, PanelState) {
    let shared = SharedState::new();
    let mut bank = PatternBank::new_empty();
    bank.replace(8, neighbour()); // B1
    bank.replace(9, left()); // B2
    bank.replace(10, neighbour()); // B3 — beside the slot a paste lands in
    bank.select(9);
    shared.set_pattern(std::sync::Arc::new(left()));
    shared.set_pattern_index(9);
    let panel = PanelState {
        pattern_id: PatternBank::id_of(9),
        ..PanelState::default()
    };
    (shared, bank, panel)
}

fn notice(panel: &PanelState) -> String {
    panel
        .notice
        .as_ref()
        .map(|(said, _)| said.clone())
        .unwrap_or_default()
}

#[test]
fn copying_notices_the_slot_and_returns_the_text() {
    let (shared, mut bank, mut panel) = rig();

    let returned = dispatch::clipboard(&[Event::Copy], false, &mut panel, &shared, &mut bank)
        .expect("a copy hands back the text for the OS clipboard");

    assert_eq!(notice(&panel), "copied B2");
    assert_eq!(
        clipboard::decode(&returned),
        Some(Clip::Pattern {
            pattern: left(),
            from: Some("B2".to_string())
        }),
        "and what it hands back is the pattern on the panel, labelled with its slot"
    );
    assert!(!panel.dirty, "copying changes nothing to save");
}

#[test]
fn pasting_lands_the_pattern_and_says_what_it_replaced() {
    let (shared, mut bank, mut panel) = rig();
    let clip = clipboard::encode(&Clip::Pattern {
        pattern: right(),
        from: Some("A1".to_string()),
    });

    let returned =
        dispatch::clipboard(&[Event::Paste(clip)], false, &mut panel, &shared, &mut bank);

    assert_eq!(returned, None, "a paste puts nothing on the clipboard");
    assert_eq!(
        *shared.pattern(),
        right(),
        "the engine plays the pasted pattern at its next block"
    );
    assert_eq!(
        bank.get(9),
        &right(),
        "and the slot it lands in holds it, so a pattern switch keeps it"
    );
    assert_eq!(
        bank.get(10),
        &neighbour(),
        "the slot beside it is untouched, and on the other side too"
    );
    assert_eq!(bank.get(8), &neighbour(), "both neighbours untouched");
    assert!(panel.dirty, "a paste is a change to save");
    assert_eq!(notice(&panel), "pasted A1 into B2");
}

#[test]
fn a_paste_of_something_else_says_so_and_changes_nothing() {
    let (shared, mut bank, mut panel) = rig();

    dispatch::clipboard(
        &[Event::Paste("hello".to_string())],
        false,
        &mut panel,
        &shared,
        &mut bank,
    );

    assert_eq!(
        *shared.pattern(),
        left(),
        "the pattern on the panel is the one it was"
    );
    assert_eq!(bank.get(9), &left());
    assert!(!panel.dirty);
    assert_eq!(notice(&panel), "the clipboard has no pattern");
}

#[test]
fn a_focused_field_keeps_the_clipboard() {
    let (shared, mut bank, mut panel) = rig();
    let clip = clipboard::encode(&Clip::Pattern {
        pattern: right(),
        from: Some("A1".to_string()),
    });

    let returned = dispatch::clipboard(
        &[Event::Copy, Event::Paste(clip)],
        true,
        &mut panel,
        &shared,
        &mut bank,
    );

    assert_eq!(returned, None, "the field's clipboard is the field's");
    assert_eq!(*shared.pattern(), left(), "and the pattern is untouched");
    assert_eq!(notice(&panel), "", "a typing hand is told nothing");
    assert!(!panel.dirty);
}
