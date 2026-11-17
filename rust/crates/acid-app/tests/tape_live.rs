//! The tape deck: its keys answer where the layout puts them, and it says what it does with shapes rather than words.

use acid_app::device::Transport;
use acid_app::state::SharedState;
use acid_app::ui::layout::layout;
use acid_app::ui::{PanelState, draw_panel, panel, tape, transport};
use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))
}

fn run(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    out.textures_delta.clear();
}

fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run(ctx, panel, shared, vec![button(true)]);
    run(ctx, panel, shared, vec![button(false)]);
}

/// Every key of the deck is where `panel::tape_cells` says it is, and pressing it asks for what it promises.
///
/// The three that are drawn but not wired ask for nothing: a key that quietly dispatched something would be worse than a key
/// that does nothing, because the notice task has no chance to say why.
#[test]
fn the_deck_answers_where_the_layout_puts_it() {
    let shared = SharedState::new();
    let top = layout(screen()).top;
    let cells = panel::tape_cells(top);
    let at = |key: tape::TapeKey| {
        cells[tape::TapeKey::ALL
            .iter()
            .position(|k| *k == key)
            .expect("the catalogue is the index")]
    };

    for key in tape::TapeKey::ALL {
        // Written out per key rather than zipped with a list: a zip truncates silently when a key is added, and this is the
        // test that says what each key asks for.
        let expected = match key {
            tape::TapeKey::Play => Some(transport::Request::Play),
            tape::TapeKey::Pause => Some(transport::Request::Pause),
            tape::TapeKey::Stop => Some(transport::Request::Stop),
            tape::TapeKey::Back | tape::TapeKey::Forward | tape::TapeKey::Record => None,
        };
        let mut panel = PanelState::default();
        let ctx = Context::default();
        run(&ctx, &mut panel, &shared, vec![]);
        let rect = at(key);
        click(&ctx, &mut panel, &shared, rect.center());
        assert_eq!(
            panel.transport_request, expected,
            "{key:?} at {rect:?} asked for the wrong thing"
        );
    }

    // And the stepper: `−` and `+` stacked beside the readout, not either side of it.
    let stepper = panel::tempo_stepper_cells(top);
    for (rect, direction) in [(stepper[0], -1), (stepper[2], 1)] {
        let mut panel = PanelState::default();
        let ctx = Context::default();
        run(&ctx, &mut panel, &shared, vec![]);
        click(&ctx, &mut panel, &shared, rect.center());
        assert_eq!(
            panel.transport_request,
            Some(transport::Request::NudgeTempo(direction)),
            "the tempo stepper at {rect:?}"
        );
    }
}

/// The deck is shapes, not characters: this face may not use a font glyph, and a glyph from a system font renders as a box on
/// some machines. Painted text inside a tape rect would mean a symbol had been typed instead of drawn.
#[test]
fn the_deck_draws_shapes_and_no_words() {
    use eframe::egui::Shape;

    fn walk(shape: &Shape, said: &mut Vec<(Pos2, String)>) {
        match shape {
            Shape::Text(text) => said.push((text.pos, text.galley.text().to_string())),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, said)),
            _ => {}
        }
    }

    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let ctx = Context::default();
    let input = RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
    out.textures_delta.clear();

    let mut said = Vec::new();
    for clipped in &out.shapes {
        walk(&clipped.shape, &mut said);
    }
    let top = layout(screen()).top;
    let mut deck: Vec<Rect> = panel::tape_cells(top).to_vec();
    deck.extend(panel::tempo_stepper_cells(top));
    for (at, text) in said {
        for rect in &deck {
            assert!(
                !rect.contains(at),
                "the deck drew the word {text:?} at {at:?}: it must draw shapes, not characters"
            );
        }
    }
}

/// A lit deck is the transport's, not a guess: Play's **face** is the raised one while the rack plays, and the plain key face
/// while it does not. The first version of this test asserted a `store`/`load` round trip and never looked at a pixel, so a `lit`
/// hard-wired to `false` passed it.
#[test]
fn the_deck_lights_from_the_transport() {
    use acid_app::device::Scheme;
    use acid_app::ui::theme;
    use eframe::egui::Shape;

    fn fills(shapes: &[Shape], rect: Rect, out: &mut Vec<eframe::egui::Color32>) {
        for shape in shapes {
            match shape {
                Shape::Rect(shape) if rect.contains(shape.rect.center()) => out.push(shape.fill),
                Shape::Vec(inner) => fills(inner, rect, out),
                _ => {}
            }
        }
    }

    // An amber scheme, where a raised face and a plain key face differ: in black they are close, and the test would be weaker.
    let shared = SharedState::new();
    let mut panel = PanelState {
        scheme: Scheme::AmbientMoss,
        ..PanelState::default()
    };
    let ctx = Context::default();
    let palette = theme::palette(Scheme::AmbientMoss);
    let play = panel::tape_cells(layout(screen()).top)[1];

    let mut painted = Vec::new();
    shared.set_transport(Transport::Playing);
    let input = RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
    out.textures_delta.clear();
    painted.extend(out.shapes.iter().map(|clipped| clipped.shape.clone()));
    let mut playing = Vec::new();
    fills(&painted, play, &mut playing);
    assert!(
        playing.contains(&palette.panel_raised),
        "a playing deck's Play key is the raised face: {playing:?}"
    );

    shared.set_transport(Transport::Stopped);
    let input = RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
    out.textures_delta.clear();
    let mut stopped = Vec::new();
    let shapes: Vec<Shape> = out
        .shapes
        .iter()
        .map(|clipped| clipped.shape.clone())
        .collect();
    fills(&shapes, play, &mut stopped);
    assert!(
        stopped.contains(&palette.key_face) && !stopped.contains(&palette.panel_raised),
        "and a stopped deck's is the plain one: {stopped:?}"
    );
}
