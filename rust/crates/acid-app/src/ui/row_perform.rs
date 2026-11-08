//! The performance row: transpose, shuffle, and the two dials that decide what RANDOM does.

use std::sync::Arc;

use eframe::egui;

use crate::midi;
use crate::state::SharedState;

use super::controls::{learn_pick, switch_key};
use super::{PanelState, knob, layout, learn, panel, randomise, scale, tips};
/// Draw the performance row into its cells.
pub(super) fn draw(
    state: &mut PanelState,
    shared: &SharedState,
    ui: &mut egui::Ui,
    l: &layout::PanelLayout,
    s: f32,
) {
    // The performance row: transpose the pattern, shuffle the clock, and the two dials that decide what RANDOM does.
    if let [
        octave_down,
        octave_up,
        shuffle_cell,
        randomise_cell,
        density_cell,
        scale_cell,
    ] = l.perform.cells[..]
    {
        for (rect, id, label, octaves, control) in [
            (
                octave_down,
                "octave-down",
                "OCT -",
                -1,
                tips::Control::OctaveDown,
            ),
            (octave_up, "octave-up", "OCT +", 1, tips::Control::OctaveUp),
        ] {
            if switch_key(ui, state, id, rect, label, false, control) {
                let moved = state.pattern.transposed(12 * octaves);
                state.pattern = Arc::new(moved);
                shared.set_pattern(Arc::clone(&state.pattern));
                state.dirty = true;
            }
        }

        // SHUFFLE is a knob, not a key: it is the `swing` value the clock has always used, and a learned controller
        // writes the same atomic, so the fader and the hand meet in one place.
        //
        // While a learn is choosing, the face stays on the panel — a control that disappears when the player is asked to
        // click it cannot be clicked — and the hand turns a scratch value instead, exactly as the tone knobs do.
        let choosing = state.learn == learn::Step::Choosing;
        let before_swing = shared.swing.get();
        let mut swing = before_swing;
        let mut scratch = before_swing;
        let response = {
            let value = if choosing { &mut scratch } else { &mut swing };
            perform_knob(ui, shuffle_cell, "shuffle", "SHUFFLE", value)
        };
        tips::hover(&response, state, tips::Control::Shuffle);
        if choosing {
            panel::learn_highlight(ui.painter(), shuffle_cell, s);
            learn_pick(
                ui,
                state,
                shared,
                "learn-shuffle",
                shuffle_cell,
                midi::cc::CcTarget::Swing,
            );
        } else if swing != before_swing {
            shared.swing.set(swing.clamp(0.0, 1.0));
            state.dirty = true;
        }

        // DENSITY is how much of the pattern a roll fills: 0 leaves the one guaranteed step, 1 fills every step.
        let before_density = state.density;
        let mut density = state.density;
        let caption = format!("{:.0}%", density * 100.0);
        let response = perform_knob(ui, density_cell, "density", &caption, &mut density);
        tips::hover(&response, state, tips::Control::Density);
        state.density = density.clamp(0.0, 1.0);
        if state.density != before_density {
            state.dirty = true;
        }

        // SCALE snaps on release, never mid-drag: a frame-by-frame snap pulls the position back onto the detent it
        // started from, and a slow drag could never cross a midpoint.
        let mut position = state.scale_knob;
        let response = perform_knob(ui, scale_cell, "scale", state.scale.name(), &mut position);
        tips::hover(&response, state, tips::Control::Scale);
        state.scale_knob = position.clamp(0.0, 1.0);
        if !response.dragged() {
            let snapped = scale::scale_from_position(state.scale_knob);
            state.scale_knob = scale::scale_position(snapped);
            if snapped != state.scale {
                state.scale = snapped;
                state.dirty = true;
            }
        }

        if switch_key(
            ui,
            state,
            "randomise",
            randomise_cell,
            "RANDOM",
            false,
            tips::Control::Randomise,
        ) {
            // Seeded from the clock: a randomiser that repeats is a bug, and the seed is there for the tests.
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos() as u64)
                .unwrap_or(1);
            let rolled = randomise::randomised(*state.pattern, seed, state.density, state.scale);
            state.pattern = Arc::new(rolled);
            shared.set_pattern(Arc::clone(&state.pattern));
            state.dirty = true;
        }
    }
}

/// A knob in a performance-row cell.
///
/// The knob is a circle, not a column: `knob::knob` allocates from its UI's top-left, so a row-wide cell would strand it
/// at the left edge where the hand does not reach. It gets a square in the middle of its cell instead.
fn perform_knob(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    id: &str,
    label: &str,
    value: &mut f32,
) -> egui::Response {
    let diameter = cell.height() * 0.78;
    let square = egui::Rect::from_center_size(cell.center(), egui::vec2(diameter, cell.height()));
    let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(square));
    cell_ui.set_clip_rect(cell);
    knob::knob(&mut cell_ui, egui::Id::unique(id), label, value, diameter)
}
