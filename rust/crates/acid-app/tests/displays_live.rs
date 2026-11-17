//! The tempo's digits: the well must actually have a number in it. This exists because the number silently vanished in round
//! 3a and only a screenshot noticed — a test that asserts the text is *there* would have failed loudly.

use acid_app::state::SharedState;
use acid_app::ui::layout::layout;
use acid_app::ui::{PanelState, draw_panel, panel};
use eframe::egui::{Context, Pos2, RawInput, Rect, Shape, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))
}

/// Every painted bar and string, with where it was put. A seven-segment display paints **rects**, not text: a test that looks
/// for text in a display well is asking the wrong question — the first version of this file did, and passed a blank well.
fn texts(ctx: &Context, panel: &mut PanelState, shared: &SharedState) -> Vec<(Pos2, String)> {
    fn walk(shape: &Shape, out: &mut Vec<(Pos2, String)>) {
        match shape {
            Shape::Text(text) => out.push((text.pos, text.galley.text().to_string())),
            Shape::Rect(rect) => out.push((rect.rect.center(), "bar".to_string())),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }
    let input = RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

#[test]
fn the_tempo_well_holds_a_number() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let ctx = Context::default();
    let top = layout(screen()).top;
    let well = panel::tempo_display_rect(top);

    let said = texts(&ctx, &mut panel, &shared);
    let inside: Vec<&(Pos2, String)> = said.iter().filter(|(at, _)| well.contains(*at)).collect();
    let bars = inside.iter().filter(|(_, what)| what == "bar").count();
    assert!(
        bars > 4,
        "the tempo well at {well:?} holds {bars} bars; the whole band painted: {said:?}"
    );
}

#[test]
fn the_pattern_well_holds_its_name() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let ctx = Context::default();
    let top = layout(screen()).top;
    let well = panel::pattern_display_rect(top);
    let said = texts(&ctx, &mut panel, &shared);
    let bars = said
        .iter()
        .filter(|(at, what)| well.contains(*at) && what == "bar")
        .count();
    assert!(
        bars > 4,
        "the pattern well at {well:?} holds {bars} bars; painted: {said:?}"
    );
}
