//! The help card's words, read back off the paint. The card is the only manual, so what it says about the strip, the
//! device card and the save limit is behaviour a player can be misled by — the card's own unit tests pin the gestures,
//! and this one pins the words the card actually paints inside a frame.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel, help, layout};
use eframe::egui::{Context, Event, FullOutput, Pos2, RawInput, Rect, vec2};

fn run_on(
    ctx: &Context,
    panel: &mut PanelState,
    shared: &SharedState,
    events: Vec<Event>,
) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

/// Every line of text the frame drew, in paint order. Copied from `device_card_live.rs`: the card's words are part of
/// what it says, and a needle in `help_sections()` would still pass if the card never painted.
fn strings(output: &FullOutput) -> Vec<String> {
    use eframe::egui::Shape;

    fn walk(shape: &Shape, said: &mut Vec<String>) {
        match shape {
            Shape::Text(text) => said.push(text.galley.text().to_string()),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, said)),
            _ => {}
        }
    }

    let mut said = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut said);
    }
    said
}

#[test]
fn the_help_card_names_the_devices_and_that_they_are_saved() {
    let state = SharedState::new();
    let mut panel = PanelState {
        help_visible: true,
        ..Default::default()
    };
    let ctx = Context::default();
    let said = strings(&run_on(&ctx, &mut panel, &state, vec![])).join(" ");

    for wanted in [
        "F3",
        "COLOUR",
        "Ctrl+1",
        "Ctrl+C",
        "Ctrl+V",
        "MUTE",
        "four devices",
        "saved with the song",
    ] {
        assert!(
            said.contains(wanted),
            "the help card should mention {wanted}: {said}"
        );
    }
    // Phase 5 made the session file hold every machine, and the card said "names and extra devices are not saved yet" for
    // two rounds after. A claim the panel contradicts cannot come back.
    assert!(
        !said.contains("not saved"),
        "the help card apologises for a format that saves everything: {said}"
    );
}

/// The drum face's keys, in the card's own words. The card is the only manual: a drum machine whose keys are not on it
/// cannot be played from the keyboard by anyone who has not read this repository.
#[test]
fn the_help_card_knows_the_drum_face() {
    let state = SharedState::new();
    let mut panel = PanelState {
        help_visible: true,
        ..Default::default()
    };
    let ctx = Context::default();
    let said = strings(&run_on(&ctx, &mut panel, &state, vec![])).join(" ");

    for wanted in [
        // Folded into the keys the two faces share, so the card stays inside its line budget.
        "move the selection, on either face",
        "on drums, which voice",
        "on drums write that voice; shift accents",
        // The arrows split at the modifier: `shift` is the rack's — the pattern and the tempo — the plain pair is the face's.
        "shift+up / down: tempo by 1 BPM",
        "up / down: note up or down; on drums, which voice",
        // The drum face's own two lines.
        "h: strike the selected voice (HIT)",
        "l / b / p: the filter's low, band, high",
    ] {
        assert!(
            said.contains(wanted),
            "the help card should say {wanted:?}: {said}"
        );
    }

    // One line per arrow pair, and the pairing is the point: the tempo behind `shift`, the note plain. A card that said both
    // behind `shift` would satisfy every needle above, so the wrong pairing is asked for by name.
    assert!(
        !said.contains("shift+up / down: note up or down"),
        "the card puts the note behind shift again: {said}"
    );
}

/// Every painted piece of text with the rectangle it occupies — the card's lines are geometry as much as words.
fn text_rects(output: &FullOutput) -> Vec<(String, Rect)> {
    use eframe::egui::Shape;

    fn walk(shape: &Shape, out: &mut Vec<(String, Rect)>) {
        match shape {
            Shape::Text(text) => out.push((
                text.galley.text().to_string(),
                Rect::from_min_size(text.pos, text.galley.size()),
            )),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }

    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// The budget is a formula and the card is painted from the same numbers — but drawn is not measured, and this round spent
/// half the slack the last three rounds left. So the card is painted at every size the panel allows and every line it draws
/// must land inside the card's own rectangle: an overflow the formula could not see fails here.
#[test]
fn the_painted_card_stays_inside_its_rect_at_every_scale() {
    let card_words: Vec<&str> = help::help_sections()
        .iter()
        .flat_map(|(heading, lines)| std::iter::once(*heading).chain(lines.iter().copied()))
        .collect();
    let mut painted = 0;
    for scale in [0.65f32, 1.0, 1.6] {
        let window = Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0) * scale);
        let panel_rect = layout::layout(window).panel;
        let card = help::card_rect(panel_rect, layout::scale(window));
        let state = SharedState::new();
        let mut panel = PanelState {
            help_visible: true,
            ..Default::default()
        };
        let ctx = Context::default();
        let input = RawInput {
            screen_rect: Some(window),
            events: vec![],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| draw_panel(&mut panel, &state, ui));
        output.textures_delta.clear();
        for (words, rect) in text_rects(&output) {
            if !card_words.contains(&words.as_str()) {
                continue;
            }
            painted += 1;
            assert!(
                card.expand(0.5).contains_rect(rect),
                "at {scale}x the card's line {words:?} lands outside it: {rect:?} vs {card:?}"
            );
        }
    }
    assert!(
        painted > 100,
        "the card painted {painted} lines across three sizes"
    );
}
