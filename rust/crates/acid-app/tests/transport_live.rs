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
fn click_transport(shared: &SharedState, panel: &mut PanelState, which: transport::Cell) {
    let ctx = egui::Context::default();
    let pos = transport::cell(layout::layout(window()).transport, which).center();
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
    s.set_transport(acid_app::device::Transport::Playing);
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
    s.set_transport(acid_app::device::Transport::Playing);
    s.set_pattern(std::sync::Arc::new(Pattern::from_steps(
        [Step::note(Pitch::A, -1); 16],
    )));
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 2_048);
    r.render_into(&mut vec![0.0f32; 4_096]);
    s.set_transport(acid_app::device::Transport::Stopped);
    r.render_into(&mut vec![0.0f32; 4_096]);
    let position = s.position.load(std::sync::atomic::Ordering::Relaxed);
    let playing = s.is_playing();
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
fn the_reverse_key_asks_for_a_direction() {
    // The row asks; the device's own engine takes the direction at the bar line. A row that wrote the atomic would be undone
    // by the next fan-out, which is the rule every key in this row follows.
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    click_transport(&shared, &mut panel, transport::Cell::Reverse);
    assert_eq!(
        panel.transport_request,
        Some(transport::Request::Reverse(true)),
        "REVERSE off asks for the line to run backwards"
    );
    assert!(
        !shared.is_reversed(),
        "and must not write the device itself"
    );

    // Lit from the state the device is in, so a second press asks for the way back.
    shared.set_reversed(true);
    click_transport(&shared, &mut panel, transport::Cell::Reverse);
    assert_eq!(
        panel.transport_request,
        Some(transport::Request::Reverse(false))
    );
}

#[test]
fn the_transpose_keys_ask_for_a_semitone_each_way() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    click_transport(&shared, &mut panel, transport::Cell::TransposeUp);
    assert_eq!(
        panel.transport_request,
        Some(transport::Request::Transpose(1))
    );
    click_transport(&shared, &mut panel, transport::Cell::TransposeDown);
    assert_eq!(
        panel.transport_request,
        Some(transport::Request::Transpose(-1))
    );
}

#[test]
fn both_tbc_keys_say_what_they_are() {
    // Two cells kept for the player, with nothing decided behind either. They say so rather than doing nothing: a silent key
    // reads as broken, and these have no feature to be silent about.
    for which in [transport::Cell::Tbc, transport::Cell::Spare] {
        let shared = SharedState::new();
        let mut panel = PanelState::default();
        click_transport(&shared, &mut panel, which);
        assert_eq!(
            panel.transport_request, None,
            "{which:?} asks the rack for nothing"
        );
        assert_eq!(
            panel.notice_now(std::time::Instant::now()),
            Some("TBC IS NOT DECIDED YET")
        );
    }
}

/// A hovered key's ring is ink on that key's own face: a lit key is a raised surface and takes `lit_ink`, an unlit one is a
/// well and keeps the accent. Written on the amber device, where the two are different colours — in black they are equal
/// by construction, so a black-scheme test could not tell the two halves apart.
#[test]
fn a_hovered_key_rings_itself_in_ink_that_reads_on_its_face() {
    // REVERSE unlit is the well key, REVERSE lit is the raised one: both halves in one frame.
    let shared = SharedState::new();
    let mut panel = PanelState {
        scheme: Scheme::AmbientMoss,
        ..PanelState::default()
    };
    let ctx = egui::Context::default();
    // Two frames: the first lays the key out, the second moves the pointer onto it and paints the ring.
    let ring = |panel: &mut PanelState, which: transport::Cell| {
        let rect = transport::cell(layout::layout(window()).transport, which);
        let at = rect.center();
        frame(&ctx, panel, &shared, vec![egui::Event::PointerMoved(at)]);
        let output = frame(&ctx, panel, &shared, vec![egui::Event::PointerMoved(at)]);
        outlines(&output, rect)
    };

    shared.set_reversed(true);
    let stop = ring(&mut panel, transport::Cell::Reverse);
    assert!(
        stop.contains(&theme::palette(Scheme::AmbientMoss).lit_ink),
        "a lit key's ring must be ink for its raised face: {stop:?}"
    );
    assert!(
        !stop.contains(&theme::palette(Scheme::AmbientMoss).led_accent),
        "and not the accent, which is ink for the face and unreadable on a raised key: {stop:?}"
    );

    shared.set_reversed(false);
    let run = ring(&mut panel, transport::Cell::Reverse);
    assert!(
        run.contains(&theme::palette(Scheme::AmbientMoss).led_accent),
        "an unlit key's ring must be the accent on its well: {run:?}"
    );
}

/// The transpose's readout is a cell with nothing to click: the row answers at every cell but the readout's, which is what
/// makes it a display rather than a key that does nothing.
#[test]
fn the_transpose_readout_answers_nothing() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let at = transport::cell(layout::layout(window()).transport, transport::Cell::Readout).center();
    let ctx = egui::Context::default();
    frame(&ctx, &mut panel, &shared, vec![]);
    frame(
        &ctx,
        &mut panel,
        &shared,
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ],
    );
    assert_eq!(
        panel.transport_request, None,
        "the readout is not a key: it asks the rack for nothing"
    );
    assert_eq!(
        panel.notice_now(std::time::Instant::now()),
        None,
        "and says nothing either, because it is not a key that has no answer"
    );
}

/// Each legend is painted inside the cell the catalogue names for it.
///
/// Every other test here finds its cell through `transport::cell`, which is the same function the drawing uses — so binding
/// a key's *cell* to the wrong name paints that key's word over its neighbour's rect and no click lands wrong. This reads the
/// painted text instead, which is the only thing in the row that knows which word belongs to which rect.
#[test]
fn each_legend_is_painted_in_the_cell_that_names_it() {
    use eframe::egui::Shape;

    fn walk(shape: &Shape, said: &mut Vec<(egui::Pos2, String)>) {
        match shape {
            Shape::Text(text) => said.push((text.pos, text.galley.text().to_string())),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, said)),
            _ => {}
        }
    }

    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let ctx = egui::Context::default();
    let output = frame(&ctx, &mut panel, &shared, vec![]);

    let mut said = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut said);
    }

    let rect = |which: transport::Cell| transport::cell(layout::layout(window()).transport, which);
    // The readout prints the transpose as it stands, and this session has it at zero.
    for (legend, which) in [
        ("REVERSE", transport::Cell::Reverse),
        ("TRANS -", transport::Cell::TransposeDown),
        ("+0", transport::Cell::Readout),
        ("TRANS +", transport::Cell::TransposeUp),
        ("LEARN", transport::Cell::Learn),
    ] {
        assert!(
            said.iter()
                .any(|(pos, text)| text == legend && rect(which).contains(*pos)),
            "no {legend:?} painted inside {which:?} at {:?}",
            rect(which)
        );
    }

    // Both `TBC` cells say it. They are the two cells the catalogue cannot tell apart by their drawing, so this pins that
    // both are painted and that each is painted in one of the two rather than somewhere else in the row.
    let tbc: Vec<egui::Pos2> = said
        .iter()
        .filter(|(_, text)| text == "TBC")
        .map(|(pos, _)| *pos)
        .collect();
    assert_eq!(tbc.len(), 2, "both spare cells print TBC");
    for pos in tbc {
        assert!(
            rect(transport::Cell::Tbc).contains(pos) || rect(transport::Cell::Spare).contains(pos),
            "a TBC is painted outside the two spare cells"
        );
    }
}
