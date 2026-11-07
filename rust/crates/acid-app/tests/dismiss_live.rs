//! The help card and the device picker must both be dismissable without the keyboard.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, draw_panel};
use eframe::egui::{self, Context, Event, PointerButton, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

fn run(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        draw_panel(panel, shared, ui);
    });
    output.textures_delta.clear();
}

fn moved(at: Pos2) -> Event {
    Event::PointerMoved(at)
}

fn button(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

/// A full click needs the widget to exist one frame earlier, so the pointer settles first.
fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    run(ctx, panel, shared, vec![moved(at)]);
    run(ctx, panel, shared, vec![button(at, true)]);
    run(ctx, panel, shared, vec![button(at, false)]);
}

#[test]
fn a_click_anywhere_closes_the_help_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState {
        help_visible: true,
        ..PanelState::default()
    };
    // Somewhere in the middle of the panel, far from the `?` dot.
    click(&ctx, &mut panel, &shared, Pos2::new(490.0, 430.0));
    assert!(!panel.help_visible, "a click on the card must take it away");
}

#[test]
fn the_question_dot_still_opens_the_help_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    assert!(!panel.help_visible);

    // The dot sits at the right end of the wordmark, vertically centred in the top strip.
    click(&ctx, &mut panel, &shared, help_dot());
    assert!(panel.help_visible, "the ? dot did not open the card");
}

/// Where `draw_panel` puts the help dot: the wordmark's right edge minus 10 points, on the strip's midline.
fn help_dot() -> Pos2 {
    let l = acid_app::ui::layout::layout(screen());
    Pos2::new(
        acid_app::ui::panel::brand_rect(l.top).right() - 10.0,
        l.top.center().y,
    )
}
