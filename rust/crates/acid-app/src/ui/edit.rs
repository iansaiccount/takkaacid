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
    Toggle,
    Accent,
    Slide,
    Clear,
}

/// Apply one gesture. `last_pitch`/`last_octave` are the pitch a newly-enabled step should take.
///
/// Resting a step keeps its pitch and flags, so toggling it back restores exactly what was there. A flag
/// gesture forces the step on — a lit accent with no note would be invisible on the panel.
pub fn apply_gesture(step: Step, gesture: StepGesture, last_pitch: Pitch, last_octave: i8) -> Step {
    let enable = |mut step: Step| -> Step {
        if !step.on {
            step.on = true;
            step.pitch = last_pitch;
            step.octave_offset = last_octave;
        }
        step
    };
    match gesture {
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

/// Which gesture an input maps to. Double-click clears; otherwise a modifier beats the plain click.
pub fn gesture_from_input(left: bool, shift: bool, alt: bool, double: bool) -> Option<StepGesture> {
    if double {
        Some(StepGesture::Clear)
    } else if shift {
        Some(StepGesture::Accent)
    } else if alt {
        Some(StepGesture::Slide)
    } else if left {
        Some(StepGesture::Toggle)
    } else {
        None
    }
}

/// Pixels of vertical drag per semitone at 1× scale.
pub const PITCH_DRAG_PIXELS_PER_SEMITONE: f32 = 12.0;

/// Canonical `(note, octave)` for an absolute semitone, clamped to the panel's `-12..=24`.
fn from_absolute(absolute: i32) -> (Pitch, i8) {
    let moved = absolute.clamp(-12, 24);
    // `24` is two octaves up, but the panel only carries `-1..=1`, so the top clamps to octave 1.
    let octave = moved.div_euclid(12).clamp(-1, 1);
    let index = moved.rem_euclid(12) as usize;
    (Pitch::ALL[index], octave as i8)
}

/// Move a pitch by whole semitones from `(pitch, octave)`, rolling into the octave and clamping to
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

/// The panel's spelling of a pitch: note name plus octave, always canonical.
pub fn pitch_name(pitch: Pitch, octave: i8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let (pitch, octave) = from_absolute(i32::from(pitch.semitones()) + 12 * i32::from(octave));
    format!("{}{octave}", NAMES[pitch.semitones() as usize % 12])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transposition_respects_the_saved_octave() {
        assert_eq!(transpose(Pitch::C, 1, 1), (Pitch::CSharp, 1));
        assert_eq!(transpose(Pitch::C, 1, -12), (Pitch::C, 0));
        assert_eq!(transpose(Pitch::B, -1, 1), (Pitch::C, 0));
        // The clamp is the panel's, not the octave's: two octaves up is the ceiling.
        assert_eq!(transpose(Pitch::C, 1, 12), (Pitch::C, 1));
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
            gesture_from_input(true, false, false, false),
            Some(StepGesture::Toggle)
        );
        assert_eq!(
            gesture_from_input(true, true, false, false),
            Some(StepGesture::Accent)
        );
        assert_eq!(
            gesture_from_input(true, false, true, false),
            Some(StepGesture::Slide)
        );
        assert_eq!(
            gesture_from_input(true, true, true, true),
            Some(StepGesture::Clear)
        );
        assert_eq!(gesture_from_input(false, false, false, false), None);
    }

    #[test]
    fn pitch_nudges_roll_into_octaves_and_clamp() {
        assert_eq!(nudge_pitch(Pitch::C, 1), (Pitch::CSharp, 0));
        // One semitone above CHigh is C sharp an octave up — the plan's (C, 1) would be a no-op.
        assert_eq!(nudge_pitch(Pitch::CHigh, 1), (Pitch::CSharp, 1));
        assert_eq!(nudge_pitch(Pitch::C, -1), (Pitch::B, -1));
        assert_eq!(nudge_pitch(Pitch::CHigh, 3), (Pitch::DSharp, 1));
        assert_eq!(nudge_pitch(Pitch::C, -12), (Pitch::C, -1));
        // Same pitch the plan spelled `(CHigh, 1)`; the canonical spelling is an octave and a plain note.
        assert_eq!(nudge_pitch(Pitch::CHigh, 12), (Pitch::C, 1));
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

    #[test]
    fn pitch_names_are_canonical() {
        assert_eq!(pitch_name(Pitch::C, 0), "C0");
        assert_eq!(pitch_name(Pitch::CSharp, 0), "C#0");
        assert_eq!(pitch_name(Pitch::CHigh, 0), "C1");
        assert_eq!(pitch_name(Pitch::B, -1), "B-1");
    }
}
