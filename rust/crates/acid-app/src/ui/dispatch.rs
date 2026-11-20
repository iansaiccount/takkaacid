//! Keyboard dispatch: which presses the panel acts on, and how each one is resolved.
//!
//! The order is the point. The step editor sees a press first, so shift+arrows never reach the transport and
//! pattern shortcuts that act on the same four keys; and a press the step editor refuses while the help card
//! is up is spent, so a modal card cannot be edited behind. Both rules live here rather than in `main.rs`
//! because `main.rs` is a binary and cannot be tested.

use acid_core::bank::PatternBank;
use acid_core::drums::Drum;
use eframe::egui::{Event, Key};

use super::{PanelState, drum_panel, kits, learn, selection, shortcuts, tape, transport};
use crate::device::{DeviceKind, MAX_DEVICES};
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
    /// A face's own key, refused because a card is modal: the step editor's, or the drum face's voice walk.
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
        // A drum face's walk is the same two keys as the 303's note nudge, but it is `handle`'s drum arm that acts on them;
        // asking the face's own mapping here says the press is the panel's for its own reason, so a change to the acid
        // editor's keys cannot silently take the drums' away.
        || drum_panel::voice_move(key, shift).is_some()
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
    events.iter().filter_map(as_press).collect()
}

/// The press one event carries, if it is one the panel acts on: a key going down, that the panel has a use for, and
/// either the real press or a repeat the key is allowed.
fn as_press(event: &Event) -> Option<KeyPress> {
    match event {
        Event::Key {
            key,
            pressed: true,
            repeat,
            modifiers,
            ..
        } => {
            // A key repeats only if it is a tempo nudge; everything else acts once per real press.
            let allowed = !*repeat || repeats_allowed(*key);
            (allowed && is_panel_key(*key, modifiers.shift, modifiers.ctrl)).then_some(KeyPress {
                key: *key,
                shift: modifiers.shift,
                ctrl: modifiers.ctrl,
            })
        }
        _ => None,
    }
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
    // A drum device's editor is the grid, not the keyboard: the step editor writes 303 pitches into a pattern and moves a
    // border the drum face has no row to show. What that face does own is its selection, and `shift` and the arrows walk it
    // up and down the grid, hearing each voice it lands on — the preview a click on the voice's label already gives. The walk
    // is a selection and not an edit, so a pending CLEAR is left alone: the pattern it would clear has not moved.
    //
    // The transport and pattern shortcuts below are both machines' — one drum voice does not make the deck another deck — so
    // only a face's own keys are decided here, and the face's rows are gone rather than every key.
    match panel.showing_kind() {
        DeviceKind::Acid => match selection::dispatch(press.key, press.shift, panel, shared) {
            selection::Press::Applied => {
                // An edit makes a pending CLEAR stale: it was armed against a pattern that no longer exists.
                super::clear::disarm(panel);
                return Handled::Selection;
            }
            selection::Press::Refused => return Handled::Refused,
            selection::Press::NotOurs => {}
        },
        DeviceKind::Drums => {
            if let Some(delta) = drum_panel::voice_move(press.key, press.shift) {
                // A card is modal for the keyboard as well as the pointer: a walk aimed at a grid nobody can see would
                // select and sound a voice behind it.
                if panel.a_card_is_up() {
                    return Handled::Refused;
                }
                // `move_selection` clamps at both ends, so `shift+up` on the first row stays on it rather than wrapping to
                // the last — the same rule the 303's step border keeps.
                let walked =
                    selection::move_selection(panel.drum.selected.index(), delta, Drum::ALL.len());
                panel.drum.selected = Drum::ALL[walked];
                shared.request_audition(panel.drum.selected);
                return Handled::Selection;
            }
        }
    }
    let Some(shortcut) = shortcuts::shortcut_for(press.key, press.shift) else {
        return Handled::Nothing;
    };
    // The transport shortcuts are the rack's: they ask for a change rather than writing one device's atomic, which the next
    // fan-out would undo. The three unwired keys are not the transport's yet, so they answer with a notice instead.
    let transport = match shortcut {
        // Space is play/pause, not play/stop: a deck's space bar does not rewind, and STOP has its own key.
        shortcuts::Shortcut::PlayStop => Some(if shared.is_playing() {
            transport::Request::Pause
        } else {
            transport::Request::Play
        }),
        shortcuts::Shortcut::TempoUp => Some(transport::Request::NudgeTempo(1)),
        shortcuts::Shortcut::TempoDown => Some(transport::Request::NudgeTempo(-1)),
        // The three keys with no feature behind them answer with the truth. A silent key reads as broken.
        shortcuts::Shortcut::SeekBack
        | shortcuts::Shortcut::SeekForward
        | shortcuts::Shortcut::Record => {
            if let Some(words) = not_wired(shortcut) {
                panel.notify(words);
            }
            return Handled::Shortcut(shortcut);
        }
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
    // The settings card is panel state too, and there is room for one card. Every other card goes through
    // `put_cards_away`, which is the one spelling of the rule — a keyboard arm that lists the others by hand forgets the
    // next card added, and the kit browser was that card.
    if matches!(shortcut, shortcuts::Shortcut::ToggleSettings) {
        let opening = !panel.settings_open;
        panel.put_cards_away();
        panel.settings_open = opening;
    }
    // The help card follows the same rule in the other direction: opening help puts every other card away, or help would
    // paint over a card that is still open and Escape would close the one nobody can see first.
    if matches!(shortcut, shortcuts::Shortcut::ToggleHelp) {
        let opening = !panel.help_visible;
        panel.put_cards_away();
        panel.help_visible = opening;
    }
    // The device card is the topmost of the five inside the panel, so it is the second thing Escape gives up on and the
    // one F3 toggles. Opening it puts the other four away — the kit browser among them, which is the card that opens from
    // this one — because there is room for one card.
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
/// card, then the kit browser, then the settings card, then help, then the songs card, then a learn in progress. Returns
/// whether there was anything to close. The topmost card first is the rule, and the songs card is the exception already — it
/// is painted over help but closed after it. It lives here rather than in `main` because the order is the behaviour, and
/// `main` is a binary no test can call.
pub fn escape(panel: &mut PanelState, shared: &SharedState) -> bool {
    // The picker is painted last, so it is the first thing Escape gives up on; then the device card under it.
    if panel.device_picker.open {
        panel.device_picker.open = false;
        return true;
    }
    if panel.device_card.themes.open {
        // The list is painted over the card, so it is what Escape gives up first: one Escape for the drop-down, the next for
        // the card under it.
        panel.device_card.themes.open = false;
        return true;
    }
    if panel.device_card.open.is_some() {
        panel.close_device_card();
        return true;
    }
    if panel.kits.open {
        // The kit browser is the device card's sibling, opened from it and painted over the panel: Escape puts it away
        // before the card that opened it, which is the same order they are painted in.
        kits::close(panel);
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

/// Take this frame's key events out of the frame and turn them into presses, leaving every other event for the widgets
/// that follow. Taking rather than reading is what makes a press happen once: egui leaves the last pass's events in place
/// for a logic-only tick, so a reader would replay a Space or an Enter every 200 ms while the window is hidden.
pub fn take_presses(events: &mut Vec<Event>, typing: bool) -> Vec<KeyPress> {
    // A focused text field's keys are the field's: its space bar is a space, not the transport.
    if typing {
        return Vec::new();
    }
    let mut taken = Vec::new();
    events.retain(|event| match as_press(event) {
        Some(press) => {
            taken.push(press);
            false
        }
        None => true,
    });
    taken
}

/// Take this frame's Escape press out of the frame, if there is one and no text field has the keyboard.
///
/// Escape is *not* a panel key — it is not in `shortcuts`, because what it does depends on what is open rather than on the key
/// — so `take_presses` does not carry it. It needs taking all the same: `key_pressed` counts presses over the frame's event
/// buffer, and a logic-only tick is handed the last pass's buffer again, so reading it would close one card per 200 ms — help,
/// then songs, discarding a name being typed — where one Escape should close one thing and stop.
pub fn take_escape(events: &mut Vec<Event>, typing: bool) -> bool {
    // A focused text field uses Escape itself: it gives up the field's focus, and the card stays up.
    if typing {
        return false;
    }
    let mut pressed = false;
    events.retain(|event| match event {
        Event::Key {
            key: Key::Escape,
            pressed: true,
            ..
        } => {
            pressed = true;
            false
        }
        _ => true,
    });
    pressed
}

/// Whether an event is one the clipboard answers. The one list: `take_clipboard` takes exactly these, and `clipboard`'s match
/// holds the arm for each — a new clipboard event added to one and not the other would either be replayed or never answered.
fn is_clipboard_event(event: &Event) -> bool {
    matches!(event, Event::Copy | Event::Paste(_) | Event::PasteImage(_))
}

/// Take this frame's clipboard events out of the frame, leaving every other event for the widgets that follow.
///
/// Taking rather than reading is what makes a gesture happen once: egui leaves the last pass's events in place for a
/// logic-only tick (a minimised window), so a reader would re-dispatch the same paste every 200 ms.
pub fn take_clipboard(events: &mut Vec<Event>, typing: bool) -> Vec<Event> {
    // A focused text field reads `Event::Paste` itself, so nothing may be taken out from under it.
    if typing {
        return Vec::new();
    }
    let mut taken = Vec::new();
    events.retain(|event| {
        let clipboard = is_clipboard_event(event);
        if clipboard {
            taken.push(event.clone());
        }
        !clipboard
    });
    taken
}

/// Answer this frame's clipboard events, and hand back the text `main` should put on the operating system's clipboard.
///
/// Both gestures arrive as *events*, not keys: `egui-winit` intercepts `Ctrl+C` and `Ctrl+V`, pushes `Event::Copy` /
/// `Event::Paste(text)` and never delivers the key press, so `shortcuts` has nothing to say about them and this sits beside
/// `escape` for the same reason — `main` is a binary, and the behaviour has to be testable.
///
/// * **Copy** takes the pattern on the panel (the one the engine is playing) and the slot label with it, notices
///   `copied B2`, and returns the clipboard text.
/// * **Paste** puts a pattern from the clipboard into the slot on the panel: the bank first, then the engine's `Arc`, because a
///   pattern that reached one and not the other is the bug `shortcuts::select_pattern` exists to avoid in the other
///   direction. It disarms CLEAR — that arming was against the pattern that just went — marks the session dirty, and notices
///   `pasted A1 into B2`.
/// * **Silence, when there is nothing to answer.** Text that is not ours, and an image, notice `the clipboard has no
///   pattern`. An *empty* clipboard produces no event at all — `egui-winit` only sends `Paste` for non-empty text — so the
///   notice covers everything the event can carry, and a `Ctrl+V` on nothing stays silent.
///
/// `typing` is `ctx.text_edit_focused()`: with a text field's cursor in it — the device card's name — the clipboard belongs to
/// the field, and the pattern takes nothing.
pub fn clipboard(
    events: &[Event],
    typing: bool,
    panel: &mut PanelState,
    shared: &SharedState,
    bank: &mut PatternBank,
) -> Option<String> {
    if typing {
        return None;
    }
    let mut out = None;
    for event in events {
        match event {
            Event::Copy => {
                let slot = PatternBank::id_of(bank.current()).to_string();
                let clip = crate::clipboard::Clip::Pattern {
                    pattern: *shared.pattern(),
                    from: Some(slot.clone()),
                };
                panel.notice = Some((format!("copied {slot}"), std::time::Instant::now()));
                out = Some(crate::clipboard::encode(&clip));
            }
            Event::Paste(text) => {
                let slot = PatternBank::id_of(bank.current()).to_string();
                match crate::clipboard::decode(text) {
                    Some(crate::clipboard::Clip::Pattern { pattern, from }) => {
                        bank.replace(bank.current(), pattern);
                        shared.set_pattern(std::sync::Arc::new(pattern));
                        // The arming was against the pattern that just went.
                        super::clear::disarm(panel);
                        panel.dirty = true;
                        let from = from.unwrap_or_else(|| "a pattern".to_string());
                        panel.notice = Some((
                            format!("pasted {from} into {slot}"),
                            std::time::Instant::now(),
                        ));
                    }
                    None => {
                        panel.notice = Some((
                            "the clipboard has no pattern".to_string(),
                            std::time::Instant::now(),
                        ));
                    }
                }
            }
            // An image is a paste of something the instrument has no use for, so it says the same thing as text that is
            // not ours: silence would read as a broken app.
            Event::PasteImage(_) => {
                panel.notice = Some((
                    "the clipboard has no pattern".to_string(),
                    std::time::Instant::now(),
                ));
            }
            _ => {}
        }
    }
    out
}

/// What an unwired deck shortcut says when it is pressed, or `None` for a shortcut that is wired.
///
/// The words are the deck's own (`ui::tape`), so the key and the shortcut answer with one sentence rather than two that can
/// drift. `None` is the honest answer for a wired shortcut: its notice never comes from here, and the arm it replaced said
/// `"NOTHING TO DO"` — a string no keystroke could ever produce.
fn not_wired(shortcut: shortcuts::Shortcut) -> Option<&'static str> {
    match shortcut {
        shortcuts::Shortcut::SeekBack => Some(tape::BACK_NOT_WIRED),
        shortcuts::Shortcut::SeekForward => Some(tape::FORWARD_NOT_WIRED),
        shortcuts::Shortcut::Record => Some(tape::RECORD_NOT_WIRED),
        // The wired shortcuts never reach this function. Spelled out rather than `_`, because a new shortcut that is not the
        // deck's should be a compile error here, not a wrong notice.
        shortcuts::Shortcut::PlayStop
        | shortcuts::Shortcut::TempoUp
        | shortcuts::Shortcut::TempoDown
        | shortcuts::Shortcut::NextPattern
        | shortcuts::Shortcut::PrevPattern
        | shortcuts::Shortcut::ToggleSettings
        | shortcuts::Shortcut::ToggleHelp
        | shortcuts::Shortcut::ToggleDeviceCard => None,
    }
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

    #[test]
    fn only_the_three_unwired_shortcuts_answer_with_words() {
        // The `None` half is what keeps a wired key from printing a notice as well as acting. No keystroke can reach it today,
        // so nothing but this test says it.
        for shortcut in [
            shortcuts::Shortcut::SeekBack,
            shortcuts::Shortcut::SeekForward,
            shortcuts::Shortcut::Record,
        ] {
            assert!(not_wired(shortcut).is_some(), "{shortcut:?} is not wired");
        }
        for shortcut in [
            shortcuts::Shortcut::PlayStop,
            shortcuts::Shortcut::TempoUp,
            shortcuts::Shortcut::TempoDown,
            shortcuts::Shortcut::NextPattern,
            shortcuts::Shortcut::PrevPattern,
            shortcuts::Shortcut::ToggleSettings,
            shortcuts::Shortcut::ToggleHelp,
            shortcuts::Shortcut::ToggleDeviceCard,
        ] {
            assert!(not_wired(shortcut).is_none(), "{shortcut:?} is wired");
        }
    }
}
