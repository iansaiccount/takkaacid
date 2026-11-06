//! Knob drags must reach `SharedState`, because that is the only path to the engine: `draw_panel` writes the
//! value, `AudioRenderer::render_mono` snapshots it and `apply` pushes it into the engine's knobs.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel, layout, waveform_from_knob};
use acid_core::oscillator::Waveform;

fn window() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(980.0, 560.0))
}

/// Drive `draw_panel` through a warm-up frame, a press on `cell`, and a vertical drag of `dy` pixels.
fn drag_knob(shared: &SharedState, panel: &mut PanelState, index: usize, dy: f32) -> f32 {
    let ctx = egui::Context::default();
    let pos = layout::layout(window()).knobs.cells[index].center();
    let press = egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    };
    let frames = [
        vec![],
        vec![egui::Event::PointerMoved(pos), press],
        vec![egui::Event::PointerMoved(pos - egui::vec2(0.0, dy))],
    ];
    for events in frames {
        let mut input = egui::RawInput {
            screen_rect: Some(window()),
            ..Default::default()
        };
        input.events = events;
        let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
        output.textures_delta.clear();
    }
    panel.knobs[index]
}

#[test]
fn dragging_the_tuning_knob_publishes_to_the_shared_state() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    drag_knob(&shared, &mut panel, 1, 18.0); // one tenth of the travel, upwards
    assert!(
        (shared.tuning.get() - 0.6).abs() < 0.02,
        "the engine reads tuning from SharedState: {}",
        shared.tuning.get()
    );
}

#[test]
fn every_knob_reaches_its_own_shared_field() {
    let shared = SharedState::new();
    for (index, read) in [
        (
            2usize,
            (|s: &SharedState| s.cutoff.get()) as fn(&SharedState) -> f32,
        ),
        (3, |s: &SharedState| s.resonance.get()),
        (4, |s: &SharedState| s.env_mod.get()),
        (5, |s: &SharedState| s.decay.get()),
        (6, |s: &SharedState| s.accent.get()),
    ] {
        let mut panel = PanelState::default();
        let before = read(&shared);
        drag_knob(&shared, &mut panel, index, 18.0);
        assert!(
            (read(&shared) - before - 0.1).abs() < 0.02,
            "knob {index} did not move its own field: {} -> {}",
            before,
            read(&shared)
        );
    }
}

#[test]
fn the_waveform_knob_is_a_switch_at_the_midpoint() {
    assert_eq!(waveform_from_knob(0.0), Waveform::Saw);
    assert_eq!(waveform_from_knob(0.49), Waveform::Saw);
    assert_eq!(waveform_from_knob(0.5), Waveform::Square);
    assert_eq!(waveform_from_knob(1.0), Waveform::Square);
}
