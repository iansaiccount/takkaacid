//! Keyboard dispatch: which presses the panel acts on, and how each one is resolved.
//!
//! The order is the point. The step editor sees a press first, so shift+arrows never reach the transport and
//! pattern shortcuts that act on the same four keys; and a press the step editor refuses while the help card
//! is up is spent, so a modal card cannot be edited behind. Both rules live here rather than in `main.rs`
//! because `main.rs` is a binary and cannot be tested.

use acid_core::bank::PatternBank;
use eframe::egui::{Event, Key};

use super::{PanelState, selection, shortcuts};
use crate::state::SharedState;

/// One press the panel handles: the key, and whether shift was down.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct KeyPress {
    pub key: Key,
    pub shift: bool,
}

/// What the panel did with a press.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Handled {
    /// The selection moved, or the selected step was edited.
    Selection,
    /// A transport, tempo, pattern or help shortcut ran.
    Shortcut(shortcuts::Shortcut),
    /// The step editor's key, refused because the help card is modal.
    Refused,
    /// Nothing in the panel wanted this key.
    Nothing,
}

/// Whether holding the key may repeat its action: only the tempo nudges, where a held key is a sweep. A held
/// Enter would flip a step at frame rate and a held arrow would race through the patterns.
fn repeats_allowed(key: Key) -> bool {
    matches!(key, Key::ArrowUp | Key::ArrowDown)
}

/// Whether the panel has anything to do with this key at all.
pub fn is_panel_key(key: Key, shift: bool) -> bool {
    shortcuts::shortcut_for(key, shift).is_some() || selection::edit_key(key, shift).is_some()
}

/// The presses in this frame's events, in the order egui saw them.
pub fn presses(events: &[Event], shift: bool) -> Vec<KeyPress> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Key {
                key,
                pressed: true,
                repeat,
                ..
            } => {
                // A key repeats only if it is a tempo nudge; everything else acts once per real press.
                let allowed = !*repeat || repeats_allowed(*key);
                (allowed && is_panel_key(*key, shift)).then_some(KeyPress { key: *key, shift })
            }
            _ => None,
        })
        .collect()
}

/// Resolve one press: the step editor first, then the transport and pattern shortcuts.
pub fn handle(
    press: KeyPress,
    panel: &mut PanelState,
    shared: &SharedState,
    bank: &mut PatternBank,
) -> Handled {
    match selection::dispatch(press.key, press.shift, panel, shared) {
        selection::Press::Applied => {
            // An edit makes a pending CLEAR stale: it was armed against a pattern that no longer exists.
            super::clear::disarm(panel);
            return Handled::Selection;
        }
        selection::Press::Refused => return Handled::Refused,
        selection::Press::NotOurs => {}
    }
    let Some(shortcut) = shortcuts::shortcut_for(press.key, press.shift) else {
        return Handled::Nothing;
    };
    shortcuts::apply(shortcut, shared, bank);
    // The help card is panel state, so it is toggled here rather than in `apply`.
    if matches!(shortcut, shortcuts::Shortcut::ToggleHelp) {
        panel.help_visible = !panel.help_visible;
    }
    // The selection names a position in the pattern on screen and the display names the pattern itself, so a
    // new pattern starts at its first step and the display follows the bank.
    if matches!(
        shortcut,
        shortcuts::Shortcut::NextPattern | shortcuts::Shortcut::PrevPattern
    ) {
        panel.selected = 0;
        panel.pattern_id = PatternBank::id_of(bank.current());
        super::clear::disarm(panel);
    }
    Handled::Shortcut(shortcut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use acid_core::pattern::Step;
    use acid_core::pitch::Pitch;
    use eframe::egui::Modifiers;

    fn key_event(key: Key, shift: bool, repeat: bool) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers: Modifiers {
                shift,
                ..Modifiers::default()
            },
        }
    }

    fn lit_shared() -> std::sync::Arc<SharedState> {
        let shared = SharedState::new();
        for index in 0..16 {
            shared.set_step(index, Step::note(Pitch::C, 0));
        }
        shared
    }

    #[test]
    fn shift_right_moves_the_selection_and_never_reaches_the_pattern() {
        // The glue line `main.rs` used to own: shift+right must be spent by the step editor, or the same press
        // also steps the pattern (Review Focus 3).
        let shared = lit_shared();
        let before = *shared.pattern();
        let mut bank = PatternBank::factory();
        let current = bank.current();
        let mut panel = PanelState::default();

        let handled = handle(
            KeyPress {
                key: Key::ArrowRight,
                shift: true,
            },
            &mut panel,
            &shared,
            &mut bank,
        );

        assert_eq!(handled, Handled::Selection);
        assert_eq!(panel.selected, 1);
        assert_eq!(
            *shared.pattern(),
            before,
            "shift+right must not touch the pattern"
        );
        assert_eq!(
            bank.current(),
            current,
            "and must not switch pattern either"
        );
    }

    #[test]
    fn the_pattern_display_follows_the_bank() {
        // The panel prints `pattern_id`, and nothing kept it up to date: the display stayed on A1 while the
        // arrows moved through the bank underneath it.
        let shared = lit_shared();
        let mut bank = PatternBank::factory();
        bank.select(0);
        let mut panel = PanelState::default();

        handle(
            KeyPress {
                key: Key::ArrowRight,
                shift: false,
            },
            &mut panel,
            &shared,
            &mut bank,
        );

        assert_eq!(panel.pattern_id, PatternBank::id_of(bank.current()));
        assert_eq!(
            panel.pattern_id,
            acid_core::pattern::PatternId { bank: 0, number: 1 }
        );
    }

    #[test]
    fn an_unshifted_arrow_steps_the_pattern_and_puts_the_selection_back_at_the_first_step() {
        let shared = lit_shared();
        let mut bank = PatternBank::factory();
        bank.select(0);
        let mut panel = PanelState {
            selected: 7,
            ..PanelState::default()
        };

        let handled = handle(
            KeyPress {
                key: Key::ArrowRight,
                shift: false,
            },
            &mut panel,
            &shared,
            &mut bank,
        );

        assert_eq!(handled, Handled::Shortcut(shortcuts::Shortcut::NextPattern));
        assert_ne!(bank.current(), 0, "the bank moved on");
        assert_eq!(panel.selected, 0, "a new pattern starts at its first step");
        assert_eq!(*shared.pattern(), *bank.current_pattern());
    }

    #[test]
    fn the_card_is_modal_for_the_editing_keys_but_not_for_its_own() {
        let shared = lit_shared();
        let mut bank = PatternBank::factory();
        let before = *shared.pattern();
        let mut panel = PanelState {
            help_visible: true,
            ..PanelState::default()
        };

        assert_eq!(
            handle(
                KeyPress {
                    key: Key::Enter,
                    shift: false,
                },
                &mut panel,
                &shared,
                &mut bank,
            ),
            Handled::Refused
        );
        assert_eq!(
            *shared.pattern(),
            before,
            "enter must not edit behind the card"
        );

        // The keys that close the card and stop the transport must still reach their shortcuts.
        assert_eq!(
            handle(
                KeyPress {
                    key: Key::Space,
                    shift: false,
                },
                &mut panel,
                &shared,
                &mut bank,
            ),
            Handled::Shortcut(shortcuts::Shortcut::PlayStop)
        );
        assert_eq!(
            handle(
                KeyPress {
                    key: Key::Questionmark,
                    shift: true,
                },
                &mut panel,
                &shared,
                &mut bank,
            ),
            Handled::Shortcut(shortcuts::Shortcut::ToggleHelp)
        );
        assert!(!panel.help_visible, "the card must close from the keyboard");
    }

    #[test]
    fn a_key_with_no_meaning_does_nothing() {
        let shared = lit_shared();
        let mut bank = PatternBank::factory();
        let mut panel = PanelState::default();
        assert_eq!(
            handle(
                KeyPress {
                    key: Key::B,
                    shift: false,
                },
                &mut panel,
                &shared,
                &mut bank,
            ),
            Handled::Nothing
        );
    }

    #[test]
    fn a_held_key_repeats_only_for_the_tempo() {
        // A held Enter used to toggle the step at frame rate; a held pattern arrow raced through the bank.
        let events = vec![
            key_event(Key::Enter, false, true),
            key_event(Key::ArrowUp, false, true),
            key_event(Key::ArrowRight, false, true),
        ];
        let pressed: Vec<Key> = presses(&events, false).iter().map(|p| p.key).collect();
        assert_eq!(pressed, vec![Key::ArrowUp], "only the tempo nudge repeats");
    }

    #[test]
    fn only_the_panels_own_keys_are_collected() {
        let events = vec![
            key_event(Key::B, false, false),
            key_event(Key::Enter, false, false),
            key_event(Key::F1, false, false),
        ];
        let pressed: Vec<Key> = presses(&events, false).iter().map(|p| p.key).collect();
        assert_eq!(pressed, vec![Key::Enter, Key::F1], "B is an audition key");
    }

    #[test]
    fn shift_travels_with_the_press_it_arrived_on() {
        let events = vec![key_event(Key::ArrowUp, true, false)];
        assert_eq!(
            presses(&events, true),
            vec![KeyPress {
                key: Key::ArrowUp,
                shift: true
            }]
        );
        // `?` is only the help key with shift, so an unshifted one is not the panel's.
        let events = vec![key_event(Key::Questionmark, false, false)];
        assert!(presses(&events, false).is_empty());
    }
}
