//! MIDI learn: press LEARN, click a knob, move a controller — the controller is bound to that knob.

use acid_app::midi::cc::{self, CcTarget};
use acid_app::state::SharedState;
use acid_app::ui::knobs::Knob;
use acid_app::ui::layout::{knob_cells, layout};
use acid_app::ui::learn::Step;
use acid_app::ui::switches;
use acid_app::ui::transport::TRANSPORT_CELLS;
use acid_app::ui::{PanelState, draw_panel, panel};
use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use std::sync::atomic::Ordering;

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

/// The BPM readout in the top strip, which a learn can teach.
fn tempo_centre() -> Pos2 {
    panel::tempo_display_rect(layout(screen()).top).center()
}

/// The PATTERN key in the switches row.
fn pattern_centre() -> Pos2 {
    switches::cell(layout(screen()).switches, switches::Switch::Pattern).center()
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
fn every_knob_in_the_row_can_be_taught_by_clicking_it() {
    // Index 0 used to be refused; it is a slot like the others, and a controller snaps onto its four faces. Each cell is
    // clicked while LEARN is asking, and the panel must start listening for *that* knob: the row and the learn map are
    // both the catalogue's order, so a reordered catalogue shows up here rather than as a mislearned knob.
    for (cell, knob) in layout(screen()).knobs.cells.into_iter().zip(Knob::ALL) {
        let ctx = Context::default();
        let shared = SharedState::new();
        let mut panel = PanelState::default();
        click(&ctx, &mut panel, &shared, learn_centre());
        click(&ctx, &mut panel, &shared, cell.center());
        assert_eq!(
            panel.learn,
            Step::Listening(knob.learn_target()),
            "{knob:?} did not answer its own cell"
        );
        assert!(
            cc::slot(knob.learn_target()).is_some(),
            "{knob:?} teaches a slot the MIDI side does not know"
        );
    }
}

/// The BPM readout and the PATTERN key are learnable too — but only while a learn is choosing, so a stray click on the
/// top strip never starts a binding that would swallow the next controller the player touches.
#[test]
fn the_tempo_readout_and_the_pattern_key_can_be_taught() {
    let ctx = Context::default();
    let shared = SharedState::new();

    let mut idle = PanelState::default();
    click(&ctx, &mut idle, &shared, tempo_centre());
    click(&ctx, &mut idle, &shared, pattern_centre());
    assert_eq!(
        idle.learn,
        Step::Idle,
        "outside a learn the readouts are inert"
    );

    let mut panel = PanelState::default();
    click(&ctx, &mut panel, &shared, learn_centre());
    click(&ctx, &mut panel, &shared, tempo_centre());
    assert_eq!(
        panel.learn,
        Step::Listening(CcTarget::Tempo),
        "the BPM readout asks for a tempo fader"
    );

    let mut panel = PanelState::default();
    click(&ctx, &mut panel, &shared, learn_centre());
    click(&ctx, &mut panel, &shared, pattern_centre());
    assert_eq!(
        panel.learn,
        Step::Listening(CcTarget::Pattern),
        "the PATTERN key asks for a pattern fader"
    );
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

/// The SHUFFLE knob is learnable, and while a learn is choosing a click picks it rather than turning it.
#[test]
fn clicking_the_shuffle_knob_while_choosing_asks_for_a_shuffle_fader() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let shuffle = layout(screen()).perform.cells[2].center();

    // Outside a learn the knob is a knob: this click is a press-and-release with no drag, so nothing moves, and no
    // binding starts.
    click(&ctx, &mut panel, &shared, shuffle);
    assert_eq!(panel.learn, Step::Idle);

    click(&ctx, &mut panel, &shared, learn_centre());
    click(&ctx, &mut panel, &shared, shuffle);

    assert_eq!(panel.learn, Step::Listening(CcTarget::Swing));
    assert_eq!(shared.swing.get(), 0.0, "picking a knob does not turn it");
}

/// The knob must stay on the panel while a learn is choosing.
///
/// A control that disappears when the player is asked to click it cannot be found, let alone clicked: the first cut of
/// the learn pick skipped drawing the knob entirely and left an empty cell. Hovering is the cheapest way to see a widget
/// from a headless frame — a knob asks for the vertical-drag cursor, an empty cell asks for nothing.
#[test]
fn the_shuffle_knob_stays_on_the_panel_while_a_learn_is_choosing() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared, learn_centre());
    assert_eq!(panel.learn, Step::Choosing);

    let at = layout(screen()).perform.cells[2].center();
    let input = RawInput {
        screen_rect: Some(screen()),
        events: vec![Event::PointerMoved(at)],
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
    output.textures_delta.clear();

    assert_eq!(
        output.platform_output.cursor_icon,
        eframe::egui::CursorIcon::ResizeVertical,
        "the shuffle knob is not there to click"
    );
}
