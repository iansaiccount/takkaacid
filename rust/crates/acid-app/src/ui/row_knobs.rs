//! The knob row: the seven tone knobs, drawn in the catalogue's order.
//!
//! Everything the row needs about a knob — its slot, caption, hover help, learn target and the atomic it drives — comes
//! from `crate::knobs`, so this file cannot disagree with the saved file or the learn map.

use eframe::egui;

use crate::knobs::{Kind, Knob};
use crate::state::SharedState;

use super::{
    PanelState, knob, layout, learn, panel, theme, tips, waveform_from_position, waveform_position,
};

/// Draw the seven tone knobs into their cells.
pub(super) fn draw(
    state: &mut PanelState,
    shared: &SharedState,
    ui: &mut egui::Ui,
    l: &layout::PanelLayout,
    s: f32,
) {
    // The row is the catalogue, walked in drawing order: a knob's slot, caption, tooltip, learn target and the atomic it
    // drives all come from `Knob`, so this loop cannot disagree with the saved file or the learn map.
    for (knob, cell) in Knob::ALL.iter().zip(l.knobs.cells.iter()) {
        let knob = *knob;
        let slot = knob.index();
        let diameter = cell.width().min(cell.height() * 0.78);
        let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(cell.shrink(2.0 * s)));
        cell_ui.set_clip_rect(*cell);
        let before = state.knobs[slot];
        // While choosing a knob to teach, the knobs stand back: a click picks the knob instead of turning it, and
        // the turn goes into a scratch value so a stray drag cannot move the patch either.
        let choosing = state.learn == learn::Step::Choosing;
        let mut scratch = before;
        // The caption is read before the knob borrows its value: the switch names the wave it is on.
        let label = knob.label(shared);
        let target = if choosing {
            &mut scratch
        } else {
            &mut state.knobs[slot]
        };
        let response = knob::knob(
            &mut cell_ui,
            egui::Id::unique(("knob", slot)),
            label,
            target,
            diameter,
        );
        tips::hover(&response, state, tips::Control::for_knob(knob));
        // A switch wears its detents, so the faces it may snap to are visible before the hand lets go.
        if knob.kind() == Kind::Switch && !choosing {
            waveform_detents(cell_ui.painter(), *cell, state.knobs[slot], s);
        }
        if choosing {
            // Every knob can be taught, so while a learn is choosing each one wears the pick colour: the question is
            // "which one?", and the answer has to be visible without hunting.
            panel::learn_highlight(ui.painter(), *cell, s);
            let hit = ui.interact(
                *cell,
                ui.scope_id().with(("learn-knob", slot)),
                egui::Sense::click(),
            );
            if hit.clicked() {
                state.learn = learn::choose(knob.learn_target(), shared);
            }
            continue;
        }
        // Publish straight away: the audio thread's next block picks it up, so the move is heard now.
        //
        // A switch has two masters: the hand and a learned controller. While the hand is on the knob it wins; when it
        // is not, the shared value is the truth, so a fader moves the face. Snapping happens on release, never mid-drag —
        // a frame-by-frame snap pulls the position back onto the detent it started from, and a slow drag could never
        // cross a midpoint.
        let moved = state.knobs[slot] != before;
        match knob.kind() {
            Kind::Switch => {
                if response.dragged() {
                    knob.publish(shared, state.knobs[slot]);
                } else {
                    let hand = waveform_from_position(state.knobs[slot]);
                    // The hand's own move wins: the wheel, or a drag that has just ended. Otherwise the switch is where
                    // the shared state says, which is where a learned controller put it.
                    let face = if moved || shared.waveform() == hand {
                        hand
                    } else {
                        shared.waveform()
                    };
                    knob.publish(shared, waveform_position(face));
                    state.knobs[slot] = knob.read(shared);
                }
            }
            Kind::Dial => knob.publish(shared, state.knobs[slot]),
        }
        // Only the hand marks the session unsaved: a switch adopting a learned controller's value is the panel
        // catching up, not an edit the player made.
        if moved {
            state.dirty = true;
        }
    }
}
/// Mark every wave's detent around the WAVEFORM knob, lighting the one the knob is on.
///
/// The knob's pointer alone cannot say *where* the other waves are, which is the difference between a dial and a switch;
/// a tick per wave turns it into one at a glance.
fn waveform_detents(painter: &egui::Painter, cell: egui::Rect, position: f32, scale: f32) {
    let diameter = cell.width().min(cell.height() * 0.78);
    let center = egui::Pos2::new(cell.center().x, cell.top() + diameter / 2.0);
    let radius = diameter / 2.0;
    let ticks = acid_core::oscillator::Waveform::ALL.len();
    let chosen = waveform_from_position(position);

    for (index, wave) in acid_core::oscillator::Waveform::ALL.iter().enumerate() {
        let angle = knob::angle_for(index as f32 / (ticks - 1) as f32);
        let lit = *wave == chosen;
        let inner = radius * 1.06;
        let outer = radius * if lit { 1.22 } else { 1.16 };
        let (sin, cos) = (angle.sin(), angle.cos());
        painter.line_segment(
            [
                egui::Pos2::new(center.x + inner * sin, center.y - inner * cos),
                egui::Pos2::new(center.x + outer * sin, center.y - outer * cos),
            ],
            if lit {
                egui::Stroke::new(2.0 * scale, theme::LED_ACCENT)
            } else {
                egui::Stroke::new(1.0 * scale, theme::SILKSCREEN_DIM)
            },
        );
    }
}
