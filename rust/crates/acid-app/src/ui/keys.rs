//! Playing the voice from the computer keyboard: a chromatic scale on the home row, Z/X for the octave.
//!
//! The mapping is the one most DAWs use, so muscle memory carries over: `A W S E D F T G Y H U J K` walks
//! `Pitch::ALL` from C to the octave above.

use acid_core::pitch::{Pitch, step_hz};
use eframe::egui::Key;

/// The home-row mapping, in `Pitch::ALL` order.
pub const KEY_MAP: [(Key, Pitch); 13] = [
    (Key::A, Pitch::C),
    (Key::W, Pitch::CSharp),
    (Key::S, Pitch::D),
    (Key::E, Pitch::DSharp),
    (Key::D, Pitch::E),
    (Key::F, Pitch::F),
    (Key::T, Pitch::FSharp),
    (Key::G, Pitch::G),
    (Key::Y, Pitch::GSharp),
    (Key::H, Pitch::A),
    (Key::U, Pitch::ASharp),
    (Key::J, Pitch::B),
    (Key::K, Pitch::CHigh),
];

/// The pitch a key plays, if it is one of the mapped keys.
pub fn pitch_for(key: Key) -> Option<Pitch> {
    KEY_MAP
        .iter()
        .find(|(mapped, _)| *mapped == key)
        .map(|(_, pitch)| *pitch)
}

/// `Z` and `X` move the keyboard's octave down and up.
pub fn octave_key(key: Key) -> Option<i8> {
    match key {
        Key::Z => Some(-1),
        Key::X => Some(1),
        _ => None,
    }
}

/// The frequency to play, at the computer keyboard's octave offset (clamped to `-2..=2`).
pub fn key_hz(pitch: Pitch, octave: i32) -> f32 {
    let offset = octave.clamp(-2, 2) as i8;
    step_hz(pitch, offset)
}

/// What one key event asks of the panel.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyAction {
    /// A key went down. `shift` is the modifier as it was at that moment.
    Press { key: Key, shift: bool },
    /// A key came up. This is what lets step mode walk on, so one press is one note and one step.
    Release { key: Key },
}

/// The frame's key events in order, with egui's auto-repeats dropped: a held key is one press, and a repeated one would
/// re-gate the voice at the repeat rate and stutter the note.
///
/// The shift flag comes from the *event*, not from the frame's input state: the event carries the modifier that was
/// down at the moment of the press, and egui only refreshes `modifiers` from a `ModifiersChanged` event.
pub fn key_actions(events: &[eframe::egui::Event]) -> Vec<KeyAction> {
    events
        .iter()
        .filter_map(|event| match event {
            eframe::egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } => Some(KeyAction::Press {
                key: *key,
                shift: modifiers.shift,
            }),
            eframe::egui::Event::Key {
                key,
                pressed: false,
                ..
            } => Some(KeyAction::Release { key: *key }),
            _ => None,
        })
        .collect()
}

/// Record a press, saying whether it is the first one for that key.
///
/// A key already down is a repeat, whatever egui labelled it, and must not write a second step. This lives here, as a
/// plain function, because a live panel cannot show the case: egui drops a `pressed` event for a key it already believes
/// is down, so the window would never deliver the duplicate that this guards against.
pub fn first_press(held: &mut Vec<Key>, key: Key) -> bool {
    if held.contains(&key) {
        return false;
    }
    held.push(key);
    true
}

/// Forget a released key, saying whether this panel had written with it. Only then does step mode walk on: an auditioned
/// note is not an edit, and a release with no press behind it moves nothing.
pub fn release(held: &mut Vec<Key>, key: Key) -> bool {
    match held.iter().position(|down| *down == key) {
        Some(index) => {
            held.remove(index);
            true
        }
        None => false,
    }
}

/// The MIDI note number a mapped key asks for at the keyboard's octave offset. The panel's zero is MIDI 33 (55 Hz),
/// which is what `edit::from_midi_note` counts back from.
pub fn midi_note(pitch: Pitch, octave: i32) -> u8 {
    let offset = octave.clamp(-2, 2);
    (33 + i32::from(pitch.semitones()) + 12 * offset).clamp(0, 127) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_home_row_is_a_chromatic_scale() {
        assert_eq!(pitch_for(Key::A), Some(Pitch::C));
        assert_eq!(pitch_for(Key::W), Some(Pitch::CSharp));
        assert_eq!(pitch_for(Key::D), Some(Pitch::E));
        assert_eq!(pitch_for(Key::K), Some(Pitch::CHigh));
        assert_eq!(pitch_for(Key::Q), None);
        assert_eq!(KEY_MAP.len(), 13);
    }

    #[test]
    fn a_held_note_key_is_one_press_not_one_per_repeat() {
        use eframe::egui::{Event, Modifiers};
        let key = |pressed: bool, repeat: bool| Event::Key {
            key: Key::A,
            physical_key: None,
            pressed,
            repeat,
            modifiers: Modifiers::default(),
        };
        // One real press, then the auto-repeats egui emits while the key is held.
        let events = vec![key(true, false), key(true, true), key(true, true)];
        let press = KeyAction::Press {
            key: Key::A,
            shift: false,
        };
        assert_eq!(&key_actions(&events)[..1], &[press]);
        // A release is its own action — it is what lets step mode walk — and other events are neither.
        assert_eq!(
            key_actions(&[key(false, false)]),
            vec![KeyAction::Release { key: Key::A }]
        );
        assert!(key_actions(&[]).is_empty());
    }

    #[test]
    fn a_second_press_of_a_key_already_down_is_not_a_second_note() {
        // The "one press, two notes" bug: a repeat must not write again, and only a release may re-arm the key.
        let mut held = Vec::new();
        assert!(first_press(&mut held, Key::A), "the first press writes");
        assert!(!first_press(&mut held, Key::A), "a repeat does not");
        assert!(!first_press(&mut held, Key::A));
        assert_eq!(held, vec![Key::A]);

        assert!(release(&mut held, Key::A), "the release re-arms it");
        assert!(
            !release(&mut held, Key::A),
            "a second release is not a release"
        );
        assert!(
            first_press(&mut held, Key::A),
            "and a fresh press writes again"
        );
    }

    #[test]
    fn a_release_of_a_key_that_never_went_down_is_not_ours() {
        // Releasing a key the panel never wrote with must not walk the border: the note that was auditioned is not an
        // edit, and a key released after the window lost focus leaves nothing behind.
        let mut held = Vec::new();
        assert!(!release(&mut held, Key::J));
        assert!(held.is_empty());
    }

    #[test]
    fn z_and_x_shift_the_octave_and_clamp() {
        assert_eq!(octave_key(Key::Z), Some(-1));
        assert_eq!(octave_key(Key::X), Some(1));
        assert_eq!(octave_key(Key::A), None);
        assert!((key_hz(Pitch::C, 0) - 55.0).abs() < 0.1);
        assert!((key_hz(Pitch::C, 1) - 110.0).abs() < 0.2);
        assert!((key_hz(Pitch::C, 9) - key_hz(Pitch::C, 2)).abs() < 1e-3);
    }

    #[test]
    fn every_mapped_key_is_distinct() {
        let mut keys: Vec<Key> = KEY_MAP.iter().map(|(k, _)| *k).collect();
        keys.sort_by_key(|k| format!("{k:?}"));
        keys.dedup();
        assert_eq!(keys.len(), 13);
    }
}
