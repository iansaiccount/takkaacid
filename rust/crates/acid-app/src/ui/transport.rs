//! The transport row: a painted rectangular key like the hardware's RUN/STOP, and the tempo nudges.
//!
//! The key is drawn rather than styled from a stock widget because the panel has no stock widgets: a lit
//! RUN is amber, a dark one is engraving on black.

use egui::{Rect, Response};

use super::theme;

/// One BPM up or down, clamped to the panel's tempo range.
pub fn nudge_tempo(bpm: f32, direction: i8) -> f32 {
    acid_core::timing::clamp_bpm(bpm + f32::from(direction))
}

/// Cells in the transport row: RUN, STOP, TEMPO −, TEMPO +, VOLUME.
pub const TRANSPORT_CELLS: usize = 5;
/// A rectangular transport key. `lit` paints the legend in `LED_ACCENT`, the way RUN glows while playing.
pub fn transport_button(
    ui: &mut egui::Ui,
    id: &str,
    rect: Rect,
    label: &str,
    lit: bool,
) -> Response {
    let response = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click());
    let scale = (rect.height() / 44.0).clamp(0.65, 1.6);
    let face = if lit {
        theme::PANEL_RAISED
    } else {
        theme::WELL
    };
    let legend = if lit {
        theme::LED_ACCENT
    } else {
        theme::SILKSCREEN
    };

    let painter = ui.painter();
    painter.rect_filled(rect, 3.0 * scale, face);
    painter.rect_stroke(
        rect,
        3.0 * scale,
        theme::silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        theme::legend(13.0 * scale),
        legend,
    );
    if response.hovered() {
        painter.rect_stroke(
            rect,
            3.0 * scale,
            egui::Stroke::new(1.5 * scale, theme::LED_ACCENT),
            egui::StrokeKind::Inside,
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}
