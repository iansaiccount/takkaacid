//! The gesture table from the panel's side: what a real click and a real drag do to the pattern and the
//! selection.

use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, step_cells};
use acid_app::ui::{PanelState, draw_panel};
use acid_core::pattern::Step;
use acid_core::pitch::Pitch;
use eframe::egui::{self, Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

fn run_on(
    ctx: &Context,
    screen_rect: Rect,
    panel: &mut PanelState,
    shared: &SharedState,
    events: Vec<Event>,
) {
    let input = RawInput {
        screen_rect: Some(screen_rect),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
}

/// Centre of step `index`'s cell, in window coordinates.
fn step_centre(index: usize) -> Pos2 {
    let cells = step_cells(layout(screen()).steps);
    cells[index].center()
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
fn click_with(
    ctx: &Context,
    panel: &mut PanelState,
    shared: &SharedState,
    at: Pos2,
    mods: Modifiers,
) {
    run_on(
        ctx,
        screen(),
        panel,
        shared,
        vec![Event::PointerMoved(at), Event::ModifiersChanged(mods)],
    );
    run_on(ctx, screen(), panel, shared, vec![press(at, mods)]);
    run_on(ctx, screen(), panel, shared, vec![release(at)]);
}




fn lit_shared() -> std::sync::Arc<SharedState> {
    let shared = SharedState::new();
    for i in 0..16 {
        shared.set_step(i, Step::note(Pitch::C, 0));
    }
    shared
}

#[test]
fn a_click_selects_without_editing() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();

    click_with(
        &ctx,
        &mut panel,
        &shared,
        step_centre(5),
        Modifiers::default(),
    );

    assert_eq!(panel.selected, 5);
    assert!(
        shared.pattern().steps[5].on,
        "a plain click must not rest the step"
    );
}

#[test]
fn ctrl_click_selects_and_toggles() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };

    click_with(&ctx, &mut panel, &shared, step_centre(5), ctrl);

    assert_eq!(panel.selected, 5, "a click always moves the selection");
    assert!(
        !shared.pattern().steps[5].on,
        "ctrl+click must toggle the step"
    );
}

#[test]
fn dragging_a_rest_selects_it_and_brings_it_in() {
    let ctx = Context::default();
    let shared = lit_shared();
    shared.set_step(3, Step::rest());
    let mut panel = PanelState::default();

    let at = step_centre(3);
    run_on(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(at)],
    );
    run_on(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![press(at, Modifiers::default())],
    );
    run_on(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(at - vec2(0.0, 24.0))],
    );
    run_on(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![release(at - vec2(0.0, 24.0))],
    );

    assert_eq!(panel.selected, 3, "a drag selects the step it started on");
    let step = shared.pattern().steps[3];
    assert!(step.on, "dragging a rest brings it in");
    assert_eq!(step.pitch, Pitch::D, "24 px up from C is D");
}

#[test]
fn a_click_while_the_help_card_is_up_does_not_select() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        help_visible: true,
        ..PanelState::default()
    };

    click_with(
        &ctx,
        &mut panel,
        &shared,
        step_centre(5),
        Modifiers::default(),
    );

    assert_eq!(panel.selected, 0, "the card swallowed the click");
    assert!(!panel.help_visible, "the click should have closed the card");
}

#[test]
fn clicking_one_step_then_another_moves_the_outline() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();

    click_with(
        &ctx,
        &mut panel,
        &shared,
        step_centre(2),
        Modifiers::default(),
    );
    assert_eq!(panel.selected, 2);
    click_with(
        &ctx,
        &mut panel,
        &shared,
        step_centre(9),
        Modifiers::default(),
    );
    assert_eq!(panel.selected, 9);
}

#[test]
fn shift_right_moves_the_selection_and_leaves_the_pattern_alone() {
    let shared = lit_shared();
    let before = shared.pattern().steps;
    let mut panel = PanelState::default();

    let consumed = selection_dispatch(egui::Key::ArrowRight, true, &mut panel, &shared);

    assert!(
        consumed,
        "shift+right must be consumed before the shortcuts see it"
    );
    assert_eq!(panel.selected, 1);
    assert_eq!(
        shared.pattern().steps,
        before,
        "shift+right must not touch the pattern"
    );
}

#[test]
fn a_plain_arrow_is_left_for_the_transport_shortcuts() {
    let shared = lit_shared();
    let mut panel = PanelState::default();
    assert!(
        !selection_dispatch(egui::Key::ArrowRight, false, &mut panel, &shared),
        "unshifted arrows belong to the pattern shortcut"
    );
    assert_eq!(
        panel.selected, 0,
        "an unshifted arrow must not move the selection"
    );
}

#[test]
fn shift_up_raises_the_selected_note_one_semitone() {
    let shared = lit_shared();
    let mut panel = PanelState::default();
    selection_dispatch(egui::Key::ArrowUp, true, &mut panel, &shared);
    let step = shared.pattern().steps[0];
    assert_eq!((step.pitch, step.octave_offset), (Pitch::CSharp, 0));
}

#[test]
fn enter_toggles_the_selected_step() {
    let shared = lit_shared();
    shared.set_step(
        0,
        Step {
            accent: true,
            ..Step::note(Pitch::E, 0)
        },
    );
    let mut panel = PanelState::default();

    selection_dispatch(egui::Key::Enter, false, &mut panel, &shared);
    assert!(!shared.pattern().steps[0].on, "enter must rest the step");

    selection_dispatch(egui::Key::Enter, false, &mut panel, &shared);
    let back = shared.pattern().steps[0];
    assert!(
        back.on && back.accent,
        "a second enter brings the flags back"
    );
}

fn selection_dispatch(
    key: egui::Key,
    shift: bool,
    panel: &mut PanelState,
    shared: &SharedState,
) -> bool {
    acid_app::ui::selection::dispatch(key, shift, panel, shared)
}

#[test]
fn the_layout_gives_sixteen_distinct_cells() {
    // Cheap guard for the harness above: overlapping cells would make every test in this file a coin toss.
    let cells = step_cells(layout(screen()).steps);
    assert_eq!(cells.len(), 16);
    for pair in cells.windows(2) {
        assert!(pair[0].right() <= pair[1].left() + 0.01);
    }
    let _ = egui::vec2(0.0, 0.0);
}
