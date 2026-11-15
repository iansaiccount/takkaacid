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

    let expected: Vec<Option<transport::Request>> = vec![
        None,                            // Back
        Some(transport::Request::Play),  // Play
        Some(transport::Request::Pause), // Pause
        Some(transport::Request::Stop),  // Stop
        None,                            // Forward
        None,                            // Record
    ];

    for (key, expected) in tape::TapeKey::ALL.iter().zip(expected) {
        let mut panel = PanelState::default();
        let ctx = Context::default();
        run(&ctx, &mut panel, &shared, vec![]);
        let rect = cells[tape::TapeKey::ALL
            .iter()
            .position(|k| k == key)
            .expect("the catalogue is the index")];
        click(&ctx, &mut panel, &shared, rect.center());
        assert_eq!(
            panel.transport_request, expected,
            "{key:?} at {rect:?} asked for the wrong thing"
        );
    }

    // And the stepper: `−` and `+` either side of the readout.
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

/// A lit deck is the transport's, not a guess: play lights while the rack plays, pause while it is paused.
#[test]
fn the_deck_lights_from_the_transport() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let ctx = Context::default();
    for state in Transport::ALL {
        shared.set_transport(state);
        run(&ctx, &mut panel, &shared, vec![]);
        assert_eq!(shared.transport(), state);
    }
}
