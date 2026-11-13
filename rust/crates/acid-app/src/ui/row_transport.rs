//! The transport row: five lit keys and the tempo they run at.

use eframe::egui;

use crate::state::SharedState;

use super::controls::switch_key;
use super::{PanelState, layout, learn, tips, transport};

/// Draw the transport row into its cells.
pub(super) fn draw(
    state: &mut PanelState,
    shared: &SharedState,
    ui: &mut egui::Ui,
    l: &layout::PanelLayout,
) {
    // Transport: RUN and STOP are lit keys; the tempo buttons nudge one BPM at a time. Both are *requests*: the rack owns
    // the transport and fans it out to every device, so a row that wrote the displayed device's atomic would be undone by
    // the next sync.
    let cells = layout::knob_cells(l.transport, transport::TRANSPORT_CELLS);
    if let [run, stop, tempo_down, tempo_up, volume_cell] = cells[..] {
        let playing = shared.is_playing();
        if switch_key(ui, state, "run", run, "RUN", playing, tips::Control::Run) {
            state.transport_request = Some(transport::Request::Play);
            state.dirty = true;
        }
        if switch_key(
            ui,
            state,
            "stop",
            stop,
            "STOP",
            !playing,
            tips::Control::Stop,
        ) {
            state.transport_request = Some(transport::Request::Stop);
            state.dirty = true;
        }
        if switch_key(
            ui,
            state,
            "tempo-down",
            tempo_down,
            "TEMPO -",
            false,
            tips::Control::TempoDown,
        ) {
            state.transport_request = Some(transport::Request::NudgeTempo(-1));
            state.dirty = true;
        }
        if switch_key(
            ui,
            state,
            "tempo-up",
            tempo_up,
            "TEMPO +",
            false,
            tips::Control::TempoUp,
        ) {
            state.transport_request = Some(transport::Request::NudgeTempo(1));
            state.dirty = true;
        }

        // The last transport cell is the MIDI LEARN key (see `ui::learn`), so the row keeps its five keys while
        // the volume knob lives in the top strip's corner.
        if switch_key(
            ui,
            state,
            "learn",
            volume_cell,
            learn::label(state.learn),
            state.learn != learn::Step::Idle,
            tips::Control::Learn,
        ) {
            state.learn = match state.learn {
                learn::Step::Idle => learn::Step::Choosing,
                _ => learn::Step::Idle,
            };
            learn::cancel(shared);
        }
    }
}
