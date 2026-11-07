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
