//! The selected step and the edits the keyboard makes to it — pure functions, so the rules are testable
//! without a window.

use acid_core::pattern::{STEPS_PER_PATTERN, Step};
use acid_core::pitch::Pitch;
use eframe::egui::Key;

use super::{PanelState, edit};
use crate::midi::message::is_accent;
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
    // The plain arrows walk the steps, because that is what an arrow does in a step sequencer; shift reaches past
    // the step to the pattern, and shift+up/down still edits the note. (Ruled by the operator: the old pairing put
    // the frequent action behind a modifier.)
    match (key, shift) {
        (Key::ArrowRight, false) => Some(EditKey::SelectNext),
        (Key::ArrowLeft, false) => Some(EditKey::SelectPrev),
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
    /// Ours, but refused: a card is modal, so this press must not also become a shortcut.
    Refused,
}

impl Press {
    /// `true` when the press is spent — applied or refused — so the caller must not pass it on.
    pub fn consumed(self) -> bool {
        !matches!(self, Press::NotOurs)
    }
}

/// Apply one key to the selected step. Both cards cover the panel and are modal, so while either is up the step
/// editor refuses its keys instead of letting them fall through to the transport behind it.
pub fn dispatch(key: Key, shift: bool, panel: &mut PanelState, shared: &SharedState) -> Press {
    let Some(edit) = edit_key(key, shift) else {
        return Press::NotOurs;
    };
    if panel.help_visible || panel.settings_open {
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

/// Write a pitch into the selected step. The border stays put: step mode walks it on the key *release*, so one press is
/// one note and the hand may hold a key for as long as it likes.
///
/// `velocity` is the note's own MIDI velocity: a MIDI keyboard says `Some(..)` and the step takes that level (and its
/// accent, when the hit was hard enough), while a computer key says `None` and the step keeps the velocity and accent it
/// had — typing a pitch should not silently flatten a step that was already shaped.
pub fn place_note(
    panel: &mut PanelState,
    shared: &SharedState,
    pitch: Pitch,
    octave: i8,
    velocity: Option<u8>,
) -> Step {
    let placed = place(panel, shared, Some((pitch, octave)), velocity);
    panel.last_pitch = placed.pitch;
    panel.last_octave = placed.octave_offset;
    placed
}

/// Empty the selected step to a rest, keeping its pitch and its flags so a re-note after a rest is one key away. The
/// border stays put here too; the release walks it.
pub fn rest_step(panel: &mut PanelState, shared: &SharedState) -> Step {
    place(panel, shared, None, None)
}

/// The one writer: `note` writes a pitch (or a rest, when it is `None`), then walks in step mode.
fn place(
    panel: &mut PanelState,
    shared: &SharedState,
    note: Option<(Pitch, i8)>,
    velocity: Option<u8>,
) -> Step {
    let index = panel.selected.min(STEPS_PER_PATTERN - 1);
    let step = shared.pattern().steps[index];
    let (on, pitch, octave_offset) = match note {
        Some((pitch, octave)) => (true, pitch, octave),
        None => (false, step.pitch, step.octave_offset),
    };
    let placed = Step {
        on,
        pitch,
        octave_offset,
        velocity: velocity.unwrap_or(step.velocity),
        accent: velocity.map(is_accent).unwrap_or(step.accent),
        ..step
    };
    shared.set_step(index, placed);
    panel.pattern = shared.pattern();
    panel.dirty = true;
    placed
}

/// Walk the border to the next step, wrapping after the last one: step mode does this on a key *release*, so one press
/// writes one step and nothing is written ahead of the hand.
pub fn walk(panel: &mut PanelState) {
    panel.selected = (panel.selected.min(STEPS_PER_PATTERN - 1) + 1) % STEPS_PER_PATTERN;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plain_arrows_walk_the_steps_and_shift_still_edits_the_note() {
        // The plain arrows are the step sequencer's; shift reaches past the step to the pattern (see `shortcuts`)
        // and keeps the note edit for up/down.
        assert_eq!(edit_key(Key::ArrowRight, false), Some(EditKey::SelectNext));
        assert_eq!(edit_key(Key::ArrowLeft, false), Some(EditKey::SelectPrev));
        assert_eq!(edit_key(Key::ArrowUp, true), Some(EditKey::NoteUp));
        assert_eq!(edit_key(Key::ArrowDown, true), Some(EditKey::NoteDown));
        assert_eq!(edit_key(Key::Enter, false), Some(EditKey::ToggleSelected));
        // Shift+left/right belongs to the pattern shortcut; this module must not swallow it.
        assert_eq!(edit_key(Key::ArrowRight, true), None);
        assert_eq!(edit_key(Key::ArrowLeft, true), None);
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
            (Key::ArrowRight, false),
            (Key::ArrowUp, true),
        ] {
            assert_eq!(
                dispatch(key, shift, &mut panel, &shared),
                Press::Refused,
                "{key:?}"
            );
        }
        // The refusal must come *after* `edit_key`: a key this module does not own has to stay `NotOurs`,
        // or the card could not be closed with the keyboard it is refusing.
        for (key, shift) in [(Key::Space, false), (Key::Questionmark, true)] {
            assert_eq!(
                dispatch(key, shift, &mut panel, &shared),
                Press::NotOurs,
                "{key:?} must still reach the shortcuts while the card is up"
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
    fn a_press_with_the_card_away_is_ours_and_applied() {
        let shared = crate::state::SharedState::new();
        shared.set_step(0, Step::note(Pitch::E, 0));
        let mut panel = PanelState::default();

        assert_eq!(
            dispatch(Key::ArrowRight, false, &mut panel, &shared),
            Press::Applied
        );
        assert_eq!(panel.selected, 1);
        assert_eq!(
            dispatch(Key::ArrowRight, true, &mut panel, &shared),
            Press::NotOurs,
            "shift+right belongs to the pattern shortcut"
        );
    }

    #[test]
    fn a_hard_note_accents_the_step_and_a_soft_one_does_not() {
        let shared = crate::state::SharedState::new();
        let mut panel = PanelState::default();

        let soft = place_note(&mut panel, &shared, Pitch::E, 0, Some(40));
        assert!(!soft.accent);
        assert_eq!(soft.velocity, 40, "a soft note keeps its own level");

        let hard = place_note(&mut panel, &shared, Pitch::E, 0, Some(110));
        assert!(hard.accent, "a hard hit accents the step it lands on");
        assert_eq!(hard.velocity, 110);
        assert_eq!(shared.pattern().steps[0].velocity, 110);
    }

    #[test]
    fn a_computer_key_keeps_the_shape_the_step_already_had() {
        // No velocity comes with a computer key, so it must not silently unaccent an accented step, accent a soft one,
        // or reset either level — `None` means "leave those fields alone".
        let shared = crate::state::SharedState::new();
        let mut panel = PanelState::default();
        shared.set_step(
            0,
            Step {
                accent: true,
                velocity: 96,
                ..Step::note(Pitch::C, 0)
            },
        );

        let kept = place_note(&mut panel, &shared, Pitch::E, 0, None);
        assert!(kept.accent, "the accent survived a typed pitch");
        assert_eq!(kept.velocity, 96, "and so did its level");
        assert_eq!(kept.pitch, Pitch::E);

        shared.set_step(1, Step::note(Pitch::C, 0));
        panel.selected = 1;
        let still_soft = place_note(&mut panel, &shared, Pitch::E, 0, None);
        assert!(!still_soft.accent);
        assert_eq!(still_soft.velocity, 127, "the untouched default level");
    }

    #[test]
    fn a_rest_keeps_the_pitch_and_the_flags() {
        let shared = crate::state::SharedState::new();
        shared.set_step(
            15,
            Step {
                accent: true,
                slide: true,
                ..Step::note(Pitch::G, 1)
            },
        );
        let mut panel = PanelState {
            selected: 15,
            step_mode: true,
            ..PanelState::default()
        };

        let rested = rest_step(&mut panel, &shared);

        assert!(!rested.on, "the step is silent");
        assert_eq!(
            rested.pitch,
            Pitch::G,
            "its pitch is waiting for the next note"
        );
        assert_eq!(rested.octave_offset, 1);
        assert!(rested.accent && rested.slide, "the flags are untouched");
        assert!(!shared.pattern().steps[15].on);
        assert_eq!(
            panel.selected, 15,
            "the border does not move on the press: the release walks it"
        );
    }

    #[test]
    fn walking_wraps_after_the_last_step() {
        let mut panel = PanelState {
            selected: 15,
            ..PanelState::default()
        };
        walk(&mut panel);
        assert_eq!(panel.selected, 0, "after step 16 comes step 1");
        walk(&mut panel);
        assert_eq!(panel.selected, 1);

        panel.selected = 99;
        walk(&mut panel);
        assert_eq!(panel.selected, 0, "a stale border is pulled back in first");
    }

    #[test]
    fn a_rest_without_step_mode_leaves_the_border_where_it_is() {
        let shared = crate::state::SharedState::new();
        shared.set_step(4, Step::note(Pitch::A, 0));
        let mut panel = PanelState {
            selected: 4,
            ..PanelState::default()
        };

        rest_step(&mut panel, &shared);

        assert!(!shared.pattern().steps[4].on);
        assert_eq!(panel.selected, 4, "no step mode, no walk");
    }
}
