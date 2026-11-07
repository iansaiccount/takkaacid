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

/// What a press turned out to be, so the caller knows whether the transport shortcuts may still see it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Press {
    /// Not a step-editor key: the caller's shortcuts handle it.
    NotOurs,
    /// Applied to the selection or to the pattern.
    Applied,
    /// Ours, but refused: the help card is modal, so this press must not also become a shortcut.
    Refused,
}

impl Press {
    /// `true` when the press is spent — applied or refused — so the caller must not pass it on.
    pub fn consumed(self) -> bool {
        !matches!(self, Press::NotOurs)
    }
}

/// Apply one key to the selected step. The card covers the panel and is modal, so while it is up the step
/// editor refuses its keys instead of letting them fall through to the transport behind it.
pub fn dispatch(key: Key, shift: bool, panel: &mut PanelState, shared: &SharedState) -> Press {
    let Some(edit) = edit_key(key, shift) else {
        return Press::NotOurs;
    };
    if panel.help_visible {
        return Press::Refused;
    }
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
    Press::Applied
}

pub fn move_selection(current: usize, delta: i8, count: usize) -> usize {
    let last = count.saturating_sub(1) as i32;
    let moved = current as i32 + i32::from(delta);
    moved.clamp(0, last) as usize
}

pub fn nudge_step(step: Step, semitones: i8, last_pitch: Pitch, last_octave: i8) -> Step {
    // A rest that still differs from `Step::rest()` remembers its note, so a nudge moves that note. A bare
    // rest remembers nothing and comes in at the last pitch used, transposed — the same rule as the panel's
    // `enable` closure.
    let start = if step.on || step != Step::rest() {
        (step.pitch, step.octave_offset)
    } else {
        (last_pitch, last_octave)
    };
    let (pitch, octave_offset) = edit::transpose(start.0, start.1, semitones);
    Step {
        on: true,
        pitch,
        octave_offset,
        ..step
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

    #[test]
    fn nudging_a_rested_note_moves_the_note_it_remembers() {
        let rested = Step {
            on: false,
            ..Step::note(Pitch::E, 0)
        };
        let up = nudge_step(rested, 1, Pitch::A, 0);
        assert!(up.on);
        assert_eq!((up.pitch, up.octave_offset), (Pitch::F, 0));
    }

    #[test]
    fn the_help_card_refuses_the_editing_keys_instead_of_passing_them_on() {
        // Review finding 2: the card is modal for the pointer, so it must be modal for the keyboard too.
        // A refused press is swallowed rather than handed to the shortcuts, or shift+right would quietly
        // change the pattern behind the card.
        let shared = crate::state::SharedState::new();
        let before = shared.pattern();
        let mut panel = PanelState {
            help_visible: true,
            ..PanelState::default()
        };
        for (key, shift) in [
            (Key::Enter, false),
            (Key::ArrowRight, true),
            (Key::ArrowUp, true),
        ] {
            assert_eq!(
                dispatch(key, shift, &mut panel, &shared),
                Press::Refused,
                "{key:?}"
            );
        }
        assert_eq!(
            panel.selected, 0,
            "a refused press must not move the border"
        );
        assert_eq!(shared.pattern(), before, "a refused press must not edit");
        assert!(!panel.dirty, "a refused press must not schedule a save");
    }

    #[test]
    fn a_press_with_the_card_down_is_ours_and_applied() {
        let shared = crate::state::SharedState::new();
        shared.set_step(0, Step::note(Pitch::E, 0));
        let mut panel = PanelState::default();

        assert_eq!(
            dispatch(Key::ArrowRight, true, &mut panel, &shared),
            Press::Applied
        );
        assert_eq!(panel.selected, 1);
        assert_eq!(
            dispatch(Key::ArrowRight, false, &mut panel, &shared),
            Press::NotOurs,
            "an unshifted arrow still belongs to the pattern shortcut"
        );
    }
}
