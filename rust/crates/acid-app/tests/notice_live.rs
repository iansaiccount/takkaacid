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
    let _ = frame(ctx, panel, shared);
}

/// Paint one frame and answer every line it drew, with the width that line really took.
fn frame(ctx: &Context, panel: &mut PanelState, shared: &SharedState) -> Vec<(String, f32)> {
    let input = RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    painted_lines(&output)
}

/// Every string egui drew this frame and how wide it was, so a test can assert on what the face says and on whether it
/// fits the room it was given.
fn painted_lines(output: &eframe::egui::FullOutput) -> Vec<(String, f32)> {
    fn walk(shape: &eframe::egui::Shape, out: &mut Vec<(String, f32)>) {
        match shape {
            eframe::egui::Shape::Text(text) => {
                out.push((text.galley.text().to_string(), text.galley.rect.width()));
            }
            eframe::egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
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

/// The song label sits on the strip's own middle — not on the midpoint of the gap it lives in. The left cluster
/// (volume, wordmark, lamp, gear, `?`) is far wider than the right (pattern, tempo), so the gap's midpoint is about
/// sixty pixels right of centre and the name reads off-centre.
#[test]
fn the_song_label_is_centred_in_the_strip() {
    let l = layout(screen());
    let s = scale(screen());
    let label = panel::song_label_rect(l.top, s);

    assert!(
        (label.center().x - l.top.center().x).abs() <= 1.0,
        "the name belongs on the middle of the strip: {} vs {}",
        label.center().x,
        l.top.center().x
    );
    for (what, rect) in [
        ("the wordmark", panel::brand_rect(l.top)),
        ("the saves lamp", panel::save_indicator_rect(l.top, s)),
        ("the gear", panel::gear_rect(l.top, s)),
        ("the help dot", panel::help_rect(l.top, s)),
        ("the pattern display", panel::pattern_display_rect(l.top)),
        ("the tempo display", panel::tempo_display_rect(l.top)),
    ] {
        assert!(!label.intersects(rect), "the name must not cover {what}");
    }
    assert!(
        l.top.contains_rect(label),
        "the label lives on the top strip"
    );
}

/// The window spends the room evenly and wastes none: half its width is exactly the room the tighter side allows, counted
/// from the *strip's* centre. Both numbers come from outside the rect, so the pre-fix rect — the whole gap, centred on the
/// gap's own midpoint — fails this (132.75 against 77.7 at 980 × 560).
#[test]
fn the_song_label_window_uses_all_the_room_the_tighter_side_allows() {
    let l = layout(screen());
    let s = scale(screen());
    let label = panel::song_label_rect(l.top, s);
    let bound_left = panel::brand_rect(l.top).right() + acid_app::ui::theme::GAP * s;
    let bound_right = panel::pattern_display_rect(l.top).left() - acid_app::ui::theme::GAP * s;
    let centre = l.top.center().x;
    let room = (centre - bound_left).min(bound_right - centre);

    assert!(
        (label.width() * 0.5 - room).abs() <= 1.0,
        "the window should be as wide as the tighter side allows: {} of {room}",
        label.width() * 0.5
    );
    // The left cluster is the tighter side at these proportions, so that is the edge the window must reach.
    assert!(
        (label.left() - bound_left).abs() <= 1.0,
        "the window should reach the tighter neighbour: {} vs {bound_left}",
        label.left()
    );
}

/// A window small enough to squeeze the strip must still yield a usable rect rather than an inverted one.
#[test]
fn the_song_label_window_survives_a_squeezed_strip() {
    // 320 × 200 and 240 × 160 squeeze the strip but still leave the two clusters apart; 40 × 30 is narrow enough that
    // the wordmark and the pattern display cross, which is the branch where the window has to collapse rather than invert.
    for size in [
        vec2(320.0, 200.0),
        vec2(240.0, 160.0),
        vec2(40.0, 30.0),
        vec2(20.0, 16.0),
    ] {
        let l = layout(Rect::from_min_size(Pos2::ZERO, size));
        let label = panel::song_label_rect(l.top, scale(Rect::from_min_size(Pos2::ZERO, size)));
        assert!(
            label.width() >= 0.0 && label.height() > 0.0,
            "an inverted or empty label at {size:?}: {label:?}"
        );
        assert!(
            l.top.contains_rect(label),
            "the label left the strip at {size:?}: {label:?} in {:?}",
            l.top
        );
    }
}

/// A notice longer than the label keeps its beginning. The clip rect used to cut the message at both ends, so the player
/// read neither the verb at the front nor the detail at the back.
#[test]
fn a_long_notice_keeps_its_beginning() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let long = "could not import: No such file or directory (os error 2), and the file it names is not here either";
    panel.notify(long);

    let lines = frame(&ctx, &mut panel, &shared);
    let (line, width) = lines
        .iter()
        .find(|(text, _)| text.starts_with("could not import"))
        .expect("the notice must be on the line");

    let l = layout(screen());
    let rect = panel::song_label_rect(l.top, scale(screen()));
    assert!(
        line.ends_with('…'),
        "a line that was cut has to say it was cut: {line}"
    );
    assert!(
        *width <= rect.width() + 0.5,
        "the notice has to fit the line it is given: {width} in {}",
        rect.width()
    );
    assert!(
        line.len() > 8,
        "the beginning is the part worth keeping: {line}"
    );
}

/// A notice that fits is left exactly as it was written: no ellipsis, no cut.
#[test]
fn a_notice_that_fits_is_untouched() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    panel.notify("saved Acid 1");

    let lines = frame(&ctx, &mut panel, &shared);
    let found = lines.iter().any(|(text, _)| text == "saved Acid 1");
    assert!(found, "a short notice is painted as written: {lines:?}");
}
