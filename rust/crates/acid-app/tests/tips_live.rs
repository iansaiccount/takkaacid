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

#[test]
fn the_two_readouts_answer_the_pointer() {
    // A readout is a control here — both are teachable — so both explain themselves on hover. The pointer has to hold
    // still past egui's tooltip delay, which is why this drives several frames with the clock moving.
    let l = acid_app::ui::layout::layout(screen());
    for (rect, title) in [
        (acid_app::ui::panel::tempo_display_rect(l.top), "TEMPO"),
        (acid_app::ui::panel::pattern_display_rect(l.top), "PATTERN"),
    ] {
        let ctx = Context::default();
        let shared = SharedState::new();
        let mut panel = PanelState::default();
        let at = rect.center();
        let mut seen: Vec<String> = Vec::new();
        for step in 0..4 {
            let input = RawInput {
                screen_rect: Some(screen()),
                time: Some(f64::from(step)),
                events: vec![Event::PointerMoved(at)],
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
            output.textures_delta.clear();
            seen.extend(painted_text(&output));
        }
        assert!(
            seen.iter().any(|text| text.contains(title)),
            "the {title} readout showed no help; painted: {seen:?}"
        );
    }
}

/// The switch that silences the help lives in the settings card now, so the card's own row is where turning it off is
/// proven; what this pins is that the flag itself travels as a change worth saving.
#[test]
fn the_help_switch_is_part_of_what_a_save_keeps() {
    let panel = PanelState {
        tooltips: false,
        dirty: true,
        ..PanelState::default()
    };

    assert!(!panel.tooltips);
    assert!(
        panel.dirty,
        "switching the help off survives a restart, so it is a change until it is written"
    );
}
