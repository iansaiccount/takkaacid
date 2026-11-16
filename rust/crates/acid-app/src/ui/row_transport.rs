//! The transport row: five lit keys and the tempo they run at.

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
    // Transport: RUN and STOP are lit keys; the tempo buttons nudge one BPM at a time. Both are *requests*: the rack owns
    // the transport and fans it out to every device, so a row that wrote the displayed device's atomic would be undone by
    // the next sync.
    let cells = layout::knob_cells(l.transport, transport::TRANSPORT_CELLS);
    if let [
        reverse_cell,
        transpose_down,
        readout,
        transpose_up,
        tbc_cell,
        learn_cell,
    ] = cells[..]
    {
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
        // TBC: a cell kept for the player. It says so rather than doing nothing, because a silent key reads as broken.
        if switch_key(
            ui,
            state,
            Key {
                id: "tbc",
                rect: tbc_cell,
                label: "TBC",
                lit: false,
                control: tips::Control::Tbc,
            },
            p,
        ) {
            state.notify("TBC IS NOT DECIDED YET");
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
        // it are one cluster, the way the tempo's `+` and `−` sit either side of its number in the top band.
        {
            let well = readout;
            let painter = ui.painter();
            painter.rect_filled(well, 3.0, p.well);
            painter.rect_stroke(
                well,
                3.0,
                p.silkscreen_stroke(1.0),
                egui::StrokeKind::Inside,
            );
            let semitones = shared.transpose();
            painter.text(
                well.center(),
                egui::Align2::CENTER_CENTER,
                format!("{semitones:+}"),
                theme::legend(14.0),
                if semitones == 0 {
                    p.silkscreen_dim
                } else {
                    p.led_accent
                },
            );
        }

        // The last transport cell is the MIDI LEARN key (see `ui::learn`), so the row keeps its five keys while
        // the volume knob lives in the top strip's corner.
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
}
