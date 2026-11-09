//! The face names the song the session is on, and a notice takes the status line for a moment — without ever becoming
//! something the player has to dismiss, and without dying with the card that caused it.

use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::{PanelState, draw_panel, panel};
use eframe::egui::{Context, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

fn run_frame(ctx: &Context, panel: &mut PanelState, shared: &SharedState) {
    let input = RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
}

/// The label sits in the middle of the strip and nowhere near the things it could cover.
#[test]
fn the_song_label_sits_clear_of_everything_it_could_cover() {
    let l = layout(screen());
    let s = scale(screen());
    let label = panel::song_label_rect(l.top, s);

    for (what, rect) in [
        ("the pattern display", panel::pattern_display_rect(l.top)),
        ("the tempo display", panel::tempo_display_rect(l.top)),
        ("the wordmark", panel::brand_rect(l.top)),
        ("the saves lamp", panel::save_indicator_rect(l.top, s)),
    ] {
        assert!(
            !label.intersects(rect),
            "the song label must not cover {what}"
        );
    }
    assert!(
        l.top.contains_rect(label),
        "the label lives on the top strip"
    );
}

/// A notice is on the status line at once, and gone by itself after its time is up.
#[test]
fn a_notice_is_shown_and_then_clears_itself() {
    let mut panel = PanelState::default();
    assert_eq!(panel.notice_now(std::time::Instant::now()), None);

    panel.notify("saved Acid 2");
    let now = std::time::Instant::now();
    assert_eq!(panel.notice_now(now), Some("saved Acid 2"));
    assert_eq!(
        panel.notice_now(now + PanelState::NOTICE_FOR),
        None,
        "a notice is not a dialog: nobody should have to dismiss it"
    );
}

/// The message belongs to the panel, not to the card, so closing the card does not take the words with it.
#[test]
fn a_notice_survives_the_card_closing() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    panel.notify("saved Acid 2");

    run_frame(&ctx, &mut panel, &shared);

    assert_eq!(
        panel.notice_now(std::time::Instant::now()),
        Some("saved Acid 2"),
        "a frame and a closed card must not eat the message"
    );
}

/// The panel draws a notice without disturbing the session: no dirty flag, no request, nothing written.
#[test]
fn drawing_a_notice_changes_nothing_else() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    panel.songs.name = Some("Acid 1".to_string());
    panel.notify("exported to /tmp/Acid 1.json");

    run_frame(&ctx, &mut panel, &shared);

    assert!(!panel.dirty, "saying something is not an edit");
    assert_eq!(panel.songs.request, None);
    assert!(!panel.save_requested);
}

/// The gear sits beside the help dot and clear of everything the strip already uses.
#[test]
fn the_gear_sits_beside_the_help_dot_and_clear_of_the_lamp() {
    let l = layout(screen());
    let s = scale(screen());
    let gear = panel::gear_rect(l.top, s);
    let help = panel::help_rect(l.top, s);

    assert!(!gear.intersects(help), "the gear is its own button");
    for (what, rect) in [
        ("the saves lamp", panel::save_indicator_rect(l.top, s)),
        ("the pattern display", panel::pattern_display_rect(l.top)),
        ("the tempo display", panel::tempo_display_rect(l.top)),
    ] {
        assert!(!gear.intersects(rect), "the gear must not cover {what}");
    }
    assert!(
        gear.width() > 0.0 && gear.height() > 0.0,
        "a button needs a hit area"
    );
}

/// The song label takes the middle of the strip, between the wordmark and the pattern display.
#[test]
fn the_song_label_is_centred_in_the_strip() {
    let l = layout(screen());
    let s = scale(screen());
    let label = panel::song_label_rect(l.top, s);

    assert!(
        (label.center().x - l.top.center().x).abs() < l.top.width() * 0.12,
        "the name belongs near the middle of the strip"
    );
    for (what, rect) in [
        ("the wordmark", panel::brand_rect(l.top)),
        ("the pattern display", panel::pattern_display_rect(l.top)),
        ("the tempo display", panel::tempo_display_rect(l.top)),
    ] {
        assert!(!label.intersects(rect), "the name must not cover {what}");
    }
}
