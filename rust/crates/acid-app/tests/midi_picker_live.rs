//! The MIDI input picker: the status line opens it, a row chooses a port, a click elsewhere dismisses it.

use acid_app::midi::ports::PortChoice;
use acid_app::state::SharedState;
use acid_app::ui::layout::layout;
use acid_app::ui::{PanelState, PickerState, draw_panel, panel};
use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

fn scale() -> f32 {
    acid_app::ui::layout::scale(screen())
}

fn run_on(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
}

fn press(at: Pos2) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::default(),
    }
}

fn release(at: Pos2) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::default(),
    }
}

/// A settled pointer, a press and a release — three frames, because a widget must exist before it is hit.
fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![press(at)]);
    run_on(ctx, panel, shared, vec![release(at)]);
}

fn status_rect() -> Rect {
    panel::midi_status_rect(layout(screen()).top, scale())
}

/// The picker's rows hang below the status line, so row `index` is centred one row down.
fn row_centre(index: usize) -> Pos2 {
    let status = status_rect();
    let row_height = 20.0 * scale();
    Pos2::new(
        status.left() + 4.0,
        status.bottom() + row_height * index as f32 + row_height * 0.5,
    )
}

#[test]
fn a_row_chooses_the_port_it_names() {
    let ctx = Context::default();
    let shared = SharedState::new();
    // The list is filled by the picker when it opens; seed it so the test does not depend on this machine's
    // MIDI hardware.
    let mut panel = PanelState {
        picker: PickerState {
            inputs: Some(vec!["KeyStep".to_string(), "Launchkey MIDI".to_string()]),
            ..PickerState::default()
        },
        ..PanelState::default()
    };

    // Row 0 is "any input", row 2 is the second port.
    click(&ctx, &mut panel, &shared, row_centre(2));

    assert_eq!(
        panel.picker.input_choice,
        Some(PortChoice::Named("Launchkey MIDI".to_string()))
    );
    assert!(panel.picker.inputs.is_none(), "choosing closes the list");
}

#[test]
fn the_any_row_asks_for_the_first_port() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState {
        picker: PickerState {
            inputs: Some(vec!["KeyStep".to_string()]),
            ..PickerState::default()
        },
        ..PanelState::default()
    };

    click(&ctx, &mut panel, &shared, row_centre(0));

    assert_eq!(panel.picker.input_choice, Some(PortChoice::Any));
}

#[test]
fn a_click_outside_the_list_closes_it_without_choosing() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState {
        picker: PickerState {
            inputs: Some(vec!["KeyStep".to_string()]),
            ..PickerState::default()
        },
        ..PanelState::default()
    };

    // Below the rows, well away from the status line.
    click(&ctx, &mut panel, &shared, Pos2::new(700.0, 300.0));

    assert!(panel.picker.inputs.is_none(), "the list should close");
    assert_eq!(panel.picker.input_choice, None, "and choose nothing");
}

#[test]
fn the_status_line_toggles_the_list() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let at = status_rect().right_center() - vec2(4.0, 0.0);

    click(&ctx, &mut panel, &shared, at);
    assert!(
        panel.picker.inputs.is_some(),
        "clicking the MIDI line opens the list (empty without hardware, but open)"
    );

    click(&ctx, &mut panel, &shared, at);
    assert!(panel.picker.inputs.is_none(), "a second click closes it");
}
