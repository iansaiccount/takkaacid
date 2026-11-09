//! One modal idiom for the cards that sit over the panel: the face dims behind the card, and a click anywhere outside
//! it takes it away.
//!
//! The shield must be registered **before** the card paints its own controls: a later widget wins an egui hit-test tie,
//! so a shield registered after a row would read that row's click as a click outside the card. Plan 35 paid for that with
//! the settings card's fold headers, which is why the rule lives here rather than in each card.

use eframe::egui::{self, Rect, Response, Ui};

use super::theme;

/// Dim `window` and register the dismissal click over it. The answer is that click: what counts as outside the card is
/// the caller's business, because only the card knows where its own rows are.
pub fn shield(ui: &mut Ui, window: Rect, id: &str) -> Response {
    ui.painter().rect_filled(window, 0.0, theme::SHADE);
    ui.interact(window, ui.scope_id().with(id), egui::Sense::click())
}
