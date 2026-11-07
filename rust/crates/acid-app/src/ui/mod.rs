// The palette and layout constants are the panel's vocabulary: they are introduced here and consumed
// from Phase 06 onwards, so the dead-code lint is waived for the module rather than per constant.
pub mod clear;
pub mod dispatch;
pub mod display;
pub mod edit;
pub mod help;
pub mod keys;
pub mod knob;
pub mod layout;
pub mod learn;
pub mod panel;
pub mod picker;
pub mod save;
pub mod selection;
pub mod shortcuts;
pub mod songs;
pub mod step_key;
#[allow(dead_code)]
pub mod theme;
pub mod transport;

use std::sync::Arc;

use acid_core::pattern::{Pattern, PatternId, STEPS_PER_PATTERN, Step};
use acid_core::timing::DEFAULT_BPM;

use crate::midi;
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
    /// Set by the panel when REPROBE is clicked; the app clears it.
    pub reprobe_requested: bool,
    /// The MIDI inputs on offer while the picker is open, `None` when it is closed.
    pub midi_ports: Option<Vec<String>>,
    /// The port the user chose; consumed by `main` on the next frame.
    pub midi_port_choice: Option<midi::ports::PortChoice>,
    /// True while the help card is showing.
    pub help_visible: bool,
    /// The step the keyboard edits; always in `0..16`, never saved.
    pub selected: usize,
    /// The output devices on offer while the picker is open, `None` when it is closed.
    pub device_list: Option<Vec<String>>,
    /// The device the user chose; consumed by `main` on the next frame.
    pub device_choice: Option<String>,
    pub volume: f32,
    /// When CLEAR was first clicked: a second click inside the window empties the pattern on screen.
    pub clear_armed_at: Option<std::time::Instant>,
    /// Set by the SAVE key; consumed by `main`, which writes the session and clears `dirty`.
    pub save_requested: bool,
    /// The songs card while it is open, `None` when it is closed.
    pub songs: Option<songs::Songs>,
    /// What the songs card asked for; consumed by `main`, which owns the files.
    pub song_request: Option<songs::Request>,
    /// The song this session is working on, once it has a name.
    pub song_name: Option<String>,
    /// Where the MIDI-learn flow is: idle, waiting for a knob, or waiting for a controller.
    pub learn: learn::Step,
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
            midi_ports: None,
            midi_port_choice: None,
            help_visible: false,
            selected: 0,
            device_list: None,
            device_choice: None,
            clear_armed_at: None,
            save_requested: false,
            songs: None,
            song_request: None,
            song_name: None,
            learn: learn::Step::Idle,
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
    let status = panel::midi_status_rect(l.top, s);
    ui.painter().text(
        status.right_center(),
        egui::Align2::RIGHT_CENTER,
        // While learning, the status line says what the panel is waiting for — it is the only free line on the face.
        learn::hint(state.learn).unwrap_or(&state.midi_status),
        theme::legend(11.0 * s),
        if state.learn != learn::Step::Idle || state.midi_given_up {
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
        // While choosing a knob to teach, the knobs stand back: a click picks the knob instead of turning it, and
        // the turn goes into a scratch value so a stray drag cannot move the patch either.
        let choosing = state.learn == learn::Step::Choosing;
        let mut scratch = state.knobs[i];
        let target = if choosing {
            &mut scratch
        } else {
            &mut state.knobs[i]
        };
        knob::knob(
            &mut cell_ui,
            egui::Id::unique(("knob", i)),
            theme::KNOB_LABELS[i],
            target,
            diameter,
        );
        if choosing {
            let hit = ui.interact(
                *cell,
                ui.scope_id().with(("learn-knob", i)),
                egui::Sense::click(),
            );
            if hit.clicked()
                && let Some(control) = learn::control_for_knob(i)
            {
                state.learn = learn::choose(control, shared);
            }
            continue;
        }
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

    // VOLUME has its own corner of the top strip, before the wordmark, so the transport row keeps its five keys.
    let volume_rect = panel::volume_knob_rect(l.top);
    let mut volume_ui = ui.new_child(egui::UiBuilder::new().max_rect(volume_rect));
    volume_ui.set_clip_rect(volume_rect);
    let volume_before = state.volume;
    let choosing = state.learn == learn::Step::Choosing;
    let mut volume_scratch = state.volume;
    let volume_target = if choosing {
        &mut volume_scratch
    } else {
        &mut state.volume
    };
    knob::knob(
        &mut volume_ui,
        egui::Id::unique(("knob", 7)),
        "",
        volume_target,
        volume_rect.width(),
    );
    if choosing {
        if ui
            .interact(
                volume_rect,
                ui.scope_id().with("learn-volume"),
                egui::Sense::click(),
            )
            .clicked()
        {
            state.learn = learn::choose(midi::cc::CcTarget::Volume, shared);
        }
    } else if state.volume != volume_before {
        shared.volume.set(state.volume);
        state.dirty = true;
    }

    // A controller arrived while listening: bind it to the chosen control and stop listening.
    if let learn::Step::Listening(control) = state.learn
        && let Some(cc) = learn::heard(shared)
    {
        midi::cc::set_cc(shared, control, cc);
        learn::cancel(shared);
        state.learn = learn::Step::Idle;
        state.dirty = true;
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
            // A slide only glides into a step that sounds, so the keycap needs to know about the next one.
            state.pattern.steps[(i + 1) % STEPS_PER_PATTERN].on,
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
            // An edit makes a pending CLEAR stale: it was armed against a pattern that no longer exists.
            clear::disarm(state);
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
            clear::disarm(state);
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

    // BANK, PATTERN and MODE are engraving for now; CLEAR empties the pattern on screen (the one action here
    // with no undo, so it takes two clicks — see `ui::clear`) and SAVE writes the session now.
    let switch_cells = layout::knob_cells(l.switches, 6);
    for (label, cell) in ["BANK", "PATTERN", "MODE"].iter().zip(switch_cells.iter()) {
        paint_switch(ui.painter(), *cell, s, label);
    }
    if let Some(&clear_cell) = switch_cells.get(3) {
        let now = std::time::Instant::now();
        let armed = clear::is_armed(state.clear_armed_at, now);
        let label = if armed { "CLEAR?" } else { "CLEAR" };
        if transport::transport_button(ui, "clear-pattern", clear_cell, label, armed).clicked() {
            match clear::click(state.clear_armed_at, now) {
                clear::Click::Armed => state.clear_armed_at = Some(now),
                clear::Click::Cleared => {
                    clear::disarm(state);
                    let empty = Arc::new(Pattern::empty());
                    shared.set_pattern(Arc::clone(&empty));
                    state.pattern = empty;
                    state.dirty = true;
                }
            }
        }
    }
    if let Some(&save_cell) = switch_cells.get(4) {
        // Lit while something is unsaved, so the key doubles as the "there is work to keep" lamp.
        let dirty = state.dirty;
        if transport::transport_button(ui, "save-session", save_cell, "SAVE", dirty).clicked() {
            state.save_requested = true;
        }
    }
    if let Some(&songs_cell) = switch_cells.get(5) {
        // The card lists what is on disk, so opening it reads the songs directory once.
        if transport::transport_button(ui, "songs", songs_cell, "SONGS", state.songs.is_some())
            .clicked()
        {
            state.songs = match state.songs {
                Some(_) => None,
                None => Some(songs::Songs {
                    names: crate::persist::list_songs(&crate::persist::data_dir()),
                    name: state.song_name.clone().unwrap_or_default(),
                    open: state.song_name.clone(),
                    ..songs::Songs::default()
                }),
            };
        }
    }
    // The top strip says which of the two it is; `SAVED` in dim silk, `UNSAVED` in amber.
    let (save_label, save_colour) = save::state_label(state.dirty);
    ui.painter().text(
        panel::save_indicator_rect(l.top, s).right_center(),
        egui::Align2::RIGHT_CENTER,
        save_label,
        theme::legend(10.0 * s),
        save_colour,
    );
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

        // The last transport cell is the MIDI LEARN key (see `ui::learn`), so the row keeps its five keys while
        // the volume knob lives in the top strip's corner.
        if transport::transport_button(
            ui,
            "learn",
            volume_cell,
            learn::label(state.learn),
            state.learn != learn::Step::Idle,
        )
        .clicked()
        {
            state.learn = match state.learn {
                learn::Step::Idle => learn::Step::Choosing,
                _ => learn::Step::Idle,
            };
            learn::cancel(shared);
        }
    }

    // MIDI input picker: the status line is the handle. Rows paint here, after everything else, so they sit
    // over the panel; the REPROBE dot keeps the left edge of the line while the supervisor has given up.
    let mut handle = status;
    if state.midi_given_up {
        handle.min.x += 24.0 * s;
    }
    if ui
        .interact(
            handle,
            ui.scope_id().with("midi-status"),
            egui::Sense::click(),
        )
        .clicked()
    {
        state.midi_ports = match state.midi_ports {
            Some(_) => None,
            None => Some(midi::ports::list_ports().unwrap_or_default()),
        };
    }
    if let Some(ports) = state.midi_ports.clone() {
        let mut entries = vec![("any input".to_string(), midi::ports::PortChoice::Any)];
        entries.extend(
            ports
                .iter()
                .map(|name| (name.clone(), midi::ports::PortChoice::Named(name.clone()))),
        );
        let row_height = 20.0 * s;
        let mut rows = Vec::new();
        let mut chosen = None;
        for (index, (label, choice)) in entries.iter().enumerate() {
            let row = egui::Rect::from_min_size(
                egui::Pos2::new(handle.left(), handle.bottom() + row_height * index as f32),
                egui::vec2(handle.width(), row_height),
            );
            rows.push(row);
            ui.painter().rect_filled(row, 2.0 * s, theme::PANEL_RAISED);
            ui.painter().text(
                row.left_center() + egui::vec2(6.0 * s, 0.0),
                egui::Align2::LEFT_CENTER,
                label,
                theme::legend(10.0 * s),
                theme::SILKSCREEN,
            );
            if ui
                .interact(
                    row,
                    ui.scope_id().with(("midi-port", index)),
                    egui::Sense::click(),
                )
                .clicked()
            {
                chosen = Some(choice.clone());
            }
        }
        if let Some(choice) = chosen {
            state.midi_port_choice = Some(choice);
            state.midi_ports = None;
        } else {
            let click = ui.input(|i| {
                if i.pointer.any_click() {
                    i.pointer.interact_pos()
                } else {
                    None
                }
            });
            if picker::should_close(click, &rows, handle) {
                state.midi_ports = None;
            }
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

    // The songs card sits over everything, so its shield is registered before the card's own controls: a click on
    // a row or the text field is theirs, a click anywhere else dismisses.
    if state.songs.is_some() {
        let shield = ui.interact(
            window,
            ui.scope_id().with("songs-shield"),
            egui::Sense::click(),
        );
        let mut request = None;
        if let Some(card) = state.songs.as_mut() {
            request = songs::paint(ui, l.panel, s, card);
        }
        if shield.clicked() {
            request = Some(songs::Request::Close);
        }
        match request {
            Some(songs::Request::Close) => state.songs = None,
            Some(other) => state.song_request = Some(other),
            None => {}
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
