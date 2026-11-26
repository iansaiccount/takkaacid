//! The performance row: the octave keys, CLEAR and RANDOM on the left; the shuffle, density and scale dials on the right.

use std::sync::Arc;

use eframe::egui;

use crate::knobs::snap_to_detent;
use crate::midi;
use crate::state::SharedState;

use super::controls::{Key, learn_pick, switch_key};
use super::{PanelState, knob, layout, learn, panel, perform, randomise, scale, theme, tips};
/// Draw the performance row into its cells.
pub(super) fn draw(
    state: &mut PanelState,
    shared: &SharedState,
    ui: &mut egui::Ui,
    l: &layout::PanelLayout,
    s: f32,
    p: &theme::Palette,
) {
    // The performance row: the octave keys, CLEAR and RANDOM on the left; the shuffle knob and the density and scale dials
    // that shape the next roll on the right.
    // One rect per control, asked for by name: the catalogue owns the order and the count, so a control added to a cluster
    // widens that cluster instead of renumbering what a test clicks.
    let area = l.perform.area;
    let octave_down = perform::cell(area, perform::Perform::OctaveDown);
    let octave_up = perform::cell(area, perform::Perform::OctaveUp);
    let clear_cell = perform::cell(area, perform::Perform::Clear);
    let randomise_cell = perform::cell(area, perform::Perform::Randomise);
    let shuffle_cell = perform::cell(area, perform::Perform::Shuffle);
    let density_cell = perform::cell(area, perform::Perform::Density);
    let scale_cell = perform::cell(area, perform::Perform::Scale);
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
        if switch_key(
            ui,
            state,
            Key {
                id,
                rect,
                label,
                lit: false,
                control,
            },
            p,
        ) {
            let moved = state.pattern.transposed(12 * octaves);
            state.pattern = Arc::new(moved);
            shared.set_pattern(Arc::clone(&state.pattern));
            state.dirty = true;
            // An octave move changes the pattern, so an armed CLEAR is stale: the second click would empty the line the
            // player has just transposed rather than the one they armed it against.
            super::clear::disarm(state);
        }
    }

    // CLEAR empties the pattern on screen. It is the one action here with no undo, so it takes two clicks
    // (`ui::clear`), and it lives beside the octave keys because it is a thing done to the pattern rather than to the roll.
    let now = std::time::Instant::now();
    let armed = super::clear::is_armed(state.clear_armed_at, now);
    if switch_key(
        ui,
        state,
        Key {
            id: "clear-pattern",
            rect: clear_cell,
            label: if armed { "CLEAR?" } else { "CLEAR" },
            lit: armed,
            control: tips::Control::Clear,
        },
        p,
    ) {
        match super::clear::click(state.clear_armed_at, now) {
            super::clear::Click::Armed => state.clear_armed_at = Some(now),
            super::clear::Click::Cleared => {
                super::clear::disarm(state);
                // CLEAR empties the bar without leaving it: a key a player meets in the first minute must not
                // silently turn thirty-two keys into sixteen.
                let empty = Arc::new(acid_core::pattern::Pattern::empty_in(shared.pattern().grid));
                shared.set_pattern(Arc::clone(&empty));
                state.pattern = empty;
                state.dirty = true;
            }
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
        perform_knob(ui, shuffle_cell, "shuffle", "SHUFFLE", value, p)
    };
    tips::hover(&response, state.help(), tips::Control::Shuffle);
    if learn::highlight(state.learn, midi::cc::CcTarget::Swing) {
        panel::learn_highlight(ui.painter(), shuffle_cell, s, p);
    }
    if choosing {
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
    let response = perform_knob(ui, density_cell, "density", &caption, &mut density, p);
    tips::hover(&response, state.help(), tips::Control::Density);
    state.density = density.clamp(0.0, 1.0);
    if state.density != before_density {
        state.dirty = true;
    }

    // SCALE snaps on release, never mid-drag: a frame-by-frame snap pulls the position back onto the detent it
    // started from, and a slow drag could never cross a midpoint.
    let mut position = state.scale_knob;
    let response = perform_knob(
        ui,
        scale_cell,
        "scale",
        state.scale.name(),
        &mut position,
        p,
    );
    tips::hover(&response, state.help(), tips::Control::Scale);
    state.scale_knob = position.clamp(0.0, 1.0);
    if !response.dragged() {
        // The knob lands on a scale's detent; read the scale back off the snapped position so the two cannot drift.
        state.scale_knob = snap_to_detent(
            state.scale_knob,
            scale::scale_from_position,
            scale::scale_position,
        );
        let snapped = scale::scale_from_position(state.scale_knob);
        if snapped != state.scale {
            state.scale = snapped;
            state.dirty = true;
        }
    }

    if switch_key(
        ui,
        state,
        Key {
            id: "randomise",
            rect: randomise_cell,
            label: "RANDOM",
            lit: false,
            control: tips::Control::Randomise,
        },
        p,
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
        // A roll changes the pattern, so an armed CLEAR is stale: the second click would empty the line that just landed
        // rather than the one the player armed it against.
        super::clear::disarm(state);
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
    p: &theme::Palette,
) -> egui::Response {
    let diameter = cell.height() * 0.78;
    let square = egui::Rect::from_center_size(cell.center(), egui::vec2(diameter, cell.height()));
    let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(square));
    cell_ui.set_clip_rect(cell);
    knob::knob(
        &mut cell_ui,
        egui::Id::unique(id),
        label,
        value,
        diameter,
        p,
    )
}
