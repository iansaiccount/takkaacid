//! Controller pickup, seen from the panel: a fader that has not reached its knob must not move the face.
//!
//! The state machine is tested in `midi::cc`; this walks the path a player actually sees — a CC message, then a drawn
//! frame — because the panel re-derives the tone knobs from the shared state every frame.

use acid_app::midi::cc;
use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel};
use eframe::egui::{Context, RawInput, Rect, vec2};

fn window() -> Rect {
    Rect::from_min_size(egui::Pos2::ZERO, vec2(980.0, 560.0))
}

fn frame(ctx: &Context, panel: &mut PanelState, shared: &SharedState) {
    let input = RawInput {
        screen_rect: Some(window()),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
}

#[test]
fn a_fader_that_has_not_reached_the_knob_leaves_the_face_alone() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    // A song (or the last session) left the cutoff open.
    shared.cutoff.set(0.8);
    frame(&ctx, &mut panel, &shared);
    assert!((panel.knobs[2] - 0.8).abs() < 1e-6);

    // The fader is parked at the bottom: the knob must not jump to it.
    cc::apply_cc(&shared, 74, 0);
    frame(&ctx, &mut panel, &shared);
    assert!(
        (panel.knobs[2] - 0.8).abs() < 1e-6,
        "the face jumped to {}",
        panel.knobs[2]
    );

    // Sweeping up to and past the knob takes over; the face follows.
    cc::apply_cc(&shared, 74, 127);
    frame(&ctx, &mut panel, &shared);
    assert!(
        (panel.knobs[2] - 1.0).abs() < 1e-6,
        "the face follows after the sweep"
    );
}
