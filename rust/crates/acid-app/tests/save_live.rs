//! The SAVE key: a click raises a request for `main`, which owns writing the file, and the top strip says
//! whether anything is unsaved.

use acid_app::state::SharedState;
use acid_app::ui::layout::{knob_cells, layout};
use acid_app::ui::{PanelState, draw_panel};
use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
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

fn press(at: Pos2) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::default(),
    }
}

fn release(at: Pos2) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::default(),
    }
}

fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![press(at)]);
    run_on(ctx, panel, shared, vec![release(at)]);
}

/// The switches row now holds five cells: BANK, PATTERN, MODE, CLEAR, SAVE.
fn save_centre() -> Pos2 {
    knob_cells(layout(screen()).switches, 8)[4].center()
}

#[test]
fn clicking_save_asks_for_a_write_and_leaves_the_dirty_flag_to_main() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState {
        dirty: true,
        ..PanelState::default()
    };

    click(&ctx, &mut panel, &shared, save_centre());

    assert!(panel.save_requested, "the key must ask for a write");
    assert!(
        panel.dirty,
        "the panel cannot know the write worked, so it leaves `dirty` to main"
    );
}

#[test]
fn a_click_elsewhere_does_not_ask_for_a_write() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    // The middle of the panel, nowhere near the switches row.
    click(&ctx, &mut panel, &shared, Pos2::new(500.0, 250.0));

    assert!(!panel.save_requested);
}

#[test]
fn the_indicator_sits_inside_the_top_strip_and_clear_of_the_help_dot() {
    // Cheap guard for the layout: a word painted outside its band would be invisible or overlap the `?`.
    let top = layout(screen()).top;
    let rect = acid_app::ui::panel::save_indicator_rect(top, acid_app::ui::layout::scale(screen()));
    assert!(top.contains_rect(rect));
    let help_dot = egui_help_dot(top);
    assert!(
        !rect.intersects(help_dot),
        "the saved word must not sit under the ? that opens the card"
    );
}

/// The `?` sits `10 px` in from the wordmark's right edge, 18 px across — the same geometry `draw_panel` uses.
fn egui_help_dot(top: Rect) -> Rect {
    let brand = acid_app::ui::panel::brand_rect(top);
    Rect::from_center_size(
        Pos2::new(brand.right() - 10.0, top.center().y),
        vec2(18.0, 18.0),
    )
}
