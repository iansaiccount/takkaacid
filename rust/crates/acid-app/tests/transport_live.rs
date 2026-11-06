//! Transport: the audio thread publishes its step so the LEDs can chase it, and the panel's buttons write
//! the transport and tempo back.

use acid_app::audio::renderer::AudioRenderer;
use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel, layout, transport, transport::nudge_tempo};
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

fn window() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(980.0, 560.0))
}

/// Click one cell of the transport row through three egui frames (warm-up, press, release).
fn click_transport(shared: &SharedState, panel: &mut PanelState, index: usize) {
    let ctx = egui::Context::default();
    let pos = layout::knob_cells(
        layout::layout(window()).transport,
        transport::TRANSPORT_CELLS,
    )[index]
        .center();
    let button = |pressed: bool| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    for events in [
        vec![],
        vec![egui::Event::PointerMoved(pos), button(true)],
        vec![button(false)],
    ] {
        let mut input = egui::RawInput {
            screen_rect: Some(window()),
            ..Default::default()
        };
        input.events = events;
        let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
        output.textures_delta.clear();
    }
}

#[test]
fn position_is_published_and_never_exceeds_fifteen() {
    let s = SharedState::new();
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);
    s.tempo_bpm.set(300.0);
    s.set_pattern(std::sync::Arc::new(Pattern::from_steps(
        [Step::note(Pitch::A, -1); 16],
    )));
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 2_048);
    let mut out = vec![0.0f32; 4_096];
    for _ in 0..200 {
        r.render_into(&mut out);
        assert!(
            s.position.load(std::sync::atomic::Ordering::Relaxed) < 16,
            "the chase must stay inside the pattern"
        );
    }
    assert!(s.position.load(std::sync::atomic::Ordering::Relaxed) > 0);
}

#[test]
fn stopping_clears_the_chase() {
    let s = SharedState::new();
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);
    s.set_pattern(std::sync::Arc::new(Pattern::from_steps(
        [Step::note(Pitch::A, -1); 16],
    )));
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 2_048);
    r.render_into(&mut vec![0.0f32; 4_096]);
    s.playing.store(false, std::sync::atomic::Ordering::Relaxed);
    r.render_into(&mut vec![0.0f32; 4_096]);
    let position = s.position.load(std::sync::atomic::Ordering::Relaxed);
    let playing = s.playing.load(std::sync::atomic::Ordering::Relaxed);
    let view = acid_app::ui::step_key::step_view(&Step::rest(), position, position, playing);
    assert!(!view.current, "an LED must not stay lit after stop");
}

#[test]
fn tempo_nudges_land_on_whole_bpm_within_range() {
    assert_eq!(nudge_tempo(130.0, 1), 131.0);
    assert_eq!(nudge_tempo(300.0, 1), 300.0);
    assert_eq!(nudge_tempo(40.0, -1), 40.0);
}

#[test]
fn the_run_button_starts_the_transport() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    click_transport(&shared, &mut panel, 0);
    assert!(
        shared.is_playing(),
        "RUN must set the shared transport flag"
    );
}

#[test]
fn the_tempo_buttons_nudge_the_shared_tempo() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let before = shared.tempo_bpm.get();
    click_transport(&shared, &mut panel, 3); // TEMPO +
    assert!(
        (shared.tempo_bpm.get() - (before + 1.0)).abs() < 1e-6,
        "TEMPO + should add one BPM: {} -> {}",
        before,
        shared.tempo_bpm.get()
    );
    click_transport(&shared, &mut panel, 2); // TEMPO −
    assert!((shared.tempo_bpm.get() - before).abs() < 1e-6);
}
