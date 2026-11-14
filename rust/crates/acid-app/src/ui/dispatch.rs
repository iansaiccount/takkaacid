//! Keyboard dispatch: which presses the panel acts on, and how each one is resolved.
//!
//! The order is the point. The step editor sees a press first, so shift+arrows never reach the transport and
//! pattern shortcuts that act on the same four keys; and a press the step editor refuses while the help card
//! is up is spent, so a modal card cannot be edited behind. Both rules live here rather than in `main.rs`
//! because `main.rs` is a binary and cannot be tested.

use acid_core::bank::PatternBank;
use eframe::egui::{Event, Key};

use super::{PanelState, learn, selection, shortcuts, transport};
use crate::device::MAX_DEVICES;
use crate::state::SharedState;

/// One press the panel handles: the key, and which of the modifiers it cares about were down.
///
/// Two flags rather than a modifier enum: shift and ctrl mean different things to different keys — shift+arrow moves
/// through the patterns, ctrl+digit picks a device — and a combination that means nothing today should not need a decision
/// until something asks for it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct KeyPress {
    pub key: Key,
    pub shift: bool,
    pub ctrl: bool,
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
    /// The rack: `Ctrl` and a digit asked for one of the devices, and `main` answers it after the frame.
    Rack,
}

/// Whether holding the key may repeat its action: only the tempo nudges, where a held key is a sweep. A held
/// Enter would flip a step at frame rate and a held arrow would race through the patterns.
fn repeats_allowed(key: Key) -> bool {
    matches!(key, Key::ArrowUp | Key::ArrowDown)
}

/// Whether the panel has anything to do with this key at all.
pub fn is_panel_key(key: Key, shift: bool, ctrl: bool) -> bool {
    (ctrl && device_key(key).is_some())
        || shortcuts::shortcut_for(key, shift).is_some()
        || selection::edit_key(key, shift).is_some()
}

/// The keys `Ctrl` and a digit use, in device order.
///
/// `MAX_DEVICES` long, so a rack that grows past four fails to compile here instead of leaving `Ctrl+5` silently dead and
/// the strip drawing a fifth cell nothing can select.
const DEVICE_DIGITS: [Key; MAX_DEVICES] = [Key::Num1, Key::Num2, Key::Num3, Key::Num4];

/// The device `Ctrl` and a digit names: the cell's index, and nothing else is a device key.
///
/// `Ctrl` is the modifier because `1..4` on their own must stay available to the instrument: a step shortcut that lands on
/// a digit later should not have to argue with this.
pub fn device_key(key: Key) -> Option<usize> {
    DEVICE_DIGITS.iter().position(|digit| *digit == key)
}

/// The presses in this frame's events, in the order egui saw them.
///
/// The modifiers come from the *event*, not from the frame's input state. The event carries the modifier that was down at
/// the moment of the press, so a key pressed in the same frame its modifier went down is still that key with that modifier;
/// filtering on `input.modifiers` dropped a ctrl+digit that arrived inside one frame, which made `Ctrl+1..4` dead for
/// anything faster than a hand (`ui::keys` documents the same rule for the audition keys). A headless run found it.
pub fn presses(events: &[Event]) -> Vec<KeyPress> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers,
                ..
            } => {
                // A key repeats only if it is a tempo nudge; everything else acts once per real press.
                let allowed = !*repeat || repeats_allowed(*key);
                (allowed && is_panel_key(*key, modifiers.shift, modifiers.ctrl)).then_some(
                    KeyPress {
                        key: *key,
                        shift: modifiers.shift,
                        ctrl: modifiers.ctrl,
                    },
                )
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
    // A ctrl digit picks the device the panel shows, and is answered before the step editor: it is not a step edit. The
    // order is a decision, written down in the plan, so a step shortcut that ever lands on a digit has to argue with it.
    if press.ctrl
        && let Some(index) = device_key(press.key)
    {
        // A switch changes what the panel is showing, so a card about the device it was showing goes with it: the card is
        // a reading of one device, and a reading of the one you just left is a card about nothing.
        panel.close_device_card();
        panel.device_request = Some(crate::device::Request::Select(index));
        return Handled::Rack;
    }
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
    // Three shortcuts are the transport's, and the transport is the rack's: they ask for a change rather than writing one
    // device's atomic, which the next fan-out would undo.
    let transport = match shortcut {
        shortcuts::Shortcut::PlayStop => Some(if shared.is_playing() {
            transport::Request::Stop
        } else {
            transport::Request::Play
        }),
        shortcuts::Shortcut::TempoUp => Some(transport::Request::NudgeTempo(1)),
        shortcuts::Shortcut::TempoDown => Some(transport::Request::NudgeTempo(-1)),
        shortcuts::Shortcut::NextPattern
        | shortcuts::Shortcut::PrevPattern
        | shortcuts::Shortcut::ToggleSettings
        | shortcuts::Shortcut::ToggleHelp
        | shortcuts::Shortcut::ToggleDeviceCard => None,
    };
    if let Some(request) = transport {
        panel.transport_request = Some(request);
        return Handled::Shortcut(shortcut);
    }
    shortcuts::apply(shortcut, shared, bank);
    // The settings card is panel state too, and there is room for one card: opening it clears the other three. The device
    // card goes the same way it came — through `close_device_card`, so its mirror cannot outlive it — and the rule is
    // symmetric because a one-way rule is a state two keystrokes away, not a corner.
    if matches!(shortcut, shortcuts::Shortcut::ToggleSettings) {
        panel.settings_open = !panel.settings_open;
        if panel.settings_open {
            panel.help_visible = false;
            panel.songs.card = None;
            panel.close_device_card();
            panel.device_picker.open = false;
        }
    }
    // The help card is panel state, so it is toggled here rather than in `apply` — and it follows the same one-card rule
    // as F2 in the other direction: opening help puts the settings card away, or help would paint over a card that is
    // still open and Escape would close the one nobody can see first.
    if matches!(shortcut, shortcuts::Shortcut::ToggleHelp) {
        panel.help_visible = !panel.help_visible;
        if panel.help_visible {
            panel.settings_open = false;
            panel.close_device_card();
            panel.device_picker.open = false;
        }
    }
    // The device card is the topmost of the four inside the panel, so it is the second thing Escape gives up on and the
    // one F3 toggles. Opening it puts the other four away, because there is room for one card.
    if matches!(shortcut, shortcuts::Shortcut::ToggleDeviceCard) {
        if panel.device_card.open.is_some() {
            panel.close_device_card();
        } else {
            let selected = panel.strip.selected;
            panel.open_device_card(selected);
        }
    }
    // The selection names a position in the pattern on screen and the display names the pattern itself, so a new pattern
    // starts at its first step: one helper, shared with a learned controller's swap and a song load.
    if matches!(
        shortcut,
        shortcuts::Shortcut::NextPattern | shortcuts::Shortcut::PrevPattern
    ) {
        super::pattern_swap::adopt(panel, shared, bank);
    }
    Handled::Shortcut(shortcut)
}

/// Escape gives up on one thing at a time, in the order the cards are painted: the picker (over everything), then the device
/// card, then the settings card, then help, then the songs card, then a learn in progress. Returns whether there was anything
/// to close.
///
/// The topmost card first is the rule; the songs card is the exception already — it is painted over help but closed after
/// it — and the device card is the one card that needed the rule to hold, because it is the one a hand opens by accident.
/// It lives here rather than in `main` because the order is the behaviour, and `main` is a binary no test can call.
pub fn escape(panel: &mut PanelState, shared: &SharedState) -> bool {
    // The picker is painted last, so it is the first thing Escape gives up on; then the device card under it.
    if panel.device_picker.open {
        panel.device_picker.open = false;
        return true;
    }
    if panel.device_card.open.is_some() {
        panel.close_device_card();
        return true;
    }
    if panel.settings_open {
        panel.settings_open = false;
        return true;
    }
    if panel.help_visible {
        panel.help_visible = false;
        return true;
    }
    if panel.songs.card.take().is_some() {
        return true;
    }
    if panel.learn != learn::Step::Idle {
        // Escape gives up on a learn in progress; the shared flag must fall with it, or the controller stays bound.
        panel.learn = learn::Step::Idle;
        learn::cancel(shared);
        return true;
    }
    false
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
    fn a_plain_arrow_moves_the_selection_and_leaves_the_pattern_alone() {
        // The plain arrows belong to the step sequencer: the same press must never also step the pattern.
        let shared = lit_shared();
        let before = *shared.pattern();
        let mut bank = PatternBank::factory();
        let current = bank.current();
        let mut panel = PanelState::default();
        let handled = handle(
            KeyPress {
                key: Key::ArrowRight,
                shift: false,
                ctrl: false,
            },
            &mut panel,
            &shared,
            &mut bank,
        );

        assert_eq!(handled, Handled::Selection);
        assert_eq!(panel.selected, 1);
        assert_eq!(*shared.pattern(), before, "a plain arrow must not edit");
        assert_eq!(
            bank.current(),
            current,
            "and must not switch pattern either"
        );
    }

    #[test]
    fn shift_right_steps_the_pattern_and_leaves_the_selection_alone() {
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
                shift: true,
                ctrl: false,
            },
            &mut panel,
            &shared,
            &mut bank,
        );

        assert_eq!(
            handled,
            Handled::Shortcut(shortcuts::Shortcut::NextPattern),
            "shift reaches past the step to the pattern"
        );
        assert_ne!(bank.current(), 0, "the bank moved on");
        assert_eq!(panel.selected, 0, "a new pattern starts at its first step");
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
                shift: true,
                ctrl: false,
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
                    ctrl: false,
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
                    ctrl: false,
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
                    ctrl: false,
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
                    ctrl: false,
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
        let pressed: Vec<Key> = presses(&events).iter().map(|p| p.key).collect();
        assert_eq!(pressed, vec![Key::ArrowUp], "only the tempo nudge repeats");
    }

    #[test]
    fn only_the_panels_own_keys_are_collected() {
        let events = vec![
            key_event(Key::B, false, false),
            key_event(Key::Enter, false, false),
            key_event(Key::F1, false, false),
        ];
        let pressed: Vec<Key> = presses(&events).iter().map(|p| p.key).collect();
        assert_eq!(pressed, vec![Key::Enter, Key::F1], "B is an audition key");
    }

    #[test]
    fn shift_travels_with_the_press_it_arrived_on() {
        let events = vec![key_event(Key::ArrowUp, true, false)];
        assert_eq!(
            presses(&events),
            vec![KeyPress {
                key: Key::ArrowUp,
                shift: true,
                ctrl: false
            }]
        );
        // `?` is only the help key with shift, so an unshifted one is not the panel's.
        let events = vec![key_event(Key::Questionmark, false, false)];
        assert!(presses(&events).is_empty());
    }

    #[test]
    fn the_settings_card_refuses_the_editing_keys_too() {
        // The card is modal for the pointer, so it is modal for the keyboard as well: Enter behind it would edit a
        // step the player cannot see, and mark the session unsaved for it. Help's rule, applied to the other card.
        let shared = lit_shared();
        let before = *shared.pattern();
        let mut bank = PatternBank::factory();
        let mut panel = PanelState {
            settings_open: true,
            ..PanelState::default()
        };

        for (key, shift) in [
            (Key::Enter, false),
            (Key::ArrowRight, false),
            (Key::ArrowUp, true),
        ] {
            assert_eq!(
                handle(
                    KeyPress {
                        key,
                        shift,
                        ctrl: false
                    },
                    &mut panel,
                    &shared,
                    &mut bank
                ),
                Handled::Refused,
                "{key:?}"
            );
        }
        assert_eq!(*shared.pattern(), before, "nothing may be edited behind it");
        assert!(!panel.dirty, "and nothing new to save");

        // The keys that leave the card must still reach their shortcuts.
        assert_eq!(
            handle(
                KeyPress {
                    key: Key::Space,
                    shift: false,
                    ctrl: false
                },
                &mut panel,
                &shared,
                &mut bank
            ),
            Handled::Shortcut(shortcuts::Shortcut::PlayStop)
        );
    }

    #[test]
    fn ctrl_and_a_digit_asks_for_a_device() {
        // The table is `MAX_DEVICES` long, so a rack that grows needs a digit named before it compiles.
        assert_eq!(device_key(DEVICE_DIGITS[0]), Some(0));
        assert_eq!(
            device_key(DEVICE_DIGITS[MAX_DEVICES - 1]),
            Some(MAX_DEVICES - 1)
        );
        assert_eq!(device_key(Key::Num5), None, "not every key is a device");
        assert_eq!(device_key(Key::ArrowUp), None);
    }

    #[test]
    fn a_ctrl_digit_is_a_panel_key_and_a_bare_digit_is_not() {
        assert!(is_panel_key(Key::Num2, false, true));
        assert!(!is_panel_key(Key::Num2, false, false));
        assert!(
            !is_panel_key(Key::Num5, false, true),
            "a digit past the last device is not the panel's"
        );
    }

    #[test]
    fn a_ctrl_digit_asks_the_rack_to_select_that_device() {
        // The whole path, not just the table: `presses` must collect the press and `handle` must answer it before the step
        // editor does. Delete the branch in `handle` and this fails while the two tests above still pass.
        let events = vec![Event::Key {
            key: Key::Num3,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::NONE
            },
        }];
        let pressed = presses(&events);
        assert_eq!(pressed.len(), 1, "a ctrl digit is the panel's");
        assert!(pressed[0].ctrl, "and the ctrl travels with the press");

        let mut panel = PanelState::default();
        assert_eq!(
            handle(
                pressed[0],
                &mut panel,
                &lit_shared(),
                &mut PatternBank::factory()
            ),
            Handled::Rack
        );
        assert_eq!(
            panel.device_request,
            Some(crate::device::Request::Select(2)),
            "the third digit is the third device"
        );
    }
}
