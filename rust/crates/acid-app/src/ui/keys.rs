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

/// Every key the frame reports as freshly pressed, with egui's auto-repeats dropped: a held key is one press, and a
/// repeated one would re-gate the voice at the repeat rate and stutter the note.
///
/// The shift flag comes from the *event*, not from the frame's input state: the event carries the modifier that was
/// down at the moment of the press, and egui only refreshes `modifiers` from a `ModifiersChanged` event.
pub fn fresh_presses(events: &[eframe::egui::Event]) -> Vec<(Key, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            eframe::egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } => Some((*key, modifiers.shift)),
            _ => None,
        })
        .collect()
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
        assert_eq!(fresh_presses(&events), vec![(Key::A, false)]);
        // Releases and other events are not presses.
        assert!(fresh_presses(&[key(false, false)]).is_empty());
        assert!(fresh_presses(&[]).is_empty());
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
