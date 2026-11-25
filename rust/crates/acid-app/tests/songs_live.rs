//! The songs card: it lists the songs on disk, reports SAVE AS and LOAD, and never touches the disk itself.

use acid_app::state::SharedState;
use acid_app::ui::banks::{Banks, Request};
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::switches::{self, Switch};
use acid_app::ui::{PanelState, SongState, draw_panel, theme};
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

/// The card covers the panel with a small inset; these mirror `songs::paint`'s own arithmetic.
fn card() -> Rect {
    layout(screen())
        .panel
        .shrink(theme::MARGIN * scale(screen()))
}

fn padding() -> f32 {
    12.0 * scale(screen())
}

fn row_height() -> f32 {
    22.0 * scale(screen())
}

fn field_y() -> f32 {
    card().top() + padding() + 18.0 * scale(screen())
}

/// The name field and SAVE share the first row of the card.
fn save_centre() -> Pos2 {
    let width = card().width() - padding() * 2.0;
    let left = card().left() + padding() + width * 0.40 + padding();
    Pos2::new(
        left + (width * 0.20 - padding()) * 0.5,
        field_y() + row_height() * 0.5,
    )
}

/// SAVE A COPY sits after SAVE on the same row.
fn save_copy_centre() -> Pos2 {
    let width = card().width() - padding() * 2.0;
    let save_left = card().left() + padding() + width * 0.40 + padding();
    let copy_left = save_left + width * 0.20 - padding() + padding();
    Pos2::new(
        copy_left + (width * 0.22 - padding()) * 0.5,
        field_y() + row_height() * 0.5,
    )
}

/// IMPORT and EXPORT sit in the footer, after NEW.
fn import_centre() -> Pos2 {
    Pos2::new(
        card().left() + padding() + 80.0 * scale(screen()) + padding() + 48.0 * scale(screen()),
        card().bottom() - padding() - row_height() * 0.5,
    )
}

fn export_centre() -> Pos2 {
    Pos2::new(
        card().left()
            + padding()
            + (80.0 + 96.0) * scale(screen())
            + padding() * 2.0
            + 48.0 * scale(screen()),
        card().bottom() - padding() - row_height() * 0.5,
    )
}

fn heading() -> f32 {
    15.0 * scale(screen())
}

fn row_step() -> f32 {
    row_height() + 2.0 * scale(screen())
}

/// Song rows start under the first heading, which follows the field that names them: the player's own songs are the first
/// list on the card.
fn songs_top() -> f32 {
    field_y() + row_height() + padding() + heading()
}

fn song_row_centre(index: usize) -> Pos2 {
    Pos2::new(
        card().left() + padding() + 20.0,
        songs_top() + row_step() * index as f32 + row_height() * 0.5,
    )
}

fn close_centre() -> Pos2 {
    Pos2::new(
        card().right() - padding() - 45.0 * scale(screen()),
        card().bottom() - padding() - row_height() * 0.5,
    )
}

fn open_card(names: Vec<&str>, name: &str) -> PanelState {
    PanelState {
        songs: SongState {
            card: Some(Banks {
                names: names.into_iter().map(str::to_string).collect(),
                name: name.to_string(),
                ..Banks::default()
            }),
            ..SongState::default()
        },
        ..PanelState::default()
    }
}
/// A panel showing a drum machine, with the songs card open over it and the session unsaved: the card's demo rows are then
/// armed by the state that says they would replace that device's grid, not the session.
fn open_card_on_drums() -> PanelState {
    use acid_app::device::DeviceKind;
    use acid_app::ui::strip::{Strip, StripCell};

    let mut panel = open_card(vec![], "");
    panel.dirty = true;
    panel.strip = Strip {
        cells: vec![StripCell {
            name: "DRUMS 1".to_string(),
            kind: DeviceKind::Drums,
            number: 1,
            scheme: acid_app::device::Scheme::Black,
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

    click(&ctx, &mut panel, &shared, demo_row_centre(0, 0));

    assert_eq!(
        panel
            .songs
            .card
            .as_ref()
            .and_then(|card| card.load_armed.clone()),
        Some("demo:0".to_string()),
        "the first click arms the row instead of loading"
    );
    let message = panel
        .songs
        .card
        .as_ref()
        .and_then(|card| card.message.clone())
        .unwrap_or_default();
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
    let mut panel = open_card(vec!["Acid 1", "Another"], "");

    click(&ctx, &mut panel, &shared, song_row_centre(1));

    assert_eq!(
        panel.songs.request,
        Some(Request::Load("Another".to_string()))
    );
}

#[test]
fn save_asks_for_the_typed_name() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec![], "Acid 2");

    click(&ctx, &mut panel, &shared, save_centre());

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
    let mut panel = open_song_card(vec!["Acid 1"], "", Some("Acid 1"));

    click(&ctx, &mut panel, &shared, save_centre());

    assert_eq!(
        panel.songs.request,
        Some(Request::Save("Acid 1".to_string()))
    );
}

#[test]
fn save_a_copy_asks_for_a_copy_of_the_typed_name() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_song_card(vec!["Acid 1"], "Acid 1", Some("Acid 1"));

    click(&ctx, &mut panel, &shared, save_copy_centre());

    assert_eq!(
        panel.songs.request,
        Some(Request::SaveCopy("Acid 1".to_string()))
    );
}

#[test]
fn import_and_export_ask_for_their_requests() {
    let ctx = Context::default();
    let shared = SharedState::new();

    let mut importing = open_card(vec![], "");
    click(&ctx, &mut importing, &shared, import_centre());
    assert_eq!(importing.songs.request, Some(Request::Import));

    let mut exporting = open_card(vec![], "Acid 1");
    click(&ctx, &mut exporting, &shared, export_centre());
    assert_eq!(exporting.songs.request, Some(Request::Export));
}

#[test]
fn a_name_that_cannot_be_a_file_asks_for_nothing() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec![], "a/b");

    click(&ctx, &mut panel, &shared, save_centre());

    assert_eq!(panel.songs.request, None, "a path is not a song name");
}

#[test]
fn close_takes_the_card_away() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1"], "");

    click(&ctx, &mut panel, &shared, close_centre());

    assert!(panel.songs.card.is_none());
    assert_eq!(panel.songs.request, None);
}

#[test]
fn the_songs_key_opens_and_closes_the_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    // Six cells now: BANK, PATTERN, MODE, CLEAR, SAVE, SONGS.
    let songs = switches::cell(layout(screen()).switches, Switch::Banks).center();

    click(&ctx, &mut panel, &shared, songs);
    assert!(panel.songs.card.is_some(), "the SONGS key opens the card");

    click(&ctx, &mut panel, &shared, songs);
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

    click(
        &ctx,
        &mut panel,
        &shared,
        switches::cell(layout(screen()).switches, Switch::Banks).center(),
    );
    assert!(panel.songs.card.is_some(), "the SONGS key opens the card");
    assert!(panel.songs.request.is_none(), "opening asks for nothing");
    assert!(
        panel.picker.output_choice.is_none(),
        "the output picker is untouched"
    );
}

/// NEW SONG sits at the bottom left of the card.
fn new_song_centre() -> Pos2 {
    Pos2::new(
        card().left() + padding() + 60.0 * scale(screen()),
        card().bottom() - padding() - row_height() * 0.5,
    )
}

/// The delete cross sits to the right of song row `index`.
fn delete_centre(index: usize) -> Pos2 {
    let width = card().width() - padding() * 2.0;
    Pos2::new(
        card().left() + padding() + width * 0.66 + padding() + width * 0.07,
        song_row_centre(index).y,
    )
}

fn open_song_card(names: Vec<&str>, name: &str, open: Option<&str>) -> PanelState {
    let mut panel = open_card(names, name);
    if let Some(card) = panel.songs.card.as_mut() {
        card.open = open.map(str::to_string);
    }
    panel
}

/// NEW replaces the session, so while the session is unsaved it asks twice. One click arms it and says so; the second
/// fires. A saved session does not need the ceremony.
#[test]
fn new_asks_twice_while_the_session_is_unsaved() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1"], "");
    panel.dirty = true;

    click(&ctx, &mut panel, &shared, new_song_centre());
    assert_eq!(panel.songs.request, None, "one click only arms it");
    assert_eq!(
        panel.songs.card.as_ref().unwrap().load_armed.as_deref(),
        Some("new"),
        "the footer says SURE? and the next click fires"
    );

    click(&ctx, &mut panel, &shared, new_song_centre());
    assert_eq!(panel.songs.request, Some(Request::New));
}

#[test]
fn the_new_song_button_asks_for_a_fresh_song() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1"], "");

    click(&ctx, &mut panel, &shared, new_song_centre());

    assert_eq!(panel.songs.request, Some(Request::New));
}

#[test]
fn deleting_a_song_takes_two_clicks() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1", "Another"], "");

    click(&ctx, &mut panel, &shared, delete_centre(1));
    assert_eq!(panel.songs.request, None, "one click only arms it");
    assert_eq!(
        panel.songs.card.as_ref().unwrap().delete_armed.as_deref(),
        Some("Another")
    );

    click(&ctx, &mut panel, &shared, delete_centre(1));
    assert_eq!(
        panel.songs.request,
        Some(Request::Delete("Another".to_string()))
    );
}

#[test]
fn loading_a_song_clears_a_pending_delete() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1", "Another"], "");

    click(&ctx, &mut panel, &shared, delete_centre(0));
    assert!(panel.songs.card.as_ref().unwrap().delete_armed.is_some());

    click(&ctx, &mut panel, &shared, song_row_centre(1));
    assert_eq!(panel.songs.card.as_ref().unwrap().delete_armed, None);
    assert_eq!(
        panel.songs.request,
        Some(Request::Load("Another".to_string()))
    );
}

/// The demo rows follow the songs: `DEMO BANKS` heads the second list, under `count` song rows.
fn demo_row_centre(count: usize, index: usize) -> Pos2 {
    let y =
        field_y() + row_height() + padding() + heading() + row_step() * count as f32 + heading();
    Pos2::new(
        card().left() + padding() + 20.0,
        y + (row_height() + 2.0 * scale(screen())) * index as f32 + row_height() * 0.5,
    )
}

fn open_card_with_demos(names: Vec<&str>, name: &str, dirty: bool) -> PanelState {
    let mut panel = open_card(names, name);
    if let Some(card) = panel.songs.card.as_mut() {
        card.demos = vec!["Slow Acid".to_string(), "Funk".to_string()];
    }
    panel.dirty = dirty;
    panel
}

#[test]
fn a_demo_row_asks_to_load_that_demo() {
    // A demo is not a file: the row asks for its index, and the card never tries to delete or rename it.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card_with_demos(vec![], "", false);

    click(&ctx, &mut panel, &shared, demo_row_centre(0, 1));

    assert_eq!(panel.songs.request, Some(Request::LoadDemo(1)));
}

#[test]
fn replacing_an_unsaved_session_takes_two_clicks() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card_with_demos(vec![], "", true);

    click(&ctx, &mut panel, &shared, demo_row_centre(0, 0));
    assert_eq!(panel.songs.request, None, "one click only arms it");
    assert!(
        panel.songs.card.as_ref().unwrap().message.is_some(),
        "and says why nothing happened"
    );

    click(&ctx, &mut panel, &shared, demo_row_centre(0, 0));
    assert_eq!(panel.songs.request, Some(Request::LoadDemo(0)));
}

#[test]
fn replacing_an_unsaved_session_with_a_saved_song_takes_two_clicks() {
    // The guard is one path for both lists: a saved row replaces an unsaved session just as a demo row does.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1"], "");
    panel.dirty = true;

    click(&ctx, &mut panel, &shared, song_row_centre(0));
    assert_eq!(panel.songs.request, None, "one click only arms it");

    click(&ctx, &mut panel, &shared, song_row_centre(0));
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

/// A player accumulates songs, and the card has one screen. Past the room above the footer the rows stop being drawn, as the
/// kit browser's do: the alternative is a list painted over `NEW`/`IMPORT`/`EXPORT`/`CLOSE` and off the bottom of the card.
#[test]
fn a_long_song_list_is_capped_above_the_footer_keys() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let names: Vec<String> = (0..60).map(|index| format!("Song {index:02}")).collect();
    let mut panel = PanelState {
        songs: SongState {
            card: Some(Banks {
                names: names.clone(),
                ..Banks::default()
            }),
            ..SongState::default()
        },
        ..PanelState::default()
    };

    let input = RawInput {
        screen_rect: Some(screen()),
        events: Vec::new(),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(&mut panel, &shared, ui));
    output.textures_delta.clear();

    // The footer's band: the last `row_height` of the card, which is where NEW/IMPORT/EXPORT/CLOSE are painted.
    let footer_top = card().bottom() - padding() - row_height();
    let mut painted = 0;
    fn walk(shape: &egui::Shape, footer_top: f32, painted: &mut usize) {
        match shape {
            egui::Shape::Text(text) if text.galley.text().starts_with("Song ") => {
                *painted += 1;
                assert!(
                    text.pos.y < footer_top,
                    "a song row is painted over the footer keys: {} at y {} (limit {footer_top})",
                    text.galley.text(),
                    text.pos.y
                );
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, footer_top, painted);
                }
            }
            _ => {}
        }
    }
    for clipped in &output.shapes {
        walk(&clipped.shape, footer_top, &mut painted);
    }
    assert!(
        painted > 0 && painted < names.len(),
        "the list is capped: {painted} of {} rows drawn",
        names.len()
    );
}

/// A session that has never been saved is a first run or a demo: the header must not call it untitled while the name
/// field and the panel's own label both show a name.
#[test]
fn a_named_session_with_no_file_is_unsaved_rather_than_untitled() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec![], "New song");
    assert!(panel.songs.file.is_none());

    // Drawing with a name and no file is the first-run shape; it must not panic and must leave the name alone.
    run_on(&ctx, &mut panel, &shared, Vec::new());
    assert_eq!(
        panel.songs.card.as_ref().unwrap().name,
        "New song",
        "drawing the header must not rewrite the field"
    );
}
