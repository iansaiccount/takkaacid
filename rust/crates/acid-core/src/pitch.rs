//! Pitch material: the thirteen note keys of the panel and their frequencies.
//!
//! [`ROOT_MIDI`] is a *frequency anchor*, not a note name: the pitch called [`Pitch::C`] at octave
//! offset `0` sounds at 55 Hz (the 303's home register), and `CHigh` is the same pitch an octave up.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The 13 pitches the panel offers: `C` up to `CHigh`, one chromatic octave plus the octave note.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Pitch {
    #[default]
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
    CHigh,
}

impl Pitch {
    /// Every pitch in chromatic order, matching the panel's 13 note keys.
    pub const ALL: [Pitch; 13] = [
        Pitch::C,
        Pitch::CSharp,
        Pitch::D,
        Pitch::DSharp,
        Pitch::E,
        Pitch::F,
        Pitch::FSharp,
        Pitch::G,
        Pitch::GSharp,
        Pitch::A,
        Pitch::ASharp,
        Pitch::B,
        Pitch::CHigh,
    ];

    /// Semitones above [`Pitch::C`], `0..=12`.
    pub fn semitones(self) -> u8 {
        self as u8
    }
}

/// MIDI note that sounds at [`Pitch::C`] with octave offset `0`.
///
/// It is MIDI 12 — C0, 16.35 Hz — so a step's `(pitch, octave)` is the note it actually plays: `(C, 3)` is MIDI 48, C3,
/// and a keyboard's C3 stores as C3. It used to be 33 (A1), which made every stored name three semitones sharp of the
/// sound, so a keyboard's C3 was stored as `D#1` while sounding as C3.
pub const ROOT_MIDI: u8 = 12;

/// The lowest and highest MIDI note the instrument plays: C-1 to B5, the panel's own span.
///
/// A step outside it is *silent* rather than clamped into it. That is what makes the octave buttons a true round trip:
/// transposing up pushes notes past B5 instead of flattening them onto it, and transposing back restores them.
pub const NOTE_LOW: i32 = 0;
pub const NOTE_HIGH: i32 = 83;

/// The MIDI note a step's `(pitch, octave)` names, with nothing clamped.
pub fn step_midi(pitch: Pitch, octave_offset: i8) -> i32 {
    i32::from(ROOT_MIDI) + i32::from(pitch.semitones()) + 12 * i32::from(octave_offset)
}

/// Equal-tempered frequency, 12-TET with A4 (MIDI 69) at 440 Hz.
pub fn midi_to_hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// Frequency of a step's pitch at an octave offset.
pub fn step_hz(pitch: Pitch, octave_offset: i8) -> f32 {
    midi_to_hz(ROOT_MIDI as f32 + pitch.semitones() as f32 + 12.0 * octave_offset as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semitones_count_up_one_octave() {
        assert_eq!(Pitch::C.semitones(), 0);
        assert_eq!(Pitch::E.semitones(), 4);
        assert_eq!(Pitch::B.semitones(), 11);
        assert_eq!(Pitch::CHigh.semitones(), 12);
        assert_eq!(Pitch::ALL.len(), 13);
    }

    #[test]
    fn concert_a_is_440() {
        assert!((midi_to_hz(69.0) - 440.0).abs() < 0.01);
        assert!((midi_to_hz(81.0) - 880.0).abs() < 0.01);
        assert!((midi_to_hz(57.0) - 220.0).abs() < 0.01);
    }

    #[test]
    fn root_is_the_c_whose_number_the_step_carries() {
        // A step's pair is the note it plays, so `(C, n)` is C-n on a keyboard: C0 is 16.35 Hz, C3 is 130.8 Hz, and the
        // 55 Hz that used to be called "C0" is now, correctly, A1.
        assert!((step_hz(Pitch::C, 0) - 16.35).abs() < 0.02);
        assert!((step_hz(Pitch::C, 3) - 130.81).abs() < 0.05);
        assert!((step_hz(Pitch::A, 1) - 55.0).abs() < 0.05);
        assert!((step_hz(Pitch::CHigh, 0) - 32.7).abs() < 0.05);
        assert!((step_hz(Pitch::C, -1) - 8.18).abs() < 0.02);
    }
}
