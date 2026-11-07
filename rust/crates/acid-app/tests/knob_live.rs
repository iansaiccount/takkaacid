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

/// Drag a knob in several frames, the way a hand actually moves, and return the knob's final value.
fn drag_knob_in_steps(
    shared: &SharedState,
    panel: &mut PanelState,
    index: usize,
    step_pixels: f32,
    steps: usize,
) -> f32 {
    let ctx = egui::Context::default();
    let pos = layout::layout(window()).knobs.cells[index].center();
    let press = egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    };
    let mut frames = vec![vec![], vec![egui::Event::PointerMoved(pos), press]];
    for step in 1..=steps {
        let dy = step_pixels * step as f32;
        frames.push(vec![egui::Event::PointerMoved(pos - egui::vec2(0.0, dy))]);
    }
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

/// The WAVEFORM knob is a switch drawn as a knob. A real drag arrives over many frames, so the value has to
/// accumulate — re-deriving it from the switch state every frame pins it at the extreme.
#[test]
fn dragging_the_waveform_knob_across_the_midpoint_selects_the_square_wave() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    assert_eq!(shared.waveform(), Waveform::Saw, "launch waveform");
    // Four frames of 30 px = 120 px, two thirds of the travel: comfortably past the midpoint.
    drag_knob_in_steps(&shared, &mut panel, 0, 30.0, 4);
    assert_eq!(
        shared.waveform(),
        Waveform::Square,
        "dragging past the midpoint must select the square wave"
    );
    assert!(
        panel.knobs[0] > 0.5,
        "the knob should stay where the hand left it: {}",
        panel.knobs[0]
    );
}

/// Dragging back below the midpoint returns to the saw.
#[test]
fn dragging_the_waveform_knob_back_below_the_midpoint_returns_to_saw() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    drag_knob_in_steps(&shared, &mut panel, 0, 30.0, 4);
    assert_eq!(shared.waveform(), Waveform::Square);
    // Down 100 px from there, in two frames.
    drag_knob_in_steps(&shared, &mut panel, 0, -50.0, 2);
    assert_eq!(shared.waveform(), Waveform::Saw);
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
