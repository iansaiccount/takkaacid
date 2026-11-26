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
    /// The arm key of the cross clicked once — `song:{name}`, `bank:{name}` or `rack:{name}`, one namespace per kind so two
    /// documents of one name cannot share a guard. Deleting a file has no undo, so it takes two clicks.
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

    /// The row's id. An `egui::Id` is behaviour — a click in progress is routed by it — so these are the ids the card's rows
    /// have always had, `rack-load-`'s verb included.
    fn row_id(self, index: usize) -> String {
        match self {
            Kind::Song => format!("song-{index}"),
            Kind::Bank => format!("bank-{index}"),
            Kind::Rack => format!("rack-load-{index}"),
            Kind::Demo => format!("demo-{index}"),
        }
    }

    /// What a click on the row's name asks for.
    fn load_request(self, index: usize, name: &str) -> Request {
        match self {
            Kind::Song => Request::Load(name.to_string()),
            Kind::Bank => Request::LoadBank(name.to_string()),
            Kind::Rack => Request::LoadRack(name.to_string()),
            Kind::Demo => Request::LoadDemo(index),
        }
    }

    /// The cross at the row's right end: its id, and what a second click asks for. A demo is factory content, so it has no
    /// file to delete and this is `None` — the same answer [`Kind::is_file`] gives, said by the function that has to say
    /// something rather than leaving a fourth delete that does not exist.
    fn cross(self, index: usize, name: &str) -> Option<(String, Request)> {
        let delete = format!("{}-delete-{index}", self.stem());
        let request = match self {
            Kind::Song => Request::Delete(name.to_string()),
            Kind::Bank => Request::DeleteBank(name.to_string()),
            Kind::Rack => Request::DeleteRack(name.to_string()),
            Kind::Demo => return None,
        };
        Some((delete, request))
    }

    /// What a row of this kind says while a click would replace work, and the line the card says in its message slot. A demo
    /// asks the device's kind, because a 303 replaces the session and a drum machine only that device's grid.
    fn guard(self, device: crate::device::DeviceKind) -> (&'static str, &'static str) {
        match self {
            Kind::Song => (REPLACE_SESSION, REPLACE_SESSION),
            Kind::Bank => (BANK_ROW_HINT, REPLACE_BANK),
            Kind::Rack => (RACK_ROW_HINT, REPLACE_RACK),
            Kind::Demo => (armed_hint(device), guard_message(device)),
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
        // Each chip names the kinds it shows rather than slicing `Kind::ALL` by position: a chip that said "the second kind"
        // would move silently the day the catalogue was reordered.
        const SONG: [Kind; 1] = [Kind::Song];
        const BANK: [Kind; 1] = [Kind::Bank];
        const RACK: [Kind; 1] = [Kind::Rack];
        const DEMO: [Kind; 1] = [Kind::Demo];
        match self {
            Filter::All => &Kind::ALL,
            Filter::Songs => &SONG,
            Filter::Banks => &BANK,
            Filter::Racks => &RACK,
            Filter::Demos => &DEMO,
        }
    }

    /// Which document's strip a chip shows. One place answers it, and `card_rects` and `paint` both read it: a second match
    /// over `Filter` would be a second answer to one question.
    pub fn door(self) -> Door {
        match self {
            Filter::All | Filter::Songs | Filter::Demos => Door::Song,
            Filter::Banks => Door::Bank,
            Filter::Racks => Door::Rack,
        }
    }
}
/// The card's three save doors: the document the strip beside the list saves, and the key that saves it.
///
/// `ALL`, `SONGS` and `DEMOS` show the song's, because the session is what is always open and saving it must never need a chip
/// change; `BANKS` and `RACKS` show their own, because that is the thing the player is looking at. The cost, accepted: a player
/// on `BANKS` who wants to save the *song* clicks `ALL` or `SONGS` first — one click, with the chips on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Door {
    Song,
    Bank,
    Rack,
}

impl Door {
    /// The field's id. Kept per document rather than one `banks-name`, because the fields are one control each and an id is
    /// behaviour: a focus request and a drag are routed by it.
    pub fn field_id(self) -> &'static str {
        match self {
            Door::Song => "song-name",
            Door::Bank => "bank-name",
            Door::Rack => "rack-name",
        }
    }

    /// What the empty field says it wants.
    pub fn field_hint(self) -> &'static str {
        match self {
            Door::Song => "song name",
            Door::Bank => "bank name",
            Door::Rack => "rack name",
        }
    }

    /// The key's words. Every key names its own verb, so no key's meaning depends on which chip is lit.
    pub fn save_label(self) -> &'static str {
        match self {
            Door::Song => "SAVE",
            Door::Bank => "SAVE BANK",
            Door::Rack => "SAVE RACK",
        }
    }

    /// The key's id, and `None` where a door has no key: only the song has `SAVE A COPY`, whose id is `song-save-copy`. The
    /// answer is the id rather than a flag so that "is there a copy key" and "what is it called" cannot disagree.
    pub fn copy_id(self) -> Option<&'static str> {
        match self {
            Door::Song => Some("song-save-copy"),
            Door::Bank | Door::Rack => None,
        }
    }

    /// The save key's id. Kept per document for the same reason the field's is: an id is behaviour.
    pub fn save_id(self) -> &'static str {
        match self {
            Door::Song => "song-save",
            Door::Bank => "bank-save",
            Door::Rack => "rack-save",
        }
    }

    /// What the key asks for, once the name has been decided.
    pub fn request(self, name: String) -> Request {
        match self {
            Door::Song => Request::Save(name),
            Door::Bank => Request::SaveBank(name),
            Door::Rack => Request::SaveRack(name),
        }
    }

    /// The same question for the song's second key, or `None` where there is none.
    pub fn copy_request(self, name: String) -> Option<Request> {
        match self {
            Door::Song => Some(Request::SaveCopy(name)),
            Door::Bank | Door::Rack => None,
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
    // A demo has no file name of its own, so it keys on its slot; every other kind keys on the name a player reads. That is
    // the same question `is_file` answers, asked where the key is made.
    if kind.is_file() {
        format!("{}:{name}", kind.stem())
    } else {
        format!("{}:{index}", kind.stem())
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
    let rects = card_rects(panel, scale, songs.filter);
    let padding = CARD_PADDING * scale;
    let mut request = None;

    {
        let painter = ui.painter();
        painter.rect_filled(rects.card, 6.0 * scale, theme::BLACK.panel_raised);
        painter.rect_stroke(
            rects.card,
            6.0 * scale,
            theme::BLACK.silkscreen_stroke(scale),
            egui::StrokeKind::Inside,
        );
        painter.text(
            egui::Pos2::new(rects.card.left() + padding, rects.card.top() + padding),
            egui::Align2::LEFT_TOP,
            "BANKS",
            theme::legend(13.0 * scale),
            theme::BLACK.silkscreen,
        );
        // What the card is about, on the card: the song the session is on and whether it is on disk. A player who opened the
        // card to save should not have to work out which song they are saving. The name is the session's, not the file's, so a
        // first run and a freshly loaded demo — named, but not yet files — say so instead of calling themselves untitled.
        painter.text(
            egui::Pos2::new(
                rects.card.right() - padding,
                rects.card.top() + padding + 2.0 * scale,
            ),
            egui::Align2::RIGHT_TOP,
            now_line(session.name, session_dirty),
            theme::legend(10.0 * scale),
            theme::BLACK.silkscreen_dim,
        );
    }

    // The search narrows the list to the rows whose name contains what is typed, as it is typed. The hint is the plain word:
    // `…` belongs to `ui::text::fit` and `settings::shorten` and to nothing else.
    let search_id = ui.scope_id().with("banks-search");
    let search_before = songs.search.clone();
    let filter_before = songs.filter;
    ui.put(
        rects.search,
        egui::TextEdit::singleline(&mut songs.search)
            .id(search_id)
            .hint_text("search")
            .desired_width(rects.search.width()),
    );

    // The chips: one row of keys, the lit one the view the list is drawn through. A chip is a view of the same rows, so
    // choosing one is not a second list — and the strip under the list follows it.
    for (index, chip) in Filter::ALL.iter().enumerate() {
        if transport::card_button(
            ui,
            &format!("banks-filter-{index}"),
            rects.chips[index],
            chip.caption(),
            songs.filter == *chip,
        )
        .clicked()
        {
            songs.filter = *chip;
        }
    }
    // A different list starts at the top: a clamped old position would leave the player's best match scrolled off it. A
    // delete only re-clamps, because the list re-flowing under the hand that clicked is not a reason to jump the view.
    if songs.search != search_before || songs.filter != filter_before {
        songs.top = 0;
    }

    // The chip clicked this frame is the chip the strip is drawn for. `rects` above was placed with the chip the frame began
    // with, so the furniture that follows the chip is placed again now — otherwise a click from `BANKS` to `ALL` would draw the
    // song's label in the bank's key and lose `SAVE A COPY` for that frame. Nothing else in `Rects` depends on the chip.
    let rects = card_rects(panel, scale, songs.filter);

    // The band: the rows this chip and this search leave, through the window the band can show. Nothing is drawn or
    // hit-tested outside it, so a click on the strip cannot land on a row.
    let list = rows(songs);
    // The window's first row, clamped to where the list actually ends, so the wheel scrolls from there rather than from a
    // request the band could not show.
    let top = window(rects.band, scale, list.len(), songs.top).1;
    songs.top = top;

    // The wheel scrolls the list and stops at either end. Read only while the pointer is over the band, so a wheel over the
    // strip or the footer — the card's fixed furniture — leaves the rows where they are. Only the floor is clamped here: the
    // ceiling belongs to `window`, which is the one place that decides what a window is.
    let over = ui
        .input(|input| input.pointer.hover_pos())
        .is_some_and(|at| rects.band.contains(at));
    if over {
        let scrolled = ui.input(|input| input.smooth_scroll_delta.y);
        if scrolled != 0.0 {
            let step = (scrolled / ((CARD_ROW + CARD_GAP) * scale)).round() as i64;
            songs.top = (top as i64 - step).max(0) as usize;
        }
    }
    let (visible, top) = window(rects.band, scale, list.len(), songs.top);
    songs.top = top;

    // The mark that says there is more of the list, and only when there is: two rounded rects in the column `CARD_TRACK`
    // reserves — a shape, never a glyph, and nothing for the pointer to drag this round.
    if let Some(mark) = thumb(rects.band, scale, list.len(), visible, top) {
        let painter = ui.painter();
        painter.rect_filled(mark.track, 2.0 * scale, theme::BLACK.well);
        painter.rect_filled(mark.thumb, 2.0 * scale, theme::BLACK.silkscreen_dim);
    }
    if list.is_empty() {
        // A demonstration of absence is cheaper than a player wondering whether the card is broken.
        ui.painter().text(
            rects.band.center(),
            egui::Align2::CENTER_CENTER,
            empty_line(songs.filter, !songs.search.is_empty()),
            theme::legend(10.0 * scale),
            theme::BLACK.silkscreen_dim,
        );
    }
    for (n, row) in list.iter().skip(top).take(visible).enumerate() {
        let key = arm_key(row.kind, row.index, &row.name);
        let armed = songs.load_armed.as_deref() == Some(key.as_str());
        let (hint, message) = row.kind.guard(kind);
        if load_row(
            ui,
            &row.kind.row_id(row.index),
            rects.row(n),
            &row.name,
            armed,
            session_dirty,
            hint,
        ) && request.is_none()
        {
            request = guarded_load(
                songs,
                key.clone(),
                row.kind.load_request(row.index, &row.name),
                session_dirty,
                message,
            );
            songs.delete_armed = None;
        }
        // The kind, in its own column: the row's word and its arm key's stem are one catalogue's. A demo has no cross — it is
        // factory content — so the three kinds that are files get one at the row's right end.
        ui.painter().text(
            rects.tag(n),
            egui::Align2::LEFT_CENTER,
            row.kind.tag(),
            theme::legend(10.0 * scale),
            theme::BLACK.silkscreen_dim,
        );
        if let Some((id, delete)) = row.kind.cross(row.index, &row.name) {
            let delete_armed = songs.delete_armed.as_deref() == Some(key.as_str());
            let label = if delete_armed { "SURE" } else { "x" };
            if transport::card_button(ui, &id, rects.cross(n), label, delete_armed).clicked() {
                if delete_armed {
                    request = Some(delete);
                    songs.delete_armed = None;
                } else {
                    songs.delete_armed = Some(key.clone());
                    request = None;
                }
            }
        }
    }

    // The strip follows the chip: the song's field and its two keys on `ALL`, `SONGS` and `DEMOS`, the bank's on `BANKS`, the
    // rack's on `RACKS`. A blank field still saves the document the session came from — SAVE's own rule, said for all three.
    //
    // The door answers which field, which key, and what it asks for; the three fields are this card's, so they are named here
    // once.
    let door = songs.filter.door();
    let (field_text, current): (&mut String, Option<&str>) = match door {
        Door::Song => (&mut songs.name, songs.open.as_deref()),
        Door::Bank => (&mut songs.bank_name, session.bank_file),
        Door::Rack => (&mut songs.rack_name, session.rack_file),
    };
    let field_id = ui.scope_id().with(door.field_id());
    ui.put(
        rects.strip_field,
        egui::TextEdit::singleline(&mut *field_text)
            .id(field_id)
            .hint_text(door.field_hint())
            .desired_width(rects.strip_field.width()),
    );
    let typed = crate::persist::document_name(field_text.as_str());
    // A blank field still saves the document the session is on. With nothing named and nothing open there is nothing to save:
    // the key puts the cursor in the field rather than dying quietly, which is what a player would expect of a save key.
    let can_save = typed.is_some() || current.is_some();
    if transport::card_button(
        ui,
        door.save_id(),
        rects.strip_save,
        door.save_label(),
        can_save,
    )
    .clicked()
    {
        match document_save_name(field_text.as_str(), current) {
            Some(name) => request = Some(door.request(name)),
            None => ui.memory_mut(|memory| memory.request_focus(field_id)),
        }
    }
    if let (Some(copy), Some(copy_id)) = (rects.strip_copy, door.copy_id())
        && transport::card_button(ui, copy_id, copy, "SAVE A COPY", typed.is_some()).clicked()
        && let Some(name) = document_save_name(field_text.as_str(), current)
        && let Some(copy) = door.copy_request(name)
    {
        request = Some(copy);
    }

    // The footer: a new song, the two doors to the file system, and close. NEW replaces the session, so it asks twice while
    // the session is unsaved, like every other action that would throw work away.
    let new_armed = songs.load_armed.as_deref() == Some(NEW_KEY);
    let new_label = if new_armed { "SURE?" } else { "NEW" };
    if transport::card_button(ui, "song-new", rects.footer_new, new_label, new_armed).clicked() {
        request = guarded_load(
            songs,
            NEW_KEY.to_string(),
            Request::New,
            session_dirty,
            REPLACE_SESSION,
        );
    }
    if transport::card_button(ui, "song-import", rects.footer_import, "IMPORT", false).clicked() {
        request = Some(Request::Import);
    }
    // `EXPORT` writes the **session**, so it is the song's name that lights it, not the strip's field: on `BANKS` a typed bank
    // name would otherwise light a key that has nothing to do with it, and a named song would leave it dim.
    let can_export = songs.open.is_some() || crate::persist::document_name(&songs.name).is_some();
    if transport::card_button(ui, "song-export", rects.footer_export, "EXPORT", can_export)
        .clicked()
    {
        request = Some(Request::Export);
    }
    if transport::card_button(ui, "song-close", rects.footer_close, "CLOSE", false).clicked() {
        request = Some(Request::Close);
    }

    if let Some(message) = &songs.message {
        ui.painter().text(
            egui::Pos2::new(
                rects.footer_export.right() + padding,
                rects.footer_new.center().y,
            ),
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
/// The width the thumb's track occupies at the band's right edge, reserved whether or not the thumb is drawn, so a row's cross
/// sits at the same x at every list size.
pub const CARD_TRACK: f32 = 4.0;

/// The rows the band can show, and the window's first row after clamping the request into it — the one place that decides what
/// a window is. The painter clamps the floor, this clamps the ceiling.
///
/// `scale` multiplies the card's own measurements, so the band and a row are counted in the same units. A band is
/// `rows * CARD_ROW + (rows - 1) * CARD_GAP` tall, which is why the sum adds one gap before the divide.
pub fn window(band: Rect, scale: f32, count: usize, top: usize) -> (usize, usize) {
    let step = (CARD_ROW + CARD_GAP) * scale;
    let visible = ((band.height() + CARD_GAP * scale) / step).floor().max(0.0) as usize;
    let visible = visible.min(count);
    (visible, top.min(count.saturating_sub(visible)))
}

/// The track and its thumb, or `None` when the list fits and has nothing to say about it.
pub struct Thumb {
    pub track: Rect,
    pub thumb: Rect,
}

/// The mark that says there is more of the list: a track at the band's right edge and a handle the window's share of it, as
/// far down as the window has scrolled.
///
/// No glyph and no click of its own — the wheel is the input — and nothing at all when everything the list has fits, so a
/// library that fits looks exactly as the card did before there was a thumb.
pub fn thumb(band: Rect, scale: f32, count: usize, visible: usize, top: usize) -> Option<Thumb> {
    if count <= visible {
        return None;
    }
    let width = CARD_TRACK * scale;
    let track = Rect::from_min_max(
        egui::Pos2::new(band.right() - width, band.top()),
        band.right_bottom(),
    );
    let height = track.height() * (visible as f32 / count as f32);
    let travel = track.height() - height;
    // The last page is the bottom of the track: `window` has already clamped `top` into `0..=count - visible`.
    let last = count.saturating_sub(visible) as f32;
    let at = if last == 0.0 { 0.0 } else { top as f32 / last };
    let thumb = Rect::from_min_size(
        egui::Pos2::new(track.left(), track.top() + travel * at),
        egui::vec2(width, height),
    );
    Some(Thumb { track, thumb })
}

/// The column a row's name starts in, in design pixels: wide enough for the longest tag (`DEMO`, `RACK`, `BANK`, `SONG`) plus
/// a gap, so the names line up whichever kind they are.
const CARD_TAG: f32 = 44.0;

/// Where everything on the card is, in design pixels times `scale`.
///
/// One function fills it; the painter draws from it and the tests click from it, so the card's arithmetic is written once. A
/// row's position is [`Rects::row`], never a `.+ 24.0` somewhere else: the two live test files used to mirror the painter and
/// the two copies did not even agree on what `songs_top` meant.
pub struct Rects {
    pub card: Rect,
    pub search: Rect,
    /// One rect per chip, in `Filter::ALL` order — a length derived from the catalogue, so a sixth chip cannot leave a chip
    /// without a place on the card.
    pub chips: [Rect; Filter::ALL.len()],
    /// The list's band: the rows live here and nothing outside it is drawn or hit-tested.
    pub band: Rect,
    pub strip_field: Rect,
    pub strip_save: Rect,
    /// `SAVE A COPY`: the song's strip has two keys, a bank's and a rack's have one.
    pub strip_copy: Option<Rect>,
    pub footer_new: Rect,
    pub footer_import: Rect,
    pub footer_export: Rect,
    pub footer_close: Rect,
    scale: f32,
}

impl Rects {
    /// Row `n` of the window: the name's key, which starts after the kind's column so the tag neither moves nor is covered.
    pub fn row(&self, n: usize) -> Rect {
        Rect::from_min_size(
            egui::Pos2::new(
                self.band.left() + CARD_TAG * self.scale,
                self.band.top() + n as f32 * self.step(),
            ),
            egui::vec2(self.band.width() * ROW_SHARE, CARD_ROW * self.scale),
        )
    }

    pub fn cross(&self, n: usize) -> Rect {
        let row = self.row(n);
        Rect::from_min_size(
            egui::Pos2::new(row.right() + CARD_PADDING * self.scale, row.top()),
            egui::vec2(self.band.width() * CROSS_SHARE, row.height()),
        )
    }

    /// Where the kind's tag word and the row's name begin: the tag in the band's first column, the name at the key the tag
    /// labels.
    ///
    /// `name` is that key's left edge rather than a second way to draw the name: the painter hands the key to `load_row`, which
    /// centres the caption in it, so this is the tests' answer to "where does the name start".
    pub fn tag(&self, n: usize) -> egui::Pos2 {
        let row = self.row(n);
        egui::Pos2::new(self.band.left(), row.center().y)
    }

    pub fn name(&self, n: usize) -> egui::Pos2 {
        let row = self.row(n);
        egui::Pos2::new(row.left(), row.center().y)
    }

    /// The y-distance from one row to the next: a row and the gap under it.
    fn step(&self) -> f32 {
        (CARD_ROW + CARD_GAP) * self.scale
    }
}

/// The row's share of the band's width, leaving the cross its own column and the thumb's track the right edge.
const ROW_SHARE: f32 = 0.66;
const CROSS_SHARE: f32 = 0.14;

/// Place every rect of the card inside `panel`. Pure arithmetic: no painting, no egui context.
///
/// The vertical budget, from the constants at `scale` 1.0 in a 638 px card: title 18, search 22 + 12, chips 22 + 12, the band,
/// strip 22 + 12, footer 22 + 12 — which leaves the band 460 px, so nineteen rows. The band is what is left after the fixed
/// furniture has been placed from both ends, so a taller window gives the band the extra and a door never moves for a list.
pub fn card_rects(panel: Rect, scale: f32, filter: Filter) -> Rects {
    let card = panel.shrink(theme::MARGIN * scale);
    let padding = CARD_PADDING * scale;
    let row_height = CARD_ROW * scale;
    let left = card.left() + padding;
    let width = card.width() - padding * 2.0;

    // The title, then everything the list needs above it.
    let mut y = card.top() + padding + 18.0 * scale;
    let search = Rect::from_min_size(egui::Pos2::new(left, y), egui::vec2(width, row_height));
    y += row_height + padding;

    let chips = Filter::ALL.len();
    let chip_gap = CARD_GAP * scale;
    let chip_width = (width - chip_gap * (chips - 1) as f32) / chips as f32;
    let chip_rects = std::array::from_fn(|index| {
        Rect::from_min_size(
            egui::Pos2::new(left + index as f32 * (chip_width + chip_gap), y),
            egui::vec2(chip_width, row_height),
        )
    });
    y += row_height + padding;

    // The footer from the bottom, the strip one padding above it, and the band is what is left between the chips and both.
    let bottom = card.bottom() - padding - row_height;
    let strip_top = bottom - padding - row_height;
    let band = Rect::from_min_max(
        egui::Pos2::new(left, y),
        egui::Pos2::new(left + width, strip_top - padding),
    );

    let door = filter.door();
    let save_share = match door {
        Door::Song => 0.20,
        Door::Bank | Door::Rack => 0.30,
    };
    let strip_field = Rect::from_min_size(
        egui::Pos2::new(left, strip_top),
        egui::vec2(width * 0.40, row_height),
    );
    let strip_save = Rect::from_min_size(
        egui::Pos2::new(strip_field.right() + padding, strip_top),
        egui::vec2(width * save_share - padding, row_height),
    );
    let strip_copy = Rect::from_min_size(
        egui::Pos2::new(strip_save.right() + padding, strip_top),
        egui::vec2(width * 0.22 - padding, row_height),
    );

    let footer_new = Rect::from_min_size(
        egui::Pos2::new(left, bottom),
        egui::vec2(80.0 * scale, row_height),
    );
    let footer_import = Rect::from_min_size(
        egui::Pos2::new(footer_new.right() + padding, bottom),
        egui::vec2(96.0 * scale, row_height),
    );
    let footer_export = Rect::from_min_size(
        egui::Pos2::new(footer_import.right() + padding, bottom),
        egui::vec2(96.0 * scale, row_height),
    );
    let footer_close = Rect::from_min_size(
        egui::Pos2::new(card.right() - padding - 90.0 * scale, bottom),
        egui::vec2(90.0 * scale, row_height),
    );

    Rects {
        card,
        search,
        chips: chip_rects,
        band,
        strip_field,
        strip_save,
        strip_copy: door.copy_id().map(|_| strip_copy),
        footer_new,
        footer_import,
        footer_export,
        footer_close,
        scale,
    }
}

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

    /// A band `rows` rows tall, built from the same two constants the painter lays a row out with, so a test's window and the
    /// card's window are one arithmetic. One pixel of slack is not needed: `window` divides by the same step it built here.
    fn band_of(rows: usize) -> Rect {
        Rect::from_min_size(
            egui::Pos2::new(100.0, 200.0),
            egui::vec2(
                400.0,
                rows as f32 * CARD_ROW + rows.saturating_sub(1) as f32 * CARD_GAP,
            ),
        )
    }

    #[test]
    fn a_list_that_fits_scrolls_nowhere() {
        // Nothing to scroll means nothing to say: the band is as tall as the list, at the top, and there is no mark on it.
        let ten = band_of(10);
        assert_eq!(window(ten, 1.0, 10, 0), (10, 0));
        assert_eq!(
            window(ten, 1.0, 3, 0),
            (3, 0),
            "a band taller than the list shows the list"
        );
        assert_eq!(
            window(ten, 1.0, 3, 5),
            (3, 0),
            "a clamp of a list that fits is still the top"
        );
        assert!(thumb(ten, 1.0, 3, 3, 0).is_none());
    }

    #[test]
    fn a_list_that_overflows_stops_at_its_last_page() {
        // A window never starts past its last page, or the band would be drawing rows that are not there.
        let four = band_of(4);
        assert_eq!(window(four, 1.0, 10, 0), (4, 0));
        assert_eq!(window(four, 1.0, 10, 3), (4, 3));
        assert_eq!(
            window(four, 1.0, 10, 99),
            (4, 6),
            "ten rows, four shown: the last page is 6"
        );
    }

    #[test]
    fn a_shorter_band_re_clamps_the_window() {
        // A window is the band's as much as the list's, so the same request in a narrower band is a different window.
        assert_eq!(window(band_of(6), 1.0, 10, 6), (6, 4));
        assert_eq!(
            window(band_of(10), 1.0, 10, 6),
            (10, 0),
            "and a band that grew again takes the window back to the top"
        );
    }

    #[test]
    fn the_thumb_is_proportional_and_inside_the_track() {
        // A shape, not a glyph: the track's width is reserved at every list size, and the handle is the window's share of it.
        let band = band_of(4);
        let top = thumb(band, 1.0, 10, 4, 0).expect("ten rows over a four-row band is more of it");
        assert_eq!(top.track.width(), CARD_TRACK);
        assert_eq!(
            top.track.right(),
            band.right(),
            "the track sits at the band's right edge"
        );
        assert!(band.contains_rect(top.track), "and inside the band");
        assert_eq!(top.thumb.width(), CARD_TRACK);
        // A rect's height is `max.y - min.y`, so placing the handle re-rounds it at its own magnitude: the share is compared
        // to a hundredth of a pixel rather than to the bit, which is all the painter can see anyway.
        assert!(
            (top.thumb.height() - top.track.height() * (4.0 / 10.0)).abs() < 0.01,
            "the handle is the window's share of the track: {} of {}",
            top.thumb.height(),
            top.track.height()
        );
        assert_eq!(
            top.thumb.top(),
            top.track.top(),
            "at the top while the window is at the top"
        );

        let end = thumb(band, 1.0, 10, 4, 6).expect("the same list, scrolled to its last page");
        assert!(
            (end.thumb.bottom() - end.track.bottom()).abs() < 0.01,
            "on the last page the handle reaches the bottom of the track: {} vs {}",
            end.thumb.bottom(),
            end.track.bottom()
        );
        assert!(
            end.thumb.top() > top.thumb.top(),
            "and it moved down the track"
        );

        let half = thumb(band, 1.0, 10, 4, 3).expect("a page in the middle");
        assert!(half.thumb.top() > top.thumb.top() && half.thumb.top() < end.thumb.top());
    }

    /// The panel the card is designed in, times `scale`: 980 × 670 is the window `layout` divides up, and the card is that
    /// panel with a margin off it.
    fn panel_at(scale: f32) -> Rect {
        Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(980.0 * scale, 670.0 * scale))
    }

    /// Every rect the card owns, so a test that says "all of them" cannot forget one a later round added.
    fn all_rects(rects: &Rects) -> Vec<Rect> {
        let mut all = vec![
            rects.search,
            rects.band,
            rects.strip_field,
            rects.strip_save,
            rects.footer_new,
            rects.footer_import,
            rects.footer_export,
            rects.footer_close,
        ];
        all.extend(rects.chips);
        all.extend(rects.strip_copy);
        all
    }

    #[test]
    fn the_card_puts_every_control_inside_itself_at_both_sizes() {
        // The design's own risk is the vertical budget, and the smallest window is where it bites: every control is inside the
        // card at the design size and at the 0.65 the scale clamps to.
        for scale in [1.0, 0.65] {
            let rects = card_rects(panel_at(scale), scale, Filter::All);
            for rect in all_rects(&rects) {
                assert!(
                    rects.card.contains_rect(rect),
                    "at scale {scale} a control leaves the card: {rect:?} outside {:?}",
                    rects.card
                );
                assert!(rect.width() > 0.0 && rect.height() > 0.0, "{rect:?}");
            }
            for pair in rects.chips.windows(2) {
                assert!(
                    pair[0].right() <= pair[1].left(),
                    "at scale {scale} two chips overlap: {:?} {:?}",
                    pair[0],
                    pair[1]
                );
            }
            assert!(rects.chips[0].left() >= rects.card.left());
            assert!(rects.chips[Filter::ALL.len() - 1].right() <= rects.card.right());
        }
    }

    #[test]
    fn the_band_sits_between_the_chips_and_the_strip() {
        // Top to bottom: the title, the search, the chips, the band, the strip, the footer — and nothing above the strip or
        // below the footer moves, because a door is fixed furniture.
        let scale = 1.0;
        let rects = card_rects(panel_at(scale), scale, Filter::All);
        let title = rects.card.top() + CARD_PADDING * scale + 18.0 * scale;
        assert!(
            rects.search.top() >= title,
            "the search sits under the title: {} vs {title}",
            rects.search.top()
        );
        for chip in rects.chips {
            assert!(
                chip.top() >= rects.search.bottom(),
                "the chips are one row under the search"
            );
        }
        assert!(
            rects.band.top() >= rects.chips[0].bottom(),
            "the band is under the chips"
        );
        assert!(
            rects.band.bottom() <= rects.strip_field.top(),
            "and above the strip"
        );
        assert!(
            rects.strip_field.bottom() <= rects.footer_new.top(),
            "the strip is above the footer"
        );
        assert!(
            rects.footer_new.bottom() <= rects.card.bottom(),
            "and the footer is inside the card"
        );
        assert_eq!(
            rects.strip_save.bottom(),
            rects.strip_field.bottom(),
            "the field and its keys share one row"
        );
    }

    #[test]
    fn the_band_holds_a_library_of_sixteen_at_the_design_size() {
        // The arithmetic from the constants, pinned: the design's estimate was eighteen, this one asks for sixteen and
        // leaves the rest to the picture.
        let scale = 1.0;
        let rects = card_rects(panel_at(scale), scale, Filter::All);
        let (visible, top) = window(rects.band, scale, 60, 0);
        assert!(visible >= 16, "the band holds only {visible} rows");
        assert_eq!(top, 0);
        let last = rects.row(visible - 1);
        assert!(
            rects.band.contains_rect(last),
            "the last row of the window is inside the band: {last:?} in {:?}",
            rects.band
        );
    }

    #[test]
    fn the_strip_offers_a_copy_key_only_to_the_song() {
        // Written out over the catalogue, so a sixth chip has to say whether its strip can copy.
        for filter in Filter::ALL {
            let song_chip = match filter {
                Filter::All | Filter::Songs | Filter::Demos => true,
                Filter::Banks | Filter::Racks => false,
            };
            let rects = card_rects(panel_at(1.0), 1.0, filter);
            assert_eq!(rects.strip_copy.is_some(), song_chip, "{filter:?}");
            match rects.strip_copy {
                Some(copy) => {
                    assert_eq!(copy.top(), rects.strip_save.top(), "{filter:?}");
                    assert!(
                        copy.left() >= rects.strip_save.right(),
                        "{filter:?}: the copy key follows the save key"
                    );
                }
                None => assert!(
                    rects.strip_save.right() <= rects.card.right() - CARD_PADDING,
                    "{filter:?}: the one key stays inside the card"
                ),
            }
        }
    }

    #[test]
    fn a_row_and_its_cross_are_inside_the_band_and_clear_of_the_track() {
        let scale = 1.0;
        let rects = card_rects(panel_at(scale), scale, Filter::All);
        let track = CARD_TRACK * scale;
        let (visible, _) = window(rects.band, scale, 60, 0);
        let mut above: Option<Rect> = None;
        for n in 0..visible {
            let row = rects.row(n);
            assert!(
                rects.band.contains_rect(row),
                "row {n} leaves the band: {row:?} in {:?}",
                rects.band
            );
            let cross = rects.cross(n);
            assert!(rects.band.contains_rect(cross), "row {n}'s cross");
            assert!(
                cross.left() > row.center().x,
                "row {n}'s cross sits right of the name it deletes"
            );
            assert!(
                cross.right() <= rects.band.right() - track,
                "row {n}'s cross runs under the reserved track: {cross:?}"
            );
            assert_eq!(rects.tag(n).y, row.center().y, "the tag is on the row");
            assert!(
                rects.name(n).x > rects.tag(n).x,
                "the name starts after the tag"
            );
            if let Some(above) = above {
                assert!(above.bottom() <= row.top(), "rows {n} overlap");
            }
            above = Some(row);
        }
    }

    #[test]
    fn every_id_a_click_is_routed_by_is_the_one_it_always_was() {
        // An `egui::Id` is behaviour — a click and a drag in progress are routed by it — and the live tests find these keys by
        // their geometry, so a rename here would leave them green and break the app. The catalogue derives them now, which is
        // why they are written out: a deliberate change has to come past this test.
        assert_eq!(Kind::Song.row_id(3), "song-3");
        assert_eq!(Kind::Bank.row_id(3), "bank-3");
        assert_eq!(
            Kind::Rack.row_id(3),
            "rack-load-3",
            "the verb the rack row has always had"
        );
        assert_eq!(Kind::Demo.row_id(3), "demo-3");

        let cross_id = |kind: Kind, name: &str| kind.cross(3, name).map(|(id, _)| id);
        assert_eq!(
            cross_id(Kind::Song, "Acid 1").as_deref(),
            Some("song-delete-3")
        );
        assert_eq!(
            cross_id(Kind::Bank, "Easy").as_deref(),
            Some("bank-delete-3")
        );
        assert_eq!(
            cross_id(Kind::Rack, "Studio").as_deref(),
            Some("rack-delete-3")
        );
        assert_eq!(
            cross_id(Kind::Demo, "Fast Acid"),
            None,
            "a demo has no cross"
        );

        assert_eq!(Door::Song.field_id(), "song-name");
        assert_eq!(Door::Song.save_id(), "song-save");
        assert_eq!(Door::Song.copy_id(), Some("song-save-copy"));
        assert_eq!(Door::Bank.field_id(), "bank-name");
        assert_eq!(Door::Bank.save_id(), "bank-save");
        assert_eq!(Door::Bank.copy_id(), None);
        assert_eq!(Door::Rack.field_id(), "rack-name");
        assert_eq!(Door::Rack.save_id(), "rack-save");
        assert_eq!(Door::Rack.copy_id(), None);
    }
}
