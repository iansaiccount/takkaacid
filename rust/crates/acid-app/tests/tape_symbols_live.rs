//! The deck's symbols and the deck's keys on the keyboard: the two things a screenshot had to catch last time.

use acid_app::device::Transport;
use acid_app::state::SharedState;
use acid_app::ui::layout::layout;
use acid_app::ui::{PanelState, dispatch, draw_panel, panel, shortcuts, tape, transport};
use eframe::egui::{Context, Key, Pos2, RawInput, Rect, Shape, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))
}

fn frame(ctx: &Context, panel: &mut PanelState, shared: &SharedState) -> Vec<Shape> {
    let input = RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .map(|clipped| clipped.shape)
        .collect()
}

/// Every path and circle the deck paints, grouped by the key it lands in. A shape belongs to the key whose rect contains its
/// centre, so a triangle drawn half outside its key would still be attributed to it — which is what the direction checks need.
fn marks(shapes: &[Shape], cells: &[Rect]) -> Vec<Vec<Shape>> {
    fn visit(shape: &Shape, cells: &[Rect], out: &mut Vec<Vec<Shape>>) {
        match shape {
            Shape::Vec(inner) => {
                for shape in inner {
                    visit(shape, cells, out);
                }
            }
            Shape::Path(path) => {
                let sum = path.points.iter().fold(Pos2::new(0.0, 0.0), |sum, p| {
                    Pos2::new(sum.x + p.x, sum.y + p.y)
                });
                let count = path.points.len().max(1) as f32;
                let centre = Pos2::new(sum.x / count, sum.y / count);
                for (index, cell) in cells.iter().enumerate() {
                    if cell.contains(centre) {
                        out[index].push(shape.clone());
                    }
                }
            }
            Shape::Circle(circle) => {
                for (index, cell) in cells.iter().enumerate() {
                    if cell.contains(circle.center) {
                        out[index].push(shape.clone());
                    }
                }
            }
            _ => {}
        }
    }

    let mut out = vec![Vec::new(); cells.len()];
    for shape in shapes {
        visit(shape, cells, &mut out);
    }
    out
}

/// The deck's cells as a slice, so `marks` does not need the array's length in its type.
fn vec_of(cells: &[Rect; tape::TapeKey::ALL.len()]) -> Vec<Rect> {
    cells.to_vec()
}

/// The shapes each key paints are the shapes it should: one arrow that points forward, two that point back, two bars, a
/// square, a circle. The arrows shipped **mirrored** once — each built by hand, each pointing the wrong way — and only a
/// screenshot caught it, so the direction is asserted now.
#[test]
fn every_deck_key_draws_its_own_symbol_the_right_way_round() {
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let ctx = Context::default();
    let shapes = frame(&ctx, &mut panel, &shared);
    let cells = panel::tape_cells(layout(screen()).top);
    let marks = marks(&shapes, &vec_of(&cells));

    // `TapeKey::ALL` order: Back, Play, Pause, Stop, Forward, Record.
    let count = |key: tape::TapeKey| {
        marks[tape::TapeKey::ALL.iter().position(|k| *k == key).unwrap()].len()
    };
    assert_eq!(count(tape::TapeKey::Play), 1, "play is one triangle");
    assert_eq!(count(tape::TapeKey::Back), 2, "back is two");
    assert_eq!(count(tape::TapeKey::Forward), 2, "forward is two");
    assert_eq!(count(tape::TapeKey::Record), 1, "record is a circle");

    // Direction is a property of each arrow, not of where it sits in its key: the apex against the base it was built with.
    // The first version of this assertion compared apexes to the key's centre, which a legitimately off-centre pair fails.
    let directions = |key: tape::TapeKey| -> Vec<f32> {
        let index = tape::TapeKey::ALL.iter().position(|k| *k == key).unwrap();
        let y = cells[index].center().y;
        marks[index]
            .iter()
            .filter_map(|shape| match shape {
                Shape::Path(path) => {
                    let apex = path
                        .points
                        .iter()
                        .min_by(|a, b| (a.y - y).abs().total_cmp(&(b.y - y).abs()))?;
                    let base: Vec<f32> = path
                        .points
                        .iter()
                        .filter(|p| (p.y - y).abs() > 0.01)
                        .map(|p| p.x)
                        .collect();
                    if base.is_empty() {
                        return None;
                    }
                    let mean = base.iter().sum::<f32>() / base.len() as f32;
                    // Positive means the point is to the right of the base, i.e. the arrow points right.
                    Some(apex.x - mean)
                }
                _ => None,
            })
            .collect()
    };

    for reach in directions(tape::TapeKey::Play) {
        assert!(reach > 0.0, "play points right");
    }
    for reach in directions(tape::TapeKey::Back) {
        assert!(reach < 0.0, "back points left");
    }
    for reach in directions(tape::TapeKey::Forward) {
        assert!(reach > 0.0, "forward points right");
    }
}

/// The keyboard reaches the deck through the panel: space plays and pauses, and the three unwired keys say so.
#[test]
fn the_deck_answers_on_the_keyboard_too() {
    let shared = SharedState::new();
    let mut bank = acid_core::bank::PatternBank::new_empty();
    let mut panel = PanelState::default();

    // Space is play/pause, not play/stop: a deck's space bar does not rewind.
    let space = dispatch::KeyPress {
        key: Key::Space,
        shift: false,
        ctrl: false,
    };
    dispatch::handle(space, &mut panel, &shared, &mut bank);
    assert_eq!(panel.transport_request, Some(transport::Request::Play));

    shared.set_transport(Transport::Playing);
    panel.transport_request = None;
    dispatch::handle(space, &mut panel, &shared, &mut bank);
    assert_eq!(
        panel.transport_request,
        Some(transport::Request::Pause),
        "space pauses while playing"
    );

    // The three with no feature behind them answer with the truth, and ask the rack for nothing.
    for (key, said) in [
        (Key::Comma, "BACK IS NOT WIRED YET"),
        (Key::Period, "FORWARD IS NOT WIRED YET"),
        (Key::R, "RECORD IS NOT WIRED YET"),
    ] {
        panel.transport_request = None;
        dispatch::handle(
            dispatch::KeyPress {
                key,
                shift: false,
                ctrl: false,
            },
            &mut panel,
            &shared,
            &mut bank,
        );
        assert_eq!(panel.transport_request, None, "{key:?} asks for nothing");
        assert_eq!(
            panel.notice_now(std::time::Instant::now()),
            Some(said),
            "{key:?} says why"
        );
    }

    // And the map itself: the three keys exist, and none of them is the pattern arrows' business.
    assert_eq!(
        shortcuts::shortcut_for(Key::Comma, false),
        Some(shortcuts::Shortcut::SeekBack)
    );
    assert_eq!(
        shortcuts::shortcut_for(Key::Period, false),
        Some(shortcuts::Shortcut::SeekForward)
    );
    assert_eq!(
        shortcuts::shortcut_for(Key::R, false),
        Some(shortcuts::Shortcut::Record)
    );
}
