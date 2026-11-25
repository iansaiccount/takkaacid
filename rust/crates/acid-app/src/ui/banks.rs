//! The songs card: start a new song, keep the session under a name, load one you have kept, delete one you do not
//! want.
//!
//! It is drawn over the panel like the help card, because the 303 face has no room for a text field. The card
//! never touches the disk itself — it reports what was asked for and `main` owns the files.

use egui::{Rect, Ui};

use super::{theme, transport};

/// What the card is showing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Banks {
    /// The songs already on disk, sorted.
    pub names: Vec<String>,
    /// The name being typed.
    pub name: String,
    /// The player's bank files, already on disk and sorted — the `banks/` library, filled by `main` the way `names` is.
    pub bank_names: Vec<String>,
    /// The bank name being typed beside `SAVE BANK`.
    pub bank_name: String,
    /// The player's rack files, already on disk and sorted — the `racks/` library, filled by `main` the way `names` is.
    pub rack_names: Vec<String>,
    /// The rack name being typed beside `SAVE RACK`.
    pub rack_name: String,
    /// The card's own hint: what the armed row or button is waiting for. Messages from the app go to the status line.
    pub message: Option<String>,
    /// The song whose delete cross has been clicked once. Deleting a file has no undo, so it takes two clicks.
    pub delete_armed: Option<String>,
    /// The demo banks built into the app, in their own order. They are not files, so they are never deleted or renamed —
    /// and the seven names are the same on either face, because they are the same demos: a 303 loads the song, a drum
    /// machine the groove it is played over.
    pub demos: Vec<String>,
    /// The row whose load has been clicked once while the session was unsaved: loading again replaces it. Replacing work
    /// that is not on disk takes two clicks, like deleting a file.
    pub load_armed: Option<String>,
    /// The song file the session is on, if any. SAVE compares against this to decide in place or new file.
    pub open: Option<String>,
    /// The text the search field holds: every row whose name contains it, case-insensitively, is the list — and a chip with
    /// no match under it draws one dim line rather than nothing. It is forgotten when the card closes, because `Banks` is
    /// dropped with it.
    pub search: String,
    /// Which chip is lit: `ALL` when the card opens, so a first run reads the whole library in its catalogue's order.
    ///
    /// Forgotten when the card closes, with the search. Keeping the chip on `SongState` would be a second place the card
    /// remembers something, and the round that added the chips chose to forget both.
    pub filter: Filter,
    /// The first row of the list the band shows. The wheel writes it, the painter clamps its floor and `window` its ceiling;
    /// the card never draws row `n` of the list, only row `n - top` of the window.
    ///
    /// It goes back to 0 whenever the chip or the search changes, because that is a different list and a re-clamped old
    /// position would leave the player's best match scrolled off the top. A delete only re-clamps.
    pub top: usize,
}

/// What the card asked for. `main` does the work, so the card stays a pure painter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Start a new song from the starter phrase.
    New,
    /// Save: in place when the name has not changed, as a new file when it has.
    Save(String),
    /// Save a copy under this name, always as a new file: the song on disk is left where it is.
    SaveCopy(String),
    Load(String),
    Delete(String),
    /// Save the selected device's bank under this name: in place when it is the bank's own file, a new one otherwise.
    SaveBank(String),
    /// Load a bank file over the selected device's bank: all sixty-four patterns.
    LoadBank(String),
    /// Delete a bank file.
    DeleteBank(String),
    /// Copy a song file the player picks into the library, and load it.
    Import,
    /// Write the session to a path the player picks.
    Export,
    /// Adopt the demo at this index, as the device on the panel can use it: a 303 takes the whole song, under a new name
    /// and unsaved; a drum machine takes its drum bank, into the grid, and keeps its kit and the session's name. `main`
    /// asks the device's kind, because the card is a painter and the rack is not its to reach.
    LoadDemo(usize),
    /// Save the whole rack as its own document, under this name: in place when it is the rack's own file, a new one
    /// otherwise.
    SaveRack(String),
    /// Load a rack document, replacing every machine in the session.
    LoadRack(String),
    /// Delete a rack file.
    DeleteRack(String),
    Close,
}

/// What the card says about the session it is working on: the song's name, whether it is on disk, the bank file the selected
/// device's bank came from, and the rack file its machines came from.
///
/// One value because they are three readings of one thing, and because a painter that took them one at a time would have more
/// arguments than a card has rows.
pub struct Session<'a> {
    /// The song the session is on, once it has a name.
    pub name: Option<&'a str>,
    /// Whether anything has changed since the session was written.
    pub dirty: bool,
    /// The bank file the selected device's bank came from, if it came from one.
    pub bank_file: Option<&'a str>,
    /// The rack file the session's machines came from, if they came from one. A rack save compares against this the way a
    /// song save compares against its name; it lives for the session, because the file records no such fact.
    pub rack_file: Option<&'a str>,
}

/// What a row in the card's list is. The catalogue the chips, the tags and the saving match on, in the order `ALL` draws
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Song,
    Bank,
    Rack,
    Demo,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Song, Kind::Bank, Kind::Rack, Kind::Demo];

    /// The word on the row's tag: `SONG`, `BANK`, `RACK`, `DEMO`.
    pub fn tag(self) -> &'static str {
        match self {
            Kind::Song => "SONG",
            Kind::Bank => "BANK",
            Kind::Rack => "RACK",
            Kind::Demo => "DEMO",
        }
    }

    /// The arm key's stem: `song`, `bank`, `rack`, `demo`. Every kind has one, because every kind can be loaded, and a load
    /// that is armed must arm on its own row — two documents may share a name, so the key carries the kind.
    pub fn stem(self) -> &'static str {
        match self {
            Kind::Song => "song",
            Kind::Bank => "bank",
            Kind::Rack => "rack",
            Kind::Demo => "demo",
        }
    }

    /// Whether this kind is a file the player owns, and so has a delete cross. A demo is factory content: no cross, no arm.
    ///
    /// A second question, so a second function — the stem says which list a key belongs to, this says whether there is a key
    /// at all.
    pub fn is_file(self) -> bool {
        match self {
            Kind::Song | Kind::Bank | Kind::Rack => true,
            Kind::Demo => false,
        }
    }

    /// The list this kind's rows come from, in its own order — which `persist` sorted.
    fn names(self, songs: &Banks) -> &[String] {
        match self {
            Kind::Song => &songs.names,
            Kind::Bank => &songs.bank_names,
            Kind::Rack => &songs.rack_names,
            Kind::Demo => &songs.demos,
        }
    }
}

/// The chips. `All` is a view over every kind; the rest are one kind each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Songs,
    Banks,
    Racks,
    Demos,
}

impl Filter {
    pub const ALL: [Filter; 5] = [
        Filter::All,
        Filter::Songs,
        Filter::Banks,
        Filter::Racks,
        Filter::Demos,
    ];

    /// The chip's word: `ALL`, `SONGS`, `BANKS`, `RACKS`, `DEMOS`.
    pub fn caption(self) -> &'static str {
        match self {
            Filter::All => "ALL",
            Filter::Songs => "SONGS",
            Filter::Banks => "BANKS",
            Filter::Racks => "RACKS",
            Filter::Demos => "DEMOS",
        }
    }

    /// Which kinds this chip shows — one, or all four.
    pub fn kinds(self) -> &'static [Kind] {
        match self {
            Filter::All => &Kind::ALL,
            Filter::Songs => &Kind::ALL[..1],
            Filter::Banks => &Kind::ALL[1..2],
            Filter::Racks => &Kind::ALL[2..3],
            Filter::Demos => &Kind::ALL[3..],
        }
    }
}

/// One row of the card's list: what it is, where it came from, and what it says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub kind: Kind,
    /// The index into the kind's own list (`songs.names`, `songs.bank_names`, …), which is what a `Request` carries.
    pub index: usize,
    pub name: String,
}

/// The rows the card draws, in `Kind::ALL` order and each kind's own order (which is sorted by `persist`), narrowed by the
/// chip and then by the search.
///
/// A pure function of `Banks`, so a test can ask it what the card would draw without a frame — and so the painter's list is
/// not a second opinion about what the list is.
pub fn rows(songs: &Banks) -> Vec<Row> {
    let search = songs.search.to_lowercase();
    let mut rows = Vec::new();
    for kind in songs.filter.kinds() {
        for (index, name) in kind.names(songs).iter().enumerate() {
            if !name.to_lowercase().contains(&search) {
                continue;
            }
            rows.push(Row {
                kind: *kind,
                index,
                name: name.clone(),
            });
        }
    }
    rows
}

/// What a row's click arms on: `song:Acid 1`, `bank:Easy`, `rack:Studio`, `demo:3`.
///
/// A demo has no file name of its own, so it keys on its slot. The others key on their name **with the kind in front**, because
/// two documents may share one name — `songs/Studio.json` and `racks/Studio.json` — and a shared guard would let a click on
/// one delete the other.
pub fn arm_key(kind: Kind, index: usize, name: &str) -> String {
    match kind {
        Kind::Song | Kind::Bank | Kind::Rack => format!("{}:{name}", kind.stem()),
        Kind::Demo => format!("{}:{index}", kind.stem()),
    }
}

/// The one line an empty band draws: `NOTHING MATCHES` when a search is typed, and the chip's own sentence when it is not.
///
/// Written out over `Filter` rather than defaulted, so a sixth chip has to decide what its emptiness says. `NO DEMOS YET` and
/// `NOTHING HERE YET` are sentences the shipped app cannot reach — seven demos ship, so `DEMOS` always has rows and `ALL`
/// always has seven — but a `_` arm is not an option and a panic is not an answer.
pub fn empty_line(filter: Filter, searching: bool) -> &'static str {
    if searching {
        return "NOTHING MATCHES";
    }
    match filter {
        Filter::All => "NOTHING HERE YET",
        Filter::Songs => "NO SONGS YET",
        Filter::Banks => "NO BANKS YET",
        Filter::Racks => "NO RACKS YET",
        Filter::Demos => "NO DEMOS YET",
    }
}

/// Paint the card inside `panel` and say what was clicked, if anything. The first request of the frame wins.
pub fn paint(
    ui: &mut Ui,
    panel: Rect,
    scale: f32,
    songs: &mut Banks,
    session: Session<'_>,
    kind: crate::device::DeviceKind,
) -> Option<Request> {
    let session_dirty = session.dirty;
    let card = panel.shrink(theme::MARGIN * scale);
    let painter = ui.painter();
    painter.rect_filled(card, 6.0 * scale, theme::BLACK.panel_raised);
    painter.rect_stroke(
        card,
        6.0 * scale,
        theme::BLACK.silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );

    let padding = CARD_PADDING * scale;
    let row_height = CARD_ROW * scale;
    let left = card.left() + padding;
    let width = card.width() - padding * 2.0;
    let mut y = card.top() + padding;
    let mut request = None;

    painter.text(
        egui::Pos2::new(left, y),
        egui::Align2::LEFT_TOP,
        "BANKS",
        theme::legend(13.0 * scale),
        theme::BLACK.silkscreen,
    );
    // What the card is about, on the card: the song the session is on and whether it is on disk. A player who opened the
    // card to save should not have to work out which song they are saving. The name is the session's, not the file's, so a
    // first run and a freshly loaded demo — named, but not yet files — say so instead of calling themselves untitled.
    let now = now_line(session.name, session_dirty);
    painter.text(
        egui::Pos2::new(card.right() - padding, y + 2.0 * scale),
        egui::Align2::RIGHT_TOP,
        now,
        theme::legend(10.0 * scale),
        theme::BLACK.silkscreen_dim,
    );
    // The field holds the song's name, so SAVE keeps the song current, or writes it under a name that is not taken; SAVE A
    // COPY always makes a new file. There is no third button: renaming is saving under another name.
    y += 18.0 * scale;

    // The field needs a name of its own so a nameless SAVE can put the cursor in it.
    let field_id = ui.scope_id().with("song-name");
    let field = Rect::from_min_size(
        egui::Pos2::new(left, y),
        egui::vec2(width * 0.40, row_height),
    );
    ui.put(
        field,
        egui::TextEdit::singleline(&mut songs.name)
            .id(field_id)
            .hint_text("song name")
            .desired_width(field.width()),
    );
    let save_rect = Rect::from_min_size(
        egui::Pos2::new(field.right() + padding, y),
        egui::vec2(width * 0.20 - padding, row_height),
    );
    let copy_rect = Rect::from_min_size(
        egui::Pos2::new(save_rect.right() + padding, y),
        egui::vec2(width * 0.22 - padding, row_height),
    );
    let typed = crate::persist::document_name(&songs.name);
    // A blank field still saves the song the session is on. With nothing open there is nothing to save and nothing to copy:
    // SAVE puts the cursor in the field rather than dying quietly, which is what a player would expect of a save button.
    let can_save = typed.is_some() || songs.open.is_some();
    if transport::card_button(ui, "song-save", save_rect, "SAVE", can_save).clicked() {
        match save_name(songs) {
            Some(name) => request = Some(Request::Save(name)),
            None => ui.memory_mut(|memory| memory.request_focus(field_id)),
        }
    }
    if transport::card_button(
        ui,
        "song-save-copy",
        copy_rect,
        "SAVE A COPY",
        typed.is_some(),
    )
    .clicked()
        && let Some(name) = save_name(songs)
    {
        request = Some(Request::SaveCopy(name));
    }
    y += row_height + padding;
    // The room a list may take: everything above the footer keys, less one row so the last row is not painted over them.
    // Past it the rows stop, as the kit browser's do — the alternative is a list drawn over NEW/IMPORT/EXPORT/CLOSE and off
    // the bottom of the card. A player with more songs than fit sees the first of them; there is no scrolling yet.
    let list_limit = card.bottom() - padding - row_height - CARD_GAP * scale;

    // The player's own songs, right under the name field that saves them: the name loads it, the cross beside it deletes it
    // (twice, because there is no undo).
    if (y + CARD_HEADING * scale) < list_limit {
        section(ui, scale, "SONGS", left, &mut y);
    }
    for index in 0..songs.names.len() {
        if (y + row_height) > list_limit {
            break;
        }
        let name = songs.names[index].clone();
        let row = Rect::from_min_size(
            egui::Pos2::new(left, y),
            egui::vec2(width * 0.66, row_height),
        );
        let key = format!("song:{name}");
        let armed = songs.load_armed.as_deref() == Some(key.as_str());
        if load_row(
            ui,
            &format!("song-{index}"),
            row,
            &name,
            armed,
            session_dirty,
            REPLACE_SESSION,
        ) && request.is_none()
        {
            request = guarded_load(
                songs,
                key,
                Request::Load(name.clone()),
                session_dirty,
                REPLACE_SESSION,
            );
            songs.delete_armed = None;
        }
        let delete_armed = songs.delete_armed.as_deref() == Some(name.as_str());
        let cross = Rect::from_min_size(
            egui::Pos2::new(row.right() + padding, y),
            egui::vec2(width * 0.14, row_height),
        );
        let label = if delete_armed { "SURE" } else { "x" };
        if transport::card_button(
            ui,
            &format!("song-delete-{index}"),
            cross,
            label,
            delete_armed,
        )
        .clicked()
        {
            if delete_armed {
                request = Some(Request::Delete(name.clone()));
                songs.delete_armed = None;
            } else {
                songs.delete_armed = Some(name.clone());
                request = None;
            }
        }
        y += row_height + CARD_GAP * scale;
    }
    // Then the demos: they are what a player reaches for when they want a sound, and they are not files, so there is no
    // cross and no rename. Loading one replaces the session on a 303 — twice, while it is unsaved — and on a drum machine
    // it fills the grid with that demo's groove and leaves the session where it was; the device's kind decides which, in
    // `main`. The line a click leaves behind says which of the two it will do — `armed_hint` and `guard_message` ask the
    // device's kind, because "session" on a drum face was the wrong word.
    section(ui, scale, "DEMO BANKS", left, &mut y);
    for index in 0..songs.demos.len() {
        if (y + row_height) > list_limit {
            break;
        }
        let name = songs.demos[index].clone();
        let row = Rect::from_min_size(
            egui::Pos2::new(left, y),
            egui::vec2(width * 0.66, row_height),
        );
        let key = format!("demo:{index}");
        let armed = songs.load_armed.as_deref() == Some(key.as_str());
        if load_row(
            ui,
            &format!("demo-{index}"),
            row,
            &name,
            armed,
            session_dirty,
            armed_hint(kind),
        ) && request.is_none()
        {
            request = guarded_load(
                songs,
                key,
                Request::LoadDemo(index),
                session_dirty,
                guard_message(kind),
            );
        }
        y += row_height + CARD_GAP * scale;
    }

    // Then the player's own bank files: a bank row replaces the selected device's whole bank — all sixty-four patterns —
    // armed while the session is unsaved like every other load; the cross deletes the file; the field under the rows saves
    // that bank under a name, which is its own door rather than a fifth footer key. The rows stop a row short of the limit so
    // the save row always gets its place: a player with a full list must still be able to save a new bank, and there is no
    // scrolling to reach a door drawn under the rows.
    if (y + CARD_HEADING * scale) < list_limit {
        section(ui, scale, "PATTERN BANKS", left, &mut y);
    }
    let bank_rows_limit = list_limit - row_height - CARD_GAP * scale;
    for index in 0..songs.bank_names.len() {
        if (y + row_height) > bank_rows_limit {
            break;
        }
        let name = songs.bank_names[index].clone();
        let row = Rect::from_min_size(
            egui::Pos2::new(left, y),
            egui::vec2(width * 0.66, row_height),
        );
        let key = format!("bank:{name}");
        let armed = songs.load_armed.as_deref() == Some(key.as_str());
        if load_row(
            ui,
            &format!("bank-{index}"),
            row,
            &name,
            armed,
            session_dirty,
            BANK_ROW_HINT,
        ) && request.is_none()
        {
            request = guarded_load(
                songs,
                key,
                Request::LoadBank(name.clone()),
                session_dirty,
                REPLACE_BANK,
            );
            songs.delete_armed = None;
        }
        let delete_key = format!("bank:{name}");
        let delete_armed = songs.delete_armed.as_deref() == Some(delete_key.as_str());
        let cross = Rect::from_min_size(
            egui::Pos2::new(row.right() + padding, y),
            egui::vec2(width * 0.14, row_height),
        );
        let label = if delete_armed { "SURE" } else { "x" };
        if transport::card_button(
            ui,
            &format!("bank-delete-{index}"),
            cross,
            label,
            delete_armed,
        )
        .clicked()
        {
            if delete_armed {
                request = Some(Request::DeleteBank(name.clone()));
                songs.delete_armed = None;
            } else {
                songs.delete_armed = Some(delete_key);
                request = None;
            }
        }
        y += row_height + CARD_GAP * scale;
    }
    // The bank's own save row, under the rows it saves from: a name and one key. A footer key whose meaning depended on which
    // heading was above it is a key nobody can describe in one sentence, and a bank has no file dialog to open.
    if (y + row_height) <= list_limit {
        let bank_field_id = ui.scope_id().with("bank-name");
        let bank_field = Rect::from_min_size(
            egui::Pos2::new(left, y),
            egui::vec2(width * 0.40, row_height),
        );
        ui.put(
            bank_field,
            egui::TextEdit::singleline(&mut songs.bank_name)
                .id(bank_field_id)
                .hint_text("bank name")
                .desired_width(bank_field.width()),
        );
        let bank_save = Rect::from_min_size(
            egui::Pos2::new(bank_field.right() + padding, y),
            egui::vec2(width * 0.30 - padding, row_height),
        );
        let can_save_bank = crate::persist::document_name(&songs.bank_name).is_some()
            || session.bank_file.is_some();
        if transport::card_button(ui, "bank-save", bank_save, "SAVE BANK", can_save_bank).clicked()
        {
            match document_save_name(&songs.bank_name, session.bank_file) {
                Some(name) => request = Some(Request::SaveBank(name)),
                None => ui.memory_mut(|memory| memory.request_focus(bank_field_id)),
            }
        }
        y += row_height + CARD_GAP * scale;
    }

    // And the rack's own document, under the bank's rule: rows that load, a cross that deletes, and the name field its SAVE
    // RACK writes under. A rack is the machines, so a row replaces every machine in the session.
    if (y + CARD_HEADING * scale) < list_limit {
        section(ui, scale, "RACKS", left, &mut y);
    }
    // The rows stop one row short of the limit so the rack's own save row always has a place, as PATTERN BANKS' do.
    let rack_rows_limit = list_limit - row_height - CARD_GAP * scale;
    for index in 0..songs.rack_names.len() {
        if (y + row_height) > rack_rows_limit {
            break;
        }
        let name = songs.rack_names[index].clone();
        let row = Rect::from_min_size(
            egui::Pos2::new(left, y),
            egui::vec2(width * 0.66, row_height),
        );
        let key = format!("rack:{name}");
        let armed = songs.load_armed.as_deref() == Some(key.as_str());
        if load_row(
            ui,
            &format!("rack-load-{index}"),
            row,
            &name,
            armed,
            session_dirty,
            RACK_ROW_HINT,
        ) && request.is_none()
        {
            request = guarded_load(
                songs,
                key,
                Request::LoadRack(name.clone()),
                session_dirty,
                REPLACE_RACK,
            );
            songs.delete_armed = None;
        }
        // The arm key is namespaced, as the bank rows' is: a song and a rack may share a name, and the crosses must not
        // share a guard, or one click on the song deletes it because the rack beside it was armed.
        let delete_key = format!("rack:{name}");
        let delete_armed = songs.delete_armed.as_deref() == Some(delete_key.as_str());
        let cross = Rect::from_min_size(
            egui::Pos2::new(row.right() + padding, y),
            egui::vec2(width * 0.14, row_height),
        );
        let label = if delete_armed { "SURE" } else { "x" };
        if transport::card_button(
            ui,
            &format!("rack-delete-{index}"),
            cross,
            label,
            delete_armed,
        )
        .clicked()
        {
            if delete_armed {
                request = Some(Request::DeleteRack(name.clone()));
                songs.delete_armed = None;
            } else {
                songs.delete_armed = Some(delete_key);
                request = None;
            }
        }
        y += row_height + CARD_GAP * scale;
    }
    // The rack's own save row: a name and one key, which is the bank's door rather than a footer key whose meaning would
    // depend on the heading above it.
    if (y + row_height) <= list_limit {
        let rack_field_id = ui.scope_id().with("rack-name");
        let rack_field = Rect::from_min_size(
            egui::Pos2::new(left, y),
            egui::vec2(width * 0.40, row_height),
        );
        ui.put(
            rack_field,
            egui::TextEdit::singleline(&mut songs.rack_name)
                .id(rack_field_id)
                .hint_text("rack name")
                .desired_width(rack_field.width()),
        );
        let rack_save = Rect::from_min_size(
            egui::Pos2::new(rack_field.right() + padding, y),
            egui::vec2(width * 0.30 - padding, row_height),
        );
        let can_save_rack = crate::persist::document_name(&songs.rack_name).is_some()
            || session.rack_file.is_some();
        if transport::card_button(ui, "rack-save", rack_save, "SAVE RACK", can_save_rack).clicked()
        {
            match document_save_name(&songs.rack_name, session.rack_file) {
                Some(name) => request = Some(Request::SaveRack(name)),
                None => ui.memory_mut(|memory| memory.request_focus(rack_field_id)),
            }
        }
    }
    // The footer: a new song, the two doors to the file system, and close. NEW replaces the session, so it asks twice while
    // the session is unsaved, like every other action that would throw work away.
    let bottom = card.bottom() - padding - row_height;
    let new_rect = Rect::from_min_size(
        egui::Pos2::new(left, bottom),
        egui::vec2(80.0 * scale, row_height),
    );
    let new_armed = songs.load_armed.as_deref() == Some(NEW_KEY);
    let new_label = if new_armed { "SURE?" } else { "NEW" };
    if transport::card_button(ui, "song-new", new_rect, new_label, new_armed).clicked() {
        request = guarded_load(
            songs,
            NEW_KEY.to_string(),
            Request::New,
            session_dirty,
            REPLACE_SESSION,
        );
    }
    let import_rect = Rect::from_min_size(
        egui::Pos2::new(new_rect.right() + padding, bottom),
        egui::vec2(96.0 * scale, row_height),
    );
    if transport::card_button(ui, "song-import", import_rect, "IMPORT", false).clicked() {
        request = Some(Request::Import);
    }
    let export_rect = Rect::from_min_size(
        egui::Pos2::new(import_rect.right() + padding, bottom),
        egui::vec2(96.0 * scale, row_height),
    );
    let can_export = songs.open.is_some() || typed.is_some();
    if transport::card_button(ui, "song-export", export_rect, "EXPORT", can_export).clicked() {
        request = Some(Request::Export);
    }
    let close = Rect::from_min_size(
        egui::Pos2::new(card.right() - padding - 90.0 * scale, bottom),
        egui::vec2(90.0 * scale, row_height),
    );
    if transport::card_button(ui, "song-close", close, "CLOSE", false).clicked() {
        request = Some(Request::Close);
    }

    if let Some(message) = &songs.message {
        ui.painter().text(
            egui::Pos2::new(export_rect.right() + padding, bottom + row_height * 0.5),
            egui::Align2::LEFT_CENTER,
            message,
            theme::legend(10.0 * scale),
            theme::BLACK.silkscreen_dim,
        );
    }

    request
}

/// The measurements the two listing cards share — this one and the kit browser — in design pixels.
///
/// They live here because this card is where the idiom started, and the two cards are the same object to a player: same
/// panel, same padding, same row, same caption. A second set of numbers beside the first is how two "sibling" cards stop
/// looking like siblings, and there is no test that would say so.
pub(super) const CARD_PADDING: f32 = 12.0;
pub(super) const CARD_ROW: f32 = 22.0;
pub(super) const CARD_GAP: f32 = 2.0;
pub(super) const CARD_HEADING: f32 = 15.0;

/// A caption over one of a card's lists, at `at`. The one way a listing card names a section.
pub(super) fn heading(ui: &mut Ui, scale: f32, title: &str, at: egui::Pos2) {
    ui.painter().text(
        at,
        egui::Align2::LEFT_TOP,
        title,
        theme::legend(10.0 * scale),
        theme::BLACK.silkscreen_dim,
    );
}

/// A heading over one of the card's two lists, and the space it takes.
fn section(ui: &mut Ui, scale: f32, title: &str, left: f32, y: &mut f32) {
    heading(ui, scale, title, egui::Pos2::new(left, *y));
    *y += CARD_HEADING * scale;
}

/// What a demo row says once its first click has been spent, per machine: on a 303 a second click throws the session away
/// and loads a song, and on a drum machine it replaces that device's eight patterns and nothing else. A sentence a player
/// reads, so it says which — the word `session` on a drum face was the wrong one.
fn armed_hint(kind: crate::device::DeviceKind) -> &'static str {
    match kind {
        crate::device::DeviceKind::Acid => "click again to replace",
        crate::device::DeviceKind::Drums => "click again to replace the grid",
    }
}

/// The line under the card when a guarded load spent its first click, per machine — the same sentence as [`armed_hint`], said
/// in the card's message slot rather than across the row.
fn guard_message(kind: crate::device::DeviceKind) -> &'static str {
    match kind {
        crate::device::DeviceKind::Acid => "unsaved session: click again to replace it",
        crate::device::DeviceKind::Drums => "unsaved: click again to replace this machine's grid",
    }
}

/// What a song row says while its load would replace the session: a song is a session on either face, so this one does not
/// ask the device's kind.
const REPLACE_SESSION: &str = "unsaved session: click again to replace it";

/// The line under the card when a guarded bank load spent its first click: a bank replaces the selected device's whole bank on
/// either face, so this sentence does not ask the device's kind.
const REPLACE_BANK: &str = "unsaved: click again to replace this machine's bank";

/// What a bank row says while its load would replace work.
const BANK_ROW_HINT: &str = "click again to replace the bank";

/// What a rack row says while its load would replace the session's machines.
const RACK_ROW_HINT: &str = "click again to replace the rack";

/// The line under the card when a guarded rack load spent its first click: a rack replaces every machine, so it asks the
/// question a song does and not the one a bank does.
const REPLACE_RACK: &str = "unsaved session: click again to replace this rack";
/// One loadable row: its name, and what it is waiting for while a click would replace unsaved work.
fn load_row(
    ui: &mut Ui,
    id: &str,
    row: Rect,
    name: &str,
    armed: bool,
    session_dirty: bool,
    hint: &str,
) -> bool {
    let label = if armed && session_dirty {
        format!("{name} - {hint}")
    } else {
        name.to_string()
    };
    transport::card_button(ui, id, row, &label, armed).clicked()
}

/// Arm or carry out a load. Replacing a session that is not on disk takes two clicks, the way deleting a file does.
fn guarded_load(
    songs: &mut Banks,
    key: String,
    request: Request,
    session_dirty: bool,
    message: &'static str,
) -> Option<Request> {
    if !session_dirty || songs.load_armed.as_deref() == Some(key.as_str()) {
        songs.load_armed = None;
        return Some(request);
    }
    songs.load_armed = Some(key);
    songs.message = Some(message.to_string());
    None
}

/// The two-click guard's key for NEW: it is not a song, so it needs a name of its own in the armed slot.
const NEW_KEY: &str = "new";

/// The name a SAVE should use: what the player typed, or the song the session is already on when the field is blank.
fn save_name(songs: &Banks) -> Option<String> {
    crate::persist::document_name(&songs.name).or_else(|| songs.open.clone())
}

/// The name a `SAVE BANK` or a `SAVE RACK` should use: what the player typed, or the file the document came from when the
/// field is blank — SAVE's own rule, said for the other two documents.
///
/// One function for both, because it is one question. A second copy would be the hand-kept list this project has paid for
/// twice, and the two would drift the day one of them gained a rule.
fn document_save_name(typed: &str, current: Option<&str>) -> Option<String> {
    crate::persist::document_name(typed).or_else(|| current.map(str::to_string))
}

/// The card's header: the session's name and whether it is on disk. A first run and a freshly loaded demo have a name and
/// no file yet, and they are unsaved, not untitled.
pub fn now_line(session: Option<&str>, dirty: bool) -> String {
    match (session, dirty) {
        (Some(name), true) => format!("NOW: {name} - UNSAVED"),
        (Some(name), false) => format!("NOW: {name} - SAVED"),
        (None, _) => "NOW: untitled - UNSAVED".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DeviceKind;

    #[test]
    fn the_guard_names_what_a_second_click_would_replace() {
        // A 303's demo row loads a whole song, so the session goes. A drum machine's loads eight patterns into that device
        // and leaves the session, the kit and the name alone — the card said "session" on both faces, which was a lie on
        // one of them.
        assert_eq!(armed_hint(DeviceKind::Acid), "click again to replace");
        assert_eq!(
            armed_hint(DeviceKind::Drums),
            "click again to replace the grid"
        );
        assert_eq!(guard_message(DeviceKind::Acid), REPLACE_SESSION);
        assert_ne!(guard_message(DeviceKind::Drums), REPLACE_SESSION);
        assert!(
            guard_message(DeviceKind::Drums).contains("grid"),
            "the drum face's warning has to name what a click replaces"
        );

        // Written out, so a third kind has to decide here rather than inherit one of the two sentences.
        for kind in DeviceKind::ALL {
            assert!(!armed_hint(kind).is_empty());
            assert!(!guard_message(kind).is_empty());
        }
    }

    /// The card's library for the catalogue's tests: two songs, one bank, one rack and two demos — with one name on two
    /// kinds (`Studio` is a song and a rack) and one demo a lower-case query finds in a capitalised name.
    fn library() -> Banks {
        Banks {
            names: vec!["Acid 1".to_string(), "Studio".to_string()],
            bank_names: vec!["Easy".to_string()],
            rack_names: vec!["Studio".to_string()],
            demos: vec!["Fast Acid".to_string(), "Warehouse".to_string()],
            ..Banks::default()
        }
    }

    /// The rows' names, in the order the card would draw them.
    fn listed(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    #[test]
    fn the_all_chip_lists_every_kind_in_catalogue_order() {
        // One list, in a fixed kind order and each kind in its own order — which `persist` sorted before the card saw it.
        let songs = library();
        let drawn = rows(&songs);
        assert_eq!(
            listed(&drawn),
            [
                "Acid 1",
                "Studio",
                "Easy",
                "Studio",
                "Fast Acid",
                "Warehouse"
            ]
        );
        assert_eq!(
            drawn.iter().map(|row| row.kind).collect::<Vec<_>>(),
            vec![
                Kind::Song,
                Kind::Song,
                Kind::Bank,
                Kind::Rack,
                Kind::Demo,
                Kind::Demo
            ],
            "every kind, in the catalogue's order"
        );
        assert_eq!(
            drawn.iter().map(|row| row.index).collect::<Vec<_>>(),
            vec![0, 1, 0, 0, 0, 1],
            "a row's index is into its own kind's list, which is what a request carries"
        );
    }

    #[test]
    fn a_chip_shows_its_own_kind_and_each_kind_keeps_its_order() {
        let mut songs = library();
        // A chip's word is the catalogue's, and every chip is a view over the same rows.
        let chips = [
            (Filter::All, "ALL", 6usize),
            (Filter::Songs, "SONGS", 2),
            (Filter::Banks, "BANKS", 1),
            (Filter::Racks, "RACKS", 1),
            (Filter::Demos, "DEMOS", 2),
        ];
        assert_eq!(
            Filter::ALL.len(),
            chips.len(),
            "a chip without a test is a chip without a word"
        );
        for (filter, caption, count) in chips {
            songs.filter = filter;
            assert_eq!(filter.caption(), caption, "{filter:?}");
            assert_eq!(rows(&songs).len(), count, "{filter:?}");
        }
        songs.filter = Filter::Songs;
        assert_eq!(
            listed(&rows(&songs)),
            ["Acid 1", "Studio"],
            "a chip narrows; it does not re-sort"
        );
        songs.filter = Filter::Demos;
        assert_eq!(listed(&rows(&songs)), ["Fast Acid", "Warehouse"]);
    }

    #[test]
    fn the_search_narrows_by_substring_without_caring_about_case() {
        let mut songs = library();
        songs.search = "acid".to_string();
        assert_eq!(listed(&rows(&songs)), ["Acid 1", "Fast Acid"]);
        songs.search = "ACID".to_string();
        assert_eq!(
            listed(&rows(&songs)),
            ["Acid 1", "Fast Acid"],
            "the query's case does not matter"
        );
        songs.search = "dio".to_string();
        assert_eq!(
            listed(&rows(&songs)),
            ["Studio", "Studio"],
            "a substring finds a name wherever in it the text sits"
        );
        songs.search.clear();
        assert_eq!(
            rows(&songs).len(),
            6,
            "clearing the search restores the list"
        );
        songs.search = "zzz".to_string();
        assert!(
            rows(&songs).is_empty(),
            "a search that matches nothing draws nothing"
        );
    }

    #[test]
    fn the_search_and_the_chip_compose() {
        // The list is the chip over the search, in that order and with nothing else: one row set, two narrowings.
        let mut songs = library();
        songs.search = "acid".to_string();
        songs.filter = Filter::Songs;
        assert_eq!(listed(&rows(&songs)), ["Acid 1"]);
        songs.filter = Filter::Demos;
        assert_eq!(listed(&rows(&songs)), ["Fast Acid"]);
        songs.filter = Filter::Banks;
        assert!(
            rows(&songs).is_empty(),
            "the chip has no row the search keeps"
        );
    }

    #[test]
    fn only_the_kinds_a_player_owns_have_a_cross() {
        // A demo is factory content: it is loaded and never deleted or renamed, so its row has no cross and no delete arm.
        assert!(Kind::Song.is_file());
        assert!(Kind::Bank.is_file());
        assert!(Kind::Rack.is_file());
        assert!(!Kind::Demo.is_file());
    }

    #[test]
    fn a_row_arms_under_a_key_that_names_its_kind() {
        // Two documents can share a name — a song `Studio` and a rack `Studio` — so a key carries its kind, or the crosses
        // share a guard and one click on a song deletes it because a rack was armed.
        assert_eq!(arm_key(Kind::Song, 0, "Studio"), "song:Studio");
        assert_eq!(arm_key(Kind::Bank, 2, "Easy"), "bank:Easy");
        assert_eq!(arm_key(Kind::Rack, 1, "Studio"), "rack:Studio");
        assert_eq!(
            arm_key(Kind::Demo, 3, "Fast Acid"),
            "demo:3",
            "a demo has no file name of its own, so it keys on its slot"
        );
        // The row's word and its key's word are one catalogue's: every tag is its stem in capitals.
        for kind in Kind::ALL {
            assert_eq!(kind.tag(), kind.stem().to_uppercase(), "{kind:?}");
        }
    }

    #[test]
    fn the_empty_list_says_what_is_missing() {
        // A demonstration of absence is cheaper than a player wondering whether the card is broken — and it says which
        // nothing it is, because a search that finds nothing is not an empty library.
        assert_eq!(empty_line(Filter::All, false), "NOTHING HERE YET");
        assert_eq!(empty_line(Filter::Songs, false), "NO SONGS YET");
        assert_eq!(empty_line(Filter::Banks, false), "NO BANKS YET");
        assert_eq!(empty_line(Filter::Racks, false), "NO RACKS YET");
        assert_eq!(empty_line(Filter::Demos, false), "NO DEMOS YET");
        for filter in Filter::ALL {
            assert_eq!(
                empty_line(filter, true),
                "NOTHING MATCHES",
                "{filter:?}: a typed search is the reason the band is empty"
            );
        }
    }
}
