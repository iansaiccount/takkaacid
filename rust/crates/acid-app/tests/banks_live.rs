//! The `BANKS` card's four sections: the demos, the player's bank files, their songs, and the rack keys that refuse.
//!
//! The card is a painter: every test here drives `draw_panel` through real egui frames and asserts on the request the card
//! left behind, because what a click writes is `main`'s to do and not the card's.

use acid_app::state::SharedState;
use acid_app::ui::banks::{Banks, Request};
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::{PanelState, SongState, draw_panel, theme};
use eframe::egui::{Context, Event, PointerButton, Pos2, RawInput, Rect, vec2};

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
        modifiers: Default::default(),
    };
    // A frame with the pointer over the widget before the press: egui only reports a click on a widget it had already seen hovered,
    // which is how a control that appears under a resting pointer cannot be clicked by accident.
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![button(true)]);
    run_on(ctx, panel, shared, vec![button(false)]);
}

/// The card covers the panel with a small inset; these mirror `banks::paint`'s own arithmetic, in design pixels multiplied by
/// the same scale the painter uses.
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

fn gap() -> f32 {
    2.0 * scale(screen())
}

fn heading() -> f32 {
    15.0 * scale(screen())
}

fn left() -> f32 {
    card().left() + padding()
}

fn width() -> f32 {
    card().width() - padding() * 2.0
}

fn field_y() -> f32 {
    card().top() + padding() + 18.0 * scale(screen())
}

/// `DEMO BANKS` sits one row below the name row, and its rows start under its heading.
fn demos_top() -> f32 {
    field_y() + row_height() + padding()
}

/// The centre of row `index` of a section whose rows start at `top`.
fn row_centre(top: f32, index: usize) -> f32 {
    top + (row_height() + gap()) * index as f32 + row_height() * 0.5
}

/// The top of the heading of the section after `above` rows: headings and rows stack without a gap of their own.
fn section_top(above: f32, rows: usize) -> f32 {
    above + heading() + (row_height() + gap()) * rows as f32
}

fn demo_row_centre(index: usize) -> Pos2 {
    Pos2::new(left() + 20.0, row_centre(demos_top() + heading(), index))
}

fn banks_top() -> f32 {
    section_top(demos_top(), 1)
}

fn bank_row_centre(index: usize) -> Pos2 {
    Pos2::new(left() + 20.0, row_centre(banks_top() + heading(), index))
}

/// `SAVE BANK` sits under the bank rows, in the pattern `PATTERN BANKS` heads.
fn bank_save_centre() -> Pos2 {
    let save_left = left() + width() * 0.40 + padding();
    Pos2::new(
        save_left + (width() * 0.30 - padding()) * 0.5,
        row_centre(banks_top() + heading(), 1),
    )
}

/// `SONGS` starts after the bank rows and the bank's own save row.
fn songs_top() -> f32 {
    section_top(banks_top(), 2)
}

fn song_row_centre(index: usize) -> Pos2 {
    Pos2::new(left() + 20.0, row_centre(songs_top() + heading(), index))
}

fn racks_top() -> f32 {
    section_top(songs_top(), 1)
}

fn rack_save_centre() -> Pos2 {
    Pos2::new(
        left() + 96.0 * scale(screen()) * 0.5,
        row_centre(racks_top() + heading(), 0),
    )
}

fn rack_load_centre() -> Pos2 {
    Pos2::new(
        left() + 96.0 * scale(screen()) + padding() + 96.0 * scale(screen()) * 0.5,
        row_centre(racks_top() + heading(), 0),
    )
}

/// A panel with the songs card open on one demo, one bank file and one song: the smallest card that draws all four
/// sections, so a section that stops being drawn moves every click below it.
fn open_card(dirty: bool) -> PanelState {
    PanelState {
        songs: SongState {
            card: Some(Banks {
                demos: vec!["Slow Acid".to_string()],
                bank_names: vec!["Easy".to_string()],
                names: vec!["Acid 1".to_string()],
                ..Banks::default()
            }),
            ..SongState::default()
        },
        dirty,
        ..PanelState::default()
    }
}

#[test]
fn the_card_lists_the_four_sections_and_asks_for_a_bank_load() {
    // Each click lands only if every section above it was drawn: the rows are laid out from the top of the card down, so a
    // heading that goes missing takes the position of everything under it with it.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);

    click(&ctx, &mut panel, &shared, demo_row_centre(0));
    assert_eq!(panel.songs.request, Some(Request::LoadDemo(0)));

    panel.songs.request = None;
    click(&ctx, &mut panel, &shared, bank_row_centre(0));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadBank("Easy".to_string())),
        "a bank row loads that bank over the selected device"
    );

    panel.songs.request = None;
    click(&ctx, &mut panel, &shared, song_row_centre(0));
    assert_eq!(
        panel.songs.request,
        Some(Request::Load("Acid 1".to_string())),
        "the player's songs are still the third section, under a heading that now says so"
    );
}

#[test]
fn a_dirty_session_arms_a_bank_load_first() {
    // Loading a bank replaces all sixty-four patterns of the selected device's bank, so an unsaved session costs two clicks,
    // exactly as the demo rows do.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(true);

    click(&ctx, &mut panel, &shared, bank_row_centre(0));
    assert_eq!(panel.songs.request, None, "the first click loads nothing");
    assert_eq!(
        panel.songs.card.as_ref().unwrap().load_armed.as_deref(),
        Some("bank:Easy"),
        "and the row says what a second click would replace"
    );
    assert!(
        panel
            .songs
            .card
            .as_ref()
            .unwrap()
            .message
            .as_deref()
            .is_some_and(|line| line.contains("replace")),
        "the card says which click it is waiting for"
    );

    click(&ctx, &mut panel, &shared, bank_row_centre(0));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadBank("Easy".to_string()))
    );
}

#[test]
fn the_rack_keys_ask_and_write_nothing() {
    // A rack document is a later round. The keys are drawn where they will live and they ask `main`, which says so; the card
    // itself has no file to write and no dialog to open, so the request being the whole effect is the assertion.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);

    click(&ctx, &mut panel, &shared, rack_save_centre());
    assert_eq!(panel.songs.request, Some(Request::SaveRack));

    panel.songs.request = None;
    click(&ctx, &mut panel, &shared, rack_load_centre());
    assert_eq!(panel.songs.request, Some(Request::LoadRack));

    let card = panel.songs.card.as_ref().expect("the card stays open");
    assert_eq!(card.load_armed, None, "a refusal is not a two-click load");
    assert_eq!(card.delete_armed, None, "and not a two-click delete");
    assert_eq!(card.bank_names, vec!["Easy".to_string()], "nothing moved");
    assert_eq!(card.names, vec!["Acid 1".to_string()]);
}

#[test]
fn saving_a_bank_asks_with_the_typed_name() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);
    panel.songs.card.as_mut().unwrap().bank_name = "Easy 2".to_string();

    click(&ctx, &mut panel, &shared, bank_save_centre());
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
    let mut panel = open_card(false);
    panel.songs.bank_file = Some("Easy".to_string());

    click(&ctx, &mut panel, &shared, bank_save_centre());
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveBank("Easy".to_string())),
        "a blank field rewrites the bank the device is on rather than making a new one"
    );
}
