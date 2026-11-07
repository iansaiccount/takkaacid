//! The selected step and the edits the keyboard makes to it — pure functions, so the rules are testable
//! without a window.

use acid_core::pattern::{STEPS_PER_PATTERN, Step};
use acid_core::pitch::Pitch;
use eframe::egui::Key;

use super::{PanelState, edit};
use crate::state::SharedState;

/// What a key does to the selected step.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EditKey {
    NoteUp,
    NoteDown,
    SelectPrev,
    SelectNext,
    ToggleSelected,
}

/// Shift turns the arrows into editing keys; enter toggles. Unshifted arrows keep tempo and pattern.
pub fn edit_key(key: Key, shift: bool) -> Option<EditKey> {
    match (key, shift) {
        (Key::ArrowRight, true) => Some(EditKey::SelectNext),
        (Key::ArrowLeft, true) => Some(EditKey::SelectPrev),
        (Key::ArrowUp, true) => Some(EditKey::NoteUp),
        (Key::ArrowDown, true) => Some(EditKey::NoteDown),
        (Key::Enter, _) => Some(EditKey::ToggleSelected),
        _ => None,
    }
}

/// Apply one key to the selected step. Returns `true` when the key was this module's to consume, so the
/// caller can skip the transport and pattern shortcuts.
pub fn dispatch(key: Key, shift: bool, panel: &mut PanelState, shared: &SharedState) -> bool {
    let Some(edit) = edit_key(key, shift) else {
        return false;
    };
    let index = panel.selected.min(STEPS_PER_PATTERN - 1);
    match edit {
        EditKey::SelectPrev => panel.selected = move_selection(index, -1, STEPS_PER_PATTERN),
        EditKey::SelectNext => panel.selected = move_selection(index, 1, STEPS_PER_PATTERN),
        EditKey::NoteUp | EditKey::NoteDown => {
            let semitones = if matches!(edit, EditKey::NoteUp) {
                1
            } else {
                -1
            };
            let step = shared.pattern().steps[index];
            let nudged = nudge_step(step, semitones, panel.last_pitch, panel.last_octave);
            panel.last_pitch = nudged.pitch;
            panel.last_octave = nudged.octave_offset;
            shared.set_step(index, nudged);
            panel.pattern = shared.pattern();
            panel.dirty = true;
        }
        EditKey::ToggleSelected => {
            let step = shared.pattern().steps[index];
            let toggled = edit::apply_gesture(
                step,
                edit::StepGesture::Toggle,
                panel.last_pitch,
                panel.last_octave,
            );
            if toggled.on {
                panel.last_pitch = toggled.pitch;
                panel.last_octave = toggled.octave_offset;
            }
            shared.set_step(index, toggled);
            panel.pattern = shared.pattern();
            panel.dirty = true;
        }
    }
    true
}

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
    fn shift_turns_the_arrows_into_editing_keys() {
        assert_eq!(edit_key(Key::ArrowRight, true), Some(EditKey::SelectNext));
        assert_eq!(edit_key(Key::ArrowLeft, true), Some(EditKey::SelectPrev));
        assert_eq!(edit_key(Key::ArrowUp, true), Some(EditKey::NoteUp));
        assert_eq!(edit_key(Key::ArrowDown, true), Some(EditKey::NoteDown));
        assert_eq!(edit_key(Key::Enter, false), Some(EditKey::ToggleSelected));
        // Unshifted arrows belong to tempo and pattern; they must not be swallowed here.
        assert_eq!(edit_key(Key::ArrowRight, false), None);
        assert_eq!(edit_key(Key::ArrowUp, false), None);
    }

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
