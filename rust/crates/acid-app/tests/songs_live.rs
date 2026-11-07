//! The songs card: it lists the songs on disk, reports SAVE AS and LOAD, and never touches the disk itself.

use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::songs::{Request, Songs};
use acid_app::ui::switches::{self, Switch};
use acid_app::ui::{PanelState, draw_panel, theme};
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

/// The name field and SAVE AS share the first row of the card.
fn save_as_centre() -> Pos2 {
    let width = card().width() - padding() * 2.0;
    let left = card().left() + padding() + width * 0.46 + padding();
    Pos2::new(
        left + (width * 0.26 - padding()) * 0.5,
        field_y() + row_height() * 0.5,
    )
}

/// Song rows start one row plus a gap below the field.
fn song_row_centre(index: usize) -> Pos2 {
    let y = field_y() + row_height() + padding();
    Pos2::new(
        card().left() + padding() + 20.0,
        y + (row_height() + 2.0 * scale(screen())) * index as f32 + row_height() * 0.5,
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
        songs: Some(Songs {
            names: names.into_iter().map(str::to_string).collect(),
            name: name.to_string(),
            ..Songs::default()
        }),
        ..PanelState::default()
    }
}

#[test]
fn a_song_row_asks_to_load_that_song() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1", "Another"], "");

    click(&ctx, &mut panel, &shared, song_row_centre(1));

    assert_eq!(
        panel.song_request,
        Some(Request::Load("Another".to_string()))
    );
}

#[test]
fn save_as_asks_for_the_typed_name() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec![], "Acid 2");

    click(&ctx, &mut panel, &shared, save_as_centre());

    assert_eq!(
        panel.song_request,
        Some(Request::Save("Acid 2".to_string()))
    );
}

#[test]
fn a_name_that_cannot_be_a_file_asks_for_nothing() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec![], "a/b");

    click(&ctx, &mut panel, &shared, save_as_centre());

    assert_eq!(panel.song_request, None, "a path is not a song name");
}

#[test]
fn close_takes_the_card_away() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1"], "");

    click(&ctx, &mut panel, &shared, close_centre());

    assert!(panel.songs.is_none());
    assert_eq!(panel.song_request, None);
}

#[test]
fn the_songs_key_opens_and_closes_the_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    // Six cells now: BANK, PATTERN, MODE, CLEAR, SAVE, SONGS.
    let songs = switches::cell(layout(screen()).switches, Switch::Songs).center();

    click(&ctx, &mut panel, &shared, songs);
    assert!(panel.songs.is_some(), "the SONGS key opens the card");

    click(&ctx, &mut panel, &shared, songs);
    assert!(panel.songs.is_none(), "and closes it again");
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

/// RENAME sits after SAVE AS on the name row.
fn rename_centre() -> Pos2 {
    let width = card().width() - padding() * 2.0;
    let save_left = card().left() + padding() + width * 0.46 + padding();
    let rename_left = save_left + width * 0.26 - padding() + padding();
    Pos2::new(
        rename_left + (width * 0.28 - padding()) * 0.5,
        field_y() + row_height() * 0.5,
    )
}

fn open_song_card(names: Vec<&str>, name: &str, open: Option<&str>) -> PanelState {
    let mut panel = open_card(names, name);
    if let Some(card) = panel.songs.as_mut() {
        card.open = open.map(str::to_string);
    }
    panel
}

#[test]
fn rename_asks_for_the_typed_name_of_the_open_song() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_song_card(vec!["Acid 1"], "Acid 2", Some("Acid 1"));

    click(&ctx, &mut panel, &shared, rename_centre());

    assert_eq!(
        panel.song_request,
        Some(Request::Rename("Acid 2".to_string()))
    );
}

#[test]
fn rename_is_dark_with_no_open_song_or_an_unchanged_name() {
    let ctx = Context::default();
    let shared = SharedState::new();

    // Nothing open: renaming has no subject.
    let mut nothing_open = open_song_card(vec!["Acid 1"], "Acid 2", None);
    click(&ctx, &mut nothing_open, &shared, rename_centre());
    assert_eq!(nothing_open.song_request, None);

    // The typed name is the open song's own name: nothing to do, so nothing is asked for.
    let mut unchanged = open_song_card(vec!["Acid 1"], "Acid 1", Some("Acid 1"));
    click(&ctx, &mut unchanged, &shared, rename_centre());
    assert_eq!(unchanged.song_request, None);
}

#[test]
fn the_new_song_button_asks_for_a_fresh_song() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1"], "");

    click(&ctx, &mut panel, &shared, new_song_centre());

    assert_eq!(panel.song_request, Some(Request::New));
}

#[test]
fn deleting_a_song_takes_two_clicks() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1", "Another"], "");

    click(&ctx, &mut panel, &shared, delete_centre(1));
    assert_eq!(panel.song_request, None, "one click only arms it");
    assert_eq!(
        panel.songs.as_ref().unwrap().delete_armed.as_deref(),
        Some("Another")
    );

    click(&ctx, &mut panel, &shared, delete_centre(1));
    assert_eq!(
        panel.song_request,
        Some(Request::Delete("Another".to_string()))
    );
}

#[test]
fn loading_a_song_clears_a_pending_delete() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(vec!["Acid 1", "Another"], "");

    click(&ctx, &mut panel, &shared, delete_centre(0));
    assert!(panel.songs.as_ref().unwrap().delete_armed.is_some());

    click(&ctx, &mut panel, &shared, song_row_centre(1));
    assert_eq!(panel.songs.as_ref().unwrap().delete_armed, None);
    assert_eq!(
        panel.song_request,
        Some(Request::Load("Another".to_string()))
    );
}
