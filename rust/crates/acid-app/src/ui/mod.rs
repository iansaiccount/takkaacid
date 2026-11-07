// The palette and layout constants are the panel's vocabulary: they are introduced here and consumed
// from Phase 06 onwards, so the dead-code lint is waived for the module rather than per constant.
pub mod dispatch;
pub mod display;
pub mod edit;
pub mod help;
pub mod keys;
pub mod knob;
pub mod layout;
pub mod panel;
pub mod picker;
pub mod selection;
pub mod shortcuts;
pub mod step_key;
#[allow(dead_code)]
pub mod theme;
pub mod transport;

use std::sync::Arc;

use acid_core::pattern::{Pattern, PatternId, STEPS_PER_PATTERN, Step};
use acid_core::timing::DEFAULT_BPM;

use crate::state::SharedState;

use acid_core::pitch::Pitch;

/// Everything the panel needs in order to draw itself.
///
/// Phase 07 replaces the interior with reads from `SharedState`; the fields and the `draw_panel` signature
/// are what the wiring depends on, so they are named now.
pub struct PanelState {
    /// The seven knob positions, left to right, in knob domain.
    pub knobs: [f32; 7],
    /// The pattern to draw in the step row.
    pub pattern: Arc<Pattern>,
    /// Which slot that pattern came from, shown in the pattern display.
    pub pattern_id: PatternId,
    /// Tempo in BPM, shown in the tempo display.
    pub tempo: f32,
    /// Transport state: drives the position LED.
    pub playing: bool,
    /// Step the transport is on, `0..16`.
    pub position: usize,
    /// Pitch a step takes when a gesture turns a rest into a note.
    pub last_pitch: Pitch,
    /// Octave that goes with `last_pitch`.
    pub last_octave: i8,
    /// A pitch drag in progress: the step, the pitch it started on, and pixels moved so far.
    pub drag: Option<(usize, Pitch, i8, f32)>,
    /// True when something has changed since the last save.
    pub dirty: bool,
    /// The VOLUME knob, kept separate from the seven tone knobs.
    /// True once the knobs have been read from `SharedState` for the first time.
    pub knobs_synced: bool,
    /// The MIDI status line, right-aligned in the top strip.
    pub midi_status: String,
    /// True when the supervisor has given up, which shows the REPROBE target.
    pub midi_given_up: bool,
    /// Set by the panel when REPROBE is clicked; the app clears it.
    pub reprobe_requested: bool,
    /// True while the help card is showing.
    pub help_visible: bool,
    /// The step the keyboard edits; always in `0..16`, never saved.
    pub selected: usize,
    /// The output devices on offer while the picker is open, `None` when it is closed.
    pub device_list: Option<Vec<String>>,
    /// The device the user chose; consumed by `main` on the next frame.
    pub device_choice: Option<String>,
    pub volume: f32,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            knobs: [0.5; 7],
            pattern: Arc::new(Pattern::empty()),
            pattern_id: PatternId { bank: 0, number: 0 },
            tempo: DEFAULT_BPM,
            playing: false,
            position: 0,
            last_pitch: acid_core::pitch::Pitch::C,
            last_octave: 0,
            drag: None,
            dirty: false,
            volume: acid_core::engine::DEFAULT_VOLUME_KNOB,
            knobs_synced: false,
            midi_status: "MIDI: none (retrying)".to_string(),
            midi_given_up: false,
            reprobe_requested: false,
            help_visible: false,
            selected: 0,
            device_list: None,
            device_choice: None,
        }
    }
}

/// Paint the whole panel into `ui`: face, displays, knobs, step keys, switches and transport legends.
///
/// Painting uses absolute rects from `layout`, so the panel lands in the same place whatever the Ui's own
/// cursor is doing. Returns nothing: Phase 07 wires the controls to the engine.
pub fn draw_panel(state: &mut PanelState, shared: &SharedState, ui: &mut egui::Ui) {
    let window = ui.max_rect();
    play_computer_keys(shared, ui);
    let l = layout::layout(window);
    let s = layout::scale(window);
    // The pattern is whatever the UI thread last published — one refcount bump, no allocation.
    state.pattern = shared.pattern();
    state.playing = shared.is_playing();
    // Knobs 1..6 round-trip exactly, so reading them back every frame is harmless. The WAVEFORM knob does
    // not: it maps a continuous position onto a two-state switch, so re-deriving it from the switch every
    // frame would pin it at the extreme and a drag could never cross the midpoint. It is read once, at the
    // first frame (which also picks up a restored session), and then the hand owns it.
    if !state.knobs_synced {
        state.knobs[0] = match shared.waveform() {
            acid_core::oscillator::Waveform::Saw => 0.0,
            acid_core::oscillator::Waveform::Square => 1.0,
        };
        state.knobs_synced = true;
    }
    state.knobs[1] = shared.tuning.get();
    state.knobs[2] = shared.cutoff.get();
    state.knobs[3] = shared.resonance.get();
    state.knobs[4] = shared.env_mod.get();
    state.knobs[5] = shared.decay.get();
    state.knobs[6] = shared.accent.get();
    state.tempo = shared.tempo_bpm.get();
    state.position = shared.position.load(std::sync::atomic::Ordering::Relaxed);
    state.volume = shared.volume.get();
    panel::paint_panel(ui.painter(), &l);

    display::paint_display(
        ui.painter(),
        panel::pattern_display_rect(l.top),
        &display::format_pattern(state.pattern_id),
        s,
    );
    display::paint_display(
        ui.painter(),
        panel::tempo_display_rect(l.top),
        &display::format_tempo(state.tempo),
        s,
    );

    // MIDI status: right-aligned between the wordmark and the pattern display, amber when the app has
    // stopped probing and a small REPROBE target appears with it.
    let status = egui::Rect::from_min_max(
        egui::Pos2::new(l.top.left() + l.top.width() * 0.42, l.top.top()),
        egui::Pos2::new(
            panel::pattern_display_rect(l.top).left() - theme::GAP * s,
            l.top.bottom(),
        ),
    );
    ui.painter().text(
        status.right_center(),
        egui::Align2::RIGHT_CENTER,
        &state.midi_status,
        theme::legend(11.0 * s),
        if state.midi_given_up {
            theme::LED_ACCENT
        } else {
            theme::SILKSCREEN_DIM
        },
    );
    if state.midi_given_up {
        let dot = egui::Rect::from_center_size(
            egui::Pos2::new(status.left() + 8.0 * s, status.center().y),
            egui::vec2(14.0 * s, 14.0 * s),
        );
        let response = ui.interact(
            dot,
            ui.scope_id().with("midi-reprobe"),
            egui::Sense::click(),
        );
        ui.painter()
            .circle_filled(dot.center(), 4.5 * s, theme::LED_ACCENT);
        if response.clicked() {
            state.reprobe_requested = true;
        }
    }

    for (i, cell) in l.knobs.cells.iter().enumerate() {
        let diameter = cell.width().min(cell.height() * 0.78);
        let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(cell.shrink(2.0 * s)));
        cell_ui.set_clip_rect(*cell);
        let before = state.knobs[i];
        knob::knob(
            &mut cell_ui,
            egui::Id::unique(("knob", i)),
            theme::KNOB_LABELS[i],
            &mut state.knobs[i],
            diameter,
        );
        // Publish straight away: the audio thread's next block picks it up, so the move is heard now.
        match i {
            0 => shared.set_waveform(waveform_from_knob(state.knobs[0])),
            1 => shared.tuning.set(state.knobs[1]),
            2 => shared.cutoff.set(state.knobs[2]),
            3 => shared.resonance.set(state.knobs[3]),
            4 => shared.env_mod.set(state.knobs[4]),
            5 => shared.decay.set(state.knobs[5]),
            6 => shared.accent.set(state.knobs[6]),
            _ => {}
        }
        if state.knobs[i] != before {
            state.dirty = true;
        }
    }

    let keys = layout::step_cells(l.steps);
    for (i, cell) in keys.iter().enumerate() {
        let step = state.pattern.steps[i % STEPS_PER_PATTERN];
        let view = step_key::step_view(
            &step,
            i,
            state.position,
            state.playing,
            i % STEPS_PER_PATTERN == state.selected,
            state.drag.is_some_and(|(dragged, ..)| dragged == i),
        );
        let response = step_key::step_key(ui, i, &step, &view, *cell);
        let (shift, alt, ctrl) =
            ui.input(|i| (i.modifiers.shift, i.modifiers.alt, i.modifiers.ctrl));
        // Every click moves the selection, editing or not: the outline must always say where the keyboard
        // and the next gesture will land.
        if response.clicked() || response.drag_started() {
            state.selected = i % STEPS_PER_PATTERN;
        }
        let gesture = edit::gesture_from_input(
            response.clicked(),
            shift,
            alt,
            ctrl,
            response.double_clicked(),
        );
        if let Some(gesture) = gesture
            && gesture != edit::StepGesture::Select
        {
            let edited = edit::apply_gesture(step, gesture, state.last_pitch, state.last_octave);
            if edited.on {
                state.last_pitch = edited.pitch;
                state.last_octave = edited.octave_offset;
            }
            shared.set_step(i % STEPS_PER_PATTERN, edited);
            state.pattern = shared.pattern();
            state.dirty = true;
        }

        // Drag changes the pitch. The movement is accumulated from where the press started, so reversing
        // the drag returns to the original note instead of walking further away. Dragging a rest brings it in
        // at the dragged pitch: with ctrl+click being the only other way to make a note, this is how a fresh
        // note gets its pitch from the mouse.
        if response.dragged() {
            let (index, start_pitch, start_octave, moved) = match state.drag {
                Some((dragged, pitch, octave, moved)) if dragged == i => {
                    (dragged, pitch, octave, moved)
                }
                _ => (i, step.pitch, step.octave_offset, 0.0),
            };
            let moved = moved + response.drag_delta().y;
            state.drag = Some((index, start_pitch, start_octave, moved));
            let (pitch, octave) = edit::drag_pitch((start_pitch, start_octave), moved, shift);
            let dragged = Step {
                on: true,
                pitch,
                octave_offset: octave,
                ..step
            };
            state.last_pitch = pitch;
            state.last_octave = octave;
            shared.set_step(index, dragged);
            state.pattern = shared.pattern();
            state.dirty = true;
            ui.painter().text(
                cell.center(),
                egui::Align2::CENTER_CENTER,
                edit::pitch_name(pitch, octave),
                theme::legend(10.0 * s),
                theme::SILKSCREEN,
            );
        } else {
            state.drag = None;
        }
    }

    // Switches are engraved legends for now; Phase 07 (Task 33) turns them into controls.
    for (label, cell) in ["BANK", "PATTERN", "MODE"]
        .iter()
        .zip(layout::knob_cells(l.switches, 3))
    {
        paint_switch(ui.painter(), cell, s, label);
    }
    // Transport: RUN and STOP are lit keys; the tempo buttons nudge one BPM at a time.
    let cells = layout::knob_cells(l.transport, transport::TRANSPORT_CELLS);
    if let [run, stop, tempo_down, tempo_up, volume_cell] = cells[..] {
        let playing = shared.is_playing();
        if transport::transport_button(ui, "run", run, "RUN", playing).clicked() {
            shared.set_playing(true);
            state.dirty = true;
        }
        if transport::transport_button(ui, "stop", stop, "STOP", !playing).clicked() {
            shared.set_playing(false);
            state.dirty = true;
        }
        let bpm = shared.tempo_bpm.get();
        if transport::transport_button(ui, "tempo-down", tempo_down, "TEMPO -", false).clicked() {
            shared.tempo_bpm.set(transport::nudge_tempo(bpm, -1));
            state.dirty = true;
        }
        if transport::transport_button(ui, "tempo-up", tempo_up, "TEMPO +", false).clicked() {
            shared.tempo_bpm.set(transport::nudge_tempo(bpm, 1));
            state.dirty = true;
        }

        // VOLUME lives in the last cell, sized to the row rather than to the tone knobs: a two-thirds-scale
        // knob would punch through the switches row and hide the MODE legend. The caption sits beside it.
        let diameter = (volume_cell.height() * 0.70).min(volume_cell.width() * 0.3);
        let knob_rect = egui::Rect::from_min_size(
            egui::Pos2::new(volume_cell.left() + 4.0 * s, volume_cell.top()),
            egui::vec2(diameter, volume_cell.height()),
        );
        let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(knob_rect));
        cell_ui.set_clip_rect(knob_rect);
        let before = state.volume;
        knob::knob(
            &mut cell_ui,
            egui::Id::unique(("knob", 7)),
            "",
            &mut state.volume,
            diameter,
        );
        ui.painter().text(
            egui::Pos2::new(knob_rect.right() + 6.0 * s, volume_cell.center().y),
            egui::Align2::LEFT_CENTER,
            "VOLUME",
            theme::legend(11.0 * s),
            theme::SILKSCREEN,
        );
        if state.volume != before {
            shared.volume.set(state.volume);
            state.dirty = true;
        }
    }

    // A small `?` at the end of the wordmark opens the help card, and the card covers the panel while it is
    // showing — it is the only thing that needs no explanation of its own.
    let help_dot = egui::Rect::from_center_size(
        egui::Pos2::new(
            panel::brand_rect(l.top).right() - 10.0 * s,
            l.top.center().y,
        ),
        egui::vec2(18.0 * s, 18.0 * s),
    );
    let help_response = ui.interact(
        help_dot,
        ui.scope_id().with("help-toggle"),
        egui::Sense::click(),
    );
    ui.painter()
        .circle_stroke(help_dot.center(), 8.0 * s, theme::silkscreen_stroke(s));
    ui.painter().text(
        help_dot.center(),
        egui::Align2::CENTER_CENTER,
        "?",
        theme::legend(11.0 * s),
        theme::SILKSCREEN,
    );
    // The card must not re-close on the click that opened it, so dismissal reads the frame-start value.
    let help_was_visible = state.help_visible;
    if help_response.clicked() && !help_was_visible {
        state.help_visible = true;
    }
    if help_was_visible {
        help::paint_help(ui.painter(), l.panel, s);
        // The card is modal while it is up: a click anywhere closes it, and the shield keeps that click
        // from also pressing whatever sits underneath.
        let shield = ui.interact(
            window,
            ui.scope_id().with("help-shield"),
            egui::Sense::click(),
        );
        if shield.clicked() {
            state.help_visible = false;
        }
    }
}

/// The computer keyboard plays through the same route as a MIDI keyboard, so both share one voice.
///
/// A held key keeps the gate open; the gate closes when every mapped key is up. Releasing a computer key
/// therefore also ends a MIDI note that is sounding — one voice, one gate.
fn play_computer_keys(shared: &SharedState, ui: &egui::Ui) {
    use std::sync::atomic::Ordering;

    let octave = shared.octave.load(Ordering::Relaxed);
    ui.input(|input| {
        // One pass over the frame's freshly pressed keys: a held key must not re-gate the voice on every
        // auto-repeat, and Z/X move the octave the same way.
        for key in keys::fresh_presses(&input.events) {
            if let Some(pitch) = keys::pitch_for(key) {
                shared.midi_hz.set(keys::key_hz(pitch, octave));
                shared.midi_accent.store(false, Ordering::Relaxed);
                shared.midi_gate.store(true, Ordering::Relaxed);
            }
            if let Some(delta) = keys::octave_key(key) {
                let next = (octave + i32::from(delta)).clamp(-2, 2);
                shared.octave.store(next, Ordering::Relaxed);
            }
        }
        if !keys::KEY_MAP.iter().any(|(key, _)| input.key_down(*key)) {
            shared.midi_gate.store(false, Ordering::Relaxed);
        }
    });
}

/// A static button face: raised, engraved, with a silkscreen caption when `label` is not empty.
fn paint_switch(painter: &egui::Painter, rect: egui::Rect, scale: f32, label: &str) {
    painter.rect_filled(rect, 4.0 * scale, theme::PANEL_RAISED);
    painter.rect_stroke(
        rect,
        4.0 * scale,
        theme::silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );
    if !label.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            theme::legend(12.0 * scale),
            theme::SILKSCREEN,
        );
    }
}

/// The WAVEFORM knob is a two-position switch drawn as a knob: past the midpoint is the square wave.
pub fn waveform_from_knob(knob: f32) -> acid_core::oscillator::Waveform {
    if knob >= 0.5 {
        acid_core::oscillator::Waveform::Square
    } else {
        acid_core::oscillator::Waveform::Saw
    }
}
