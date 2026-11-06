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

/// Move a pitch by whole semitones, rolling into `octave_offset` and clamping at `-12..=24` absolute.
///
/// The result is always canonical: an index in `0..=11` (plain `C`..`B`) plus an octave, so `CHigh` is
/// spelled `(C, 1)` on the way out.
pub fn nudge_pitch(pitch: Pitch, semitones: i8) -> (Pitch, i8) {
    const LOWEST: i32 = -12;
    const HIGHEST: i32 = 24;
    let absolute = i32::from(pitch.semitones()).clamp(LOWEST, HIGHEST);
    let moved = (absolute + i32::from(semitones)).clamp(LOWEST, HIGHEST);
    let octave = moved.div_euclid(12).clamp(-1, 1);
    let index = moved.rem_euclid(12) as usize;
    (Pitch::ALL[index], octave as i8)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(apply_gesture(s, StepGesture::Clear, Pitch::C, 0), Step::rest());
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
}
