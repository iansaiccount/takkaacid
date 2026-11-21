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
    /// The card's own hint: what the armed row or button is waiting for. Messages from the app go to the status line.
    pub message: Option<String>,
    /// The song whose delete cross has been clicked once. Deleting a file has no undo, so it takes two clicks.
    pub delete_armed: Option<String>,
    /// The demo banks built into the app, in their own order. They are not files, so they are never deleted or renamed —
    /// and the five names are the same on either face, because they are the same demos: a 303 loads the song, a drum
    /// machine the groove it is played over.
    pub demos: Vec<String>,
    /// The row whose load has been clicked once while the session was unsaved: loading again replaces it. Replacing work
    /// that is not on disk takes two clicks, like deleting a file.
    pub load_armed: Option<String>,
    /// The song file the session is on, if any. SAVE compares against this to decide in place or new file.
    pub open: Option<String>,
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
    /// Copy a song file the player picks into the library, and load it.
    Import,
    /// Write the session to a path the player picks.
    Export,
    /// Adopt the demo at this index, as the device on the panel can use it: a 303 takes the whole song, under a new name
    /// and unsaved; a drum machine takes its drum bank, into the grid, and keeps its kit and the session's name. `main`
    /// asks the device's kind, because the card is a painter and the rack is not its to reach.
    LoadDemo(usize),
    Close,
}

/// Paint the card inside `panel` and say what was clicked, if anything. The first request of the frame wins.
pub fn paint(
    ui: &mut Ui,
    panel: Rect,
    scale: f32,
    songs: &mut Banks,
    session: Option<&str>,
    session_dirty: bool,
    kind: crate::device::DeviceKind,
) -> Option<Request> {
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
    let now = now_line(session, session_dirty);
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
    let typed = crate::persist::song_name(&songs.name);
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

    // The demos first: they are what a player reaches for when they want a sound, and they are not files, so there is no
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

    // Then the player's own songs: the name loads it, the cross beside it deletes it (twice, because there is no undo).
    if (y + CARD_HEADING * scale) < list_limit {
        section(ui, scale, "MY BANKS", left, &mut y);
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
    crate::persist::song_name(&songs.name).or_else(|| songs.open.clone())
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
}
