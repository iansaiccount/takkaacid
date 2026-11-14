//! The device card: F3 and the caret open it, Escape and CLOSE close it, and every row is a request.

use acid_app::device::Request;
use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, strip_cells};
use acid_app::ui::strip::{StripCell, caret_rect};
use acid_app::ui::{PanelState, device_card, dispatch, draw_panel};
use eframe::egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))
}

fn run(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
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
            running: false,
            muted: false,
        },
        StripCell {
            name: "BASS".into(),
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

    // The card's own face still takes the click, and the text is still there afterwards.
    click(&ctx, &mut panel, &state, rows.role.center());
    assert_eq!(panel.device_request, None, "no rename on a focus loss");
    assert_eq!(
        panel.device_card.name, "BASS 2",
        "and the typed name remains"
    );
}

/// A card is dismissed by CLOSE, Escape, or a click outside it — never by a click on one of its own rows. The rows that do
/// not answer yet must not be dismissal targets either: a player aiming at MUTE must not lose the card instead.
#[test]
fn a_click_on_one_of_the_cards_own_rows_leaves_it_standing() {
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
        assert_eq!(
            panel.device_request, None,
            "nothing was asked for at {at:?}"
        );
    }
    assert_eq!(panel.device_card.open, Some(0), "and the card is still up");
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
