// The palette and layout constants are the panel's vocabulary: they are introduced here and consumed
// from Phase 06 onwards, so the dead-code lint is waived for the module rather than per constant.
pub mod clear;
pub mod controls;
pub mod dispatch;
pub mod display;
pub mod edit;
pub mod help;
pub mod keys;
pub mod knob;
pub mod layout;
pub mod learn;
pub mod panel;
pub mod pattern_swap;
pub mod picker;
pub mod randomise;
pub mod row_knobs;
pub mod row_perform;
pub mod row_transport;
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

/// Where the AUTO key sits: immediately left of TIPS, so the two switches that change how the session behaves read as one
/// pair. Named here for the same reason as [`tips_rect`]: the live test clicks it without guessing pixels.
pub fn autosave_rect(window: egui::Rect) -> egui::Rect {
    let scale = layout::scale(window);
    let tips = tips_rect(window);
    let width = 72.0 * scale;
    egui::Rect::from_min_size(
        egui::Pos2::new(tips.left() - (width + theme::GAP * 2.0 * scale), tips.top()),
        egui::vec2(width, tips.height()),
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

use crate::knobs::{Kind, Knob};

use self::controls::{learn_pick, switch_key};

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
    pub reprobe_requested: bool,
    /// True while the help card is showing.
    pub help_visible: bool,
    /// The step the keyboard edits; always in `0..16`, never saved.
    pub selected: usize,
    /// The port picker: what is on offer and what the player chose.
    pub picker: PickerState,
    /// The VOLUME knob, kept separate from the seven tone knobs.
    pub volume: f32,
    /// When CLEAR was first clicked: a second click inside the window empties the pattern on screen.
    pub clear_armed_at: Option<std::time::Instant>,
    /// Set by the SAVE key; consumed by `main`, which writes the session and clears `dirty`.
    pub save_requested: bool,
    /// The songs card, what it asked for, and the song this session is working on.
    pub songs: SongState,
    /// Where the MIDI-learn flow is: idle, waiting for a knob, or waiting for a controller.
    pub learn: learn::Step,
    /// Step mode: a note typed on a keyboard writes the selected step and walks to the next one.
    pub step_mode: bool,
    /// The keys that have written a step and not been released yet. A press for a key that is already here is a repeat
    /// however egui labelled it, so it writes nothing; the release is what walks the border on.
    pub placed: Vec<egui::Key>,
    /// Whether hovering a knob or a button explains it. On unless the player turns it off with the TIPS key.
    pub tooltips: bool,
    /// Whether the session writes itself when it changes. Off unless the player turns it on with the AUTO key: saving is
    /// manual, and the lamp plus the SAVE key are how a player knows where they stand.
    pub autosave: bool,
    /// Something the panel wants to say — "saved as Acid 2", "could not import: …" — with the moment it was said. It
    /// outlives the card that asked for it, and it clears itself, so a message is never something to dismiss.
    pub notice: Option<(String, std::time::Instant)>,
    /// How full a randomise fills the pattern, `0.0..=1.0`.
    pub density: f32,
    /// The scale a randomise draws from.
    pub scale: scale::Scale,
    /// The SCALE knob's own position: the hand owns it while it is on the knob, and the panel snaps the scale from it
    /// when the hand lets go. Anything that sets `scale` from outside — a song load, a restart — must set this too, or
    /// the next frame snaps the scale back to where the knob was; [`PanelState::set_scale`] is the way to do it.
    pub scale_knob: f32,
}

/// The songs card and everything it asked for.
///
/// One value because a request without a card, or a name for a song nobody opened, is not a state the panel can be in:
/// the card asks, `main` answers, and the answer is cleared with the card.
#[derive(Default)]
pub struct SongState {
    /// The card while it is open, `None` when it is closed.
    pub card: Option<songs::Songs>,
    /// What the card asked for; consumed by `main`, which owns the files.
    pub request: Option<songs::Request>,
    /// The song this session is working on, once it has a name.
    pub name: Option<String>,
    /// The song file this session is saved as, if it is one. A demo arrives with `None`: it is a new song under a name, and
    /// nothing of it is on disk until the player saves it.
    pub file: Option<String>,
}

/// The port picker: the two lists `main` hands the panel, and the two choices it takes back.
///
/// One value for the same reason as [`SongState`]: a choice belongs to the list it was made from, and both are consumed
/// in the same frame.
#[derive(Default)]
pub struct PickerState {
    /// The MIDI inputs on offer while the picker is open, `None` when it is closed.
    pub inputs: Option<Vec<String>>,
    /// The input the user chose; consumed by `main` on the next frame.
    pub input_choice: Option<midi::ports::PortChoice>,
    /// The output devices on offer while the picker is open, `None` when it is closed.
    pub outputs: Option<Vec<String>>,
    /// The output the user chose; consumed by `main` on the next frame.
    pub output_choice: Option<String>,
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
            picker: PickerState::default(),
            help_visible: false,
            selected: 0,
            clear_armed_at: None,
            save_requested: false,
            songs: SongState::default(),
            learn: learn::Step::Idle,
            step_mode: false,
            placed: Vec::new(),
            tooltips: true,
            autosave: false,
            notice: None,
            density: randomise::DEFAULT_DENSITY,
            scale: scale::Scale::default(),
            scale_knob: scale::scale_position(scale::Scale::default()),
        }
    }
}

impl PanelState {
    /// How long a notice stays on the status line before the MIDI status comes back.
    pub const NOTICE_FOR: std::time::Duration = std::time::Duration::from_secs(4);

    /// Say something on the status line. It survives the card that asked for it, and it clears itself.
    pub fn notify(&mut self, message: impl Into<String>) {
        self.notice = Some((message.into(), std::time::Instant::now()));
    }

    /// The notice still worth showing at `now`, if any.
    pub fn notice_now(&self, now: std::time::Instant) -> Option<&str> {
        self.notice
            .as_ref()
            .filter(|(_, at)| now.duration_since(*at) < Self::NOTICE_FOR)
            .map(|(text, _)| text.as_str())
    }

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
    // Every dial follows its own atomic: the audio thread (or a learned controller) is the truth between frames, so the
    // face is read back from the catalogue rather than from a slot list. The WAVEFORM knob is the exception — a switch,
    // its position is worked out where the knob is drawn, because only there is it known whether the hand or a learned
    // controller owns it.
    for knob in Knob::ALL {
        if knob.kind() == Kind::Dial {
            state.knobs[knob.index()] = knob.read(shared);
        }
    }
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
    // Both readouts answer the pointer, so a player who does not know what "A1" or "128" means can ask: a display is
    // still a control, and it is teachable besides.
    for (name, rect, control) in [
        (
            "hover-pattern",
            panel::pattern_display_rect(l.top),
            tips::Control::Pattern,
        ),
        (
            "hover-tempo",
            panel::tempo_display_rect(l.top),
            tips::Control::Tempo,
        ),
    ] {
        let response = ui.interact(rect, ui.scope_id().with(name), egui::Sense::hover());
        tips::hover(&response, state, control);
    }
    // Both readouts wear the pick colour: while choosing, they are the answer to "which one?"; while a learn is
    // listening for one of them, that one keeps its border so the player can see what the controller will bind.
    if learn::highlight(state.learn, midi::cc::CcTarget::Tempo) {
        panel::learn_highlight(ui.painter(), panel::tempo_display_rect(l.top), s);
    }
    if learn::highlight(state.learn, midi::cc::CcTarget::Pattern) {
        panel::learn_highlight(ui.painter(), panel::pattern_display_rect(l.top), s);
    }
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
        panel::pattern_display_rect(l.top),
        midi::cc::CcTarget::Pattern,
    );

    // The song the session is working on, in the middle of the strip: a player should never have to open the card to find
    // out what they are working on. The status, or a notice that outlives the card, takes the right half of the band.
    let label = panel::song_label_rect(l.top, s);
    ui.painter().with_clip_rect(label).text(
        label.center(),
        egui::Align2::CENTER_CENTER,
        song_label(state.songs.name.as_deref()),
        theme::legend(10.0 * s),
        if state.songs.file.is_some() {
            theme::SILKSCREEN_DIM
        } else {
            theme::SILKSCREEN_DIM.gamma_multiply(0.55)
        },
    );

    // A notice is said once and clears itself; the status line underneath comes back when it has.
    let now = std::time::Instant::now();
    if state
        .notice
        .as_ref()
        .is_some_and(|(_, at)| now.duration_since(*at) >= PanelState::NOTICE_FOR)
    {
        state.notice = None;
    }
    let notice = state.notice_now(now);
    let status = panel::midi_status_rect(l.top, s);
    let line = notice.map(str::to_string).unwrap_or_else(|| {
        learn::hint(state.learn)
            .unwrap_or(&state.midi_status)
            .to_string()
    });
    // Clipped to the band: a notice carries a path when it is a failure, and it must not paint over the song label it
    // shares the line with.
    ui.painter().with_clip_rect(status).text(
        status.right_center(),
        egui::Align2::RIGHT_CENTER,
        // While learning, the status line says what the panel is waiting for — it is the only free line on the face.
        line,
        theme::legend(11.0 * s),
        if notice.is_some() || state.learn != learn::Step::Idle || state.midi_given_up {
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
    row_knobs::draw(state, shared, ui, &l, s);
    row_perform::draw(state, shared, ui, &l, s);
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
    if learn::highlight(state.learn, midi::cc::CcTarget::Volume) {
        panel::learn_highlight(ui.painter(), volume_rect, s);
    }
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
            state.songs.card.is_some(),
            tips::Control::Songs,
        ) {
            state.songs.card = match state.songs.card {
                Some(_) => None,
                None => Some(songs::Songs {
                    names: crate::persist::list_songs(&crate::persist::data_dir()),
                    // The demos are built into the binary, so the card only needs their names to list them.
                    demos: crate::demo::songs()
                        .iter()
                        .map(|demo| demo.name.to_string())
                        .collect(),
                    name: state.songs.name.clone().unwrap_or_default(),
                    open: state.songs.file.clone(),
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
    // Clipped to the band: a notice carries a path when it is a failure, and it must not paint over the song label it
    // shares the line with.
    ui.painter().with_clip_rect(status).text(
        panel::save_indicator_rect(l.top, s).right_center(),
        egui::Align2::RIGHT_CENTER,
        save_label,
        theme::legend(10.0 * s),
        save_colour,
    );
    // AUTO: whether the session writes itself. Saving is manual unless this is lit, and the SAVED / UNSAVED lamp beside it
    // is then the only promise about what is on disk.
    let auto_rect = autosave_rect(window);
    let auto_label = if state.autosave {
        "AUTO ON"
    } else {
        "AUTO OFF"
    };
    if switch_key(
        ui,
        state,
        "autosave",
        auto_rect,
        auto_label,
        state.autosave,
        tips::Control::Autosave,
    ) {
        state.autosave = !state.autosave;
        state.dirty = true;
    }
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

    // The transport row comes after the keys above it, in the order it has always been drawn: a later widget wins a
    // hit-test tie, so keeping the order keeps the panel's behaviour.
    row_transport::draw(state, shared, ui, &l);
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
        state.picker.inputs = match state.picker.inputs {
            Some(_) => None,
            None => Some(midi::ports::list_ports().unwrap_or_default()),
        };
    }
    if let Some(ports) = state.picker.inputs.clone() {
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
            state.picker.input_choice = Some(choice);
            state.picker.inputs = None;
        } else {
            let click = ui.input(|i| {
                if i.pointer.any_click() {
                    i.pointer.interact_pos()
                } else {
                    None
                }
            });
            if picker::should_close(click, &rows, handle) {
                state.picker.inputs = None;
            }
        }
    }

    // A small `?` at the end of the wordmark opens the help card, and the card covers the panel while it is
    // showing — it is the only thing that needs no explanation of its own.
    // The help dot and the settings gear, side by side at the end of the wordmark: two round buttons that need no
    // explanation of their own. Both paint unclipped — the clip that belongs to the status band is not theirs.
    let help_dot = panel::help_rect(l.top, s);
    let gear_dot = panel::gear_rect(l.top, s);
    let help_response = ui.interact(
        help_dot,
        ui.scope_id().with("help-toggle"),
        egui::Sense::click(),
    );
    let _gear_response = ui.interact(
        gear_dot,
        ui.scope_id().with("settings-toggle"),
        egui::Sense::click(),
    );
    let painter = ui.painter();
    painter.circle_stroke(help_dot.center(), 8.0 * s, theme::silkscreen_stroke(s));
    painter.text(
        help_dot.center(),
        egui::Align2::CENTER_CENTER,
        "?",
        theme::legend(11.0 * s),
        theme::SILKSCREEN,
    );
    paint_gear(painter, gear_dot.center(), s);
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
    if state.songs.card.is_some() {
        let shield = ui.interact(
            window,
            ui.scope_id().with("songs-shield"),
            egui::Sense::click(),
        );
        let mut request = None;
        // The card names the session by what the session is on, not by what the library file happens to be: a first run
        // and a demo have a name and no file, and the header must not call them untitled while the field says otherwise.
        let session = state.songs.name.clone();
        if let Some(card) = state.songs.card.as_mut() {
            request = songs::paint(ui, l.panel, s, card, session.as_deref(), state.dirty);
        }
        if shield.clicked() {
            request = Some(songs::Request::Close);
        }
        match request {
            Some(songs::Request::Close) => state.songs.card = None,
            Some(other) => state.songs.request = Some(other),
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

// The control position of a wave is the switch's business, so it lives with the catalogue; the panel keeps these names
// because a detent marker and a learned controller both speak in positions.
pub use crate::knobs::{waveform_from_position, waveform_position};

/// What the song label says: the name the session is on, or that it has none. Truncated so a long name cannot run into
/// the status line beside it.
fn song_label(name: Option<&str>) -> String {
    let Some(name) = name else {
        return "SONG untitled".to_string();
    };
    let mut short: String = name.chars().take(18).collect();
    if name.chars().count() > 18 {
        short.push('…');
    }
    format!("SONG {short}")
}

/// A gear, drawn rather than typed: a ring, eight teeth and a hub. The face may not use font glyphs, so the shape is the
/// icon — and at 18 px it still reads as a gear because the teeth are strokes, not pixels.
fn paint_gear(painter: &egui::Painter, centre: egui::Pos2, scale: f32) {
    let stroke = theme::silkscreen_stroke(scale);
    painter.circle_stroke(centre, 7.0 * scale, stroke);
    for tooth in 0..8 {
        let angle = std::f32::consts::FRAC_PI_4 * tooth as f32;
        let (sin, cos) = angle.sin_cos();
        painter.line_segment(
            [
                centre + egui::vec2(cos, sin) * 7.0 * scale,
                centre + egui::vec2(cos, sin) * 10.0 * scale,
            ],
            stroke,
        );
    }
    painter.circle_filled(centre, 2.0 * scale, theme::SILKSCREEN_DIM);
}
