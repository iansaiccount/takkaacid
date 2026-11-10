//! Hover help: on by default, and the controls that are teachable say so when the pointer rests on them.
//!
//! The TIPS and AUTO keys used to live in the top strip and were clicked from here. Both are now switches in the settings
//! card, and `settings_live` clicks them there; what stays here is that the help is on to begin with and the readouts
//! still answer the pointer.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel};
use eframe::egui::{Context, Event, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

/// Every piece of text egui painted this frame, tooltips included: a tooltip is drawn into a floating layer, so its galley
/// arrives in the same shape list as the panel's own captions.
fn painted_text(output: &eframe::egui::FullOutput) -> Vec<String> {
    fn walk(shape: &eframe::egui::Shape, out: &mut Vec<String>) {
        match shape {
            eframe::egui::Shape::Text(text) => out.push(text.galley.text().to_string()),
            eframe::egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

#[test]
fn the_help_is_on_when_the_app_opens() {
    assert!(
        PanelState::default().tooltips,
        "a player should not have to find the switch to be told what things do"
    );
}

/// `body` is a phrase from the tip's *body*, not its title: the panel paints `TEMPO -` and `TEMPO +` on its transport
/// keys, so matching the title alone proves nothing about a tooltip.
#[test]
fn the_two_readouts_answer_the_pointer() {
    // A readout is a control here — both are teachable — so both explain themselves on hover. The pointer settles on the
    // first frame and then stays: egui waits for the hand to stop moving before it shows a tooltip, so a PointerMoved
    // every frame at one position keeps it away (this test used to pass on the panel's own `TEMPO -` caption).
    let l = acid_app::ui::layout::layout(screen());
    for (rect, body) in [
        (
            acid_app::ui::panel::tempo_display_rect(l.top),
            "How fast the sequencer runs",
        ),
        (
            acid_app::ui::panel::pattern_display_rect(l.top),
            "Which pattern is playing",
        ),
    ] {
        let ctx = Context::default();
        let shared = SharedState::new();
        let mut panel = PanelState::default();
        let at = rect.center();
        let mut seen: Vec<String> = Vec::new();
        for step in 0..5 {
            let events = if step == 0 {
                vec![Event::PointerMoved(at)]
            } else {
                Vec::new()
            };
            let input = RawInput {
                screen_rect: Some(screen()),
                time: Some(f64::from(step)),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
            output.textures_delta.clear();
            seen.extend(painted_text(&output));
        }
        assert!(
            seen.iter().any(|text| text.contains(body)),
            "the {body:?} readout showed no help; painted: {seen:?}"
        );
    }
}

/// The switch that silences the help lives in the settings card now. What this pins is what the switch *does*: with it
/// off, the readouts stop answering the pointer — same pointer, same frames, no tip. Whether the flag reaches disk is
/// `persist`'s business (`SavedState::tooltips`), not the panel's; the test this replaces asserted only the two fields it
/// had just set itself, which is no test at all.
#[test]
fn the_help_switch_silences_the_hints() {
    let l = acid_app::ui::layout::layout(screen());
    let at = acid_app::ui::panel::tempo_display_rect(l.top).center();
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState {
        tooltips: false,
        ..PanelState::default()
    };

    // The same hover technique as the test above, so this proves the switch is what silences the tip rather than the
    // pointer never having settled.
    let mut seen: Vec<String> = Vec::new();
    for step in 0..5 {
        let events = if step == 0 {
            vec![Event::PointerMoved(at)]
        } else {
            Vec::new()
        };
        let input = RawInput {
            screen_rect: Some(screen()),
            time: Some(f64::from(step)),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
        output.textures_delta.clear();
        seen.extend(painted_text(&output));
    }

    assert!(
        !seen
            .iter()
            .any(|text| text.contains("How fast the sequencer runs")),
        "help that is turned off must not talk: {seen:?}"
    );
}
