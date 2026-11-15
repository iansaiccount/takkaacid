//! The device card: F3 and the caret open it, Escape and CLOSE close it, and every row is a request.

use acid_app::device::Request;
use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, strip_cells};
use acid_app::ui::strip::{StripCell, caret_rect};
use acid_app::ui::{PanelState, device_card, dispatch, draw_panel};
use eframe::egui::{
    Context, Event, FullOutput, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2,
};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))
}

fn run(
    ctx: &Context,
    panel: &mut PanelState,
    shared: &SharedState,
    events: Vec<Event>,
) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

/// Every line of text the frame drew, in paint order. Copied from `settings_live.rs`, which needs it for the same reason:
/// a card's own words are part of what it says, and `CONFIRM?` is the only feedback the first REMOVE click gives.
fn strings(output: &FullOutput) -> Vec<String> {
    use eframe::egui::Shape;

    fn walk(shape: &Shape, said: &mut Vec<String>) {
        match shape {
            Shape::Text(text) => said.push(text.galley.text().to_string()),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, said)),
            _ => {}
        }
    }

    let mut said = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut said);
    }
    said
}

/// The fill of the smallest shape drawn that covers `bounds` — how a surface's colour is read back off the paint.
///
/// Smallest rather than first, because a card is laid over a scrim that covers the whole panel: asking for "the fill under
/// the card's own keys" has one answer, and it is the card.
fn fill_around(output: &FullOutput, bounds: Rect) -> Option<eframe::egui::Color32> {
    use eframe::egui::Shape;

    fn walk(shape: &Shape, bounds: Rect, best: &mut Option<(f32, eframe::egui::Color32)>) {
        match shape {
            Shape::Rect(rect)
                if rect.fill != eframe::egui::Color32::TRANSPARENT
                    && rect.rect.contains_rect(bounds) =>
            {
                let area = rect.rect.width() * rect.rect.height();
                if best.is_none_or(|(smallest, _)| area < smallest) {
                    *best = Some((area, rect.fill));
                }
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, bounds, best)),
            _ => {}
        }
    }

    let mut best = None;
    for clipped in &output.shapes {
        walk(&clipped.shape, bounds, &mut best);
    }
    best.map(|(_, fill)| fill)
}

/// Three frames, as `tests/devices_live.rs` does it: move, press, release. A click is never one frame in egui.
fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run(ctx, panel, shared, vec![button(true)]);
    run(ctx, panel, shared, vec![button(false)]);
}

fn two_device_panel() -> PanelState {
    let mut panel = PanelState::default();
    panel.strip.cells = vec![
        StripCell {
            name: "ACID 1".into(),
            number: 1,
            scheme: acid_app::device::Scheme::Black,
            running: false,
            muted: false,
        },
        StripCell {
            name: "BASS".into(),
            number: 2,
            scheme: acid_app::device::Scheme::Black,
            running: false,
            muted: false,
        },
    ];
    panel
}

#[test]
fn f3_opens_the_selected_devices_card_and_escape_closes_it() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let mut bank = acid_core::bank::PatternBank::new_empty();
    dispatch::handle(
        dispatch::KeyPress {
            key: eframe::egui::Key::F3,
            shift: false,
            ctrl: false,
        },
        &mut panel,
        &state,
        &mut bank,
    );
    assert_eq!(
        panel.device_card.open,
        Some(1),
        "F3 opens the card of the device on the panel"
    );

    dispatch::escape(&mut panel, &state);
    assert_eq!(panel.device_card.open, None);
}

/// The caret is the other way in, and it is the one a hand finds first. It asks for the card of the cell it is drawn on,
/// which is the *selected* device — a cell click would ask to select, and the device is already on the panel.
#[test]
fn the_caret_on_the_selected_cell_asks_for_that_devices_card() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 0;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let cells = strip_cells(layout(screen()).strip, 2);
    click(&ctx, &mut panel, &state, caret_rect(cells[0]).center());
    assert_eq!(panel.device_request, Some(Request::Open(0)));

    // The unselected cell draws no caret, so the pixels where one would be are the cell's own.
    panel.device_request = None;
    click(&ctx, &mut panel, &state, caret_rect(cells[1]).center());
    assert_eq!(panel.device_request, Some(Request::Select(1)));
}

/// One card at a time: opening the device card puts the settings card away, and Escape closes the device card first
/// because it is the one painted over everything.
#[test]
fn the_device_card_is_the_only_card_and_escape_takes_the_top_one() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.settings_open = true;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let mut bank = acid_core::bank::PatternBank::new_empty();
    dispatch::handle(
        dispatch::KeyPress {
            key: Key::F3,
            shift: false,
            ctrl: false,
        },
        &mut panel,
        &state,
        &mut bank,
    );
    assert_eq!(panel.device_card.open, Some(0));
    assert!(!panel.settings_open, "the settings card went away");

    assert!(dispatch::escape(&mut panel, &state));
    assert_eq!(panel.device_card.open, None);
    assert!(
        !panel.settings_open,
        "and Escape closed the card it opened, not one under it"
    );
}

/// CLOSE is the card's own lifetime, and it is the one key on the card that answers in this commit: it asks `main` to put
/// the card away, which also drops the mirror the card was showing.
#[test]
fn the_close_key_asks_main_to_put_the_card_away() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    panel.device_card.open = Some(1);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(
        device_card::card_rect(layout(screen()).panel, s),
        s,
        &panel.device_card,
    );
    click(&ctx, &mut panel, &state, rows.close.center());
    assert_eq!(panel.device_request, Some(Request::Close));
}

/// The card is painted over the panel, so a click meant for it never reaches the face underneath: the shade is a shield,
/// and the only thing a click outside the card can say is "put it away".
#[test]
fn a_click_outside_the_card_closes_it_rather_than_pressing_the_panel() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    // A step key, well outside the card, would select step 3 if the panel underneath were still listening.
    let steps = acid_app::ui::layout::step_cells(layout(screen()).steps);
    click(&ctx, &mut panel, &state, steps[3].center());
    assert_eq!(panel.device_request, Some(Request::Close));
    assert_eq!(panel.selected, 0, "the step under the shade was not edited");
}

/// The name row is a text field: the hand types, Enter asks the rack, and the card shows whatever the store answered — which
/// is why the answer is written back into the field rather than left as the typed string (`BASS` typed twice is `BASS (2)`).
#[test]
fn typing_a_name_and_confirming_asks_the_rack_to_rename_the_device() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    panel.device_card.open = Some(1);
    panel.device_card.name = "BASS".into();
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(
        device_card::card_rect(layout(screen()).panel, s),
        s,
        &panel.device_card,
    );
    // Clicking the field, moving the caret to the end, typing, and pressing Enter: the card asks for the name it now holds.
    // The End key is the panel's business only through the field, which is why a text edit behind a card has to be modal.
    click(&ctx, &mut panel, &state, rows.name.center());
    run(
        &ctx,
        &mut panel,
        &state,
        vec![Event::Key {
            key: Key::End,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }],
    );
    run(&ctx, &mut panel, &state, vec![Event::Text(" 2".into())]);
    run(
        &ctx,
        &mut panel,
        &state,
        vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }],
    );
    assert_eq!(
        panel.device_request,
        Some(Request::Rename(1, "BASS 2".into()))
    );
}

/// Confirming is Enter, and only Enter: a click that puts the card away is a dismissal, not a rename. The typed string stays
/// in the field while the card is open, so a hand that clicks the card's own face has lost nothing.
#[test]
fn a_click_away_from_the_field_does_not_rename_anything() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    panel.device_card.name = "BASS".into();
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(
        device_card::card_rect(layout(screen()).panel, s),
        s,
        &panel.device_card,
    );
    click(&ctx, &mut panel, &state, rows.name.center());
    run(
        &ctx,
        &mut panel,
        &state,
        vec![Event::Key {
            key: Key::End,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }],
    );
    run(&ctx, &mut panel, &state, vec![Event::Text(" 2".into())]);
    panel.device_request = None;

    // Clicking another row takes the focus away — and that row answers for itself, which is the point: the focus loss
    // itself asks for nothing, so the rename has not happened and the field still holds what was typed.
    click(&ctx, &mut panel, &state, rows.role.center());
    assert!(
        !matches!(panel.device_request, Some(Request::Rename(..))),
        "a focus loss is not a rename: {:?}",
        panel.device_request
    );
    assert_eq!(
        panel.device_card.name, "BASS 2",
        "and the typed name remains"
    );
}

/// A card is dismissed by CLOSE, Escape or a click outside it, never by a click on one of its own rows: a row answers for
/// itself, and a player aiming at MUTE must not lose the card instead.
#[test]
fn a_click_on_one_of_the_cards_own_rows_answers_and_leaves_it_standing() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(
        device_card::card_rect(layout(screen()).panel, s),
        s,
        &panel.device_card,
    );
    for at in [rows.mute.center(), rows.role.center(), rows.name.center()] {
        panel.device_request = None;
        click(&ctx, &mut panel, &state, at);
        assert_ne!(
            panel.device_request,
            Some(Request::Close),
            "a click on a row is the row's, not a dismissal: {at:?}"
        );
        assert_eq!(
            panel.device_card.open,
            Some(0),
            "and the card is still up: {at:?}"
        );
    }
}

/// One card at a time, both ways round: the device card puts the others away, and F2 or F1 puts the device card away.
/// A one-way rule is a state two keystrokes away — two shades over one face and a live card hidden under another.
#[test]
fn opening_another_card_puts_the_device_card_away() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let mut bank = acid_core::bank::PatternBank::new_empty();
    let mut press = |key: Key, panel: &mut PanelState| {
        dispatch::handle(
            dispatch::KeyPress {
                key,
                shift: false,
                ctrl: false,
            },
            panel,
            &state,
            &mut bank,
        );
    };

    press(Key::F3, &mut panel);
    assert_eq!(panel.device_card.open, Some(0));
    press(Key::F2, &mut panel);
    assert!(panel.settings_open);
    assert_eq!(panel.device_card.open, None, "F2 put the card away");

    press(Key::F1, &mut panel);
    assert!(panel.help_visible);
    assert_eq!(panel.device_card.open, None);
}

/// The card covers the panel, so it is modal for the keyboard too: Enter confirms the name field, and Enter is also the step
/// editor's key. Without the refusal a rename would toggle a step the player cannot see, and mark the session unsaved for it.
#[test]
fn enter_on_an_open_card_does_not_edit_the_step_behind_it() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    let before = *state.pattern();
    let mut bank = acid_core::bank::PatternBank::new_empty();

    // The step editor's own keys, and the plain arrows: the selected step and the selection behind the card do not move.
    for (key, shift) in [
        (Key::Enter, false),
        (Key::ArrowRight, false),
        (Key::ArrowUp, true),
    ] {
        assert_eq!(
            dispatch::handle(
                dispatch::KeyPress {
                    key,
                    shift,
                    ctrl: false
                },
                &mut panel,
                &state,
                &mut bank
            ),
            dispatch::Handled::Refused,
            "{key:?} with shift {shift}"
        );
    }
    assert_eq!(
        *state.pattern(),
        before,
        "nothing behind the card was edited"
    );
    assert_eq!(panel.selected, 0, "and the selection did not move");
    assert!(!panel.dirty);

    // The shortcuts that stop the transport and close the card still reach them: refusing every key would trap a player
    // behind a card that the keyboard cannot dismiss.
    assert_eq!(
        dispatch::handle(
            dispatch::KeyPress {
                key: Key::Space,
                shift: false,
                ctrl: false
            },
            &mut panel,
            &state,
            &mut bank
        ),
        dispatch::Handled::Shortcut(acid_app::ui::shortcuts::Shortcut::PlayStop)
    );
}

/// A focused field owns the keyboard. A real keyboard sends the key *and* the text, so typing a capital into the name field
/// used to write a step (shift held), play a note, and mark the session unsaved — and with STEP mode on, Backspace emptied
/// the selected step instead of deleting a letter. The panel's key readers ask egui whether something is being typed into.
#[test]
fn typing_into_the_name_field_writes_no_step_and_stops_no_transport() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    // The worst case: every pitch key writes a step, and Backspace empties one.
    panel.step_mode = true;
    panel.dirty = false;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(
        device_card::card_rect(layout(screen()).panel, s),
        s,
        &panel.device_card,
    );
    click(&ctx, &mut panel, &state, rows.name.center());
    let before = *state.pattern();
    panel.dirty = false;

    let key = |key: Key, shift: bool| Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            shift,
            ..Modifiers::default()
        },
    };
    run(
        &ctx,
        &mut panel,
        &state,
        vec![key(Key::A, true), Event::Text("A".into())],
    );
    assert_eq!(*state.pattern(), before, "a letter is not a step");
    assert!(!panel.dirty, "and typing a name is not an unsaved edit");
    assert_eq!(panel.selected, 0, "and the border did not walk");
    assert!(
        panel.device_card.name.ends_with('A'),
        "the field took the letter: {:?}",
        panel.device_card.name
    );

    run(&ctx, &mut panel, &state, vec![key(Key::Backspace, false)]);
    assert_eq!(
        *state.pattern(),
        before,
        "backspace in a field deletes a letter, not a step"
    );
    assert_eq!(
        panel.transport_request, None,
        "and nothing here asked the rack to start or stop"
    );
}

fn open_card(panel: &mut PanelState, index: usize, ports: &[(&str, bool)]) {
    panel.device_card.open = Some(index);
    panel.device_card.ports = ports
        .iter()
        .map(|(name, heard)| ((*name).to_string(), *heard))
        .collect();
}

fn card_rows(panel: &PanelState) -> device_card::Rows {
    let s = acid_app::ui::layout::scale(screen());
    device_card::rows(
        device_card::card_rect(layout(screen()).panel, s),
        s,
        &panel.device_card,
    )
}

/// Every row on the card is a request, and the port rows are a set: ticking one adds it to the ones the device already hears,
/// and the answer is in the machine's port order rather than the order they were ticked in.
#[test]
fn a_port_row_ticks_and_unticks_and_asks_for_the_new_set() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    open_card(
        &mut panel,
        1,
        &[("Launchkey MIDI", true), ("nanoKONTROL", false)],
    );
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.ports[1].center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetPorts(
            1,
            acid_app::midi::route::Ports::Named(vec![
                "Launchkey MIDI".into(),
                "nanoKONTROL".into()
            ])
        )),
        "ticking a port adds it to the set the device already hears"
    );

    // What `main`'s `route_from` writes back once the request lands: both ports ticked. The panel never applies a request
    // itself, so the mirror is what `main` would have made of it — and the next click must build on that, not on the click
    // before it.
    panel.device_card.ports[1].1 = true;
    panel.device_request = None;
    click(&ctx, &mut panel, &state, rows.ports[0].center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetPorts(
            1,
            acid_app::midi::route::Ports::Named(vec!["nanoKONTROL".into()])
        )),
        "unticking one leaves the other"
    );

    // Everything ticked, and one row clicked: the set is everything *but* that one, which is what "I do not want this one"
    // means when the device hears every port. This is the case `toggled`'s arithmetic exists for.
    open_card(
        &mut panel,
        1,
        &[("Launchkey MIDI", true), ("nanoKONTROL", true)],
    );
    panel.device_request = None;
    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.ports[0].center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetPorts(
            1,
            acid_app::midi::route::Ports::Named(vec!["nanoKONTROL".into()])
        )),
        "a row clicked from ALL PORTS leaves every other port"
    );
}

/// A device added by `+` arrives hearing nothing, and the first port it is given is exactly that one — never "all of them".
#[test]
fn a_device_that_hears_nothing_is_given_one_port_at_a_time() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    open_card(
        &mut panel,
        1,
        &[("Launchkey MIDI", false), ("nanoKONTROL", false)],
    );
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.ports[0].center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetPorts(
            1,
            acid_app::midi::route::Ports::Named(vec!["Launchkey MIDI".into()])
        )),
        "a new device hears one port because it was given one, not because it hears everything"
    );
}

/// `ALL PORTS` is the way back to the factory filter, so a player who has routed a device to one controller can undo it.
#[test]
fn the_all_ports_row_asks_for_every_port() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    open_card(&mut panel, 0, &[("Launchkey MIDI", false)]);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.all_ports.center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetPorts(0, acid_app::midi::route::Ports::All))
    );
}

/// The channel stepper walks the store's `0..=15`, and the card shows it as `1..=16`. Past `ANY` comes channel 1, and the
/// step past the last channel is `ANY` again, so leaving `ANY` is never a one-way door.
#[test]
fn the_channel_stepper_walks_from_any_channel_into_one_two_three() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    panel.device_card.channel = acid_app::midi::route::Channel::All;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.channel_up.center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetChannel(
            0,
            acid_app::midi::route::Channel::One(0)
        )),
        "past ANY comes channel 1, which the store calls 0"
    );

    // Up from the last channel is `ANY`, and down from channel 1 is `ANY` too: the stepper wraps through `ANY` rather than
    // around the sixteen channels, or a player who left `ANY` could never get back to it.
    use acid_app::midi::route::Channel;
    for (stored, up, down) in [
        (0u8, Some(Channel::One(1)), Some(Channel::All)),
        (7, Some(Channel::One(8)), Some(Channel::One(6))),
        (15, Some(Channel::All), Some(Channel::One(14))),
    ] {
        panel.device_card.channel = Channel::One(stored);
        panel.device_request = None;
        let rows = card_rows(&panel);
        click(&ctx, &mut panel, &state, rows.channel_up.center());
        assert_eq!(
            panel.device_request,
            up.map(|next| Request::SetChannel(0, next)),
            "up from channel {}",
            stored + 1
        );

        panel.device_request = None;
        click(&ctx, &mut panel, &state, rows.channel_down.center());
        assert_eq!(
            panel.device_request,
            down.map(|next| Request::SetChannel(0, next)),
            "down from channel {}",
            stored + 1
        );
    }
}

/// The role row switches between playing what the device hears and only answering controllers.
#[test]
fn the_role_row_switches_between_playing_and_controlling() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    panel.device_card.role = acid_app::midi::route::Role::PlayAndControl;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.role.center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetRole(
            0,
            acid_app::midi::route::Role::ControlOnly
        ))
    );

    panel.device_card.role = acid_app::midi::route::Role::ControlOnly;
    panel.device_request = None;
    click(&ctx, &mut panel, &state, rows.role.center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetRole(
            0,
            acid_app::midi::route::Role::PlayAndControl
        )),
        "and back again"
    );
}

/// The MUTE key on the card mutes the device the card is about — not the device on the panel, and not the one the click
/// passed on its way there.
#[test]
fn the_mute_key_on_the_card_asks_for_the_device_it_is_about() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.cells[1].muted = true;
    panel.device_card.open = Some(1);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.mute.center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetMute(1, false)),
        "a muted device's key asks to unmute it"
    );
}

/// Removing a device has no undo, so it takes two clicks the way CLEAR does: the first asks nothing of the rack, the second
/// asks for the removal.
#[test]
fn the_remove_key_asks_twice_before_it_asks_the_rack() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    panel.device_card.open = Some(1);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.remove.center());
    assert_eq!(panel.device_request, Some(Request::AskRemove(1)));
    assert!(
        panel.device_card.armed_remove.is_some(),
        "and the card says so"
    );

    panel.device_request = None;
    run(&ctx, &mut panel, &state, vec![]);
    click(&ctx, &mut panel, &state, rows.remove.center());
    assert_eq!(panel.device_request, Some(Request::ConfirmRemove(1)));
    assert_eq!(
        panel.device_card.armed_remove, None,
        "and the question is spent by its own answer"
    );
}

/// Any other key on the card answers a different question, so it puts the arming away: a hand that has moved on to the
/// channel stepper is not confirming a removal.
#[test]
fn any_other_key_on_the_card_disarms_the_removal() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.remove.center());
    assert!(panel.device_card.armed_remove.is_some());

    panel.device_request = None;
    click(&ctx, &mut panel, &state, rows.channel_up.center());
    assert_eq!(
        panel.device_card.armed_remove, None,
        "the arming is put away by the next key"
    );

    panel.device_request = None;
    click(&ctx, &mut panel, &state, rows.remove.center());
    assert_eq!(
        panel.device_request,
        Some(Request::AskRemove(0)),
        "so the next REMOVE asks again rather than removing"
    );
}

/// An arming is a question asked a moment ago, not a standing permission: one older than the two-click window is spent, so a
/// REMOVE clicked after a pause asks again.
#[test]
fn an_old_arming_is_spent_and_the_next_click_asks_again() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    let stale = std::time::Instant::now().checked_sub(std::time::Duration::from_secs_f32(
        acid_app::ui::clear::ARM_SECONDS + 1.0,
    ));
    assert!(
        stale.is_some(),
        "the test needs an instant to look back from"
    );
    panel.device_card.armed_remove = stale;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.remove.center());
    assert_eq!(
        panel.device_request,
        Some(Request::AskRemove(0)),
        "a stale question is not an answer"
    );
}

/// The first click's only feedback is the key itself: `REMOVE` becomes `CONFIRM?`, in the accent colour, or a player cannot
/// tell a question from a key that did nothing.
#[test]
fn the_armed_remove_key_says_so() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    let ctx = Context::default();
    let before = strings(&run(&ctx, &mut panel, &state, vec![]));
    assert!(before.iter().any(|said| said == "REMOVE"), "{before:?}");
    assert!(!before.iter().any(|said| said == "CONFIRM?"), "{before:?}");

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.remove.center());
    let armed = strings(&run(&ctx, &mut panel, &state, vec![]));
    assert!(armed.iter().any(|said| said == "CONFIRM?"), "{armed:?}");
}

/// The keyboard can still switch devices while the card is up, and a card about the device you just left is a card about
/// nothing: the switch puts it away.
#[test]
fn switching_device_from_the_keyboard_closes_the_card() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let mut bank = acid_core::bank::PatternBank::new_empty();
    dispatch::handle(
        dispatch::KeyPress {
            key: Key::Num2,
            shift: false,
            ctrl: true,
        },
        &mut panel,
        &state,
        &mut bank,
    );
    assert_eq!(panel.device_request, Some(Request::Select(1)));
    assert_eq!(
        panel.device_card.open, None,
        "the card went with the device"
    );
}

/// The picker offers every kind there is, and its footnote is the promise the round keeps: names and extra devices are not
/// saved yet. The line is part of what the card says, so it is read back from the paint.
#[test]
fn the_picker_offers_every_kind_and_says_what_is_not_saved() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    // The strip's cells are named without a kind's name, so the only place the word can come from is a picker row.
    for (index, cell) in panel.strip.cells.iter_mut().enumerate() {
        cell.name = format!("CELL {}", index + 1);
    }
    panel.device_picker.open = true;
    let ctx = Context::default();
    let output = run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let area = device_card::picker_rect(layout(screen()).panel, s);
    let rows = device_card::picker_rows(area, s);
    assert_eq!(
        rows.kinds.len(),
        acid_app::device::DeviceKind::ALL.len(),
        "one row per kind, derived from the catalogue"
    );

    let said = strings(&output).join(" ");
    assert!(said.contains("ACID"), "every kind is named: {said}");
    assert!(
        said.contains("NOT SAVED"),
        "the card must say that extra devices and names do not survive a restart: {said}"
    );
}

#[test]
fn a_picker_row_asks_for_that_kind() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_picker.open = true;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let area = device_card::picker_rect(layout(screen()).panel, s);
    let rows = device_card::picker_rows(area, s);
    click(&ctx, &mut panel, &state, rows.kinds[0].center());
    assert_eq!(
        panel.device_request,
        Some(Request::Add(acid_app::device::DeviceKind::Acid))
    );
    assert!(
        !panel.device_picker.open,
        "and the picker closes on its answer"
    );
}

/// At the cap every row is greyed out — the same rule that greys the strip's `+` — but a click still asks, so the refusal is
/// `Rack::add`'s to say. A dead key would say nothing at all.
#[test]
fn at_the_cap_every_row_is_greyed_and_a_click_still_says_so() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.cells = (0..acid_app::device::MAX_DEVICES)
        .map(|index| StripCell {
            name: format!("ACID {}", index + 1),
            number: index + 1,
            scheme: acid_app::device::Scheme::Black,
            running: false,
            muted: false,
        })
        .collect();
    panel.device_picker.open = true;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let area = device_card::picker_rect(layout(screen()).panel, s);
    let rows = device_card::picker_rows(area, s);
    click(&ctx, &mut panel, &state, rows.kinds[0].center());
    assert_eq!(
        panel.device_request,
        Some(Request::Add(acid_app::device::DeviceKind::Acid)),
        "the key still answers, so the refusal is the rack's to say"
    );
}

/// The picker closes itself and says nothing to the rack: it has no mirror to drop, so it is the one card whose CLOSE is
/// panel state rather than a request.
#[test]
fn the_picker_close_key_puts_it_away() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_picker.open = true;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let area = device_card::picker_rect(layout(screen()).panel, s);
    let rows = device_card::picker_rows(area, s);
    click(&ctx, &mut panel, &state, rows.close.center());
    assert!(!panel.device_picker.open);
    assert_eq!(panel.device_request, None);
}

/// One card at a time: the picker is a card, so Escape closes it first (it is painted last) and opening it puts the device
/// card away.
#[test]
fn the_picker_is_the_top_card_and_escape_takes_it() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    // Opening the picker is what puts the device card away, so the test opens it the way `main` does.
    panel.device_card.open = Some(0);
    panel.settings_open = true;
    panel.open_device_picker();
    assert_eq!(panel.device_card.open, None);
    assert!(!panel.settings_open);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    assert!(dispatch::escape(&mut panel, &state));
    assert!(!panel.device_picker.open, "Escape closed the picker");
    assert_eq!(
        panel.device_card.open, None,
        "the card it replaced stayed closed"
    );
}

/// The rule the other cards already keep, both ways round: the picker puts the device card away, and F3 puts the picker away
/// — otherwise two cards stand at once and the picker paints over one that is live underneath.
#[test]
fn opening_a_card_puts_the_picker_away() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.open_device_picker();
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let mut bank = acid_core::bank::PatternBank::new_empty();
    dispatch::handle(
        dispatch::KeyPress {
            key: Key::F3,
            shift: false,
            ctrl: false,
        },
        &mut panel,
        &state,
        &mut bank,
    );
    assert_eq!(panel.device_card.open, Some(0));
    assert!(!panel.device_picker.open, "F3 put the picker away");
}

/// The COLOUR key names the theme its device is on and, until the list lands, asks for the next one in the catalogue — so the
/// row both says where you are and moves you on.
#[test]
fn the_colour_key_names_the_theme_and_cycles_the_catalogue() {
    use acid_app::device::Scheme;

    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    panel.device_card.scheme = Scheme::Black;
    let ctx = Context::default();
    let said = strings(&run(&ctx, &mut panel, &state, vec![])).join(" ");
    assert!(said.contains("COLOUR"), "the row needs its caption: {said}");
    assert!(
        said.contains("BLACK"),
        "and shows the theme it is on: {said}"
    );

    let rows = card_rows(&panel);
    click(&ctx, &mut panel, &state, rows.scheme.center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetScheme(0, Scheme::ALL[1])),
        "a black device's key asks for the next theme in the catalogue"
    );

    // And the caption follows the device, not a fixed word.
    panel.device_request = None;
    panel.device_card.scheme = Scheme::AmbientMoss;
    let said = strings(&run(&ctx, &mut panel, &state, vec![])).join(" ");
    assert!(
        said.contains("AMBIENT MOSS"),
        "the key prints the theme the device is on: {said}"
    );
}

/// The conversation's decision, pinned: the face inverts but the cards do not. A card is an object laid over the
/// instrument rather than part of its face, so it stays black even when everything under it is amber.
#[test]
fn a_card_stays_black_over_a_themed_face() {
    use acid_app::device::Scheme;
    use acid_app::ui::theme;

    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.scheme = Scheme::AmbientMoss;
    panel.device_card.open = Some(0);
    let ctx = Context::default();
    let output = run(&ctx, &mut panel, &state, vec![]);

    assert_eq!(
        fill_around(&output, acid_app::ui::layout::layout(screen()).panel),
        Some(theme::palette(Scheme::AmbientMoss).panel),
        "the face behind the card is the theme's, or this proves nothing"
    );
    // The card's own surface: the smallest painted thing that covers its first row and its last key is the card itself, since
    // its keys are separate shapes and the scrim under it reaches to the panel's edges.
    let rows = card_rows(&panel);
    assert_eq!(
        fill_around(
            &output,
            Rect::from_two_pos(rows.name.center(), rows.close.center())
        ),
        Some(theme::BLACK.panel_raised),
        "and the card is the black furniture it has always been"
    );
}
