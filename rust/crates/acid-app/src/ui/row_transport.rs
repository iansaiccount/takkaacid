//! The transport row: the device's direction, the rack's transpose and its readout, two cells still labelled `TBC`, and the
//! MIDI LEARN key.

use eframe::egui;

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
    let tbc_cell = transport::cell(l.transport, transport::Cell::Tbc);
    let spare_cell = transport::cell(l.transport, transport::Cell::Spare);
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
    // TBC: two cells kept for the player — the row is seven to match the switch row above it, and a spare cell that said
    // nothing would be a hole. Both say so rather than doing nothing, because a silent key reads as broken.
    for (id, rect) in [("tbc", tbc_cell), ("tbc-spare", spare_cell)] {
        if switch_key(
            ui,
            state,
            Key {
                id,
                rect,
                label: "TBC",
                lit: false,
                control: tips::Control::Tbc,
            },
            p,
        ) {
            state.notify("TBC IS NOT DECIDED YET");
        }
    }
    // The transpose pair: the whole rack, a semitone at a time, as performance and never as the patch.
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
