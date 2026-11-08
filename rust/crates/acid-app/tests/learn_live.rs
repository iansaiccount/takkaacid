//! MIDI learn: press LEARN, click a knob, move a controller — the controller is bound to that knob.

use acid_app::midi::cc::{self, CcTarget};
use acid_app::state::SharedState;
use acid_app::ui::layout::{knob_cells, layout};
use acid_app::ui::learn::Step;
use acid_app::ui::switches;
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
fn the_waveform_selector_is_a_learnable_knob_now() {
    // Index 0 used to be refused; it is a slot like the others, and a controller snaps onto its four faces.
    assert_eq!(
        acid_app::ui::learn::control_for_knob(0),
        Some(acid_app::midi::cc::CcTarget::Waveform)
    );
    for index in 1..=6 {
        assert!(acid_app::ui::learn::control_for_knob(index).is_some());
    }
}

#[test]
fn the_switches_row_has_no_reset_key() {
    // RESET rewrote the whole map from a dark key beside LEARN; it was removed rather than moved, so the row is seven
    // keys and none of them prints it.
    assert_eq!(switches::Switch::ALL.len(), 7);
    for switch in switches::Switch::ALL {
        assert_ne!(switch.label(), "RESET", "{switch:?} is back");
    }
}
