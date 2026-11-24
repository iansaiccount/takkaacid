//! The transport row: the device's direction and transpose, the pattern's grid and its readout, and the MIDI LEARN key.

use eframe::egui;

use acid_core::pattern::Grid;

use crate::state::SharedState;

use super::controls::{Key, switch_key};
use super::{PanelState, layout, learn, theme, tips, transport};

/// Draw the transport row into its cells.
pub(super) fn draw(
    state: &mut PanelState,
    shared: &SharedState,
    ui: &mut egui::Ui,
    l: &layout::PanelLayout,
    p: &theme::Palette,
) {
    // Every key here is a *request*: the rack owns the transport and fans it out to every device, so a row that wrote the
    // displayed device's atomic would be undone by the next sync.
    // One rect per cell, asked for by name. The catalogue owns the count, so adding a cell widens the row and draws here
    // without a second list of numbers.
    let reverse_cell = transport::cell(l.transport, transport::Cell::Reverse);
    let transpose_down = transport::cell(l.transport, transport::Cell::TransposeDown);
    let readout = transport::cell(l.transport, transport::Cell::Readout);
    let transpose_up = transport::cell(l.transport, transport::Cell::TransposeUp);
    let grid_cell = transport::cell(l.transport, transport::Cell::Grid);
    let grid_well = transport::cell(l.transport, transport::Cell::GridWell);
    let learn_cell = transport::cell(l.transport, transport::Cell::Learn);
    // REVERSE is the displayed device's direction, read from its own state and asked for as a request: the engine takes
    // it at the bar line, so the key may be lit for a bar before the line turns.
    let reversed = shared.is_reversed();
    if switch_key(
        ui,
        state,
        Key {
            id: "reverse",
            rect: reverse_cell,
            label: "REVERSE",
            lit: reversed,
            control: tips::Control::Reverse,
        },
        p,
    ) {
        // Not `dirty`: the direction lives in `SharedState` and no save file has a field for it. The transpose keys
        // beside it do not dirty for the same reason, and one of them dirtying is a file written for nothing.
        state.transport_request = Some(transport::Request::Reverse(!reversed));
    }
    // The transpose pair: this device, a semitone at a time, as performance and never as the patch. The readout and both keys'
    // litness read `shared`, the device on the panel, so the row shows exactly the cell the keys write.
    if switch_key(
        ui,
        state,
        Key {
            id: "transpose-down",
            rect: transpose_down,
            label: "TRANS -",
            lit: shared.transpose() < 0,
            control: tips::Control::TransposeDown,
        },
        p,
    ) {
        state.transport_request = Some(transport::Request::Transpose(-1));
    }
    if switch_key(
        ui,
        state,
        Key {
            id: "transpose-up",
            rect: transpose_up,
            label: "TRANS +",
            lit: shared.transpose() > 0,
            control: tips::Control::TransposeUp,
        },
        p,
    ) {
        state.transport_request = Some(transport::Request::Transpose(1));
    }

    // The transpose reads out between the two keys that move it, in a well of its own: the value and the keys that change
    // it are one cluster, the way the tempo's `-` and `+` stack beside its number in the top band. The well scales off its
    // own rect like the keys beside it, so a small window shows a smaller number rather than a key-sized one in a row that
    // shrank around it.
    {
        let well = readout;
        let scale = transport::key_scale(well);
        let painter = ui.painter();
        painter.rect_filled(well, 3.0 * scale, p.well);
        painter.rect_stroke(
            well,
            3.0 * scale,
            p.silkscreen_stroke(scale),
            egui::StrokeKind::Inside,
        );
        let semitones = shared.transpose();
        painter.text(
            well.center(),
            egui::Align2::CENTER_CENTER,
            format!("{semitones:+}"),
            theme::legend(14.0 * scale),
            if semitones == 0 {
                p.silkscreen_dim
            } else {
                p.led_accent
            },
        );
    }

    // GRID: a **pattern edit**, not a transport request — the grid belongs to the pattern, so the key writes it at once and
    // the engine hears it on its next block. The well beside it prints the grid the pattern is on, dim at `1/16` and in
    // the accent otherwise: "this pattern is not on the plain grid" at a glance.
    if switch_key(
        ui,
        state,
        Key {
            id: "grid",
            rect: grid_cell,
            label: "GRID",
            lit: false,
            control: tips::Control::Grid,
        },
        p,
    ) {
        // What the press costs is counted on the pattern that is going, before it goes: the shorter bar rests the steps
        // it cannot hold, and the status line says how many of them had sound.
        let before = shared.pattern();
        let next = before.grid.next();
        let held = before.sounding_steps_beyond(next);
        shared.set_grid(next);
        state.pattern = shared.pattern();
        // The border is clamped into the bar that just arrived: a press that narrows thirty-two steps to twelve with the
        // border on step 21 would leave no key outlined at all, and the keyboard's next write would land somewhere the
        // face is not pointing at.
        state.selected = state.selected.min(state.pattern.len() - 1);
        // A pending `CLEAR` was armed against the pattern as it was, and this press may have rested steps inside it.
        super::clear::disarm(state);
        state.dirty = true;
        let notice = transport::grid_notice(before.len(), state.pattern.len(), held);
        state.notify(notice);
    }
    {
        let well = grid_well;
        let scale = transport::key_scale(well);
        let painter = ui.painter();
        painter.rect_filled(well, 3.0 * scale, p.well);
        painter.rect_stroke(
            well,
            3.0 * scale,
            p.silkscreen_stroke(scale),
            egui::StrokeKind::Inside,
        );
        let grid = state.pattern.grid;
        painter.text(
            well.center(),
            egui::Align2::CENTER_CENTER,
            grid.name(),
            theme::legend(14.0 * scale),
            if grid == Grid::Sixteenth {
                p.silkscreen_dim
            } else {
                p.led_accent
            },
        );
    }

    // The last cell is the MIDI LEARN key (see `ui::learn`). It is asked for as `Cell::Learn`, so a re-ordered row cannot
    // bind a learn to whatever now sits in slot six.
    if switch_key(
        ui,
        state,
        Key {
            id: "learn",
            rect: learn_cell,
            label: learn::label(state.learn),
            lit: state.learn != learn::Step::Idle,
            control: tips::Control::Learn,
        },
        p,
    ) {
        state.learn = match state.learn {
            learn::Step::Idle => learn::Step::Choosing,
            _ => learn::Step::Idle,
        };
        learn::cancel(shared);
    }
}
