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
    /// The song the session is on, if any: RENAME acts on this one, and is dark when there is none.
    pub open: Option<String>,
}

/// What the card asked for. `main` does the work, so the card stays a pure painter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Start a new song from the starter phrase.
    New,
    Save(String),
    Load(String),
    Delete(String),
    /// Rename the open song to this name.
    Rename(String),
    Close,
}

/// Paint the card inside `panel` and say what was clicked, if anything. The first request of the frame wins.
pub fn paint(ui: &mut Ui, panel: Rect, scale: f32, songs: &mut Songs) -> Option<Request> {
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
    y += 18.0 * scale;

    // The name field, then SAVE AS beside it.
    let field = Rect::from_min_size(
        egui::Pos2::new(left, y),
        egui::vec2(width * 0.46, row_height),
    );
    ui.put(
        field,
        egui::TextEdit::singleline(&mut songs.name)
            .hint_text("song name")
            .desired_width(field.width()),
    );
    let save_rect = Rect::from_min_size(
        egui::Pos2::new(field.right() + padding, y),
        egui::vec2(width * 0.26 - padding, row_height),
    );
    let rename_rect = Rect::from_min_size(
        egui::Pos2::new(save_rect.right() + padding, y),
        egui::vec2(width * 0.28 - padding, row_height),
    );
    let typed = crate::persist::song_name(&songs.name);
    if transport::transport_button(ui, "song-save-as", save_rect, "SAVE AS", typed.is_some())
        .clicked()
    {
        request = typed.clone().map(Request::Save);
    }
    // RENAME renames the song the session is on, so it is dark until there is one and the name really differs.
    let can_rename = matches!((&typed, &songs.open), (Some(new), Some(open)) if new != open);
    if can_rename
        && transport::transport_button(ui, "song-rename", rename_rect, "RENAME", can_rename)
            .clicked()
        && let Some(new) = typed.clone()
    {
        request = Some(Request::Rename(new));
    }
    y += row_height + padding;

    // One row per song: the name loads it, the cross beside it deletes it (twice, because there is no undo).
    for (index, name) in songs.names.iter().enumerate() {
        let row = Rect::from_min_size(
            egui::Pos2::new(left, y),
            egui::vec2(width * 0.66, row_height),
        );
        if transport::transport_button(ui, &format!("song-{index}"), row, name, false).clicked() {
            request = Some(Request::Load(name.clone()));
            songs.delete_armed = None;
        }
        let armed = songs.delete_armed.as_deref() == Some(name.as_str());
        let cross = Rect::from_min_size(
            egui::Pos2::new(row.right() + padding, y),
            egui::vec2(width * 0.14, row_height),
        );
        let label = if armed { "SURE" } else { "x" };
        if transport::transport_button(ui, &format!("song-delete-{index}"), cross, label, armed)
            .clicked()
        {
            if armed {
                request = Some(Request::Delete(name.clone()));
                songs.delete_armed = None;
            } else {
                songs.delete_armed = Some(name.clone());
                request = None;
            }
        }
        y += row_height + 2.0 * scale;
    }

    // The bottom row: start a new song on the left, close on the right.
    let bottom = card.bottom() - padding - row_height;
    let new_rect = Rect::from_min_size(
        egui::Pos2::new(left, bottom),
        egui::vec2(120.0 * scale, row_height),
    );
    if transport::transport_button(ui, "song-new", new_rect, "NEW SONG", false).clicked() {
        request = Some(Request::New);
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
            egui::Pos2::new(left + new_rect.width() + padding, bottom + row_height * 0.5),
            egui::Align2::LEFT_CENTER,
            message,
            theme::legend(10.0 * scale),
            theme::SILKSCREEN_DIM,
        );
    }

    request
}
