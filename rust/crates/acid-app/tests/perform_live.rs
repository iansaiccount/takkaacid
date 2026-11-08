//! The performance row: octave transpose, shuffle and randomise, driven through the panel.

use acid_app::state::SharedState;
use acid_app::ui::layout::layout;
use acid_app::ui::{PanelState, draw_panel};
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;
use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

fn window() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 620.0))
}

fn run_on(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(window()),
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

fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![pointer(at, true)]);
    run_on(ctx, panel, shared, vec![pointer(at, false)]);
}

/// A row of four cells: octave down, octave up, shuffle, randomise.
fn cell(index: usize) -> Pos2 {
    layout(window()).perform.cells[index].center()
}

fn lit_shared() -> std::sync::Arc<SharedState> {
    let shared = SharedState::new();
    for index in [0, 2, 4, 6] {
        shared.set_step(index, Step::note(Pitch::C, 1));
    }
    shared
}

#[test]
fn the_octave_keys_move_the_whole_pattern_and_back() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();
    let before = shared.pattern();
    let midi_before = before.steps[0].midi();

    click(&ctx, &mut panel, &shared, cell(1));
    assert_eq!(shared.pattern().steps[0].midi(), midi_before + 12);
    assert_eq!(
        panel.pattern.steps[0].midi(),
        midi_before + 12,
        "the panel draws what it published"
    );
    assert!(panel.dirty, "a transposed pattern is worth saving");

    click(&ctx, &mut panel, &shared, cell(0));
    assert_eq!(shared.pattern().steps, before.steps, "and back is exact");
}

#[test]
fn the_shuffle_knob_writes_the_swing_the_clock_uses() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();
    let at = cell(2);
    assert_eq!(shared.swing.get(), 0.0);

    // A hand on the knob: a warm-up frame, the press, then the move.
    let press = pointer(at, true);
    run_on(&ctx, &mut panel, &shared, vec![]);
    run_on(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::PointerMoved(at), press],
    );
    run_on(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::PointerMoved(at - vec2(0.0, 30.0))],
    );

    assert!(shared.swing.get() > 0.0, "the knob should have turned");
    assert!(panel.dirty);
}

#[test]
fn randomise_rerolls_the_whole_pattern() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();
    let before = shared.pattern();

    click(&ctx, &mut panel, &shared, cell(3));
    let after = shared.pattern();

    assert_ne!(after, before, "nothing was randomised");
    assert!(
        after.steps.iter().any(|step| step.on),
        "a jump start must leave something sounding"
    );
    assert!(
        after
            .steps
            .iter()
            .filter(|step| step.on)
            .all(|step| step.audible()),
        "a randomise must not write a silent step"
    );
    // The player's touch survives: accents and slides are not the randomiser's to move.
    for index in 0..16 {
        assert_eq!(after.steps[index].accent, before.steps[index].accent);
        assert_eq!(after.steps[index].slide, before.steps[index].slide);
    }
    assert!(panel.dirty);
}

#[test]
fn an_empty_pattern_survives_the_octave_keys() {
    // Nothing to transpose is not an error: the rests keep the notes they carry, so up and back is still exact.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared, cell(1));
    assert!(
        shared.pattern().steps.iter().all(|step| !step.on),
        "an empty pattern stays silent"
    );

    click(&ctx, &mut panel, &shared, cell(0));
    assert_eq!(*shared.pattern(), Pattern::empty());
}
