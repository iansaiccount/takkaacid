//! Transport: the audio thread publishes its step so the LEDs can chase it, and the panel's buttons write
//! the transport and tempo back.

use acid_app::audio::renderer::AudioRenderer;
use acid_app::device::Scheme;
use acid_app::state::SharedState;
use acid_app::ui::theme;
use acid_app::ui::{PanelState, draw_panel, layout, transport, transport::nudge_tempo};
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

fn window() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(980.0, 560.0))
}

/// One egui frame of the panel, so a test can read the shapes it painted.
fn frame(
    ctx: &egui::Context,
    panel: &mut PanelState,
    shared: &SharedState,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut input = egui::RawInput {
        screen_rect: Some(window()),
        ..Default::default()
    };
    input.events = events;
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

/// The outline colours the frame painted on `rect`: a key's border and, when hovered, the ring inside it.
fn outlines(output: &egui::FullOutput, rect: egui::Rect) -> Vec<egui::Color32> {
    fn walk(shape: &egui::Shape, rect: egui::Rect, out: &mut Vec<egui::Color32>) {
        match shape {
            egui::Shape::Rect(shape) => {
                let close = |a: f32, b: f32| (a - b).abs() < 1.0;
                if shape.stroke.width > 0.0
                    && close(shape.rect.left(), rect.left())
                    && close(shape.rect.top(), rect.top())
                    && close(shape.rect.width(), rect.width())
                    && close(shape.rect.height(), rect.height())
                {
                    out.push(shape.stroke.color);
                }
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, rect, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, rect, &mut out);
    }
    out
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
        frame(&ctx, panel, shared, events);
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
    let view = acid_app::ui::step_key::step_view(
        &Step::rest(),
        position,
        position,
        playing,
        false,
        false,
        true,
    );
    assert!(!view.current, "an LED must not stay lit after stop");
}

#[test]
fn tempo_nudges_land_on_whole_bpm_within_range() {
    assert_eq!(nudge_tempo(130.0, 1), 131.0);
    assert_eq!(nudge_tempo(300.0, 1), 300.0);
    assert_eq!(nudge_tempo(40.0, -1), 40.0);
}

#[test]
fn the_run_button_asks_for_the_transport() {
    // The rack owns the transport; the row asks for a change and `main` fans it out. A row that wrote the displayed
    // device's own atomic would be undone by the next sync.
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    click_transport(&shared, &mut panel, 0);
    assert_eq!(
        panel.transport_request,
        Some(transport::Request::Play),
        "RUN must ask the rack to start"
    );
    assert!(!shared.is_playing(), "and must not write the device itself");
    click_transport(&shared, &mut panel, 1);
    assert_eq!(panel.transport_request, Some(transport::Request::Stop));
}

#[test]
fn the_tempo_buttons_ask_for_a_nudge() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    click_transport(&shared, &mut panel, 3); // TEMPO +
    assert_eq!(
        panel.transport_request,
        Some(transport::Request::NudgeTempo(1))
    );
    click_transport(&shared, &mut panel, 2); // TEMPO −
    assert_eq!(
        panel.transport_request,
        Some(transport::Request::NudgeTempo(-1))
    );
}

/// A hovered key's ring is ink on that key's own face: a lit key is a raised surface and takes `lit_ink`, an unlit one is a
/// well and keeps the accent. Written on the amber device, where the two are different colours — in black they are equal
/// by construction, so a black-scheme test could not tell the two halves apart.
#[test]
fn a_hovered_key_rings_itself_in_ink_that_reads_on_its_face() {
    // The transport is stopped, so STOP is the lit key and RUN is the unlit one: both halves in one frame.
    let shared = SharedState::new();
    let mut panel = PanelState {
        scheme: Scheme::Yellow,
        ..PanelState::default()
    };
    let ctx = egui::Context::default();
    let cells = layout::knob_cells(
        layout::layout(window()).transport,
        transport::TRANSPORT_CELLS,
    );

    // Two frames: the first lays the key out, the second moves the pointer onto it and paints the ring.
    let ring = |panel: &mut PanelState, index: usize| {
        let at = cells[index].center();
        frame(&ctx, panel, &shared, vec![egui::Event::PointerMoved(at)]);
        let output = frame(&ctx, panel, &shared, vec![egui::Event::PointerMoved(at)]);
        outlines(&output, cells[index])
    };

    let stop = ring(&mut panel, 1);
    assert!(
        stop.contains(&theme::YELLOW.lit_ink),
        "a lit key's ring must be ink for its raised face: {stop:?}"
    );
    assert!(
        !stop.contains(&theme::YELLOW.led_accent),
        "and not the accent, which is ink for the face and unreadable on a raised key: {stop:?}"
    );

    let run = ring(&mut panel, 0);
    assert!(
        run.contains(&theme::YELLOW.led_accent),
        "an unlit key's ring must be the accent on its well: {run:?}"
    );
}
