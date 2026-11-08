//! The highlight that says "this one can be taught", seen from a headless frame.
//!
//! egui hands back every shape it painted, so a colour that only a learn draws can be counted from a test: nothing else on
//! the panel uses it, which is the point of choosing it.

use acid_app::state::SharedState;
use acid_app::ui::theme;
use acid_app::ui::{PanelState, draw_panel, panel};
use eframe::egui::{self, Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

fn window() -> Rect {
    Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE)
}

fn screen() -> RawInput {
    RawInput {
        screen_rect: Some(window()),
        ..Default::default()
    }
}

/// Run the frames, dropping textures, and return the last frame's output.
fn run(
    panel: &mut PanelState,
    shared: &SharedState,
    frames: Vec<Vec<Event>>,
) -> eframe::egui::FullOutput {
    let ctx = Context::default();
    let mut frame = |events: Vec<Event>| {
        let mut input = screen();
        input.events = events;
        let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
        output.textures_delta.clear();
        output
    };

    // The first frame always runs, so the caller can pass a bare event list for a single frame.
    let mut frames = frames.into_iter();
    let mut output = frame(frames.next().unwrap_or_default());
    for events in frames {
        output = frame(events);
    }
    output
}

/// Every stroke painted in the learn colour.
fn learn_strokes(output: &eframe::egui::FullOutput) -> usize {
    output
        .shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(rect) if rect.stroke.color == theme::LEARN_TARGET)
        })
        .count()
}

/// One click: hover, press, release — three frames, because a press and a release in the same frame is not a click.
fn click(at: Pos2) -> Vec<Vec<Event>> {
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    vec![
        vec![Event::PointerMoved(at)],
        vec![button(true)],
        vec![button(false)],
    ]
}

fn learn_centre() -> Pos2 {
    let cells = acid_app::ui::layout::knob_cells(
        acid_app::ui::layout::layout(window()).transport,
        acid_app::ui::transport::TRANSPORT_CELLS,
    );
    cells[4].center()
}

#[test]
fn nothing_is_highlighted_when_no_learn_is_running() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let output = run(&mut panel, &shared, vec![vec![]]);
    assert_eq!(learn_strokes(&output), 0);
    assert_eq!(panel.learn, acid_app::ui::learn::Step::Idle);
}

#[test]
fn every_teachable_control_wears_the_pick_colour_while_choosing() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    run(&mut panel, &shared, click(learn_centre()));
    assert_eq!(panel.learn, acid_app::ui::learn::Step::Choosing);

    let output = run(&mut panel, &shared, vec![vec![]]);
    // Seven tone knobs, VOLUME, the BPM readout, the PATTERN readout and SHUFFLE.
    assert_eq!(
        learn_strokes(&output),
        11,
        "expected every teachable control to be outlined"
    );
}

#[test]
fn the_picked_control_keeps_its_border_while_listening() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    run(&mut panel, &shared, click(learn_centre()));

    // Pick the BPM readout: the "which one?" question is answered, and that readout alone keeps the border — it is the
    // control the next message from the controller will bind.
    let at = panel::tempo_display_rect(acid_app::ui::layout::layout(window()).top).center();
    run(&mut panel, &shared, click(at));
    assert_eq!(
        panel.learn,
        acid_app::ui::learn::Step::Listening(acid_app::midi::cc::CcTarget::Tempo)
    );
    let output = run(&mut panel, &shared, vec![vec![]]);
    assert_eq!(learn_strokes(&output), 1, "the target lost its border");
}
#[test]
fn the_pick_colour_is_not_one_of_the_panels_own_lights() {
    // It has to read as a different kind of light: far from amber, and never used for a lit key.
    let learn = theme::LEARN_TARGET;
    for own in [theme::SILKSCREEN, theme::LED_ON, theme::LED_ACCENT] {
        let distance = (i32::from(learn.r()) - i32::from(own.r())).abs()
            + (i32::from(learn.g()) - i32::from(own.g())).abs()
            + (i32::from(learn.b()) - i32::from(own.b())).abs();
        assert!(distance > 150, "too close to a panel light: {distance}");
    }
    // And the highlight is drawn outside the control, so it never covers what it points at.
    let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(40.0, 40.0));
    assert!(panel::learn_highlight_bounds(rect, 1.0).contains_rect(rect));
}
