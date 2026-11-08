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
pub mod randomise;
pub mod save;
pub mod scale;
pub mod selection;
pub mod shortcuts;
pub mod songs;
pub mod step_key;
pub mod switches;
pub mod tips;

/// Where the TIPS key sits: in the top strip, to the left of the SAVED / UNSAVED lamp.
///
/// Named here rather than inside `draw_panel` so the live test can click the key without guessing its pixels.
pub fn tips_rect(window: egui::Rect) -> egui::Rect {
    let scale = layout::scale(window);
    let lamp = panel::save_indicator_rect(layout::layout(window).top, scale);
    egui::Rect::from_center_size(
        lamp.left_center() - egui::vec2(40.0 * scale, 0.0),
        egui::vec2(64.0 * scale, 18.0 * scale),
    )
}
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
    /// The VOLUME knob, kept separate from the seven tone knobs.
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
    /// Step mode: a note typed on a keyboard writes the selected step and walks to the next one.
    pub step_mode: bool,
    /// The keys that have written a step and not been released yet. A press for a key that is already here is a repeat
    /// however egui labelled it, so it writes nothing; the release is what walks the border on.
    pub placed: Vec<egui::Key>,
    /// Whether hovering a knob or a button explains it. On unless the player turns it off with the TIPS key.
    pub tooltips: bool,
    /// How full a randomise fills the pattern, `0.0..=1.0`.
    pub density: f32,
    /// The scale a randomise draws from.
    pub scale: scale::Scale,
    /// The SCALE knob's own position: the hand owns it while it is on the knob, and the panel snaps the scale from it
    /// when the hand lets go. Anything that sets `scale` from outside — a song load, a restart — must set this too, or
    /// the next frame snaps the scale back to where the knob was; [`PanelState::set_scale`] is the way to do it.
    pub scale_knob: f32,
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
            step_mode: false,
            placed: Vec::new(),
            tooltips: true,
            density: randomise::DEFAULT_DENSITY,
            scale: scale::Scale::default(),
            scale_knob: scale::scale_position(scale::Scale::default()),
        }
    }
}

impl PanelState {
    /// Adopt a scale and put the knob where it belongs.
    ///
    /// Two fields, one setting: while the hand is off the SCALE knob the panel snaps the scale from the knob's position,
    /// so setting one without the other would be undone on the next frame.
    pub fn set_scale(&mut self, scale: scale::Scale) {
        self.scale = scale;
        self.scale_knob = scale::scale_position(scale);
    }

    /// Adopt a randomise density, clamped to the knob's travel.
    pub fn set_density(&mut self, density: f32) {
        self.density = density.clamp(0.0, 1.0);
    }
}

/// Paint the whole panel into `ui`: face, displays, knobs, step keys, switches and transport legends.
///
/// Painting uses absolute rects from `layout`, so the panel lands in the same place whatever the Ui's own
/// cursor is doing. Returns nothing: Phase 07 wires the controls to the engine.
pub fn draw_panel(state: &mut PanelState, shared: &SharedState, ui: &mut egui::Ui) {
    let window = ui.max_rect();
    play_computer_keys(shared, ui, state);

    // Every MIDI note still waiting while the panel is editing (step mode, or shift held) belongs to the selected
    // step. The queue is drained rather than peeked: two notes inside one frame are two steps, each at its own
    // velocity — one slot and a counter used to lose the first of them.
    let editing = state.step_mode || ui.input(|input| input.modifiers.shift);
    while let Some(waiting) = shared.notes.pop() {
        if !editing {
            continue;
        }
        let (pitch, octave) = edit::from_midi_note(waiting.note);
        crate::trace::note("place midi", || {
            format!(
                "note {} velocity {} -> {pitch:?} {octave} on step {}",
                waiting.note,
                waiting.velocity,
                state.selected + 1
            )
        });
        selection::place_note(state, shared, pitch, octave, Some(waiting.velocity));
        // A MIDI keyboard's release never reaches this queue, so its note walks here, on arrival: one note, one step.
        if state.step_mode {
            selection::walk(state);
        }
    }
    let l = layout::layout(window);
    let s = layout::scale(window);
    // The pattern is whatever the UI thread last published — one refcount bump, no allocation.
    state.pattern = shared.pattern();
    state.playing = shared.is_playing();
    // Knobs 1..6 round-trip exactly, so reading them back every frame is harmless. The WAVEFORM knob is a switch: its
    // position is worked out where the knob is drawn, because only there is it known whether the hand or a learned
    // controller owns it.
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
    // Both readouts can be taught while a learn is choosing: click PATTERN and the next controller picks the pattern,
    // click the BPM and the next controller sets the tempo.
    learn_pick(
        ui,
        state,
        shared,
        "learn-tempo",
        panel::tempo_display_rect(l.top),
        midi::cc::CcTarget::Tempo,
    );
    learn_pick(
        ui,
        state,
        shared,
        "learn-pattern",
        switches::cell(l.switches, switches::Switch::Pattern),
        midi::cc::CcTarget::Pattern,
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
        // The WAVEFORM knob is a switch, not a dial: its caption names the wave it is on, and its value is snapped to
        // the nearest detent after the hand lets go, so it can never rest between two waves. The caption is read before
        // the knob borrows its value.
        let label = if i == 0 {
            waveform_from_position(state.knobs[0]).name()
        } else {
            theme::KNOB_LABELS[i]
        };
        let target = if choosing {
            &mut scratch
        } else {
            &mut state.knobs[i]
        };
        let response = knob::knob(
            &mut cell_ui,
            egui::Id::unique(("knob", i)),
            label,
            target,
            diameter,
        );
        tips::hover(&response, state, tips::Control::knob(i));
        if i == 0 && !choosing {
            waveform_detents(cell_ui.painter(), *cell, state.knobs[0], s);
        }
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
        //
        // The switch has two masters: the hand and a learned controller. While the hand is on the knob it wins; when it
        // is not, the shared value is the truth, so a fader moves the face. Snapping happens on release, never mid-drag —
        // a frame-by-frame snap pulls the position back onto the detent it started from, and a slow drag could never
        // cross a midpoint.
        let switch_moved = i == 0 && state.knobs[0] != before;
        match i {
            0 => {
                if response.dragged() {
                    shared.set_waveform(waveform_from_position(state.knobs[0]));
                } else {
                    let hand = waveform_from_position(state.knobs[0]);
                    // The hand's own move wins: the wheel, or a drag that has just ended. Otherwise the switch is where
                    // the shared state says, which is where a learned controller put it.
                    let face = if switch_moved || shared.waveform() == hand {
                        hand
                    } else {
                        shared.waveform()
                    };
                    state.knobs[0] = waveform_position(face);
                    shared.set_waveform(face);
                }
            }
            1 => shared.tuning.set(state.knobs[1]),
            2 => shared.cutoff.set(state.knobs[2]),
            3 => shared.resonance.set(state.knobs[3]),
            4 => shared.env_mod.set(state.knobs[4]),
            5 => shared.decay.set(state.knobs[5]),
            6 => shared.accent.set(state.knobs[6]),
            _ => {}
        }
        // Only the hand marks the session unsaved: the switch adopting a learned controller's value is the panel
        // catching up, not an edit the player made.
        let moved = if i == 0 {
            switch_moved
        } else {
            state.knobs[i] != before
        };
        if moved {
            state.dirty = true;
        }
    }

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

    // BANK, PATTERN and MODE are engraving for now; CLEAR empties the pattern on screen (the one action here with no
    // undo, so it takes two clicks — see `ui::clear`), SAVE writes the session, and the rest are their own modules.
    // Every key comes from `ui::switches`, so adding one does not mean renumbering anything.
    for switch in switches::Switch::ALL.iter().filter(|s| !s.is_live()) {
        paint_switch(
            ui.painter(),
            switches::cell(l.switches, *switch),
            s,
            switch.label(),
        );
    }
    {
        let clear_cell = switches::cell(l.switches, switches::Switch::Clear);
        let now = std::time::Instant::now();
        let armed = clear::is_armed(state.clear_armed_at, now);
        let label = if armed { "CLEAR?" } else { "CLEAR" };
        if switch_key(
            ui,
            state,
            "clear-pattern",
            clear_cell,
            label,
            armed,
            tips::Control::Clear,
        ) {
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
    {
        let save_cell = switches::cell(l.switches, switches::Switch::Save);
        // Lit while something is unsaved, so the key doubles as the "there is work to keep" lamp.
        let dirty = state.dirty;
        if switch_key(
            ui,
            state,
            "save-session",
            save_cell,
            "SAVE",
            dirty,
            tips::Control::Save,
        ) {
            state.save_requested = true;
        }
    }
    {
        let songs_cell = switches::cell(l.switches, switches::Switch::Songs);
        // The card lists what is on disk, so opening it reads the songs directory once.
        if switch_key(
            ui,
            state,
            "songs",
            songs_cell,
            "SONGS",
            state.songs.is_some(),
            tips::Control::Songs,
        ) {
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
    {
        let step_cell = switches::cell(l.switches, switches::Switch::Step);
        // STEP mode: a note typed on a keyboard writes the selected step, then walks to the next one.
        if switch_key(
            ui,
            state,
            "step-mode",
            step_cell,
            "STEP",
            state.step_mode,
            tips::Control::Step,
        ) {
            state.step_mode = !state.step_mode;
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
    // TIPS: a small key that turns the hover help on and off. It carries its own help, so a player who turned the help
    // off can always find out what the key does.
    let tips_rect = tips_rect(window);
    let tips_label = if state.tooltips {
        "TIPS ON"
    } else {
        "TIPS OFF"
    };
    if switch_key(
        ui,
        state,
        "tips",
        tips_rect,
        tips_label,
        state.tooltips,
        tips::Control::Tooltips,
    ) {
        state.tooltips = !state.tooltips;
        state.dirty = true;
    }
    // Transport: RUN and STOP are lit keys; the tempo buttons nudge one BPM at a time.
    let cells = layout::knob_cells(l.transport, transport::TRANSPORT_CELLS);
    if let [run, stop, tempo_down, tempo_up, volume_cell] = cells[..] {
        let playing = shared.is_playing();
        if switch_key(ui, state, "run", run, "RUN", playing, tips::Control::Run) {
            shared.set_playing(true);
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
            shared.set_playing(false);
            state.dirty = true;
        }
        let bpm = shared.tempo_bpm.get();
        if switch_key(
            ui,
            state,
            "tempo-down",
            tempo_down,
            "TEMPO -",
            false,
            tips::Control::TempoDown,
        ) {
            shared.tempo_bpm.set(transport::nudge_tempo(bpm, -1));
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
            shared.tempo_bpm.set(transport::nudge_tempo(bpm, 1));
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
/// A held key keeps the gate open; the gate closes when every mapped key is up. Releasing a computer key therefore
/// also ends a MIDI note that is sounding — one voice, one gate.
///
/// With shift held, or in step mode, a key is an *edit*: the note goes into the selected step instead of the voice.
///
/// One press writes one step, and step mode walks the border on the key **release**: a held key cannot fill the pattern
/// at its repeat rate, and a press the platform repeats (which egui may not flag) writes nothing because the key is
/// already in [`PanelState::placed`].
fn play_computer_keys(shared: &SharedState, ui: &egui::Ui, state: &mut PanelState) {
    use std::sync::atomic::Ordering;

    let octave = shared.octave.load(Ordering::Relaxed);
    ui.input(|input| {
        // A window that lost the focus will never deliver the releases for what was held, so forget them rather than
        // leave a key wedged down forever.
        if !input.focused {
            state.placed.clear();
        }
        for action in keys::key_actions(&input.events) {
            match action {
                keys::KeyAction::Press { key, shift } => {
                    let editing = state.step_mode || shift;
                    // Only a key that *writes a step* is remembered. Every other key (arrows, Z/X, anything else) must be
                    // left alone: its press is replied to elsewhere, and its release would otherwise walk the border —
                    // pressing left walked back one step and the release immediately walked forward again.
                    let writes = keys::pitch_for(key).is_some() || key == eframe::egui::Key::Backspace;
                    // A key already down is a repeat, whatever egui calls it: it writes nothing and walks nothing.
                    let fresh = writes && keys::first_press(&mut state.placed, key);
                    if let Some(pitch) = keys::pitch_for(key) {
                        if editing {
                            if fresh {
                                // The octave is the keyboard's, and `transpose` canonicalises it into the panel's
                                // range by moving whole octaves — the pitch class always survives, which is what a
                                // round trip through a MIDI number used to lose at the ends of the octave range.
                                let (step_pitch, step_octave) = edit::from_keyboard(pitch, octave);
                                crate::trace::note("place key", || {
                                    format!(
                                        "{key:?} at octave {octave} -> {step_pitch:?} {step_octave} on step {} (t={:.3})",
                                        state.selected + 1,
                                        input.time
                                    )
                                });
                                selection::place_note(state, shared, step_pitch, step_octave, None);
                            }
                        } else {
                            shared.midi_hz.set(keys::key_hz(pitch, octave));
                            shared.midi_accent.store(false, Ordering::Relaxed);
                            shared.midi_gate.store(true, Ordering::Relaxed);
                        }
                    }
                    // Backspace empties the selected step to a rest while editing: the keyboard's counterpart to
                    // clicking a step with the modifier that clears it. Plain Backspace is not ours.
                    if editing && key == eframe::egui::Key::Backspace && fresh {
                        selection::rest_step(state, shared);
                    }
                    if let Some(delta) = keys::octave_key(key) {
                        let next = (octave + i32::from(delta)).clamp(-2, 2);
                        shared.octave.store(next, Ordering::Relaxed);
                    }
                }
                keys::KeyAction::Release { key } => {
                    // Only a key this panel wrote with walks on: an auditioned note is not an edit, and a release with
                    // no press behind it (a lost frame, a stuck key) moves nothing.
                    if keys::release(&mut state.placed, key) && state.step_mode {
                        selection::walk(state);
                    }
                }
            }
        }
        if !keys::KEY_MAP.iter().any(|(key, _)| input.key_down(*key)) {
            shared.midi_gate.store(false, Ordering::Relaxed);
        }
    });
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

/// A click on a control that can be taught, while a learn is choosing.
///
/// Guarded on `Choosing` rather than on the click alone: outside a learn these are the player's controls, and a stray
/// click on the tempo readout must not start a binding that then swallows the next controller they touch.
fn learn_pick(
    ui: &mut egui::Ui,
    state: &mut PanelState,
    shared: &SharedState,
    id: &str,
    rect: egui::Rect,
    control: midi::cc::CcTarget,
) {
    if state.learn != learn::Step::Choosing {
        return;
    }
    let hit = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click());
    if hit.clicked() {
        state.learn = learn::choose(control, shared);
    }
}
/// A panel button with its hover help attached: draw it, explain it, and say whether it was clicked.
///
/// One helper for every button, so the tip follows the control that was actually drawn — the CLEAR key reads `CLEAR?`
/// while it is armed, and its help still describes CLEAR.
fn switch_key(
    ui: &mut egui::Ui,
    state: &PanelState,
    id: &str,
    rect: egui::Rect,
    label: &str,
    lit: bool,
    control: tips::Control,
) -> bool {
    let response = transport::transport_button(ui, id, rect, label, lit);
    let clicked = response.clicked();
    tips::hover(&response, state, control);
    clicked
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

/// The control position of a wave: `0.0` is the first entry of the catalogue, `1.0` the last.
pub fn waveform_position(wave: acid_core::oscillator::Waveform) -> f32 {
    // The arithmetic lives in `acid_core`, because a learned controller snaps with the same rule and the MIDI thread
    // may not borrow the panel's copy.
    wave.position()
}

/// The wave nearest a control position. Rounded, not floored, so the detents divide the travel evenly and the knob can
/// never rest between two waves.
pub fn waveform_from_position(position: f32) -> acid_core::oscillator::Waveform {
    acid_core::oscillator::Waveform::from_position(position)
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
