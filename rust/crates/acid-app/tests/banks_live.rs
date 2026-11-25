//! The `BANKS` card's four sections: the demos, the player's bank files, their songs, and the rack files.
//!
//! The card is a painter: every test here drives `draw_panel` through real egui frames and asserts on the request the card
//! left behind, because what a click writes is `main`'s to do and not the card's.
//!
//! The rack section's tests live here rather than in a file of their own: the geometry that finds a section's rows is
//! measured by the helpers above, and a second copy of it would be the hand-kept list this project has paid for twice.

use acid_app::device::{DeviceKind, Rack};
use acid_app::persist::{BankDocument, SavedState, list_banks, load_bank, save_bank};
use acid_app::state::SharedState;
use acid_app::ui::banks::{Banks, Request};
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::{PanelState, SongState, draw_panel, theme};
use acid_core::bank::PatternBank;
use acid_core::pattern::Grid;
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

/// The room a list may take: everything above the footer keys less one row, as `banks::paint` measures it.
fn list_limit() -> f32 {
    card().bottom() - padding() - row_height() - gap()
}

/// How many bank rows the card draws before its save row. The painter stops the rows one row short of the limit so the save
/// row always has a place, and this mirrors that rule.
fn bank_rows_drawn(count: usize) -> usize {
    let rows_limit = list_limit() - row_height() - gap();
    let top = banks_top() + heading();
    (0..count)
        .take_while(|index| {
            top + (row_height() + gap()) * *index as f32 + row_height() <= rows_limit
        })
        .count()
}

/// `SAVE BANK` sits under the bank rows, in the pattern `PATTERN BANKS` heads.
fn bank_save_centre(rows: usize) -> Pos2 {
    let save_left = left() + width() * 0.40 + padding();
    Pos2::new(
        save_left + (width() * 0.30 - padding()) * 0.5,
        row_centre(banks_top() + heading(), rows),
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

fn rack_row_centre(index: usize) -> Pos2 {
    Pos2::new(left() + 20.0, row_centre(racks_top() + heading(), index))
}

/// `SAVE RACK` sits under the rack rows, in the pattern `PATTERN BANKS` heads: a name field, then one key.
fn rack_save_centre(rows: usize) -> Pos2 {
    let save_left = left() + width() * 0.40 + padding();
    Pos2::new(
        save_left + (width() * 0.30 - padding()) * 0.5,
        row_centre(racks_top() + heading(), rows),
    )
}

/// The cross at a rack row's right end, where deleting takes two clicks.
fn rack_delete_centre(index: usize) -> Pos2 {
    Pos2::new(
        left() + width() * 0.66 + padding() + width() * 0.14 * 0.5,
        row_centre(racks_top() + heading(), index),
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
                rack_names: vec!["Studio".to_string()],
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
fn the_rack_section_lists_the_files_and_asks_for_a_load() {
    // A rack file is a document now, so its section is the bank's shape: rows that load, and a field and a key that save.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);

    click(&ctx, &mut panel, &shared, rack_row_centre(0));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadRack("Studio".to_string())),
        "a row loads that rack"
    );

    let card = panel.songs.card.as_ref().expect("the card stays open");
    assert_eq!(card.load_armed, None, "a saved session loads on one click");
    assert_eq!(card.rack_names, vec!["Studio".to_string()]);
}

#[test]
fn a_dirty_session_arms_a_rack_load_first() {
    // A rack load replaces every machine in the session, so an unsaved session costs two clicks, as a song's load does.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(true);

    click(&ctx, &mut panel, &shared, rack_row_centre(0));
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

    click(&ctx, &mut panel, &shared, rack_row_centre(0));
    assert_eq!(
        panel.songs.request,
        Some(Request::LoadRack("Studio".to_string()))
    );
}

#[test]
fn saving_a_rack_asks_with_the_typed_name_or_the_file_it_came_from() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);
    panel.songs.card.as_mut().unwrap().rack_name = "Studio 2".to_string();

    click(&ctx, &mut panel, &shared, rack_save_centre(1));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveRack("Studio 2".to_string())),
        "the typed name"
    );

    // A blank field keeps the rack the session came from, SAVE's own rule for the other document.
    panel.songs.request = None;
    panel.songs.card.as_mut().unwrap().rack_name.clear();
    panel.songs.rack_file = Some("Studio".to_string());
    click(&ctx, &mut panel, &shared, rack_save_centre(1));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveRack("Studio".to_string())),
        "a blank field names the rack the session is on"
    );
}

#[test]
fn a_rack_rows_cross_asks_to_delete_twice() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);

    click(&ctx, &mut panel, &shared, rack_delete_centre(0));
    assert_eq!(panel.songs.request, None, "one click only arms the cross");
    assert_eq!(
        panel.songs.card.as_ref().unwrap().delete_armed.as_deref(),
        Some("Studio"),
        "and the cross turns into SURE"
    );

    click(&ctx, &mut panel, &shared, rack_delete_centre(0));
    assert_eq!(
        panel.songs.request,
        Some(Request::DeleteRack("Studio".to_string())),
        "deleting a file has no undo, so it takes two clicks"
    );
}

#[test]
fn saving_a_bank_asks_with_the_typed_name() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);
    panel.songs.card.as_mut().unwrap().bank_name = "Easy 2".to_string();

    click(&ctx, &mut panel, &shared, bank_save_centre(1));
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

    click(&ctx, &mut panel, &shared, bank_save_centre(1));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveBank("Easy".to_string())),
        "a blank field rewrites the bank the device is on rather than making a new one"
    );
}

#[test]
fn a_full_bank_list_still_leaves_save_bank_on_the_card() {
    // The save row is the section's door, and the card has no scrolling: a list long enough to reach the limit must therefore
    // stop a row short of it, or a player who has filled the list cannot save another bank at all.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);
    let names: Vec<String> = (0..30).map(|index| format!("Bank {index:02}")).collect();
    let drawn = bank_rows_drawn(names.len());
    assert!(drawn < names.len(), "the list is long enough to be cut");
    {
        let card = panel.songs.card.as_mut().unwrap();
        card.bank_names = names;
        card.bank_name = "Thirty One".to_string();
    }

    click(&ctx, &mut panel, &shared, bank_save_centre(drawn));
    assert_eq!(
        panel.songs.request,
        Some(Request::SaveBank("Thirty One".to_string())),
        "the save row is still on the card, one row under the last bank it drew"
    );
}

/// The delete cross sits to the right of bank row `index`, where the song rows' crosses sit.
fn bank_delete_centre(index: usize) -> Pos2 {
    let row_right = left() + width() * 0.66;
    Pos2::new(
        row_right + padding() + width() * 0.14 * 0.5,
        row_centre(banks_top() + heading(), index),
    )
}

#[test]
fn deleting_a_bank_takes_two_clicks() {
    // Deleting a file has no undo, so the cross arms first and says so, exactly as a song's does.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_card(false);

    click(&ctx, &mut panel, &shared, bank_delete_centre(0));
    assert_eq!(panel.songs.request, None, "the first click deletes nothing");
    assert_eq!(
        panel.songs.card.as_ref().unwrap().delete_armed.as_deref(),
        Some("bank:Easy"),
        "the cross says SURE and waits"
    );

    click(&ctx, &mut panel, &shared, bank_delete_centre(0));
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
    let mut panel = open_card(false);
    panel.songs.card.as_mut().unwrap().bank_names = list_banks(dir.path());

    assert_eq!(
        panel.songs.card.as_ref().unwrap().bank_names,
        vec!["Easy".to_string()],
        "the file is in the library the card reads"
    );
    click(&ctx, &mut panel, &shared, bank_row_centre(0));
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
