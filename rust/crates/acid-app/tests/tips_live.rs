//! The TIPS key: the hover help is on when the app opens, and one click turns it off again.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel, tips_rect};
use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, acid_app::ui::theme::DESIGN_SIZE)
}

fn run_on(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
}

fn pointer(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    }
}

fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState) {
    let at = tips_rect(screen()).center();
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![pointer(at, true)]);
    run_on(ctx, panel, shared, vec![pointer(at, false)]);
}

#[test]
fn the_help_is_on_when_the_app_opens() {
    assert!(
        PanelState::default().tooltips,
        "a player should not have to find the switch to be told what things do"
    );
}

#[test]
fn the_tips_key_turns_the_help_off_and_back_on() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared);
    assert!(!panel.tooltips, "one click should silence the hints");

    click(&ctx, &mut panel, &shared);
    assert!(panel.tooltips, "the same key brings them back");
}

#[test]
fn turning_the_help_off_is_a_change_worth_saving() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared);
    assert!(
        panel.dirty,
        "the switch survives a restart, so the session is unsaved until it is written"
    );
}

#[test]
fn the_tips_key_sits_in_the_top_strip_clear_of_the_saved_lamp() {
    let top = acid_app::ui::layout::layout(screen()).top;
    let lamp = acid_app::ui::panel::save_indicator_rect(top, acid_app::ui::layout::scale(screen()));
    let key = tips_rect(screen());
    assert!(!key.intersects(lamp), "the key must not cover the lamp");
    assert!(
        key.right() <= lamp.left(),
        "the key belongs to the left of it"
    );
    assert!(key.width() > 20.0 && key.height() > 8.0, "and be clickable");
}

/// Every piece of text egui painted this frame, tooltips included: a tooltip is drawn into a floating layer, so its galley
/// arrives in the same shape list as the panel's own captions.
fn painted_text(output: &eframe::egui::FullOutput) -> Vec<String> {
    fn walk(shape: &eframe::egui::Shape, out: &mut Vec<String>) {
        match shape {
            eframe::egui::Shape::Text(text) => out.push(text.galley.text().to_string()),
            eframe::egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

#[test]
fn the_two_readouts_answer_the_pointer() {
    // A readout is a control here — both are teachable — so both explain themselves on hover. The pointer has to hold
    // still past egui's tooltip delay, which is why this drives several frames with the clock moving.
    let l = acid_app::ui::layout::layout(screen());
    for (rect, title) in [
        (acid_app::ui::panel::tempo_display_rect(l.top), "TEMPO"),
        (acid_app::ui::panel::pattern_display_rect(l.top), "PATTERN"),
    ] {
        let ctx = Context::default();
        let shared = SharedState::new();
        let mut panel = PanelState::default();
        let at = rect.center();
        let mut seen: Vec<String> = Vec::new();
        for step in 0..4 {
            let input = RawInput {
                screen_rect: Some(screen()),
                time: Some(f64::from(step)),
                events: vec![Event::PointerMoved(at)],
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
            output.textures_delta.clear();
            seen.extend(painted_text(&output));
        }
        assert!(
            seen.iter().any(|text| text.contains(title)),
            "the {title} readout showed no help; painted: {seen:?}"
        );
    }
}
