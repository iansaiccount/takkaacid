//! MIDI learn: press LEARN, click a knob, move a controller — the controller is bound to that knob.

use acid_app::midi::cc::{self, CcTarget};
use acid_app::state::SharedState;
use acid_app::ui::layout::{knob_cells, layout};
use acid_app::ui::learn::Step;
use acid_app::ui::transport::TRANSPORT_CELLS;
use acid_app::ui::{PanelState, draw_panel, panel};
use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
use std::sync::atomic::Ordering;

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

fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![button(true)]);
    run_on(ctx, panel, shared, vec![button(false)]);
}

/// LEARN is the fifth transport key.
fn learn_centre() -> Pos2 {
    knob_cells(layout(screen()).transport, TRANSPORT_CELLS)[4].center()
}

/// The CUT OFF knob is the third cell of the knob row.
fn cutoff_centre() -> Pos2 {
    layout(screen()).knobs.cells[2].center()
}

fn volume_centre() -> Pos2 {
    panel::volume_knob_rect(layout(screen()).top).center()
}

#[test]
fn learn_then_a_knob_then_a_controller_binds_it() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared, learn_centre());
    assert_eq!(panel.learn, Step::Choosing, "LEARN asks which knob");

    click(&ctx, &mut panel, &shared, cutoff_centre());
    assert_eq!(panel.learn, Step::Listening(CcTarget::Cutoff));
    assert!(
        shared.learn_active.load(Ordering::Relaxed),
        "the MIDI side must be listening"
    );

    // A controller arrives on the MIDI thread: it is recorded, not applied.
    shared.learn_cc.store(22, Ordering::Relaxed);
    run_on(&ctx, &mut panel, &shared, vec![]);

    assert_eq!(panel.learn, Step::Idle, "the learn is finished");
    assert!(!shared.learn_active.load(Ordering::Relaxed));
    assert_eq!(cc::cc_for(&shared, CcTarget::Cutoff), Some(22));
    assert_eq!(
        cc::cc_target(&shared, 74),
        None,
        "the factory controller is released"
    );
    assert!(panel.dirty, "a learned mapping is worth saving");
}

#[test]
fn the_volume_knob_can_be_taught_too() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared, learn_centre());
    click(&ctx, &mut panel, &shared, volume_centre());

    assert_eq!(panel.learn, Step::Listening(CcTarget::Volume));
    assert!(shared.learn_active.load(Ordering::Relaxed));
}

#[test]
fn pressing_learn_again_gives_up() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared, learn_centre());
    click(&ctx, &mut panel, &shared, cutoff_centre());
    assert!(shared.learn_active.load(Ordering::Relaxed));

    click(&ctx, &mut panel, &shared, learn_centre());
    assert_eq!(panel.learn, Step::Idle);
    assert!(
        !shared.learn_active.load(Ordering::Relaxed),
        "and stops listening"
    );
}

#[test]
fn the_waveform_selector_is_not_learnable() {
    // Index 0 has no controller of its own: clicking it while choosing must not start a binding.
    assert_eq!(acid_app::ui::learn::control_for_knob(0), None);
    for index in 1..=6 {
        assert!(acid_app::ui::learn::control_for_knob(index).is_some());
    }
}

/// RESET is the seventh switches cell, after BANK..SONGS.
fn reset_centre() -> Pos2 {
    knob_cells(layout(screen()).switches, 7)[6].center()
}

#[test]
fn reset_puts_the_factory_map_back() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    // Teach CUT OFF a different controller first.
    click(&ctx, &mut panel, &shared, learn_centre());
    click(&ctx, &mut panel, &shared, cutoff_centre());
    shared.learn_cc.store(22, Ordering::Relaxed);
    run_on(&ctx, &mut panel, &shared, vec![]);
    assert_eq!(cc::cc_for(&shared, CcTarget::Cutoff), Some(22));

    // RESET is offered inside a learn: press LEARN, then RESET.
    click(&ctx, &mut panel, &shared, learn_centre());
    click(&ctx, &mut panel, &shared, reset_centre());

    assert_eq!(
        cc::cc_for(&shared, CcTarget::Cutoff),
        Some(74),
        "the factory pair is back"
    );
    assert_eq!(panel.learn, Step::Idle, "and the learn is over");
    assert!(panel.dirty, "the reset is worth saving");
}

#[test]
fn reset_is_dark_until_a_learn_is_in_progress() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    // A rig taught, then a stray click on RESET while idle: it must survive.
    click(&ctx, &mut panel, &shared, learn_centre());
    click(&ctx, &mut panel, &shared, cutoff_centre());
    shared.learn_cc.store(23, Ordering::Relaxed);
    run_on(&ctx, &mut panel, &shared, vec![]);
    assert_eq!(cc::cc_for(&shared, CcTarget::Cutoff), Some(23));

    click(&ctx, &mut panel, &shared, reset_centre());

    assert_eq!(
        cc::cc_for(&shared, CcTarget::Cutoff),
        Some(23),
        "a dark RESET must not wipe the map"
    );
}
