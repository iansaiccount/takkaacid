//! The computer keyboard must reach the same gate a MIDI keyboard writes, so both play one voice.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel};
use eframe::egui::{self, Key, Pos2, Rect, vec2};

fn window() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

fn key_event(key: Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

/// Run `draw_panel` through the given frames.
fn frames(shared: &SharedState, panel: &mut PanelState, per_frame: Vec<Vec<egui::Event>>) {
    let ctx = egui::Context::default();
    for events in per_frame {
        let mut input = egui::RawInput {
            screen_rect: Some(window()),
            ..Default::default()
        };
        input.events = events;
        let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
        output.textures_delta.clear();
    }
}

#[test]
fn holding_a_key_opens_the_gate_and_releasing_it_closes_it() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    frames(
        &shared,
        &mut panel,
        vec![vec![], vec![key_event(Key::A, true)]],
    );
    assert!(
        shared.midi_gate.load(std::sync::atomic::Ordering::Relaxed),
        "A should open the gate"
    );
    assert!(
        (shared.midi_hz.get() - 55.0).abs() < 0.5,
        "A is the 55 Hz anchor"
    );

    frames(&shared, &mut panel, vec![vec![key_event(Key::A, false)]]);
    assert!(
        !shared.midi_gate.load(std::sync::atomic::Ordering::Relaxed),
        "releasing every mapped key should close the gate"
    );
}

#[test]
fn the_octave_keys_move_the_range_and_clamp() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    frames(
        &shared,
        &mut panel,
        vec![vec![], vec![key_event(Key::X, true)]],
    );
    assert_eq!(shared.octave.load(std::sync::atomic::Ordering::Relaxed), 1);

    // Hold X for many frames: the offset must stop at 2.
    let holds: Vec<Vec<egui::Event>> = (0..4).map(|_| vec![key_event(Key::X, true)]).collect();
    frames(&shared, &mut panel, holds);
    assert_eq!(shared.octave.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[test]
fn the_keyboard_plays_in_the_selected_octave() {
    let shared = SharedState::new();
    shared.octave.store(1, std::sync::atomic::Ordering::Relaxed);
    let mut panel = PanelState::default();
    frames(
        &shared,
        &mut panel,
        vec![vec![], vec![key_event(Key::A, true)]],
    );
    assert!(
        (shared.midi_hz.get() - 110.0).abs() < 1.0,
        "A an octave up is 110 Hz, got {}",
        shared.midi_hz.get()
    );
}

/// A card over the face is modal: a key that would write a step behind it is a note instead, and a MIDI note waiting to be
/// placed is drained without landing. The class the second review found one instance of, closed for all three writers.
#[test]
fn a_card_up_stops_a_key_and_a_midi_note_from_writing_a_step() {
    use acid_app::ui::banks::Banks;

    let shared = SharedState::new();
    let mut panel = PanelState {
        step_mode: true,
        songs: acid_app::ui::SongState {
            card: Some(Banks::default()),
            ..acid_app::ui::SongState::default()
        },
        ..PanelState::default()
    };
    let before = shared.pattern().steps;

    // Step mode would write this key into the selected step; behind a card it plays instead.
    frames(
        &shared,
        &mut panel,
        vec![vec![], vec![key_event(Key::A, true)]],
    );
    assert_eq!(
        shared.pattern().steps,
        before,
        "a key behind a card writes no step"
    );
    assert!(
        shared.midi_gate.load(std::sync::atomic::Ordering::Relaxed),
        "it is an audition instead: the gate is what a card does not hide"
    );
    assert!(!panel.dirty, "and the session is not marked unsaved by it");

    frames(&shared, &mut panel, vec![vec![key_event(Key::A, false)]]);

    // The same for a note that arrived from MIDI while the card was up: drained, not placed.
    assert!(shared.notes.push(45, 100), "a note is waiting");
    frames(&shared, &mut panel, vec![vec![]]);
    assert_eq!(
        shared.pattern().steps,
        before,
        "a MIDI note behind a card writes no step either"
    );
    assert!(
        shared.notes.is_empty(),
        "the note was drained, not left waiting to land when the card closes"
    );
}
