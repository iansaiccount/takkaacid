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
pub mod modal;
pub mod panel;
pub mod pattern_swap;
pub mod randomise;
pub mod row_knobs;
pub mod row_perform;
pub mod row_transport;
pub mod save;
pub mod scale;
pub mod selection;
pub mod settings;
pub mod shortcuts;
pub mod songs;
pub mod step_key;
pub mod switches;
pub mod text;
pub mod tips;

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
    /// True when the supervisor has given up, which is what makes the settings card show its RETRY row.
    pub midi_given_up: bool,
    /// Set by the settings card's RETRY row; the app clears it and probes again.
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
    /// Whether hovering a knob or a button explains it. On unless the player turns it off with the TOOLTIPS switch in
    /// the settings card.
    pub tooltips: bool,
    /// Whether the session writes itself when it changes. Off unless the player turns it on with the AUTOSAVE switch:
    /// saving is manual, and the lamp plus the SAVE key are how a player knows where they stand.
    pub autosave: bool,
    /// Something the panel wants to say — "saved as Acid 2", "could not import: …" — with the moment it was said. It
    /// outlives the card that asked for it, and it clears itself, so a message is never something to dismiss.
    pub notice: Option<(String, std::time::Instant)>,
    /// True while the settings card is showing.
    pub settings_open: bool,
    /// What that card shows: the two device lists (read on the rising edge of `settings_open`, because enumerating them
    /// costs a round trip each) and, copied every frame while the card is up, which devices are in use and what the two
    /// supervisors say. `main` fills it because it owns the ports and the streams.
    pub settings: settings::Settings,
    /// Set by the card, consumed by `main` on the next frame.
    pub settings_request: Option<settings::Request>,
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
/// What the player chose on the panel, consumed by `main` on the next frame. The lists themselves live in the settings card,
/// which asks for a choice the same way the old status-line pickers did, so the stream handling is unchanged.

#[derive(Default)]
pub struct PickerState {
    /// The MIDI input the player chose.
    pub input_choice: Option<midi::ports::PortChoice>,
    /// The audio output the player chose.
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
            settings_open: false,
            settings: settings::Settings::default(),
            settings_request: None,
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
        tips::hover(&response, state.tooltips, control);
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

    // The middle of the strip says one thing at a time: the song the session is working on, or — for a few seconds — what
    // just happened. Notices used to share the line with a MIDI status readout; that readout now lives in the settings card,
    // so the line is the notice's or the song's, never a fight between them.
    let now = std::time::Instant::now();
    if state
        .notice
        .as_ref()
        .is_some_and(|(_, at)| now.duration_since(*at) >= PanelState::NOTICE_FOR)
    {
        state.notice = None;
    }
    let notice = state.notice_now(now);
    let label = panel::song_label_rect(l.top, s);
    let (line, colour) = match notice {
        Some(message) => (message.to_string(), theme::LED_ACCENT),
        None => (
            song_label(state.songs.name.as_deref()),
            if state.songs.file.is_some() {
                theme::SILKSCREEN_DIM
            } else {
                theme::SILKSCREEN_DIM.gamma_multiply(0.55)
            },
        ),
    };
    // Measured, not guessed: the label rect holds a song's name or a notice, and a notice can be longer than the rest
    // of the face. Cutting it with the clip rect alone lost both ends; this keeps the beginning and says it was cut.
    // Measured, not guessed: the label rect holds a song's name or a notice, and a notice can be longer than the rest
    // of the face. Cutting it with the clip rect alone lost both ends; this keeps the beginning and says it was cut.
    // One font for the measurement and the paint, or the fit would be honest only while two expressions stayed equal.
    let font = theme::legend(10.0 * s);
    let line = text::fit(ui.painter(), &line, font.clone(), label.width());
    ui.painter().with_clip_rect(label).text(
        label.center(),
        egui::Align2::CENTER_CENTER,
        line,
        font,
        colour,
    );
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
    // The lamp says one word: SAVED in dim silk, UNSAVED in amber, inside its own corner of the strip.
    ui.painter()
        .with_clip_rect(panel::save_indicator_rect(l.top, s))
        .text(
            panel::save_indicator_rect(l.top, s).right_center(),
            egui::Align2::RIGHT_CENTER,
            save_label,
            theme::legend(10.0 * s),
            save_colour,
        );
    // The transport row: the five keys under the switches, drawn after everything above them, so a later widget wins a
    // hit-test tie and the panel behaves in the order it always has.
    row_transport::draw(state, shared, ui, &l);

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
    let gear_response = ui.interact(
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
    if gear_response.clicked() {
        // One card at a time: the gear asks for settings and puts the other two away.
        state.settings_open = true;
        state.help_visible = false;
        state.songs.card = None;
    }
    if help_response.clicked() && !help_was_visible {
        state.settings_open = false;
        state.help_visible = true;
    }
    if help_was_visible {
        // The card is modal while it is up: a click anywhere closes it, and the shield keeps that click from also
        // pressing whatever sits underneath.
        let shield = modal::shield(ui, window, "help-shield");
        help::paint_help(ui.painter(), l.panel, s);
        if shield.clicked() {
            state.help_visible = false;
        }
    }

    draw_settings(ui, state, &l, s, window);

    // The songs card sits over everything, so its shield is registered before the card's own controls: a click on
    // a row or the text field is theirs, a click anywhere else dismisses.
    if state.songs.card.is_some() {
        let shield = modal::shield(ui, window, "songs-shield");
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

/// What the song label says: the name the session is on, or that it has none. How much of it fits is not decided here —
/// `text::fit` measures the line against the rect it is painted in, so there is one truncation rule rather than two.
fn song_label(name: Option<&str>) -> String {
    match name {
        Some(name) => format!("SONG {name}"),
        None => "SONG untitled".to_string(),
    }
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

/// The settings card, drawn after everything else so it covers the panel like help and songs do. One card at a time: help
/// and the songs card take precedence, and this closes itself rather than fight them for the shield.
fn draw_settings(
    ui: &mut egui::Ui,
    state: &mut PanelState,
    l: &layout::PanelLayout,
    s: f32,
    window: egui::Rect,
) {
    if !state.settings_open || state.help_visible || state.songs.card.is_some() {
        return;
    }
    // The shield dims the window, so it is registered before the card's own controls: a later widget wins a hit-test tie,
    // and a click on a fold header or a device row is the card's, not a dismissal. Only a click outside them closes.
    let shield = modal::shield(ui, window, "settings-shield");
    let mut request = settings::paint(
        ui,
        l.panel,
        s,
        &state.settings,
        state.tooltips,
        state.autosave,
    );
    if shield.clicked() {
        request = Some(settings::Click::Own(settings::Own::Close));
    }
    match request {
        // The folds and CLOSE are the card's own state, so they are applied here rather than going through `main`.
        Some(settings::Click::Own(settings::Own::ToggleMidi)) => {
            state.settings.midi_open = !state.settings.midi_open;
        }
        Some(settings::Click::Own(settings::Own::ToggleSound)) => {
            state.settings.sound_open = !state.settings.sound_open;
        }
        Some(settings::Click::Own(settings::Own::Close)) => state.settings_open = false,
        Some(settings::Click::Ask(request)) => state.settings_request = Some(request),
        None => {}
    }
}
