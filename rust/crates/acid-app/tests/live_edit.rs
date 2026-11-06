//! Live editing: the audio thread picks up a new pattern at the next step boundary, and a click on a step
//! key publishes one.

use acid_app::audio::renderer::AudioRenderer;
use acid_app::state::SharedState;
use acid_app::ui::{PanelState, layout};
use acid_core::analysis::peak;
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

#[test]
fn an_edit_made_while_rendering_is_heard_on_the_next_pass() {
    let s = SharedState::new();
    s.playing
        .store(true, std::sync::atomic::Ordering::Relaxed);
    s.tempo_bpm.set(240.0); // 3000 samples per step at 48k
    s.set_pattern(std::sync::Arc::new(Pattern::empty())); // everything silent
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 4_096);

    let mut silent = vec![0.0f32; 8_192];
    r.render_into(&mut silent);
    assert!(peak(&silent) < 1e-6);

    // Turn step 0 into a note while the loop is running.
    s.set_step(0, Step::note(Pitch::A, -1));
    // Render a full pattern plus a little more, so step 0 comes round again.
    let mut buf = vec![0.0f32; 48_000 * 2];
    r.render_into(&mut buf);
    assert!(peak(&buf) > 0.01, "the edited step was never heard");
}

#[test]
fn a_thousand_edits_from_another_thread_never_break_the_pattern() {
    let s = SharedState::new();
    s.set_pattern(std::sync::Arc::new(Pattern::empty()));
    let writer = {
        let s = s.clone();
        std::thread::spawn(move || {
            for i in 0..1_000 {
                s.set_step(i % 16, Step::note(Pitch::ALL[i % 13], 0));
            }
        })
    };
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 1_024);
    let mut out = vec![0.0f32; 2_048];
    while !writer.is_finished() {
        r.render_into(&mut out);
        assert_eq!(s.pattern().steps.len(), 16);
    }
    writer.join().unwrap();
    assert!(s.pattern().steps.iter().filter(|st| st.on).count() >= 1);
}

/// The UI half: a click on a step key publishes that step to `SharedState`, which is what the renderer reads.
#[test]
fn clicking_a_step_key_toggles_it_in_the_shared_state() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let ctx = egui::Context::default();
    let cells = layout::step_cells(
        layout::layout(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(980.0, 560.0),
        ))
        .steps,
    );
    let pos = cells[3].center();

    let press = egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    };
    let release = egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    };
    // Warm-up frame, then press, then release: egui routes interaction through the previous frame.
    for events in [vec![], vec![egui::Event::PointerMoved(pos), press], vec![release]] {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(980.0, 560.0),
            )),
            ..Default::default()
        };
        input.events = events;
        let mut output = ctx.run_ui(input, |ui| {
            acid_app::ui::draw_panel(&mut panel, &shared, ui);
        });
        output.textures_delta.clear();
    }

    assert!(
        shared.pattern().steps[3].on,
        "clicking step 4 should have published a note there"
    );
}
