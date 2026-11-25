//! The songs on the `BANKS` card: the field that names the session, the two keys that save it, the footer's doors, and the
//! list's rows as this file's half of the card's behaviour.
//!
//! The card never touches the disk: every test here asserts on the request it left behind. Every position comes from
//! `common`, which is a call into `banks::Rects` — round 02's review found the card's arithmetic in two files here, and this
//! one no longer has any of it.

mod common;

use acid_app::device::{DeviceKind, Scheme};
use acid_app::state::SharedState;
use acid_app::ui::banks::{Banks, Filter, Request};
use acid_app::ui::strip::{Strip, StripCell};
use acid_app::ui::{PanelState, draw_panel};
use common::*;
use eframe::egui::{Context, RawInput};

fn library(names: Vec<&str>, name: &str) -> Banks {
    Banks {
        names: names.into_iter().map(str::to_string).collect(),
        name: name.to_string(),
        ..Banks::default()
    }
}

/// A panel showing a drum machine, with the card open over it and the session unsaved: the demo rows are then armed by the
/// state that says they would replace that device's grid, not the session.
fn open_card_on_drums() -> PanelState {
    let mut panel = open_card(library(vec![], ""), true);
    panel.strip = Strip {
        cells: vec![StripCell {
            name: "DRUMS 1".to_string(),
            kind: DeviceKind::Drums,
            number: 1,
            scheme: Scheme::Black,
            running: false,
            muted: false,
        }],
        selected: 0,
    };
    if let Some(card) = panel.songs.card.as_mut() {
        card.demos = vec!["Slow Acid".to_string()];
    }
    panel
}

#[test]
fn a_drums_demo_row_is_armed_against_its_grid_not_the_session() {
    // The two-click guard names what a second click throws away. On a 303 that is the session; on a drum machine it is that
    // device's eight patterns, and the card said "session" on both faces until the face it was wrong on was fixed.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card_on_drums();

    click(&ctx, &mut panel, &shared, row_centre(0));

    let card = panel.songs.card.as_ref().unwrap();
    assert_eq!(
        card.load_armed.as_deref(),
        Some("demo:0"),
        "the first click arms the row instead of loading"
    );
    let message = card.message.clone().unwrap_or_default();
    assert!(
        message.contains("grid") && !message.contains("session"),
        "the drum face's guard names the wrong thing: {message}"
    );
    assert_eq!(panel.songs.request, None, "the first click loads nothing");
}

#[test]
fn a_song_row_asks_to_load_that_song() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1", "Another"], ""), false);

    click(&ctx, &mut panel, &shared, row_centre(1));

    assert_eq!(
        panel.songs.request,
        Some(Request::Load("Another".to_string()))
    );
}

#[test]
fn save_asks_for_the_typed_name() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec![], "Acid 2"), false);

    let at = save_centre(&panel);
    click(&ctx, &mut panel, &shared, at);

    assert_eq!(
        panel.songs.request,
        Some(Request::Save("Acid 2".to_string()))
    );
}

/// A blank field is not a dead button: it saves the song the session is already on.
#[test]
fn a_blank_field_still_saves_the_open_song() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1"], ""), false);
    panel.songs.card.as_mut().unwrap().open = Some("Acid 1".to_string());

    let at = save_centre(&panel);
    click(&ctx, &mut panel, &shared, at);

    assert_eq!(
        panel.songs.request,
        Some(Request::Save("Acid 1".to_string()))
    );
}

#[test]
fn save_a_copy_asks_for_a_copy_of_the_typed_name() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1"], "Acid 1"), false);
    panel.songs.card.as_mut().unwrap().open = Some("Acid 1".to_string());

    let at = copy_centre(&panel);
    click(&ctx, &mut panel, &shared, at);

    assert_eq!(
        panel.songs.request,
        Some(Request::SaveCopy("Acid 1".to_string()))
    );
}

#[test]
fn import_and_export_ask_for_their_requests() {
    let ctx = Context::default();
    let shared = SharedState::new();

    let mut importing = open_card(library(vec![], ""), false);
    click(&ctx, &mut importing, &shared, footer_import_centre());
    assert_eq!(importing.songs.request, Some(Request::Import));

    let mut exporting = open_card(library(vec![], "Acid 1"), false);
    click(&ctx, &mut exporting, &shared, footer_export_centre());
    assert_eq!(exporting.songs.request, Some(Request::Export));
}

#[test]
fn a_name_that_cannot_be_a_file_asks_for_nothing() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec![], "a/b"), false);

    let at = save_centre(&panel);
    click(&ctx, &mut panel, &shared, at);

    assert_eq!(panel.songs.request, None, "a path is not a song name");
}

#[test]
fn close_takes_the_card_away() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1"], ""), false);

    click(&ctx, &mut panel, &shared, footer_close_centre());

    assert!(panel.songs.card.is_none());
    assert_eq!(panel.songs.request, None);
}

#[test]
fn the_songs_key_opens_and_closes_the_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared, banks_key());
    assert!(panel.songs.card.is_some(), "the key opens the card");

    click(&ctx, &mut panel, &shared, banks_key());
    assert!(panel.songs.card.is_none(), "and closes it again");
}

#[test]
fn the_card_the_request_and_the_name_are_one_piece_of_state() {
    // A request without a card, or a name for a song nobody opened, is not a state the panel can be in: they travel
    // together, so they are one value. Opening the card must not disturb the port picker, which is its own story.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    assert!(panel.songs.card.is_none());
    assert!(panel.songs.request.is_none());
    assert!(panel.songs.name.is_none());

    click(&ctx, &mut panel, &shared, banks_key());
    assert!(panel.songs.card.is_some(), "the key opens the card");
    assert!(panel.songs.request.is_none(), "opening asks for nothing");
    assert!(
        panel.picker.output_choice.is_none(),
        "the output picker is untouched"
    );
}

/// NEW replaces the session, so while the session is unsaved it asks twice. One click arms it and says so; the second
/// fires. A saved session does not need the ceremony.
#[test]
fn new_asks_twice_while_the_session_is_unsaved() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1"], ""), true);

    click(&ctx, &mut panel, &shared, footer_new_centre());
    assert_eq!(panel.songs.request, None, "one click only arms it");
    assert_eq!(
        panel.songs.card.as_ref().unwrap().load_armed.as_deref(),
        Some("new"),
        "the footer says SURE? and the next click fires"
    );

    click(&ctx, &mut panel, &shared, footer_new_centre());
    assert_eq!(panel.songs.request, Some(Request::New));
}

#[test]
fn the_new_song_button_asks_for_a_fresh_song() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1"], ""), false);

    click(&ctx, &mut panel, &shared, footer_new_centre());

    assert_eq!(panel.songs.request, Some(Request::New));
}

#[test]
fn deleting_a_song_takes_two_clicks() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1", "Another"], ""), false);

    click(&ctx, &mut panel, &shared, cross_centre(1));
    assert_eq!(panel.songs.request, None, "one click only arms it");
    assert_eq!(
        panel.songs.card.as_ref().unwrap().delete_armed.as_deref(),
        Some("song:Another"),
        "the cross arms under the song's own key, not its bare name"
    );

    click(&ctx, &mut panel, &shared, cross_centre(1));
    assert_eq!(
        panel.songs.request,
        Some(Request::Delete("Another".to_string()))
    );
}

#[test]
fn loading_a_song_clears_a_pending_delete() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1", "Another"], ""), false);

    click(&ctx, &mut panel, &shared, cross_centre(0));
    assert!(panel.songs.card.as_ref().unwrap().delete_armed.is_some());

    click(&ctx, &mut panel, &shared, row_centre(1));
    assert_eq!(panel.songs.card.as_ref().unwrap().delete_armed, None);
    assert_eq!(
        panel.songs.request,
        Some(Request::Load("Another".to_string()))
    );
}

#[test]
fn a_demo_row_asks_to_load_that_demo() {
    // A demo is not a file: the row asks for its index, and the card never tries to delete or rename it.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec![], ""), false);
    panel.songs.card.as_mut().unwrap().demos = vec!["Slow Acid".to_string(), "Funk".to_string()];

    click(&ctx, &mut panel, &shared, row_centre(1));

    assert_eq!(panel.songs.request, Some(Request::LoadDemo(1)));
}

#[test]
fn replacing_an_unsaved_session_takes_two_clicks() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec![], ""), true);
    panel.songs.card.as_mut().unwrap().demos = vec!["Slow Acid".to_string()];

    click(&ctx, &mut panel, &shared, row_centre(0));
    assert_eq!(panel.songs.request, None, "one click only arms it");
    assert!(
        panel.songs.card.as_ref().unwrap().message.is_some(),
        "and says why nothing happened"
    );

    click(&ctx, &mut panel, &shared, row_centre(0));
    assert_eq!(panel.songs.request, Some(Request::LoadDemo(0)));
}

#[test]
fn replacing_an_unsaved_session_with_a_saved_song_takes_two_clicks() {
    // The guard is one path for every kind: a saved row replaces an unsaved session just as a demo row does.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec!["Acid 1"], ""), true);

    click(&ctx, &mut panel, &shared, row_centre(0));
    assert_eq!(panel.songs.request, None, "one click only arms it");

    click(&ctx, &mut panel, &shared, row_centre(0));
    assert_eq!(
        panel.songs.request,
        Some(Request::Load("Acid 1".to_string()))
    );
}

/// The header tells the truth about the session: its name, and whether that name is on disk. A named-but-unsaved session
/// — a first run, a demo — is unsaved, not untitled.
#[test]
fn the_header_names_the_session_and_its_state() {
    use acid_app::ui::banks::now_line;
    assert_eq!(now_line(None, true), "NOW: untitled - UNSAVED");
    assert_eq!(now_line(None, false), "NOW: untitled - UNSAVED");
    assert_eq!(now_line(Some("New song"), true), "NOW: New song - UNSAVED");
    assert_eq!(now_line(Some("Acid 1"), false), "NOW: Acid 1 - SAVED");
}

/// A player accumulates songs, and the card's band is one fixed height. The rows are a window over the list, so the ones the
/// band cannot show are scrolled to rather than drawn past it — the alternative is a list painted over
/// `NEW`/`IMPORT`/`EXPORT`/`CLOSE` and outside the card.
#[test]
fn a_long_song_list_is_drawn_only_inside_the_band() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let names: Vec<String> = (0..60).map(|index| format!("Song {index:02}")).collect();
    let mut panel = open_card(
        Banks {
            names: names.clone(),
            ..Banks::default()
        },
        false,
    );

    let input = RawInput {
        screen_rect: Some(screen()),
        events: Vec::new(),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
    output.textures_delta.clear();

    let band = rects().band;
    let mut painted = 0;
    fn walk(shape: &eframe::egui::Shape, band: eframe::egui::Rect, painted: &mut usize) {
        match shape {
            eframe::egui::Shape::Text(text) if text.galley.text().starts_with("Song ") => {
                *painted += 1;
                assert!(
                    band.contains(text.pos),
                    "a song row is painted outside the band: {} at {:?} (band {band:?})",
                    text.galley.text(),
                    text.pos
                );
            }
            eframe::egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, band, painted);
                }
            }
            _ => {}
        }
    }
    for clipped in &output.shapes {
        walk(&clipped.shape, band, &mut painted);
    }
    assert!(
        painted > 0 && painted < names.len(),
        "the list is a window on the band: {painted} of {} rows drawn",
        names.len()
    );
}

/// A session that has never been saved is a first run or a demo: the header must not call it untitled while the name
/// field and the panel's own label both show a name.
#[test]
fn a_named_session_with_no_file_is_unsaved_rather_than_untitled() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(vec![], "New song"), false);
    assert!(panel.songs.file.is_none());

    // Drawing with a name and no file is the first-run shape; it must not panic and must leave the name alone.
    run_on(&ctx, &mut panel, &shared, Vec::new());
    assert_eq!(
        panel.songs.card.as_ref().unwrap().name,
        "New song",
        "drawing the header must not rewrite the field"
    );
}

/// The card opens on `ALL` with nothing typed: the chips and the search are the card's own memory, and it is dropped when the
/// card closes, so both are forgotten with it.
#[test]
fn a_fresh_card_opens_on_all_with_an_empty_search() {
    let mut panel = PanelState::default();
    let ctx = Context::default();
    let shared = SharedState::new();

    click(&ctx, &mut panel, &shared, banks_key());

    let card = panel.songs.card.as_ref().expect("the key opens the card");
    assert_eq!(card.filter, Filter::All);
    assert_eq!(card.search, "");
    assert_eq!(card.top, 0);
}
