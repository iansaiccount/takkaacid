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
    if let [run, stop, tempo_down, tempo_up, volume_cell] = cells[..] {
        let playing = shared.is_playing();
        if switch_key(
            ui,
            state,
            Key {
                id: "run",
                rect: run,
                label: "RUN",
                lit: playing,
                control: tips::Control::Run,
            },
            p,
        ) {
            state.transport_request = Some(transport::Request::Play);
            state.dirty = true;
        }
        if switch_key(
            ui,
            state,
            Key {
                id: "stop",
                rect: stop,
                label: "STOP",
                lit: !playing,
                control: tips::Control::Stop,
            },
            p,
        ) {
            state.transport_request = Some(transport::Request::Stop);
            state.dirty = true;
        }
        if switch_key(
            ui,
            state,
            Key {
                id: "tempo-down",
                rect: tempo_down,
                label: "TEMPO -",
                lit: false,
                control: tips::Control::TempoDown,
            },
            p,
        ) {
            state.transport_request = Some(transport::Request::NudgeTempo(-1));
            state.dirty = true;
        }
        if switch_key(
            ui,
            state,
            Key {
                id: "tempo-up",
                rect: tempo_up,
                label: "TEMPO +",
                lit: false,
                control: tips::Control::TempoUp,
            },
            p,
        ) {
            state.transport_request = Some(transport::Request::NudgeTempo(1));
            state.dirty = true;
        }

        // The last transport cell is the MIDI LEARN key (see `ui::learn`), so the row keeps its five keys while
        // the volume knob lives in the top strip's corner.
        if switch_key(
            ui,
            state,
            Key {
                id: "learn",
                rect: volume_cell,
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
