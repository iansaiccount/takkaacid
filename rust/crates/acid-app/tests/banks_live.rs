//! The `BANKS` card: one list of every document the app knows — songs, pattern banks, racks and the factory demos —
//! searchable, filtered by kind, and scrolled by the band it is drawn in.
//!
//! The card is a painter: every test here drives `draw_panel` through real egui frames and asserts on the request the card
//! left behind, because what a click writes is `main`'s to do and not the card's. Every position comes from `common`, which
//! is a call into `banks::Rects`; there is no second copy of the card's arithmetic in this file.

mod common;

use acid_app::device::{DeviceKind, Rack};
use acid_app::persist::{BankDocument, SavedState, list_banks, load_bank, save_bank};
use acid_app::state::SharedState;
use acid_app::ui::PanelState;
use acid_app::ui::banks::{self, Banks, Filter, Kind, Request, rows};
use acid_app::ui::draw_panel;
use acid_core::bank::PatternBank;
use acid_core::pattern::Grid;
use common::*;
use eframe::egui::{Context, Event};

/// The smallest library that draws every kind once, in `Kind::ALL` order: the song's row, the bank's, the rack's, a demo's.
fn library() -> Banks {
    Banks {
        names: vec!["Acid 1".to_string()],
        bank_names: vec!["Easy".to_string()],
        rack_names: vec!["Studio".to_string()],
        demos: vec!["Slow Acid".to_string()],
        ..Banks::default()
    }
}

fn armed(panel: &PanelState) -> Option<&str> {
    panel
        .songs
        .card
        .as_ref()
        .and_then(|card| card.delete_armed.as_deref())
}

#[test]
fn every_kind_of_row_loads_what_its_tag_says() {
    // One list, one row per kind, each asking its own request: the row's kind is the only thing that decides what a click
    // means, and the four kinds sit in the catalogue's order.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);

    for (n, expected) in [
        (0, Request::Load("Acid 1".to_string())),
        (1, Request::LoadBank("Easy".to_string())),
        (2, Request::LoadRack("Studio".to_string())),
        (3, Request::LoadDemo(0)),
    ] {
        click(&ctx, &mut panel, &shared, row_centre(n));
        assert_eq!(panel.songs.request, Some(expected), "row {n}");
        panel.songs.request = None;
    }
}

#[test]
fn a_chip_narrows_the_list_and_the_strip_follows_it() {
    // The chips are views of one list, and the strip under it is the door for the kind you are looking at — every key naming
    // its own verb, so no key's meaning depends on where it sits.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);

    click(&ctx, &mut panel, &shared, chip_centre(Filter::Banks));
    let card = panel.songs.card.as_ref().unwrap();
    assert_eq!(card.filter, Filter::Banks);
    assert_eq!(rows(card).len(), 1, "the list is the bank rows only");
    panel.songs.card.as_mut().unwrap().bank_name = "Easy 2".to_string();
    click(&ctx, &mut panel, &shared, save_centre(Filter::Banks));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveBank("Easy 2".to_string())),
        "the strip is the bank's, and its key says SAVE BANK"
    );

    panel.songs.request = None;
    click(&ctx, &mut panel, &shared, chip_centre(Filter::Racks));
    assert_eq!(rows(panel.songs.card.as_ref().unwrap()).len(), 1);
    panel.songs.card.as_mut().unwrap().rack_name = "Studio 2".to_string();
    click(&ctx, &mut panel, &shared, save_centre(Filter::Racks));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveRack("Studio 2".to_string())),
        "and the rack's says SAVE RACK"
    );

    panel.songs.request = None;
    click(&ctx, &mut panel, &shared, chip_centre(Filter::All));
    panel.songs.card.as_mut().unwrap().name = "Acid 2".to_string();
    click(&ctx, &mut panel, &shared, save_centre(Filter::All));
    assert_eq!(
        panel.songs.request,
        Some(Request::Save("Acid 2".to_string())),
        "ALL puts the song's door back"
    );
}

#[test]
fn the_search_narrows_the_list_to_the_row_that_loads() {
    // The field is wired to `rows()`: what is typed narrows the list, and the row that survives is the one a click loads.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);

    click(&ctx, &mut panel, &shared, search_centre());
    run_on(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::Text("easy".to_string())],
    );
    assert_eq!(
        panel.songs.card.as_ref().unwrap().search,
        "easy",
        "the field took the text"
    );
    assert_eq!(
        rows(panel.songs.card.as_ref().unwrap()).len(),
        1,
        "one row survives the search"
    );

    click(&ctx, &mut panel, &shared, row_centre(0));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadBank("Easy".to_string())),
        "and it loads what its tag says"
    );
}

#[test]
fn a_search_that_matches_nothing_leaves_the_strip_working() {
    // An empty band is not a broken card: the strip is fixed furniture, and the door still answers.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);
    {
        let card = panel.songs.card.as_mut().unwrap();
        card.search = "zzz".to_string();
        card.name = "Kept".to_string();
    }
    assert!(rows(panel.songs.card.as_ref().unwrap()).is_empty());

    click(&ctx, &mut panel, &shared, save_centre(Filter::All));
    assert_eq!(
        panel.songs.request,
        Some(Request::Save("Kept".to_string())),
        "the strip still saves the song under an empty list"
    );
}

#[test]
fn the_song_row_and_the_rack_row_arm_under_their_own_keys() {
    // Two documents can share a name — `songs/Studio.json` and `racks/Studio.json` are different things to the app and one
    // word to a player — so each cross arms under a key that carries its kind, or the second click lands on the other file
    // and deletes it, which is the one action on this card with no undo.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);
    {
        let card = panel.songs.card.as_mut().unwrap();
        card.names = vec!["Studio".to_string()];
        card.rack_names = vec!["Studio".to_string()];
    }

    // The song's cross first: the rack's stays cold, and the song's own second click deletes the song.
    click(&ctx, &mut panel, &shared, cross_centre(0));
    assert_eq!(panel.songs.request, None, "the song's cross only arms");
    assert_eq!(armed(&panel), Some("song:Studio"));
    click(&ctx, &mut panel, &shared, cross_centre(2));
    assert_eq!(
        panel.songs.request, None,
        "the rack's cross did not fire on the song's arm"
    );
    assert_eq!(
        armed(&panel),
        Some("rack:Studio"),
        "it armed the rack instead"
    );
    click(&ctx, &mut panel, &shared, cross_centre(2));
    assert_eq!(
        panel.songs.request,
        Some(Request::DeleteRack("Studio".to_string())),
        "and its own second click deletes the rack"
    );
}

#[test]
fn a_library_of_thirty_songs_still_leaves_the_doors() {
    // Round 02's review found that twelve songs took SAVE RACK off the card. The doors are in the strip now rather than under
    // the list, so a library too big for the band cannot reach them: each chip's key answers with thirty songs above it.
    let ctx = Context::default();
    let shared = SharedState::new();
    let names: Vec<String> = (0..30).map(|index| format!("Song {index:02}")).collect();
    let mut panel = open_card(
        Banks {
            names,
            bank_names: vec!["Easy".to_string()],
            rack_names: vec!["Studio".to_string()],
            ..Banks::default()
        },
        false,
    );

    let count = rows(panel.songs.card.as_ref().unwrap()).len();
    let (visible, _) = banks::window(rects().band, scale_of(), count, 0);
    assert!(
        count > visible,
        "the library is bigger than the band: {count} rows into {visible}"
    );
    // And the band says so, or a player would not know there was more of the list than they can see.
    assert!(
        track_drawn(&ctx, &mut panel, &shared),
        "a list longer than the band draws the mark that says there is more of it"
    );

    click(&ctx, &mut panel, &shared, chip_centre(Filter::Banks));
    panel.songs.card.as_mut().unwrap().bank_name = "Another".to_string();
    click(&ctx, &mut panel, &shared, save_centre(Filter::Banks));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveBank("Another".to_string())),
        "SAVE BANK answers with thirty songs in the library"
    );

    panel.songs.request = None;
    click(&ctx, &mut panel, &shared, chip_centre(Filter::Racks));
    panel.songs.card.as_mut().unwrap().rack_name = "Another rack".to_string();
    click(&ctx, &mut panel, &shared, save_centre(Filter::Racks));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveRack("Another rack".to_string())),
        "and SAVE RACK, which is what round 02 lost"
    );

    panel.songs.request = None;
    click(&ctx, &mut panel, &shared, chip_centre(Filter::All));
    panel.songs.card.as_mut().unwrap().name = "A song".to_string();
    click(&ctx, &mut panel, &shared, save_centre(Filter::All));
    assert_eq!(
        panel.songs.request,
        Some(Request::Save("A song".to_string())),
        "and the song's own door"
    );
}

#[test]
fn a_dirty_session_arms_a_bank_load_first() {
    // Loading a bank replaces all sixty-four patterns of the selected device's bank, so an unsaved session costs two clicks,
    // exactly as the demo rows do.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), true);

    click(&ctx, &mut panel, &shared, row_centre(1));
    assert_eq!(panel.songs.request, None, "the first click loads nothing");
    let card = panel.songs.card.as_ref().unwrap();
    assert_eq!(
        card.load_armed.as_deref(),
        Some("bank:Easy"),
        "and the row says what a second click would replace"
    );
    assert!(
        card.message
            .as_deref()
            .is_some_and(|line| line.contains("replace")),
        "the card says which click it is waiting for"
    );

    click(&ctx, &mut panel, &shared, row_centre(1));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadBank("Easy".to_string()))
    );
}

#[test]
fn a_dirty_session_arms_a_rack_load_first() {
    // A rack load replaces every machine in the session, so an unsaved session costs two clicks, as a song's load does.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), true);

    click(&ctx, &mut panel, &shared, row_centre(2));
    assert_eq!(panel.songs.request, None, "the first click loads nothing");
    let card = panel.songs.card.as_ref().unwrap();
    assert_eq!(
        card.load_armed.as_deref(),
        Some("rack:Studio"),
        "the row says which click it is waiting for"
    );
    assert!(
        card.message
            .as_deref()
            .is_some_and(|line| line.contains("replace")),
        "and the card says so in words"
    );

    click(&ctx, &mut panel, &shared, row_centre(2));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadRack("Studio".to_string()))
    );
}

#[test]
fn a_rack_rows_cross_asks_to_delete_twice() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);

    click(&ctx, &mut panel, &shared, cross_centre(2));
    assert_eq!(panel.songs.request, None, "one click only arms the cross");
    assert_eq!(
        armed(&panel),
        Some("rack:Studio"),
        "and the cross turns into SURE, armed under the rack's own key"
    );

    click(&ctx, &mut panel, &shared, cross_centre(2));
    assert_eq!(
        panel.songs.request,
        Some(Request::DeleteRack("Studio".to_string())),
        "deleting a file has no undo, so it takes two clicks"
    );
}

#[test]
fn saving_a_rack_asks_with_the_typed_name_or_the_file_it_came_from() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);
    click(&ctx, &mut panel, &shared, chip_centre(Filter::Racks));
    panel.songs.card.as_mut().unwrap().rack_name = "Studio 2".to_string();

    click(&ctx, &mut panel, &shared, save_centre(Filter::Racks));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveRack("Studio 2".to_string())),
        "the typed name"
    );

    // A blank field keeps the rack the session came from, SAVE's own rule for the other document.
    panel.songs.request = None;
    panel.songs.card.as_mut().unwrap().rack_name.clear();
    panel.songs.rack_file = Some("Studio".to_string());
    click(&ctx, &mut panel, &shared, save_centre(Filter::Racks));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveRack("Studio".to_string())),
        "a blank field names the rack the session is on"
    );
}

#[test]
fn opening_the_card_prefills_the_rack_field_from_the_rack_the_session_is_on() {
    // `SAVE RACK` writes in place when the field names the rack the machines came from, so the field opens on it — the rule
    // `SAVE BANK`'s field keeps for the selected device's bank, said for the other document.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState {
        songs: acid_app::ui::SongState {
            rack_file: Some("Studio".to_string()),
            ..acid_app::ui::SongState::default()
        },
        ..PanelState::default()
    };

    click(&ctx, &mut panel, &shared, banks_key());
    assert!(panel.songs.card.is_some(), "the key opens the card");
    assert_eq!(
        panel.songs.card.as_ref().unwrap().rack_name,
        "Studio",
        "the rack field opens on the rack the session came from"
    );
}

#[test]
fn saving_a_bank_asks_with_the_typed_name() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);
    click(&ctx, &mut panel, &shared, chip_centre(Filter::Banks));
    panel.songs.card.as_mut().unwrap().bank_name = "Easy 2".to_string();

    click(&ctx, &mut panel, &shared, save_centre(Filter::Banks));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveBank("Easy 2".to_string()))
    );
}

#[test]
fn a_blank_bank_field_saves_the_bank_the_device_came_from() {
    // SAVE BANK's own rule, the one SAVE has: an untouched field means "this bank, where it is". `bank_file` is the session's
    // memory of which file the selected device's bank came from, so the card does not have to hold it.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);
    click(&ctx, &mut panel, &shared, chip_centre(Filter::Banks));
    panel.songs.bank_file = Some("Easy".to_string());

    click(&ctx, &mut panel, &shared, save_centre(Filter::Banks));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveBank("Easy".to_string())),
        "a blank field rewrites the bank the device is on rather than making a new one"
    );
}

#[test]
fn deleting_a_bank_takes_two_clicks() {
    // Deleting a file has no undo, so the cross arms first and says so, exactly as a song's does.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);

    click(&ctx, &mut panel, &shared, cross_centre(1));
    assert_eq!(panel.songs.request, None, "the first click deletes nothing");
    assert_eq!(
        armed(&panel),
        Some("bank:Easy"),
        "the cross says SURE and waits"
    );

    click(&ctx, &mut panel, &shared, cross_centre(1));
    assert_eq!(
        panel.songs.request,
        Some(Request::DeleteBank("Easy".to_string()))
    );
}

#[test]
fn a_saved_bank_is_a_row_the_card_lists_and_loads() {
    // The card lists what `main` puts in `bank_names`, and `main` fills that from `list_banks`. So a bank saved through one
    // door is a row at the other, and a click on it asks for that bank by name.
    let dir = tempfile::tempdir().unwrap();
    let written = BankDocument::of(&PatternBank::factory(), DeviceKind::Acid);
    save_bank(dir.path(), "Easy", &written).unwrap();

    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(library(), false);
    panel.songs.card.as_mut().unwrap().bank_names = list_banks(dir.path());

    assert_eq!(
        panel.songs.card.as_ref().unwrap().bank_names,
        vec!["Easy".to_string()],
        "the file is in the library the card reads"
    );
    click(&ctx, &mut panel, &shared, row_centre(1));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadBank("Easy".to_string()))
    );
}

#[test]
fn loading_a_bank_gives_the_device_all_sixty_four_patterns() {
    // What `main`'s load arm does once the card has asked: the document's bank becomes the selected device's whole bank, and
    // the device opens on its first pattern. The dirty flag and the notice are `main`'s to set, and are its own lines.
    let dir = tempfile::tempdir().unwrap();
    let mut written = PatternBank::factory();
    for index in 1..64 {
        let mut pattern = *written.get(0);
        pattern.steps[index % Grid::Sixteenth.steps_per_bar()].on = index % 2 == 0;
        written.replace(index, pattern);
    }
    save_bank(
        dir.path(),
        "Easy",
        &BankDocument::of(&written, DeviceKind::Acid),
    )
    .unwrap();

    let loaded = load_bank(dir.path(), "Easy").unwrap();
    let mut rack = Rack::from_saved(&SavedState::default());
    rack.device_mut().load_bank(loaded.bank);

    assert_eq!(
        rack.bank(),
        &written,
        "every one of the sixty-four patterns lands in the device"
    );
    assert_eq!(
        rack.bank().current(),
        0,
        "and the device opens on the first"
    );
}

/// Thirty songs, one bank and one rack: the library round 02's review broke the card with, and what a row below the band has
/// to be reached through.
fn crowded() -> Banks {
    Banks {
        names: (0..30).map(|index| format!("Song {index:02}")).collect(),
        bank_names: vec!["Easy".to_string(), "Second".to_string()],
        rack_names: vec!["Studio".to_string()],
        ..Banks::default()
    }
}

#[test]
fn the_wheel_scrolls_the_list_and_stops_at_the_ends() {
    // A wheel over the band moves the window, and stops at the last page rather than running off it — the shape the theme
    // list's own test uses, because both lists are one idiom.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(crowded(), false);
    let count = rows(panel.songs.card.as_ref().unwrap()).len();
    let (visible, _) = banks::window(rects().band, scale_of(), count, 0);

    wheel(&ctx, &mut panel, &shared, -60.0);
    let after = panel.songs.card.as_ref().unwrap().top;
    assert!(after > 0, "the wheel moved the window: {after}");

    for _ in 0..40 {
        wheel(&ctx, &mut panel, &shared, -120.0);
    }
    assert_eq!(
        panel.songs.card.as_ref().unwrap().top,
        count - visible,
        "the last page ends on the last row"
    );

    for _ in 0..40 {
        wheel(&ctx, &mut panel, &shared, 120.0);
    }
    assert_eq!(
        panel.songs.card.as_ref().unwrap().top,
        0,
        "and the other way returns to the top"
    );
}

#[test]
fn a_wheel_over_the_strip_does_not_scroll_the_list() {
    // The strip and the footer are fixed furniture: a wheel over them must not move the rows under the hand that is aiming at
    // a key.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(crowded(), false);

    for _ in 0..5 {
        wheel_at(&ctx, &mut panel, &shared, save_centre(Filter::All), -120.0);
    }

    assert_eq!(
        panel.songs.card.as_ref().unwrap().top,
        0,
        "a wheel over the strip leaves the list where it is"
    );
}

#[test]
fn a_delete_re_clamps_the_window_without_re_topping_it() {
    // The list re-flows under the hand that clicked the cross. Jumping the view to the top would be a surprise; clamping it
    // into the shorter list is a correction.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(crowded(), false);

    // Where the wheel would have left it, one page past the end of the list.
    panel.songs.card.as_mut().unwrap().top = 20;
    run_on(&ctx, &mut panel, &shared, Vec::new());
    let count = rows(panel.songs.card.as_ref().unwrap()).len();
    let (visible, _) = banks::window(rects().band, scale_of(), count, 0);
    assert_eq!(
        panel.songs.card.as_ref().unwrap().top,
        count - visible,
        "the request is clamped into the window"
    );

    panel.songs.card.as_mut().unwrap().names.remove(0);
    run_on(&ctx, &mut panel, &shared, Vec::new());
    assert_eq!(
        panel.songs.card.as_ref().unwrap().top,
        count - visible - 1,
        "deleting one row re-clamps the window by one rather than re-topping it"
    );
}

#[test]
fn a_list_that_fits_draws_no_track_or_thumb() {
    // A library that fits looks exactly as the card did before there was a thumb: no line down its right edge.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(
        Banks {
            names: vec!["Acid 1".to_string(), "Another".to_string()],
            ..Banks::default()
        },
        false,
    );

    let count = rows(panel.songs.card.as_ref().unwrap()).len();
    let (visible, _) = banks::window(rects().band, scale_of(), count, 0);
    assert_eq!(visible, count, "two rows fit the band");
    assert!(
        banks::thumb(rects().band, scale_of(), count, visible, 0).is_none(),
        "a list that fits has nothing to say about there being more of it"
    );
    assert!(
        !track_drawn(&ctx, &mut panel, &shared),
        "and the card draws no line down the band"
    );
}

#[test]
fn a_row_below_the_band_is_reached_by_scrolling_to_it() {
    // The other half of round 02's finding: the list shows what fits, and the wheel reaches the rest. A bank row past the
    // first page loads after a turn of the wheel, which is what "a row the list cannot show is reachable" means.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(crowded(), false);

    let list = rows(panel.songs.card.as_ref().unwrap());
    let bank = list
        .iter()
        .position(|row| row.kind == Kind::Bank && row.name == "Second")
        .expect("the second bank is in the list");
    let (visible, _) = banks::window(rects().band, scale_of(), list.len(), 0);
    assert!(bank >= visible, "the row starts below the first page");

    for _ in 0..40 {
        wheel(&ctx, &mut panel, &shared, -120.0);
    }
    let top = panel.songs.card.as_ref().unwrap().top;
    assert!(
        bank >= top && bank < top + visible,
        "the row is in the last page"
    );

    click(&ctx, &mut panel, &shared, row_centre(bank - top));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadBank("Second".to_string())),
        "the far bank row loads once it is scrolled into the band"
    );
}

/// Whether the card drew the mark that says there is more of the list: a filled rect inside the band's right-hand column, the
/// width `CARD_TRACK` reserves. Read out of a real frame's shapes, because the mark is a shape and not a widget.
fn track_drawn(ctx: &Context, panel: &mut PanelState, shared: &SharedState) -> bool {
    let input = eframe::egui::RawInput {
        screen_rect: Some(screen()),
        events: Vec::new(),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();

    let band = rects().band;
    let track = banks::CARD_TRACK * scale_of();
    fn walk(shape: &eframe::egui::Shape, band: eframe::egui::Rect, track: f32) -> bool {
        match shape {
            eframe::egui::Shape::Rect(rect) => {
                band.contains_rect(rect.rect) && rect.rect.right() > band.right() - track
            }
            eframe::egui::Shape::Vec(shapes) => shapes.iter().any(|shape| walk(shape, band, track)),
            _ => false,
        }
    }
    output
        .shapes
        .iter()
        .any(|clipped| walk(&clipped.shape, band, track))
}
