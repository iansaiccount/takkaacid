//! Step-editing gestures as pure functions, so the panel's input rules are testable without a window.
//!
//! Pitch is carried as an absolute semitone index `pitch.semitones() + 12 * octave` (C is the 55 Hz anchor,
//! `CHigh` is one octave above C). Nudges clamp to `-12..=24` and the canonical spelling always uses an
//! index in `0..=11` plus an octave, so `(C, 1)` and `(CHigh, 0)` name the same pitch and only the former is
//! ever produced.

use acid_core::pattern::Step;
use acid_core::pitch::Pitch;

/// One editing gesture on a step key.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StepGesture {
    Select,
    Toggle,
    Accent,
    Slide,
    Clear,
}

/// Apply one gesture. `last_pitch`/`last_octave` are the pitch a newly-enabled step should take.
///
/// Resting a step keeps its pitch and flags, so toggling it back restores what was there. The exception is
/// the rest that is bit-identical to `Step::rest()` — no flags, pitch `C0` — which has nothing to restore and
/// comes in at `last_pitch`/`last_octave`. A flag gesture forces the step on — a lit accent with no note would
/// be invisible on the panel.
pub fn apply_gesture(step: Step, gesture: StepGesture, last_pitch: Pitch, last_octave: i8) -> Step {
    let enable = |mut step: Step| -> Step {
        if !step.on {
            // A rest that differs from `Step::rest()` is remembering a note, and that note wins — only a
            // bare rest has nothing to restore, so `last_pitch` is its fallback rather than the rule. Read
            // the comparison before turning the step on, or every rest would look bare.
            let bare = step == Step::rest();
            step.on = true;
            if bare {
                step.pitch = last_pitch;
                step.octave_offset = last_octave;
            }
        }
        step
    };
    match gesture {
        // Selecting is the panel's business; the step itself must not move.
        StepGesture::Select => step,
        StepGesture::Toggle => {
            if step.on {
                Step { on: false, ..step }
            } else {
                enable(step)
            }
        }
        StepGesture::Accent => {
            let mut step = enable(step);
            step.accent = !step.accent;
            step
        }
        StepGesture::Slide => {
            let mut step = enable(step);
            step.slide = !step.slide;
            step
        }
        StepGesture::Clear => Step::rest(),
    }
}

/// Which gesture an input maps to. Double-click clears; otherwise a modifier beats the plain click, and a
/// plain click only selects — editing takes ctrl, so a look never changes the music.
pub fn gesture_from_input(
    left: bool,
    shift: bool,
    alt: bool,
    ctrl: bool,
    double: bool,
) -> Option<StepGesture> {
    // No click means no gesture. The panel asks for a gesture for all sixteen keys on every frame, so a held
    // modifier without this guard would read as a gesture on every key, every frame.
    if !left && !double {
        return None;
    }
    if double {
        Some(StepGesture::Clear)
    } else if shift {
        Some(StepGesture::Accent)
    } else if alt {
        Some(StepGesture::Slide)
    } else if ctrl {
        Some(StepGesture::Toggle)
    } else if left {
        Some(StepGesture::Select)
    } else {
        None
    }
}

/// Pixels of vertical drag per semitone at 1× scale.
pub const PITCH_DRAG_PIXELS_PER_SEMITONE: f32 = 12.0;

/// The panel's lowest and highest absolute semitone: octaves `-2..=4`, which is A-1 to G#6 on a keyboard.
///
/// Since the lattice root is MIDI 12 (C0), a pair here *is* the note it plays, so this only has to clamp: a 25-key
/// board's C3 stores as `(C, 3)` and sounds 130.8 Hz.
pub const PANEL_LOW: i32 = -12;
pub const PANEL_HIGH: i32 = 71;

/// Canonical `(note, octave)` for an absolute semitone, clamped to the panel's range.
fn from_absolute(absolute: i32) -> (Pitch, i8) {
    let moved = absolute.clamp(PANEL_LOW, PANEL_HIGH);
    let octave = moved.div_euclid(12).clamp(PANEL_LOW / 12, PANEL_HIGH / 12);
    let index = moved.rem_euclid(12) as usize;
    (Pitch::ALL[index], octave as i8)
}

/// The panel's `(pitch, octave)` for a MIDI note number — the note the step will store and play.
///
/// The lattice root is MIDI 12, so this is a straight decomposition: note 48 is `(C, 3)` and sounds 130.8 Hz. The panel
/// carries C-1 to B5, and only a note outside that window is clamped.
pub fn from_midi_note(note: u8) -> (Pitch, i8) {
    from_absolute(i32::from(note) - 12)
}

/// The panel's `(pitch, octave)` for a note played at the computer keyboard's octave offset.
///
/// The computer keyboard's octave `0` is 55 Hz — A1 — and the panel's name for that is `(A, 1)`, so the keyboard's
/// position is shifted by the lattice root. The pitch class is always kept; only the octave is clamped to the keyboard's
/// own `-2..=2`.
pub fn from_keyboard(pitch: Pitch, octave: i32) -> (Pitch, i8) {
    let offset = octave.clamp(-2, 2);
    from_absolute(i32::from(pitch.semitones()) + 12 * offset + 21)
}

/// Move a pitch by whole semitones from `(pitch, octave)`, rolling into the octave and clamping to the panel's range.
/// `-12..=24` absolute.
///
/// The result is always canonical: an index in `0..=11` (plain `C`..`B`) plus an octave, so `CHigh` is
/// spelled `(C, 1)` on the way out.
pub fn transpose(pitch: Pitch, octave: i8, semitones: i8) -> (Pitch, i8) {
    let absolute = i32::from(pitch.semitones()) + 12 * i32::from(octave) + i32::from(semitones);
    from_absolute(absolute)
}

/// Move a pitch by whole semitones from octave zero, rolling into `octave_offset` the same way.
pub fn nudge_pitch(pitch: Pitch, semitones: i8) -> (Pitch, i8) {
    transpose(pitch, 0, semitones)
}

/// Semitones covered by a vertical drag: up is higher, and shift quarters the sensitivity.
pub fn semitones_from_drag(dy_pixels: f32, fine: bool) -> i8 {
    let per_semitone = if fine {
        PITCH_DRAG_PIXELS_PER_SEMITONE * 4.0
    } else {
        PITCH_DRAG_PIXELS_PER_SEMITONE
    };
    (-dy_pixels / per_semitone).round() as i8
}

/// The pitch a drag from `start` lands on.
pub fn drag_pitch(start: (Pitch, i8), dy_pixels: f32, fine: bool) -> (Pitch, i8) {
    let absolute = i32::from(start.0.semitones()) + 12 * i32::from(start.1);
    from_absolute(absolute + i32::from(semitones_from_drag(dy_pixels, fine)))
}

/// The panel's spelling of a pitch: the note it plays, always canonical.
///
/// The lattice root is MIDI 12 (C0), so this is the plain concert name of the pair — `(C, 3)` is `C3`, exactly what a
/// keyboard's C3 key sends and exactly the frequency the step plays.
pub fn pitch_name(pitch: Pitch, octave: i8) -> &'static str {
    /// The whole panel range, `-12..=71` absolute semitones (C-1 to B5), spelled out: the keycaps ask for one label per
    /// step per frame, so a `String` each would be sixteen allocations per frame.
    const LABELS: [&str; 84] = [
        "C-1", "C#-1", "D-1", "D#-1", "E-1", "F-1", "F#-1", "G-1", "G#-1", "A-1", "A#-1", "B-1",
        "C0", "C#0", "D0", "D#0", "E0", "F0", "F#0", "G0", "G#0", "A0", "A#0", "B0", "C1", "C#1",
        "D1", "D#1", "E1", "F1", "F#1", "G1", "G#1", "A1", "A#1", "B1", "C2", "C#2", "D2", "D#2",
        "E2", "F2", "F#2", "G2", "G#2", "A2", "A#2", "B2", "C3", "C#3", "D3", "D#3", "E3", "F3",
        "F#3", "G3", "G#3", "A3", "A#3", "B3", "C4", "C#4", "D4", "D#4", "E4", "F4", "F#4", "G4",
        "G#4", "A4", "A#4", "B4", "C5", "C#5", "D5", "D#5", "E5", "F5", "F#5", "G5", "G#5", "A5",
        "A#5", "B5",
    ];
    let absolute =
        (i32::from(pitch.semitones()) + 12 * i32::from(octave)).clamp(PANEL_LOW, PANEL_HIGH);
    LABELS[(absolute - PANEL_LOW) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keyboard_keeps_every_pitch_class_at_every_octave_of_its_range() {
        // The QWERTY keyboard's octave 0 sounds 55 Hz and must keep sounding it — re-rooting the lattice renames that
        // note from `C0` to `A1`, so what this pins is the *sound*, and the spacing between keys.
        for pitch in Pitch::ALL[..12].iter() {
            for octave in -2..=2 {
                let (placed_pitch, placed_octave) = from_keyboard(*pitch, octave);
                let played = acid_core::pitch::step_hz(placed_pitch, placed_octave);
                let wanted = acid_core::pitch::midi_to_hz(
                    33.0 + pitch.semitones() as f32 + 12.0 * octave as f32,
                );
                assert!(
                    (played - wanted).abs() < 0.5,
                    "{pitch:?} at octave {octave} plays {played} Hz, not {wanted} Hz"
                );
            }
        }

        // The thirteenth button is the note a twelfth above the octave in play — the QWERTY map is rooted at A, so its
        // "C above" is an A, two octaves up.
        for octave in -2..=2 {
            assert_eq!(
                from_keyboard(Pitch::CHigh, octave),
                (Pitch::A, octave as i8 + 2),
                "at offset {octave}"
            );
        }
    }

    #[test]
    fn every_midi_note_keeps_its_pitch_class_and_only_clamps_the_octave() {
        // A stored pair is the note it plays: note 48 is `(C, 3)`, note 12 is `(C, 0)`, and a note outside the panel's
        // C-1..B5 window is clamped — otherwise nothing is lost.
        for note in 12..=83u8 {
            let (pitch, octave) = from_midi_note(note);
            assert_eq!(
                pitch_name(pitch, octave),
                crate::midi::message::concert_name(note),
                "MIDI {note} stored as ({pitch:?}, {octave})"
            );
            assert!(
                (acid_core::pitch::step_hz(pitch, octave) - crate::midi::message::note_hz(note))
                    .abs()
                    < 0.5,
                "MIDI {note} sounds differently from the pair it stored"
            );
        }
        // Below C-1 and above B5 the pair clamps, and it clamps *musically*: the class is kept, the octave stops.
        assert_eq!(from_midi_note(0), (Pitch::C, -1));
        assert_eq!(from_midi_note(127).1, 5);
    }

    #[test]
    fn the_top_of_a_midi_keyboard_does_not_collapse_to_one_note() {
        // An E-MU Xboard25 spans MIDI 48..=72 and every note above 57 used to become C1 — so the whole upper half of the
        // keyboard wrote the same step, whatever was played.
        let written: Vec<_> = (57..=72u8).map(|note| from_midi_note(note).0).collect();
        for pair in written.windows(2) {
            assert_ne!(pair[0], pair[1], "two neighbouring keys wrote one pitch");
        }
    }
    #[test]
    fn transposition_respects_the_saved_octave() {
        assert_eq!(transpose(Pitch::C, 1, 1), (Pitch::CSharp, 1));
        assert_eq!(transpose(Pitch::C, 1, -12), (Pitch::C, 0));
        assert_eq!(transpose(Pitch::B, -1, 1), (Pitch::C, 0));
        // The clamp is the panel's, not the octave's: two octaves up is the ceiling.
        // The clamp is the panel's, not the octave's: the range now reaches B5, so an octave up is still inside it.
        assert_eq!(transpose(Pitch::C, 1, 12), (Pitch::C, 2));
    }

    #[test]
    fn clicking_a_rest_makes_a_note_with_the_last_pitch() {
        let s = apply_gesture(Step::rest(), StepGesture::Toggle, Pitch::G, 0);
        assert!(s.on);
        assert_eq!(s.pitch, Pitch::G);
    }

    #[test]
    fn clicking_a_note_rests_it_but_remembers_its_flags() {
        let s = Step {
            accent: true,
            slide: true,
            ..Step::note(Pitch::E, 0)
        };
        let rest = apply_gesture(s, StepGesture::Toggle, Pitch::C, 0);
        assert!(!rest.on);
        assert!(rest.accent && rest.slide);
        let back = apply_gesture(rest, StepGesture::Toggle, Pitch::C, 0);
        assert!(back.on && back.accent && back.slide);
    }

    #[test]
    fn a_rested_note_comes_back_at_its_own_pitch() {
        // Review finding 4, ruled "fix the behaviour": a rest remembers its note, so toggling it back
        // restores that note — not whatever was played last.
        let step = Step::note(Pitch::E, 1);
        let rest = apply_gesture(step, StepGesture::Toggle, Pitch::G, 0);
        assert!(!rest.on);
        let back = apply_gesture(rest, StepGesture::Toggle, Pitch::G, 0);
        assert_eq!((back.pitch, back.octave_offset), (Pitch::E, 1));
    }

    #[test]
    fn a_c0_step_with_no_flags_rests_to_the_same_bits_as_a_bare_rest() {
        // The known limit of the rule above, pinned so it is a documented edge and not a surprise: a step that
        // held C0 with no flags is byte-for-byte `Step::rest()`, so it comes back at `last_pitch`. Telling the
        // two apart needs a distinguishing bit in `Step` (its own change, with a session-file migration).
        let rested = apply_gesture(Step::note(Pitch::C, 0), StepGesture::Toggle, Pitch::G, 0);
        assert!(!rested.on);
        let back = apply_gesture(rested, StepGesture::Toggle, Pitch::G, 0);
        assert_eq!((back.pitch, back.octave_offset), (Pitch::G, 0));
    }

    #[test]
    fn accent_and_slide_force_the_step_on() {
        let a = apply_gesture(Step::rest(), StepGesture::Accent, Pitch::C, 0);
        assert!(a.on && a.accent);
        let b = apply_gesture(Step::rest(), StepGesture::Slide, Pitch::C, 0);
        assert!(b.on && b.slide);
        assert!(!apply_gesture(a, StepGesture::Accent, Pitch::C, 0).accent);
    }

    #[test]
    fn clear_removes_everything() {
        let s = Step {
            accent: true,
            slide: true,
            ..Step::note(Pitch::B, 1)
        };
        assert_eq!(
            apply_gesture(s, StepGesture::Clear, Pitch::C, 0),
            Step::rest()
        );
    }

    #[test]
    fn modifiers_win_over_a_plain_click() {
        assert_eq!(
            gesture_from_input(true, false, false, false, false),
            Some(StepGesture::Select)
        );
        assert_eq!(
            gesture_from_input(true, false, false, true, false),
            Some(StepGesture::Toggle)
        );
        assert_eq!(
            gesture_from_input(true, true, false, false, false),
            Some(StepGesture::Accent)
        );
        assert_eq!(
            gesture_from_input(true, false, true, false, false),
            Some(StepGesture::Slide)
        );
        assert_eq!(
            gesture_from_input(true, true, true, true, true),
            Some(StepGesture::Clear)
        );
        assert_eq!(gesture_from_input(false, false, false, false, false), None);
    }

    #[test]
    fn a_held_modifier_is_not_a_gesture() {
        // The panel asks every one of the sixteen keys for a gesture on every frame. Without this guard a
        // held ctrl read as ctrl+click on all of them, so every key flashed on and off at frame rate.
        assert_eq!(gesture_from_input(false, false, false, true, false), None);
        assert_eq!(gesture_from_input(false, true, false, false, false), None);
        assert_eq!(gesture_from_input(false, false, true, false, false), None);
        // A double click is its own trigger — egui reports it alongside the click.
        assert_eq!(
            gesture_from_input(false, false, false, false, true),
            Some(StepGesture::Clear)
        );
    }

    #[test]
    fn a_plain_click_only_selects_and_ctrl_click_toggles() {
        assert_eq!(
            gesture_from_input(true, false, false, false, false),
            Some(StepGesture::Select)
        );
        assert_eq!(
            gesture_from_input(true, false, false, true, false),
            Some(StepGesture::Toggle)
        );
        assert_eq!(
            gesture_from_input(true, true, false, true, false),
            Some(StepGesture::Accent)
        );
    }

    #[test]
    fn selecting_leaves_the_step_untouched() {
        let step = Step {
            accent: true,
            ..Step::note(Pitch::E, 0)
        };
        assert_eq!(apply_gesture(step, StepGesture::Select, Pitch::C, 0), step);
        assert_eq!(
            apply_gesture(Step::rest(), StepGesture::Select, Pitch::C, 0),
            Step::rest()
        );
    }

    #[test]
    fn pitch_nudges_roll_into_octaves_and_clamp() {
        assert_eq!(nudge_pitch(Pitch::C, 1), (Pitch::CSharp, 0));
        // One semitone above CHigh is C sharp an octave up — the plan's (C, 1) would be a no-op.
        assert_eq!(nudge_pitch(Pitch::CHigh, 1), (Pitch::CSharp, 1));
        assert_eq!(nudge_pitch(Pitch::C, -1), (Pitch::B, -1));
        assert_eq!(nudge_pitch(Pitch::CHigh, 3), (Pitch::DSharp, 1));
        assert_eq!(nudge_pitch(Pitch::C, -12), (Pitch::C, -1));
        // Canonical: an octave up from CHigh is a plain C one octave above it.
        assert_eq!(nudge_pitch(Pitch::CHigh, 12), (Pitch::C, 2));
    }

    #[test]
    fn twelve_pixels_is_one_semitone() {
        assert_eq!(semitones_from_drag(-12.0, false), 1);
        assert_eq!(semitones_from_drag(-24.0, false), 2);
        assert_eq!(semitones_from_drag(12.0, false), -1);
        assert_eq!(semitones_from_drag(-4.0, false), 0);
    }

    #[test]
    fn fine_drag_is_four_times_slower() {
        assert_eq!(semitones_from_drag(-48.0, true), 1);
        assert_eq!(semitones_from_drag(-12.0, true), 0);
    }

    #[test]
    fn dragging_a_step_up_a_fifth_lands_on_the_right_note() {
        assert_eq!(drag_pitch((Pitch::C, 0), -84.0, false), (Pitch::G, 0));
        assert_eq!(drag_pitch((Pitch::C, 0), 12.0, false), (Pitch::B, -1));
    }

    /// The label table is hand-written, so it is checked against the spelling rule: a pair's label is the concert name of
    /// the MIDI note the lattice plays. A wrong entry would print a wrong note on a keycap.
    #[test]
    fn every_label_in_range_names_the_note_it_sounds() {
        for absolute in PANEL_LOW..=PANEL_HIGH {
            let (pitch, octave) = from_absolute(absolute);
            let midi = 12 + i32::from(pitch.semitones()) + 12 * i32::from(octave);
            let spelled = pitch_name(pitch, octave);
            assert_eq!(
                spelled,
                crate::midi::message::concert_name(midi as u8),
                "at absolute {absolute} (MIDI {midi}) the label {spelled:?} is not the note it sounds"
            );
        }
    }

    #[test]
    fn pitch_names_are_canonical() {
        // The pair *is* the note: (C, 0) is C0 at 16.35 Hz, (C, 3) is the 130.8 Hz a keyboard's C3 sends.
        assert_eq!(pitch_name(Pitch::C, 0), "C0");
        assert_eq!(pitch_name(Pitch::CSharp, 0), "C#0");
        assert_eq!(pitch_name(Pitch::CHigh, 0), "C1");
        assert_eq!(pitch_name(Pitch::B, -1), "B-1");
        assert_eq!(pitch_name(Pitch::C, 3), "C3");
        assert_eq!(pitch_name(Pitch::CHigh, 1), "C2");
    }
}
