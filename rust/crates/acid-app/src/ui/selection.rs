//! The selected step and the edits the keyboard makes to it — pure functions, so the rules are testable
//! without a window.

use acid_core::pattern::Step;
use acid_core::pitch::Pitch;

use super::edit;

pub fn move_selection(current: usize, delta: i8, count: usize) -> usize {
    let last = count.saturating_sub(1) as i32;
    let moved = current as i32 + i32::from(delta);
    moved.clamp(0, last) as usize
}

pub fn nudge_step(step: Step, semitones: i8, last_pitch: Pitch, last_octave: i8) -> Step {
    if step.on {
        let (pitch, octave_offset) = edit::transpose(step.pitch, step.octave_offset, semitones);
        Step {
            pitch,
            octave_offset,
            ..step
        }
    } else {
        let (pitch, octave_offset) = edit::transpose(last_pitch, last_octave, semitones);
        Step {
            on: true,
            pitch,
            octave_offset,
            ..step
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selection_moves_and_clamps_at_both_ends() {
        assert_eq!(move_selection(0, 1, 16), 1);
        assert_eq!(move_selection(15, 1, 16), 15);
        assert_eq!(move_selection(0, -1, 16), 0);
        assert_eq!(move_selection(3, -2, 16), 1);
    }

    #[test]
    fn nudging_a_note_moves_it_a_semitone_and_keeps_its_flags() {
        let step = Step {
            accent: true,
            slide: true,
            ..Step::note(Pitch::C, 1)
        };
        let up = nudge_step(step, 1, Pitch::A, 0);
        assert_eq!((up.pitch, up.octave_offset), (Pitch::CSharp, 1));
        assert!(up.on && up.accent && up.slide);
    }

    #[test]
    fn nudging_a_rest_takes_the_last_pitch_already_moved() {
        let rest = nudge_step(Step::rest(), 1, Pitch::A, 0);
        assert!(rest.on);
        assert_eq!((rest.pitch, rest.octave_offset), (Pitch::ASharp, 0));
    }
}
