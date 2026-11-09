//! The settings card: opened by F2 or the gear, folded and unfolded by its own headers, and dismissed only by CLOSE, Escape
//! or a click outside it — never by a click on one of its own rows.

use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::settings::{self, Request};
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

/// A panel with the settings card open and a couple of devices on offer.
fn open_settings(midi_ports: &[&str], outputs: &[&str]) -> PanelState {
    let mut panel = PanelState {
        settings_open: true,
        ..PanelState::default()
    };

    panel.settings.midi_ports = midi_ports.iter().map(|s| s.to_string()).collect();
    panel.settings.outputs = outputs.iter().map(|s| s.to_string()).collect();
    panel
}

fn rows(panel: &PanelState) -> settings::Rows {
    let l = layout(screen());
    settings::rows(
        l.panel,
        scale(screen()),
        panel.settings.midi_ports.len(),
        panel.settings.outputs.len(),
        panel.settings.midi_given_up,
        panel.settings.midi_open,
        panel.settings.sound_open,
    )
}

/// The bug this pins: the dismissal shield was registered after the card, so it won the hit test and every click inside the
/// card — a fold header most visibly — shut the card instead of doing what it said.
#[test]
fn folding_a_panel_does_not_close_the_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &["Speakers", "HDMI"]);
    assert!(!panel.settings.midi_open, "the sections start folded");

    let at = rows(&panel).midi_header.center();
    click(&ctx, &mut panel, &shared, at);

    assert!(panel.settings_open, "clicking the fold must not dismiss");
    assert!(panel.settings.midi_open, "it unfolds the panel");
}

/// The same click again folds it, and the card still stands.
#[test]
fn folding_a_panel_shut_keeps_the_card_open() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &["Speakers"]);
    panel.settings.midi_open = true;

    let at = rows(&panel).midi_header.center();
    click(&ctx, &mut panel, &shared, at);

    assert!(panel.settings_open);
    assert!(!panel.settings.midi_open);
}

/// A click on the dark area outside the card is a dismissal — the shield still does its job.
#[test]
fn a_click_outside_the_card_closes_it() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&[], &[]);

    click(&ctx, &mut panel, &shared, Pos2::new(6.0, 6.0));

    assert!(!panel.settings_open, "clicking away closes the card");
}

/// Both switches ask for their own flip, and neither click takes the card away with it.
#[test]
fn the_switches_ask_for_their_flip() {
    let ctx = Context::default();
    let shared = SharedState::new();

    let mut tooltips = open_settings(&[], &[]);
    let at = rows(&tooltips).tooltips.center();
    click(&ctx, &mut tooltips, &shared, at);
    assert_eq!(tooltips.settings_request, Some(Request::Tooltips));
    assert!(tooltips.settings_open);

    let mut autosave = open_settings(&[], &[]);
    let at = rows(&autosave).autosave.center();
    click(&ctx, &mut autosave, &shared, at);
    assert_eq!(autosave.settings_request, Some(Request::Autosave));
    assert!(autosave.settings_open);
}

/// The sound section folds and unfolds on its own, independently of the MIDI one.
#[test]
fn the_two_sections_fold_independently() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &["Speakers", "HDMI"]);

    let at = rows(&panel).sound_header.center();
    click(&ctx, &mut panel, &shared, at);
    assert!(panel.settings.sound_open);
    assert!(!panel.settings.midi_open, "the MIDI panel stayed shut");

    let screen_rows = rows(&panel);
    assert!(
        screen_rows.sound_panel.height() > 0.0,
        "an open panel needs a panel to paint in"
    );
    assert_eq!(
        screen_rows.midi_panel.height(),
        0.0,
        "a shut panel takes no space"
    );
    let _ = screen_rows.retry;
}

/// An open panel never grows past four rows, however long the machine's list is: what does not fit scrolls.
#[test]
fn an_open_panel_is_four_rows_tall_at_most() {
    let l = layout(screen());
    let s = scale(screen());
    let short = settings::rows(l.panel, s, 2, 2, false, true, true);
    let long = settings::rows(l.panel, s, 12, 12, false, true, true);

    let four_rows = 4.0 * (22.0 + 2.0) * s;
    assert!(short.midi_panel.height() <= four_rows);
    assert!(
        long.midi_panel.height() <= four_rows,
        "twelve outputs still show four rows"
    );
    assert!(
        long.midi_panel.height() < l.panel.height() * 0.35,
        "twelve outputs must not fill the card"
    );
}
