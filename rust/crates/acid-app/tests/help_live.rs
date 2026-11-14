//! The help card's words, read back off the paint. The card is the only manual, so what it says about the strip, the
//! device card and the save limit is behaviour a player can be misled by — the card's own unit tests pin the gestures,
//! and this one pins the words the card actually paints inside a frame.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel};
use eframe::egui::{Context, Event, FullOutput, Pos2, RawInput, Rect, vec2};

fn run_on(
    ctx: &Context,
    panel: &mut PanelState,
    shared: &SharedState,
    events: Vec<Event>,
) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

/// Every line of text the frame drew, in paint order. Copied from `device_card_live.rs`: the card's words are part of
/// what it says, and a needle in `help_sections()` would still pass if the card never painted.
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

#[test]
fn the_help_card_names_the_devices_and_what_is_not_saved() {
    let state = SharedState::new();
    let mut panel = PanelState {
        help_visible: true,
        ..Default::default()
    };
    let ctx = Context::default();
    let said = strings(&run_on(&ctx, &mut panel, &state, vec![])).join(" ");

    for wanted in [
        "F3",
        "Ctrl+1",
        "Ctrl+C",
        "Ctrl+V",
        "MUTE",
        "four devices",
        "not saved",
    ] {
        assert!(
            said.contains(wanted),
            "the help card should mention {wanted}: {said}"
        );
    }
}
