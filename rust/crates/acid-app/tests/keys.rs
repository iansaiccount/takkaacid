//! Taking the panel's key presses out of the frame, so a logic-only tick cannot replay one.
//!
//! `main` runs its key handling in `logic`, which egui calls every frame — including the 200 ms ticks of a hidden window,
//! where it hands the last pass's events back in `input.events`. A reader would fire the same Space or Enter again on
//! every tick; `dispatch::take_presses` takes the press out of the frame instead, which is what makes it happen once.
//! The mapping from a key to a `KeyPress` stays in `dispatch::presses`, and these tests drive the taking, not the frames —
//! `keys_live.rs` is the keyboard through the panel.

use acid_app::ui::dispatch::{self, KeyPress};
use eframe::egui::{Event, Key, Modifiers, Pos2};

/// A key event as egui delivers it, with the two flags the taking rule reads.
fn press(key: Key, repeat: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers: Modifiers::default(),
    }
}

/// A panel key is taken out of the frame and handed back, so the same event cannot be dispatched a second time.
#[test]
fn a_panel_key_is_taken_and_returned_as_a_press() {
    let mut events = vec![
        press(Key::Space, false),
        Event::PointerMoved(Pos2::new(4.0, 4.0)),
        press(Key::Enter, false),
        Event::Text("A".to_string()),
    ];

    let taken = dispatch::take_presses(&mut events, false);

    assert_eq!(
        taken,
        vec![
            KeyPress {
                key: Key::Space,
                shift: false,
                ctrl: false
            },
            KeyPress {
                key: Key::Enter,
                shift: false,
                ctrl: false
            },
        ],
        "both presses come back, in the order egui saw them"
    );
    assert_eq!(
        events,
        vec![
            Event::PointerMoved(Pos2::new(4.0, 4.0)),
            Event::Text("A".to_string())
        ],
        "and nothing else leaves the frame: the pointer and the text are the widgets'"
    );
}

/// A key the panel has no use for is the keyboard's, so it stays where the widgets can read it.
#[test]
fn a_key_the_panel_does_not_use_stays_in_the_frame() {
    // `B` auditions a note; a copy is the clipboard's, and neither is a press the panel acts on.
    let mut events = vec![press(Key::B, false), Event::Copy];

    assert!(dispatch::take_presses(&mut events, false).is_empty());
    assert_eq!(
        events,
        vec![press(Key::B, false), Event::Copy],
        "an audition key and a clipboard event are not the key path's to take"
    );
}

/// A focused text field owns the keyboard: the space bar is a space, not the transport.
#[test]
fn a_typing_hand_takes_nothing_out_of_the_frame() {
    let mut events = vec![press(Key::Space, false), press(Key::Enter, false)];

    assert!(dispatch::take_presses(&mut events, true).is_empty());
    assert_eq!(
        events,
        vec![press(Key::Space, false), press(Key::Enter, false)],
        "the field's keys are the field's"
    );
}

/// Only the tempo nudges repeat; a held Enter flips a step at frame rate and a held pattern arrow races the bank.
#[test]
fn only_a_tempo_nudge_repeats_out_of_the_frame() {
    let mut events = vec![press(Key::ArrowUp, true), press(Key::Enter, true)];

    assert_eq!(
        dispatch::take_presses(&mut events, false),
        vec![KeyPress {
            key: Key::ArrowUp,
            shift: false,
            ctrl: false
        }],
        "a held arrow is a sweep; a held Enter is refused"
    );
    assert_eq!(
        events,
        vec![press(Key::Enter, true)],
        "and the press it refused stays where it was: the panel did not act on it"
    );
}

/// Escape is not a panel key (what it does depends on what is open), so it travels by its own take — and it must be taken,
/// because a reader replays the last pass's buffer on a logic-only tick and one Escape closes one card per tick.
#[test]
fn escape_is_taken_once_and_never_out_of_a_text_field() {
    let escape = Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    let mut events = vec![escape.clone(), press(Key::Space, false)];
    assert!(
        dispatch::take_escape(&mut events, false),
        "an Escape press is taken"
    );
    assert_eq!(
        events,
        vec![press(Key::Space, false)],
        "and nothing else leaves the frame with it"
    );
    assert!(
        !dispatch::take_escape(&mut events, false),
        "the buffer is empty of Escapes now: a tick that follows takes nothing"
    );

    // A focused field uses Escape itself, so nothing may be taken out from under it.
    let mut events = vec![escape];
    assert!(!dispatch::take_escape(&mut events, true));
    assert_eq!(events.len(), 1, "the field's Escape stays where it was");
}
