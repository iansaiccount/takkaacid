//! Knob drags must reach `SharedState`, because that is the only path to the engine: `draw_panel` writes the
//! value, `AudioRenderer::render_mono` snapshots it and `apply` pushes it into the engine's knobs.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel, layout};
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

/// The WAVEFORM knob is a switch with four detents. A drag arrives over many frames, so the value has to accumulate —
/// re-deriving it from the switch state every frame would pin it at the extreme.
#[test]
fn dragging_the_waveform_knob_moves_one_wave_per_detent() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    assert_eq!(shared.waveform(), Waveform::Saw, "launch waveform");

    // 180 px of travel over four detents: 60 px is one wave, so 4 x 20 px steps reach the square and stop on it.
    drag_knob_in_steps(&shared, &mut panel, 0, 20.0, 4);
    assert_eq!(
        shared.waveform(),
        Waveform::Square,
        "one detent up is the next wave"
    );
    // The pointer sits between detents while the hand is down; the value is snapped when it comes up (the arithmetic is
    // covered by `the_waveform_knob_is_a_switch_with_a_detent_per_wave`).
    assert!(
        panel.knobs[0] > 1.0 / 3.0 && panel.knobs[0] < 2.0 / 3.0,
        "the drag accumulated: {}",
        panel.knobs[0]
    );

    // Another step's worth carries on to the sine.
    drag_knob_in_steps(&shared, &mut panel, 0, 20.0, 4);
    assert_eq!(shared.waveform(), Waveform::Sine);
}

/// Dragging back a detent returns to the previous wave, and the ends of the travel are hard stops.
#[test]
fn the_waveform_knob_stops_at_the_ends_of_the_catalogue() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    // All the way up: the triangle, and no further.
    drag_knob_in_steps(&shared, &mut panel, 0, 40.0, 12);
    assert_eq!(shared.waveform(), Waveform::Triangle);
    assert!((panel.knobs[0] - 1.0).abs() < 0.01);

    // All the way back down: the saw, and no further.
    drag_knob_in_steps(&shared, &mut panel, 0, -40.0, 12);
    assert_eq!(shared.waveform(), Waveform::Saw);
    assert!(panel.knobs[0].abs() < 0.01);
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
fn the_waveform_knob_is_a_switch_with_a_detent_per_wave() {
    use acid_app::ui::{waveform_from_position, waveform_position};

    // Four detents, evenly spread: the knob can rest anywhere, and always means exactly one wave.
    assert_eq!(waveform_position(Waveform::Saw), 0.0);
    assert_eq!(waveform_position(Waveform::Square), 1.0 / 3.0);
    assert_eq!(waveform_position(Waveform::Sine), 2.0 / 3.0);
    assert_eq!(waveform_position(Waveform::Triangle), 1.0);

    // No gaps and no overlap: either side of each midpoint stays on its own wave.
    for wave in Waveform::ALL {
        let position = waveform_position(wave);
        assert_eq!(
            waveform_from_position(position),
            wave,
            "{wave:?} at its detent"
        );
        assert_eq!(
            waveform_from_position(position - 0.02),
            wave,
            "{wave:?} just below"
        );
        assert_eq!(
            waveform_from_position(position + 0.02),
            wave,
            "{wave:?} just above"
        );
    }
    assert_eq!(waveform_from_position(0.0), Waveform::Saw);
    assert_eq!(waveform_from_position(1.0), Waveform::Triangle);
    // A position out of range still names a wave rather than panicking.
    assert_eq!(waveform_from_position(-1.0), Waveform::Saw);
    assert_eq!(waveform_from_position(2.0), Waveform::Triangle);
}
