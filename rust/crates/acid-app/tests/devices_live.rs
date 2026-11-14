//! The strip: one cell per device, a click to select, a click on MUTE for that device, and `+`.

use acid_app::device::{DeviceKind, Request};
use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, strip_add_rect, strip_cells};
use acid_app::ui::strip::{StripCell, mute_rect};
use acid_app::ui::{PanelState, draw_panel};
use eframe::egui::{
    Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2,
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

fn panel_of(names: &[(&str, bool, bool)]) -> PanelState {
    let mut panel = PanelState::default();
    panel.strip.cells = names
        .iter()
        .map(|(name, running, muted)| StripCell {
            name: (*name).to_string(),
            running: *running,
            muted: *muted,
        })
        .collect();
    panel
}

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

#[test]
fn the_add_key_asks_for_the_only_kind_of_device_there_is() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false)]);
    panel.strip.selected = 0;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let add = strip_add_rect(layout(screen()).strip);
    click(&ctx, &mut panel, &state, add.center());
    assert_eq!(panel.device_request, Some(Request::Add(DeviceKind::Acid)));
}

#[test]
fn clicking_the_second_cell_selects_the_second_device() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false), ("BASS", false, false)]);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let cells = strip_cells(layout(screen()).strip, 2);
    click(&ctx, &mut panel, &state, cells[1].center());
    assert_eq!(panel.device_request, Some(Request::Select(1)));
}

#[test]
fn the_mute_key_carries_the_cell_it_was_drawn_for() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false), ("BASS", true, true)]);
    panel.strip.selected = 0; // device 1 is on the panel; the click is on device 2's key
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let cells = strip_cells(layout(screen()).strip, 2);
    click(&ctx, &mut panel, &state, mute_rect(cells[1]).center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetMute(1, false)),
        "muting a device that is already muted must ask to unmute that device"
    );
}

/// The panel may see a removal one frame after `main` made it, so the mirror it was handed is clamped before anything
/// draws: a selection pointing past the end must not index out of bounds.
#[test]
fn a_selection_past_the_end_is_clamped_rather_than_panicking() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false)]);
    panel.strip.selected = 3;
    let ctx = Context::default();

    run(&ctx, &mut panel, &state, vec![]);
    assert_eq!(panel.strip.selected, 0);

    // And a strip with no cells at all is a first frame, not a crash.
    panel.strip.cells.clear();
    panel.strip.selected = 2;
    run(&ctx, &mut panel, &state, vec![]);
    assert_eq!(panel.strip.selected, 0);
}

/// The plan's headline: the editor is not a mirror, so looking at the other device and back must keep the step the hand was
/// on, the step mode and the randomise dials — and a device the panel has never shown starts on its own first step.
#[test]
fn looking_at_the_other_device_and_back_keeps_the_step_you_were_on() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false), ("BASS", true, false)]);
    panel.selected = 4;
    panel.step_mode = true;
    panel.density = 0.75;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    // The rack has moved the selection; the panel has not caught up yet.
    panel.strip.selected = 1;
    run(&ctx, &mut panel, &state, vec![]);
    assert_eq!(panel.focused, 1);
    assert_eq!(
        panel.selected, 0,
        "a device the panel has never edited starts on step 1"
    );
    assert!(!panel.step_mode);
    assert!((panel.density - acid_app::ui::randomise::DEFAULT_DENSITY).abs() < 1e-6);

    panel.strip.selected = 0;
    run(&ctx, &mut panel, &state, vec![]);
    assert_eq!(panel.focused, 0);
    assert_eq!(panel.selected, 4, "the step the hand was on came back");
    assert!(panel.step_mode);
    assert!((panel.density - 0.75).abs() < 1e-6);
}

/// Every filled rectangle and every stroked one egui drew: a cell's face, its lamp, its border. A circle counts as filled,
/// because that is how the lamp is drawn — the face may not use a font glyph for it.
fn painted(output: &FullOutput) -> (Vec<(Rect, eframe::egui::Color32)>, Vec<Rect>) {
    use eframe::egui::Shape;

    fn walk(
        shape: &Shape,
        filled: &mut Vec<(Rect, eframe::egui::Color32)>,
        stroked: &mut Vec<Rect>,
    ) {
        match shape {
            Shape::Rect(rect) if rect.fill != eframe::egui::Color32::TRANSPARENT => {
                filled.push((rect.rect, rect.fill))
            }
            Shape::Rect(rect) => stroked.push(rect.rect),
            Shape::Circle(circle) => filled.push((
                Rect::from_center_size(
                    circle.center,
                    vec2(circle.radius * 2.0, circle.radius * 2.0),
                ),
                circle.fill,
            )),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, filled, stroked)),
            _ => {}
        }
    }

    let mut filled = Vec::new();
    let mut stroked = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut filled, &mut stroked);
    }
    (filled, stroked)
}

/// The selected cell wears the accent border and its neighbour does not; a running device's lamp is lit and a stopped one's
/// is dark. Both are painter shapes, so both are read back from what was drawn rather than from any private state.
#[test]
fn the_selected_cell_wears_the_accent_and_the_running_one_lights_its_lamp() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", false, false), ("BASS", true, false)]);
    panel.strip.selected = 1;
    let ctx = Context::default();
    let output = run(&ctx, &mut panel, &state, vec![]);

    let cells = strip_cells(layout(screen()).strip, 2);
    let (filled, stroked) = painted(&output);

    // The border is stroked inside the cell, so it is a rect within a pixel of the cell's own bounds.
    let outlined = |cell: Rect| {
        stroked.iter().any(|rect| {
            (rect.left() - cell.left()).abs() < 2.0
                && (rect.width() - cell.width()).abs() < 2.0
                && (rect.height() - cell.height()).abs() < 2.0
        })
    };
    assert!(outlined(cells[1]), "the selected cell's border is drawn");
    assert!(!outlined(cells[0]), "and its neighbour's is not");

    // The lamp is drawn either way — it is the *colour* that says whether that device is playing — and it is the only small
    // filled shape inside a cell: the face and the MUTE key are much wider.
    let lamp = |cell: Rect| {
        filled
            .iter()
            .find(|(rect, _)| {
                rect.width() < 20.0 && rect.width() > 2.0 && cell.contains_rect(*rect)
            })
            .map(|(_, colour)| *colour)
    };
    assert_eq!(
        lamp(cells[1]),
        Some(acid_app::ui::theme::LED_ON),
        "a running device's lamp is lit"
    );
    assert_eq!(
        lamp(cells[0]),
        Some(acid_app::ui::theme::LED_OFF),
        "and a stopped one's is dark"
    );
}
