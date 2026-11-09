//! The songs card: start a new song, keep the session under a name, load one you have kept, delete one you do not
//! want.
//!
//! It is drawn over the panel like the help card, because the 303 face has no room for a text field. The card
//! never touches the disk itself — it reports what was asked for and `main` owns the files.

use egui::{Rect, Ui};

use super::{theme, transport};

/// What the card is showing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Songs {
    /// The songs already on disk, sorted.
    pub names: Vec<String>,
    /// The name being typed.
    pub name: String,
    /// One line of feedback: "loaded X", "saved", or why the last attempt failed.
    pub message: Option<String>,
    /// The song whose delete cross has been clicked once. Deleting a file has no undo, so it takes two clicks.
    pub delete_armed: Option<String>,
    /// The demo songs built into the app, in their own order. They are not files, so they are never deleted or renamed.
    pub demos: Vec<String>,
    /// The row whose load has been clicked once while the session was unsaved: loading again replaces it. Replacing work
    /// that is not on disk takes two clicks, like deleting a file.
    pub load_armed: Option<String>,
    /// The song the session is on, if any: RENAME acts on this one, and is dark when there is none.
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
    /// Adopt the demo at this index as a new song: it is not a file, so it arrives unsaved, under a new name.
    LoadDemo(usize),
    Close,
}

/// Paint the card inside `panel` and say what was clicked, if anything. The first request of the frame wins.
pub fn paint(
    ui: &mut Ui,
    panel: Rect,
    scale: f32,
    songs: &mut Songs,
    session_dirty: bool,
) -> Option<Request> {
    let card = panel.shrink(theme::MARGIN * scale);
    let painter = ui.painter();
    painter.rect_filled(card, 6.0 * scale, theme::PANEL_RAISED);
    painter.rect_stroke(
        card,
        6.0 * scale,
        theme::silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );

    let padding = 12.0 * scale;
    let row_height = 22.0 * scale;
    let left = card.left() + padding;
    let width = card.width() - padding * 2.0;
    let mut y = card.top() + padding;
    let mut request = None;

    painter.text(
        egui::Pos2::new(left, y),
        egui::Align2::LEFT_TOP,
        "SONGS",
        theme::legend(13.0 * scale),
        theme::SILKSCREEN,
    );
    // What the card is about, on the card: the song the session is on and whether it is on disk. A player who opened the
    // card to save should not have to work out which song they are saving.
    let now = match (&songs.open, session_dirty) {
        (Some(name), true) => format!("NOW: {name} · UNSAVED"),
        (Some(name), false) => format!("NOW: {name} · SAVED"),
        (None, _) => "NOW: untitled · UNSAVED".to_string(),
    };
    painter.text(
        egui::Pos2::new(card.right() - padding, y + 2.0 * scale),
        egui::Align2::RIGHT_TOP,
        now,
        theme::legend(10.0 * scale),
        theme::SILKSCREEN_DIM,
    );
    // The field holds the song's name, so SAVE keeps the song current, or writes it under a name that is not taken; SAVE A
    // COPY always makes a new file. There is no third button: renaming is saving under another name.
    y += 18.0 * scale;

    let field = Rect::from_min_size(
        egui::Pos2::new(left, y),
        egui::vec2(width * 0.40, row_height),
    );
    ui.put(
        field,
        egui::TextEdit::singleline(&mut songs.name)
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
    // A blank field still saves the song the session is on; with nothing open there is nothing to save.
    let can_save = typed.is_some() || songs.open.is_some();
    if transport::transport_button(ui, "song-save", save_rect, "SAVE", can_save).clicked() {
        request = save_name(songs).map(Request::Save);
    }
    if transport::transport_button(
        ui,
        "song-save-copy",
        copy_rect,
        "SAVE A COPY",
        typed.is_some(),
    )
    .clicked()
        && let Some(name) = typed.clone()
    {
        request = Some(Request::SaveCopy(name));
    }
    y += row_height + padding;

    // The demos first: they are what a player reaches for when they want a sound, and they are not files, so there is no
    // cross and no rename. Loading one replaces the session, which takes two clicks while it is unsaved.
    section(ui, scale, "DEMO SONGS", left, &mut y);
    for index in 0..songs.demos.len() {
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
        ) && request.is_none()
        {
            request = guarded_load(songs, key, Request::LoadDemo(index), session_dirty);
        }
        y += row_height + 2.0 * scale;
    }

    // Then the player's own songs: the name loads it, the cross beside it deletes it (twice, because there is no undo).
    section(ui, scale, "MY SONGS", left, &mut y);
    for index in 0..songs.names.len() {
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
        ) && request.is_none()
        {
            request = guarded_load(songs, key, Request::Load(name.clone()), session_dirty);
            songs.delete_armed = None;
        }
        let delete_armed = songs.delete_armed.as_deref() == Some(name.as_str());
        let cross = Rect::from_min_size(
            egui::Pos2::new(row.right() + padding, y),
            egui::vec2(width * 0.14, row_height),
        );
        let label = if delete_armed { "SURE" } else { "x" };
        if transport::transport_button(
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
        y += row_height + 2.0 * scale;
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
    if transport::transport_button(ui, "song-new", new_rect, new_label, new_armed).clicked() {
        request = guarded_load(songs, NEW_KEY.to_string(), Request::New, session_dirty);
    }
    let import_rect = Rect::from_min_size(
        egui::Pos2::new(new_rect.right() + padding, bottom),
        egui::vec2(96.0 * scale, row_height),
    );
    if transport::transport_button(ui, "song-import", import_rect, "IMPORT", false).clicked() {
        request = Some(Request::Import);
    }
    let export_rect = Rect::from_min_size(
        egui::Pos2::new(import_rect.right() + padding, bottom),
        egui::vec2(96.0 * scale, row_height),
    );
    let can_export = songs.open.is_some() || typed.is_some();
    if transport::transport_button(ui, "song-export", export_rect, "EXPORT", can_export).clicked() {
        request = Some(Request::Export);
    }
    let close = Rect::from_min_size(
        egui::Pos2::new(card.right() - padding - 90.0 * scale, bottom),
        egui::vec2(90.0 * scale, row_height),
    );
    if transport::transport_button(ui, "song-close", close, "CLOSE", false).clicked() {
        request = Some(Request::Close);
    }

    if let Some(message) = &songs.message {
        ui.painter().text(
            egui::Pos2::new(export_rect.right() + padding, bottom + row_height * 0.5),
            egui::Align2::LEFT_CENTER,
            message,
            theme::legend(10.0 * scale),
            theme::SILKSCREEN_DIM,
        );
    }

    request
}

/// A heading over one of the card's two lists, and the space it takes.
fn section(ui: &mut Ui, scale: f32, title: &str, left: f32, y: &mut f32) {
    ui.painter().text(
        egui::Pos2::new(left, *y),
        egui::Align2::LEFT_TOP,
        title,
        theme::legend(10.0 * scale),
        theme::SILKSCREEN_DIM,
    );
    *y += 15.0 * scale;
}

/// One loadable row: its name, and a warning across it while a click would replace an unsaved session.
fn load_row(
    ui: &mut Ui,
    id: &str,
    row: Rect,
    name: &str,
    armed: bool,
    session_dirty: bool,
) -> bool {
    let label = if armed && session_dirty {
        format!("{name} - click again to replace")
    } else {
        name.to_string()
    };
    transport::transport_button(ui, id, row, &label, armed).clicked()
}

/// Arm or carry out a load. Replacing a session that is not on disk takes two clicks, the way deleting a file does.
fn guarded_load(
    songs: &mut Songs,
    key: String,
    request: Request,
    session_dirty: bool,
) -> Option<Request> {
    if !session_dirty || songs.load_armed.as_deref() == Some(key.as_str()) {
        songs.load_armed = None;
        return Some(request);
    }
    songs.load_armed = Some(key);
    songs.message = Some("unsaved session: click again to replace it".to_string());
    None
}

/// The two-click guard's key for NEW: it is not a song, so it needs a name of its own in the armed slot.
const NEW_KEY: &str = "new";

/// The name a SAVE should use: what the player typed, or the song the session is already on when the field is blank.
fn save_name(songs: &Songs) -> Option<String> {
    crate::persist::song_name(&songs.name).or_else(|| songs.open.clone())
}
