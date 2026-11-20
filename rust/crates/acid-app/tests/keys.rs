//! Taking the panel's key presses out of the frame, so a logic-only tick cannot replay one.
//!
//! `main` runs its key handling in `logic`, which egui calls every frame — including the 200 ms ticks of a hidden window,
//! where it hands the last pass's events back in `input.events`. A reader would fire the same Space or Enter again on
//! every tick; `dispatch::take_presses` takes the press out of the frame instead, which is what makes it happen once.
//! The mapping from a key to a `KeyPress` stays in `dispatch::presses`, and these tests drive the taking, not the frames —
//! `keys_live.rs` is the keyboard through the panel.

use acid_app::device::DeviceKind;
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

    let taken = dispatch::take_presses(&mut events, DeviceKind::Acid, false);

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
    // `B` is neither a note on an acid face (`A W S E D F T G Y H U J K` are) nor a shortcut, and a copy is the
    // clipboard's: neither is a press the panel acts on. On a drum face `B` is the filter's band side, which is why this
    // test names its face.
    let mut events = vec![press(Key::B, false), Event::Copy];

    assert!(dispatch::take_presses(&mut events, DeviceKind::Acid, false).is_empty());
    assert_eq!(
        events,
        vec![press(Key::B, false), Event::Copy],
        "a key the panel has no use for and a clipboard event are not the key path's to take"
    );
}

/// The same key is a note on an acid device and `HIT`'s key on a drum machine, so the taking asks which face is showing.
#[test]
fn the_hit_key_is_taken_on_a_drum_face_and_left_for_the_note_on_an_acid_one() {
    let mut acid = vec![press(Key::H, false)];
    assert!(
        dispatch::take_presses(&mut acid, DeviceKind::Acid, false).is_empty(),
        "`h` is one of the 303's notes and must reach the keyboard"
    );
    assert_eq!(acid, vec![press(Key::H, false)]);

    let mut drums = vec![press(Key::H, false)];
    assert_eq!(
        dispatch::take_presses(&mut drums, DeviceKind::Drums, false),
        vec![KeyPress {
            key: Key::H,
            shift: false,
            ctrl: false
        }],
        "on a drum face `h` is the hit key"
    );
    assert!(drums.is_empty(), "and the press is spent");

    // The filter's three letters are the drum face's alone: on an acid face nothing in the panel wants them, so they stay in
    // the frame for the widgets, and on a drum face every one of them is taken.
    for key in [Key::L, Key::B, Key::P] {
        let mut acid = vec![press(key, false)];
        assert!(
            dispatch::take_presses(&mut acid, DeviceKind::Acid, false).is_empty(),
            "{key:?} is not an acid face's key"
        );
        assert_eq!(acid, vec![press(key, false)], "{key:?}: and it stays put");

        let mut drums = vec![press(key, false)];
        assert_eq!(
            dispatch::take_presses(&mut drums, DeviceKind::Drums, false).len(),
            1,
            "{key:?} is a filter key on a drum face"
        );
        assert!(drums.is_empty(), "{key:?}: and the press is spent");
    }
    // `Enter` and the plain arrows are the step editor's on **both** faces, for different jobs: the 303 toggles and selects
    // a step with them, the drum face writes the selected voice into the selected step and moves that selection. Both faces
    // take them, and a text field still owns them first.
    for kind in [DeviceKind::Acid, DeviceKind::Drums] {
        let mut events = vec![press(Key::Enter, false), press(Key::ArrowLeft, false)];
        assert_eq!(
            dispatch::take_presses(&mut events, kind, false).len(),
            2,
            "{kind:?} takes the step editor's keys"
        );
        assert!(events.is_empty(), "{kind:?}: and spends them");
    }
}

/// A focused text field owns the keyboard: the space bar is a space, not the transport.
#[test]
fn a_typing_hand_takes_nothing_out_of_the_frame() {
    let mut events = vec![press(Key::Space, false), press(Key::Enter, false)];

    assert!(dispatch::take_presses(&mut events, DeviceKind::Acid, true).is_empty());
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
        dispatch::take_presses(&mut events, DeviceKind::Acid, false),
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
