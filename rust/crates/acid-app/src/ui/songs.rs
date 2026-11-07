//! The songs card: keep the session under a name, and load one you have kept.
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
}

/// What the card asked for. `main` does the work, so the card stays a pure painter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Save(String),
    Load(String),
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
        egui::vec2(width * 0.62, row_height),
    );
    ui.put(
        field,
        egui::TextEdit::singleline(&mut songs.name)
            .hint_text("song name")
            .desired_width(field.width()),
    );
    let save_rect = Rect::from_min_size(
        egui::Pos2::new(field.right() + padding, y),
        egui::vec2(width * 0.38 - padding, row_height),
    );
    let can_save = crate::persist::song_name(&songs.name).is_some();
    let mut request = None;
    if transport::transport_button(ui, "song-save-as", save_rect, "SAVE AS", can_save).clicked() {
        request = crate::persist::song_name(&songs.name).map(Request::Save);
    }
    y += row_height + padding;

    for (index, name) in songs.names.iter().enumerate() {
        let row = Rect::from_min_size(
            egui::Pos2::new(left, y),
            egui::vec2(width * 0.7, row_height),
        );
        if transport::transport_button(ui, &format!("song-{index}"), row, name, false).clicked() {
            request = Some(Request::Load(name.clone()));
        }
        y += row_height + 2.0 * scale;
    }

    let close = Rect::from_min_size(
        egui::Pos2::new(
            card.right() - padding - 90.0 * scale,
            card.bottom() - padding - row_height,
        ),
        egui::vec2(90.0 * scale, row_height),
    );
    if transport::transport_button(ui, "song-close", close, "CLOSE", false).clicked() {
        request = Some(Request::Close);
    }

    if let Some(message) = &songs.message {
        ui.painter().text(
            egui::Pos2::new(left, close.top() + 4.0 * scale),
            egui::Align2::LEFT_TOP,
            message,
            theme::legend(10.0 * scale),
            theme::SILKSCREEN_DIM,
        );
    }

    request
}
