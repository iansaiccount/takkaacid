//! The CLEAR button: emptying the pattern is the one action on the panel with no undo, so it takes two clicks.

use acid_app::state::SharedState;
use acid_app::ui::layout::{knob_cells, layout};
use acid_app::ui::{PanelState, draw_panel};
use acid_core::pattern::Step;
use acid_core::pitch::Pitch;
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

/// The CLEAR button is the fourth cell of the switches row.
fn clear_centre() -> Pos2 {
    knob_cells(layout(screen()).switches, 6)[3].center()
}

fn step_centre(index: usize) -> Pos2 {
    acid_app::ui::layout::step_cells(layout(screen()).steps)[index].center()
}

fn press(at: Pos2, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
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

/// A settled pointer, a press and a release — three frames, because a widget must exist before it is hit.
fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2, mods: Modifiers) {
    run_on(
        ctx,
        panel,
        shared,
        vec![Event::PointerMoved(at), Event::ModifiersChanged(mods)],
    );
    run_on(ctx, panel, shared, vec![press(at, mods)]);
    run_on(ctx, panel, shared, vec![release(at)]);
}

fn lit_shared() -> std::sync::Arc<SharedState> {
    let shared = SharedState::new();
    for index in 0..16 {
        shared.set_step(index, Step::note(Pitch::C, 0));
    }
    shared
}

#[test]
fn clearing_takes_two_clicks_and_empties_the_pattern() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();

    click(
        &ctx,
        &mut panel,
        &shared,
        clear_centre(),
        Modifiers::default(),
    );
    assert!(
        shared.pattern().steps.iter().any(|step| step.on),
        "one click must not empty the pattern"
    );
    assert!(
        panel.clear_armed_at.is_some(),
        "the first click arms the button"
    );

    click(
        &ctx,
        &mut panel,
        &shared,
        clear_centre(),
        Modifiers::default(),
    );
    assert!(
        shared.pattern().steps.iter().all(|step| !step.on),
        "the second click empties the pattern"
    );
    assert!(panel.clear_armed_at.is_none(), "and disarms again");
}

#[test]
fn a_step_edit_disarms_a_pending_clear() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();

    click(
        &ctx,
        &mut panel,
        &shared,
        clear_centre(),
        Modifiers::default(),
    );
    assert!(panel.clear_armed_at.is_some());

    // Any edit makes a pending clear stale, or the second click would throw away the edit as well.
    click(
        &ctx,
        &mut panel,
        &shared,
        step_centre(4),
        Modifiers {
            ctrl: true,
            ..Modifiers::default()
        },
    );
    assert!(
        panel.clear_armed_at.is_none(),
        "editing a step must disarm the button"
    );
    assert!(
        shared.pattern().steps.iter().any(|step| step.on),
        "nothing was cleared"
    );
}

#[test]
fn a_click_on_the_panel_elsewhere_leaves_the_pattern_alone() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();

    click(
        &ctx,
        &mut panel,
        &shared,
        step_centre(4),
        Modifiers::default(),
    );
    assert!(panel.clear_armed_at.is_none(), "selecting is not clearing");
    assert!(shared.pattern().steps.iter().all(|step| step.on));
}

#[test]
fn the_clear_button_sits_where_the_layout_says() {
    // Cheap guard for the harness: a button outside the window could never be clicked.
    let cell = knob_cells(layout(screen()).switches, 6)[3];
    assert!(screen().contains_rect(cell));
}
