//! The transport row: a painted rectangular key like the hardware's RUN/STOP, and the tempo nudges.
//!
//! The key is drawn rather than styled from a stock widget because the panel has no stock widgets: a lit
//! RUN is amber, a dark one is engraving on black.

use egui::{Rect, Response};

use super::theme;

/// What the transport row asked the player for.
///
/// A request rather than a write: the rack owns the transport, and it fans a change out to every device. A row that wrote
/// the displayed device's own atomic instead would be undone by the next `sync_transport`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Request {
    Play,
    Stop,
    /// One BPM, in the direction of the key.
    NudgeTempo(i8),
}

/// One BPM up or down, clamped to the panel's tempo range.
pub fn nudge_tempo(bpm: f32, direction: i8) -> f32 {
    acid_core::timing::clamp_bpm(bpm + f32::from(direction))
}

/// Cells in the transport row: RUN, STOP, TEMPO −, TEMPO +, LEARN.
///
/// The fifth cell is the MIDI learn key, not a volume: the volume knob lives in the top strip's corner, and the name this
/// comment used to carry was a leftover from before learn had a cell.
pub const TRANSPORT_CELLS: usize = 5;
/// A key on one of the cards. Every card is painted in the black scheme whatever the face behind it does, so this is the one
/// place that says so — a card that took the face's palette would go amber over an amber device.
pub fn card_button(ui: &mut egui::Ui, id: &str, rect: Rect, label: &str, lit: bool) -> Response {
    transport_button(ui, id, rect, label, lit, &theme::BLACK)
}

/// A rectangular transport key. `lit` paints the legend in the palette's `lit_ink`, the way RUN glows while playing.
pub fn transport_button(
    ui: &mut egui::Ui,
    id: &str,
    rect: Rect,
    label: &str,
    lit: bool,
    p: &theme::Palette,
) -> Response {
    let response = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click());
    let scale = key_scale(rect);
    let face = if lit { p.panel_raised } else { p.key_face };
    // A lit key is a raised face and a legend on it, so its legend is `lit_ink`: `led_accent` is ink for the *face*, and in
    // the amber scheme a colour dark enough to read on amber is too dark to read on a near-black key.
    let legend = if lit { p.lit_ink } else { p.silkscreen };

    let painter = ui.painter();
    painter.rect_filled(rect, 3.0 * scale, face);
    painter.rect_stroke(
        rect,
        3.0 * scale,
        p.silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        key_font(rect),
        legend,
    );
    if response.hovered() {
        // The ring follows the legend's rule, because it is drawn on the same face: a lit key's is `panel_raised`, where
        // the face's accent is unreadable in the amber scheme, and an unlit key's well takes the accent.
        painter.rect_stroke(
            rect,
            3.0 * scale,
            egui::Stroke::new(1.5 * scale, if lit { p.lit_ink } else { p.led_accent }),
            egui::StrokeKind::Inside,
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A key's own scale, from its height: the corner radius, the stroke and the legend all follow it.
fn key_scale(rect: Rect) -> f32 {
    (rect.height() / 44.0).clamp(0.65, 1.6)
}

/// The font a key's caption is drawn at. Public because a card that fits a label into a key — a port's name belongs to the
/// machine and can be as long as it likes — has to measure with the font that will draw it, and a second spelling of this
/// arithmetic would drift from this one.
pub fn key_font(rect: Rect) -> egui::FontId {
    theme::legend(13.0 * key_scale(rect))
}
