// The palette and layout constants are the panel's vocabulary: they are introduced here and consumed
// from Phase 06 onwards, so the dead-code lint is waived for the module rather than per constant.
pub mod banks;
pub mod clear;
pub mod controls;
pub mod device_card;
pub mod dispatch;
pub mod display;
pub mod drum_panel;
pub mod edit;
pub mod help;
pub mod keys;
pub mod kits;
pub mod knob;
pub mod layout;
pub mod learn;
pub mod modal;
pub mod panel;
pub mod pattern_swap;
pub mod perform;
pub mod randomise;
pub mod row_knobs;
pub mod row_perform;
pub mod row_transport;
pub mod scale;
pub mod selection;
pub mod settings;
pub mod shortcuts;
pub mod step_key;
pub mod strip;
pub mod switches;
pub mod tape;
pub mod text;
pub mod tips;

pub mod theme;
pub mod transport;

use std::collections::BTreeMap;
use std::sync::Arc;

use acid_core::pattern::{Pattern, PatternId, STEPS_PER_PATTERN, Step};
use acid_core::sequencer::sounding_step;
use acid_core::timing::DEFAULT_BPM;

use crate::midi;
use crate::state::SharedState;

use crate::knobs::{Kind, Knob};

use self::controls::{Key, learn_pick, switch_key};

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
    /// The step the face edits — the 303's border, the drum grid's lit column; always in `0..16`, never saved.
    pub selected: usize,
    /// The drum face's editor: which voice its dials are on. Panel-wide, not per device — plan 01, task 4's ruling. Not
    /// saved: the kit it edits is in memory only this phase.
    pub drum: drum_panel::DrumView,
    /// The kit browser: the kits that ship and the player's own, and the list `main` reads for it. The list outlives the
    /// card, because the copy a first edit of a factory kit makes has to know which names are already taken.
    pub kits: kits::Kits,
    /// **Which** device's kit has been edited since its folder was last written, by its number — the identity that does not
    /// move when a device before it is removed. A kit is two things on disk (a name in the save file, a folder of samples), and
    /// writing two megabytes of WAV on every save is not a save. The number is here rather than a bare `bool` because the
    /// device that was edited need not be the one selected when the save happens, and a write aimed at the wrong device loses
    /// the player's kit.
    pub kit_dirty: Option<usize>,
    /// What the kit browser asked for; consumed by `main`, which owns the kits on disk.
    pub kits_request: Option<kits::Request>,
    /// The port picker: what is on offer and what the player chose.
    pub picker: PickerState,
    /// The VOLUME knob, kept separate from the seven tone knobs.
    pub volume: f32,
    /// When CLEAR was first clicked: a second click inside the window empties the pattern on screen.
    pub clear_armed_at: Option<std::time::Instant>,
    /// The pattern text this panel last put on the clipboard, so the COPY key can read as pressed while the clipboard still
    /// holds the pattern on screen. `None` until something is copied — a paste from elsewhere is not this panel's copy.
    pub copied: Option<String>,
    /// The pattern on screen encoded as clipboard text, kept beside the `Arc` it was encoded from. The COPY key asks every
    /// frame whether the clipboard still holds what is on screen, and encoding it each time was a `String` per frame on the UI
    /// thread; this way the encoding happens when the pattern changes. Never saved: it caches what is already there.
    pub copy_text: Option<(Arc<Pattern>, PatternId, String)>,
    /// Set by the SAVE key; consumed by `main`, which writes the session and clears `dirty`.
    pub save_requested: bool,
    /// Set by the transport row; consumed by `main`, which owns the rack's transport. A request rather than a write: the
    /// rack fans a change out to every device, so writing the displayed device's own atomic would be undone by the next sync.
    pub transport_request: Option<transport::Request>,
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
    /// The strip: one cell per device, and which of them is on the panel. `main` fills it from the rack every frame, the
    /// way `pattern` is filled from the device's state, because the panel paints and never owns the list.
    pub strip: strip::Strip,
    /// Which way round to paint the face: the selected device's scheme, filled by `main` every frame the way the strip is.
    /// The five cards ignore it — they are furniture, and stay black whatever the face does. The picker is the sixth surface
    /// over the panel and is furniture too.
    pub scheme: crate::device::Scheme,
    /// Set by the strip and by `Ctrl+1..4`, drained by `main` after the frame. The panel cannot reach the rack, so asking
    /// is the only way it can select a device, mute one or add one.
    pub device_request: Option<crate::device::Request>,
    /// The device card: which device it is about and what `main` last read into it. One card at a time with help, songs
    /// and settings — `open_device_card` is the only way in, so the rule cannot be forgotten at one of the three.
    pub device_card: device_card::Card,
    /// The picker: whether the card that offers the kinds of device is up. The strip's `+` asks for it and `main` opens it;
    /// it closes itself, because nothing on it is a reading of the machine.
    pub device_picker: device_card::Picker,
    /// The device whose editor the flat fields currently hold, by its number — the identity that does not move when a
    /// device before it is removed. `None` until the first frame shows one.
    pub focused: Option<usize>,
    /// One editor per device, keyed by that device's number: written when a switch leaves a device, read when one
    /// arrives, so looking away and back keeps the step, the step mode and the randomise dials. Keyed by number rather
    /// than by position because the position is exactly what a removal moves: a survivor would otherwise inherit the
    /// removed device's editor. A device that is gone has its slot dropped on the next frame.
    pub edit_slots: BTreeMap<usize, PanelEdit>,
}

/// The songs card and everything it asked for.
///
/// One value because a request without a card, or a name for a song nobody opened, is not a state the panel can be in:
/// the card asks, `main` answers, and the answer is cleared with the card.
#[derive(Default)]
pub struct SongState {
    /// The card while it is open, `None` when it is closed.
    pub card: Option<banks::Banks>,
    /// What the card asked for; consumed by `main`, which owns the files.
    pub request: Option<banks::Request>,
    /// The song this session is working on, once it has a name.
    pub name: Option<String>,
    /// The song file this session is saved as, if it is one. A demo arrives with `None`: it is a new song under a name, and
    /// nothing of it is on disk until the player saves it.
    pub file: Option<String>,
}
/// What the player chose on the panel, consumed by `main` on the next frame. The output list lives in the settings card,
/// which asks for a choice the same way the old status-line picker did, so the stream handling is unchanged. The inputs
/// are not here: a port row is a switch, so opening one is a state `main` acts on, not a choice it waits for.
#[derive(Default)]
pub struct PickerState {
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
            drum: drum_panel::DrumView::default(),
            kits: kits::Kits::default(),
            kit_dirty: None,
            kits_request: None,
            clear_armed_at: None,
            copied: None,
            copy_text: None,
            save_requested: false,
            transport_request: None,
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
            strip: strip::Strip::default(),
            scheme: crate::device::Scheme::Black,
            device_request: None,
            device_card: device_card::Card::default(),
            device_picker: device_card::Picker::default(),
            focused: None,
            edit_slots: BTreeMap::new(),
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

    /// Show the device card for `index`, and put every other card away: there is room for one, and this one is painted over
    /// the panel last.
    ///
    /// F3, the caret on the strip's selected cell and `main`'s `Request::Open` all arrive here, so the one-card rule cannot
    /// be remembered at two of the three and forgotten at the last.
    pub fn open_device_card(&mut self, index: usize) {
        self.put_cards_away();
        self.device_card.open = Some(index);
    }

    /// Put every card away: the one spelling of "one card at a time".
    ///
    /// Every path that *opens* a card calls this and then sets its own flag, so a card added later cannot be forgotten by
    /// four callers that each list the others by hand — which is what the kit browser was, and what made Escape give up a
    /// card nobody could see.
    pub fn put_cards_away(&mut self) {
        self.device_card = device_card::Card::default();
        self.device_picker.open = false;
        self.settings_open = false;
        self.help_visible = false;
        self.songs.card = None;
        kits::close(self);
    }

    /// Put the device card away, mirror and all: its name, its route and its arming are readings of one device, and none of
    /// them may be shown against the next one.
    ///
    /// F3, Escape, the card's CLOSE key and its shield all arrive here, and would otherwise be four places that have to
    /// remember to clear the same seven fields.
    pub fn close_device_card(&mut self) {
        self.device_card = device_card::Card::default();
    }
    /// Whether any card is open over the panel.
    ///
    /// The one spelling of "the face is covered", asked by everything that would otherwise change something a card hides: the
    /// step editor's arrows and `Enter`, the drum face's voice walk, step move and step writes, the computer keyboard's step
    /// writes, and a MIDI note waiting to be placed. A card is modal by design, so a writer that does not ask this is a write a
    /// player cannot see.
    pub fn a_card_is_up(&self) -> bool {
        self.help_visible
            || self.settings_open
            || self.kits.open
            || self.songs.card.is_some()
            || self.device_card.open.is_some()
            || self.device_picker.open
    }

    /// Show the picker, and put every other card away: there is room for one, and this is the one that adds a device.
    pub fn open_device_picker(&mut self) {
        self.put_cards_away();
        self.device_picker.open = true;
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

    /// What kind of machine the strip says is on the panel.
    ///
    /// `main` owns the rack and fills the strip each frame, so this is a reading of that mirror — the panel may be a frame
    /// behind a removal, which is why it is a `get` and why an empty rack answers the acid face. One method for the two
    /// places that ask: `draw_panel` decides which face to paint from it, and `dispatch` decides whether the keyboard's step
    /// editor belongs to this machine.
    pub fn showing_kind(&self) -> crate::device::DeviceKind {
        self.strip
            .cells
            .get(
                self.strip
                    .selected
                    .min(self.strip.cells.len().saturating_sub(1)),
            )
            .map_or(crate::device::DeviceKind::Acid, |cell| cell.kind)
    }
}

/// What the panel remembers about one device while another is displayed.
///
/// Exactly the fields that are *not* re-read from `SharedState` every frame: `knobs`, `volume`, `pattern`, `tempo`,
/// `playing` and `position` are mirrors and need no slot. A field added later that is neither a mirror nor here will reset
/// on every device switch, which is why the test names all nine.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelEdit {
    pub pattern_id: PatternId,
    pub selected: usize,
    pub step_mode: bool,
    pub placed: Vec<egui::Key>,
    pub last_pitch: Pitch,
    pub last_octave: i8,
    pub density: f32,
    pub scale: scale::Scale,
    pub scale_knob: f32,
}

impl PanelEdit {
    /// The editor a device the panel has never shown starts with: the panel's own defaults, taken from `PanelState::default`
    /// so the two cannot drift apart.
    pub fn of_defaults() -> Self {
        let mut fresh = PanelState::default();
        fresh.save_edit()
    }
}

impl PanelState {
    /// Take the flat editor fields into a slot — the outgoing half of a device switch.
    pub fn save_edit(&mut self) -> PanelEdit {
        PanelEdit {
            pattern_id: self.pattern_id,
            selected: self.selected,
            step_mode: self.step_mode,
            placed: self.placed.clone(),
            last_pitch: self.last_pitch,
            last_octave: self.last_octave,
            density: self.density,
            scale: self.scale,
            scale_knob: self.scale_knob,
        }
    }

    /// Put a slot back into the flat fields — the incoming half of a device switch.
    pub fn load_edit(&mut self, edit: PanelEdit) {
        self.pattern_id = edit.pattern_id;
        self.selected = edit.selected;
        self.step_mode = edit.step_mode;
        self.placed = edit.placed;
        self.last_pitch = edit.last_pitch;
        self.last_octave = edit.last_octave;
        self.density = edit.density;
        self.scale = edit.scale;
        self.scale_knob = edit.scale_knob;
    }
}
/// Paint the whole panel into `ui`: face, displays, knobs, step keys, switches and transport legends.
///
/// Painting uses absolute rects from `layout`, so the panel lands in the same place whatever the Ui's own
/// cursor is doing. Returns nothing: Phase 07 wires the controls to the engine.
pub fn draw_panel(state: &mut PanelState, shared: &SharedState, ui: &mut egui::Ui) {
    // Read every frame, so a change made by a hand that is not the panel's — a learned tempo fader, writing from the MIDI
    // thread — marks the session unsaved the same way a key does. The tempo is in the save file, which is the whole rule.
    if shared.take_tempo_written() {
        state.dirty = true;
    }
    // `main` owns the rack and may have removed a device since it filled the mirror, so the count the strip draws is
    // derived here rather than trusted: the panel may be one frame behind, and it must not be one index out of bounds.
    state.strip.selected = state
        .strip
        .selected
        .min(state.strip.cells.len().saturating_sub(1));
    // What kind of machine is on the panel. The strip is the only place the panel can learn it.
    let kind = state.showing_kind();
    // The strip is a mirror, so the device under the editor can change between frames: save the outgoing device's editor and
    // load the incoming one's. Both are keyed by the device's *number*, because a removal moves every later device's
    // position and would otherwise hand the survivor the removed device's step, step mode and randomise dials. A device the
    // panel has never shown starts on its own first step with the panel's defaults — but keeps the pattern `main` has just
    // pointed the display at, because that is that device's bank, not a default.
    let showing = state
        .strip
        .cells
        .get(state.strip.selected)
        .map(|cell| cell.number);
    // A device that is gone keeps no editor: the map is bounded by the rack, and a number is never reused.
    let present: Vec<usize> = state.strip.cells.iter().map(|cell| cell.number).collect();
    state
        .edit_slots
        .retain(|number, _| present.contains(number));
    if let Some(showing) = showing
        && state.focused != Some(showing)
    {
        if let Some(leaving) = state.focused {
            let edited = state.save_edit();
            state.edit_slots.insert(leaving, edited);
        }
        state.focused = Some(showing);
        let arriving = match state.edit_slots.get(&showing) {
            Some(edit) => edit.clone(),
            None => PanelEdit {
                pattern_id: state.pattern_id,
                ..PanelEdit::of_defaults()
            },
        };
        state.load_edit(arriving);
        // A learn belongs to the device it was started on: switching away cancels it rather than teaching the neighbour.
        state.learn = learn::Step::Idle;
        learn::cancel(shared);
    }
    let window = ui.max_rect();
    // A focused text field owns the keyboard, and the panel's audition keys are not an exception: with the cursor in the
    // device card's name field, `A` and shift+`A` are a letter and a capital, not a note and a step. The device card is the
    // first field on the face that a player types a *name* into, which is what made this reachable.
    //
    // A drum device does not answer the computer keyboard at all: its notes are not the 303's pitches, and the grid is the
    // editor. The keys are left to the shortcuts and to nothing else.
    if kind == crate::device::DeviceKind::Acid {
        play_computer_keys(shared, ui, state);
    }

    // Every MIDI note still waiting while the panel is editing (step mode, or shift held) belongs to the selected
    // step. The queue is drained rather than peeked: two notes inside one frame are two steps, each at its own
    // velocity — one slot and a counter used to lose the first of them.
    // A card over the face is modal: a note that would land in a step nobody can see is drained and dropped, not placed.
    let editing =
        (state.step_mode || ui.input(|input| input.modifiers.shift)) && !state.a_card_is_up();
    while let Some(waiting) = shared.notes.pop() {
        // The queue is drained for every kind — a note that arrived while a drum device was on the panel must not be placed
        // later — but only the 303 writes it into a step. A drum device's MIDI notes are a later round.
        if !editing || kind != crate::device::DeviceKind::Acid {
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
    // The face's palette, from the device on the panel. The cards stay `BLACK`: they are furniture laid over the face, not
    // part of it, and an amber card over an amber face would read as a second instrument.
    let p = theme::palette(state.scheme);
    // The pattern is whatever the UI thread last published — one refcount bump, no allocation.
    state.pattern = shared.pattern();
    state.playing = shared.is_playing();
    // Every dial follows its own atomic: the audio thread (or a learned controller) is the truth between frames, so the
    // face is read back from the catalogue rather than from a slot list. The WAVEFORM knob is the exception — a switch,
    // its position is worked out where the knob is drawn, because only there is it known whether the hand or a learned
    // controller owns it.
    // A drum face fills its seven slots from the selected voice's kit fields inside `drum_panel`, so this 303 read must not
    // run and overwrite them with the 303's atomics.
    if kind == crate::device::DeviceKind::Acid {
        for knob in Knob::ALL {
            if knob.kind() == Kind::Dial {
                state.knobs[knob.index()] = knob.read(shared);
            }
        }
    }
    state.tempo = shared.tempo_bpm.get();
    state.position = shared.position.load(std::sync::atomic::Ordering::Relaxed);
    state.volume = shared.volume.get();
    panel::paint_panel(ui.painter(), &l, p);
    // The strip sits on the face and asks for what a click meant; `main` answers after the frame.
    strip::paint(ui, state, l.strip, s, p);

    display::paint_display(
        ui.painter(),
        panel::pattern_display_rect(l.top),
        &display::format_pattern(state.pattern_id),
        s,
        p,
    );
    display::paint_display(
        ui.painter(),
        panel::tempo_display_rect(l.top),
        &display::format_tempo(state.tempo),
        s,
        p,
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
        panel::learn_highlight(ui.painter(), panel::tempo_display_rect(l.top), s, p);
    }
    if learn::highlight(state.learn, midi::cc::CcTarget::Pattern) {
        panel::learn_highlight(ui.painter(), panel::pattern_display_rect(l.top), s, p);
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

    // The tape deck and the tempo stepper, between the session name and the pattern readout. Its presses are *requests* for
    // the same reason the switches' are: the rack owns the transport, and a key that wrote a device's atomic would be undone by
    // the next fan-out.
    if let Some(request) = tape::draw(ui, state, shared, l.top, s, p) {
        state.transport_request = Some(request);
        // `dirty` is not set here: the tempo's request is applied in `main` through `transport::apply_nudge`, so a nudge
        // from the arrow keys dirties the session the same way the stepper in this deck does.
    }

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
    // The name carries the save state now that the word is gone (round 3a): the accent while there are unsaved changes,
    // plain silk when the session is written, and `SONG` alone while it has never been named.
    let unsaved = state.dirty;
    let named = state.songs.name.is_some();
    let (line, colour) = match notice {
        Some(message) => (message.to_string(), p.led_accent),
        // `song_label` already carries the `SONG` prefix; a second one made the band read `SONG SONG New song`.
        None if named => (
            song_label(state.songs.name.as_deref()),
            if unsaved { p.led_accent } else { p.silkscreen },
        ),
        None => ("SONG".to_string(), p.silkscreen_dim),
    };
    // The name is drawn in `led_accent` while there are unsaved changes and in plain silk when there are none. The plate the
    // design first proposed was cut by the player, who read it as a stray rectangle under the name rather than as emphasis;
    // the state still reaches the pointer, because the tooltip ends `(unsaved)` or `(saved)`.
    if notice.is_none() {
        // The only dynamic tooltip on this face: it carries a name the player typed, so it cannot come from `ui::tips`.
        let full = song_label(state.songs.name.as_deref());
        let said = if named {
            // `full` already begins with `SONG`: prefixing it again is what made the hover read `SONG SONG New song`.
            format!("{full}\n({})", if unsaved { "unsaved" } else { "saved" })
        } else {
            "SONG\nThis session has no name yet".to_string()
        };
        // The name is a door as well as a label: the bank browser is how a session is loaded and saved, and the icons that
        // used to open it were cut in round 3a for room the deck needed. A label that answers a click is the cheapest way back in.
        let response = ui.interact(
            label,
            ui.scope_id().with("song-title"),
            egui::Sense::click(),
        );
        if response.hovered() {
            response
                .clone()
                .on_hover_text(format!("{said}\nClick for the banks"));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            open_banks(state);
        }
    }
    // Measured, not guessed: the label rect holds a song's name or a notice, and a notice can be longer than the rest
    // of the face. Cutting it with the clip rect alone lost both ends; this keeps the beginning and says it was cut.
    // One font for the measurement and the paint, or the fit would be honest only while two expressions stayed equal.
    let font = theme::legend(10.0 * s);
    let line = text::fit(ui.painter(), &line, font.clone(), label.width());
    // Left-aligned in the corridor: a name is read from its first letter, and a centred one jumps every time it is typed.
    ui.painter().with_clip_rect(label).text(
        egui::Pos2::new(label.left(), label.center().y),
        egui::Align2::LEFT_CENTER,
        line,
        font,
        colour,
    );
    match kind {
        crate::device::DeviceKind::Acid => {
            row_knobs::draw(state, shared, ui, &l, s, p);
            row_perform::draw(state, shared, ui, &l, s, p);
        }
        crate::device::DeviceKind::Drums => {
            // The dials, the filter key, SHUFFLE, HIT and the grid replace the knob, performance and step rows: on a drum
            // device the 303's rows are gone, not inert.
            drum_panel::draw(state, shared, ui, &l, s, p);
        }
    }
    // VOLUME has its own corner of the top strip, before the wordmark, so the row below never needed a cell for it.
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
        p,
    );
    if learn::highlight(state.learn, midi::cc::CcTarget::Volume) {
        panel::learn_highlight(ui.painter(), volume_rect, s, p);
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

    // The step row is the 303's editor; a drum device's columns are the grid's cells, and drawing the row would give the
    // face two editors and the player a step border nothing on the drum face shows.
    if kind == crate::device::DeviceKind::Acid {
        let keys = layout::step_cells(l.steps);
        // The LED is on the step that **sounds**, not the step the clock is on: under `REVERSE` the line reads the bar from
        // its other end, so the light walks back along the keys with the notes it is sounding. The direction is the one the
        // audio thread published, not the one the key asked for — a running line takes a request at the bar line, and a light
        // on the request would turn a bar ahead of the sound.
        let sounding = sounding_step(state.position, shared.is_reverse_active());
        for (i, cell) in keys.iter().enumerate() {
            let step = state.pattern.steps[i % STEPS_PER_PATTERN];
            let view = step_key::step_view(
                &step,
                i,
                sounding,
                state.playing,
                i % STEPS_PER_PATTERN == state.selected,
                state.drag.is_some_and(|(dragged, ..)| dragged == i),
                // A slide only glides into a step that sounds, so the keycap needs to know about the next one.
                state.pattern.steps[(i + 1) % STEPS_PER_PATTERN].on,
            );
            let response = step_key::step_key(ui, i, &step, &view, *cell, p);
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
                let edited =
                    edit::apply_gesture(step, gesture, state.last_pitch, state.last_octave);
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
                    p.silkscreen,
                );
            } else {
                state.drag = None;
            }
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
            p,
        );
    }
    {
        // COPY writes the same text `Ctrl+C` writes — the gesture and the key are one format, and `main` puts it on the OS
        // clipboard from the same function — so the two cannot drift.
        let copy_cell = switches::cell(l.switches, switches::Switch::Copy);
        let slot = display::format_pattern(state.pattern_id);
        // Encoded once per pattern rather than once per frame: the key only asks whether the clipboard still holds what is on
        // screen, and the encoding is a `String` of sixteen steps.
        if !state.copy_text.as_ref().is_some_and(|(pattern, id, _)| {
            Arc::ptr_eq(pattern, &state.pattern) && *id == state.pattern_id
        }) {
            state.copy_text = Some((
                Arc::clone(&state.pattern),
                state.pattern_id,
                pattern_text(shared, &slot),
            ));
        }
        let current = state.copy_text.as_ref().map(|(_, _, text)| text.as_str());
        let holding = current.is_some() && state.copied.as_deref() == current;
        let pressed = switch_key(
            ui,
            state,
            Key {
                id: "copy-pattern",
                rect: copy_cell,
                label: "COPY",
                lit: false,
                control: tips::Control::Copy,
            },
            p,
        );
        if holding {
            // Indented while the clipboard holds the pattern on screen: a second border inside the key, the way a pressed
            // collar sits inside a real switch. It is the only feedback that says "this is already copied".
            ui.painter().rect_stroke(
                copy_cell.shrink(2.0 * s),
                2.0 * s,
                p.silkscreen_stroke(1.0 * s),
                egui::StrokeKind::Inside,
            );
        }
        if pressed {
            // This frame's own encoding, so a copy costs no second `String` of the same sixteen steps. The cache is already
            // correct, so a press leaves it alone.
            let text = state
                .copy_text
                .as_ref()
                .map(|(_, _, text)| text.clone())
                .unwrap_or_else(|| pattern_text(shared, &slot));
            ui.ctx().copy_text(text.clone());
            state.copied = Some(text);
            state.notify(format!("copied {slot}"));
        }
    }
    {
        // PASTE asks egui for a paste, so the event path that `Ctrl+V` uses does the work — including refusing when a text
        // field has focus. A key with its own paste would be a second behaviour to keep in step.
        let paste_cell = switches::cell(l.switches, switches::Switch::Paste);
        if switch_key(
            ui,
            state,
            Key {
                id: "paste-pattern",
                rect: paste_cell,
                label: "PASTE",
                lit: false,
                control: tips::Control::Paste,
            },
            p,
        ) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
    }
    {
        // SAVE writes the session; whether it worked is `main`'s to know, so it asks and leaves `dirty` alone.
        let save_cell = switches::cell(l.switches, switches::Switch::Save);
        if switch_key(
            ui,
            state,
            Key {
                id: "save",
                rect: save_cell,
                label: "SAVE",
                lit: false,
                control: tips::Control::Save,
            },
            p,
        ) {
            state.save_requested = true;
        }
    }
    {
        let songs_cell = switches::cell(l.switches, switches::Switch::Banks);
        // The card lists what is on disk, so opening it reads the songs directory once — in `open_banks`, because the
        // session name is the second way in and the two must not drift.
        if switch_key(
            ui,
            state,
            Key {
                id: "songs",
                rect: songs_cell,
                label: "BANKS",
                lit: state.songs.card.is_some(),
                control: tips::Control::Banks,
            },
            p,
        ) {
            open_banks(state);
        }
    }
    {
        let step_cell = switches::cell(l.switches, switches::Switch::Step);
        // STEP mode: a note typed on a keyboard writes the selected step, then walks to the next one.
        if switch_key(
            ui,
            state,
            Key {
                id: "step-mode",
                rect: step_cell,
                label: "STEP",
                lit: state.step_mode,
                control: tips::Control::Step,
            },
            p,
        ) {
            state.step_mode = !state.step_mode;
        }
    }
    // The transport row: the seven keys under the switches, drawn after everything above them, so a later widget wins a
    // hit-test tie and the panel behaves in the order it always has.
    row_transport::draw(state, shared, ui, &l, p);

    // The help dot and the settings gear, side by side at the end of the wordmark: two round buttons that need no
    // explanation of their own. Neither is clipped: the top strip's clip belongs to the name and the notice, not to them.
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
    painter.circle_stroke(help_dot.center(), 8.0 * s, p.silkscreen_stroke(s));
    painter.text(
        help_dot.center(),
        egui::Align2::CENTER_CENTER,
        "?",
        theme::legend(11.0 * s),
        p.silkscreen,
    );
    paint_gear(painter, gear_dot.center(), s, p);
    // The card must not re-close on the click that opened it, so dismissal reads the frame-start value.
    let help_was_visible = state.help_visible;
    if gear_response.clicked() {
        // One card at a time: the gear asks for settings, and every other card goes — the kit browser included, or Escape
        // would give up a card nobody can see.
        state.put_cards_away();
        state.settings_open = true;
    }
    if help_response.clicked() && !help_was_visible {
        state.put_cards_away();
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
        // Which machine the panel is showing, so the card's armed rows warn about the right thing: a 303's demo row replaces
        // the session, a drum machine's replaces that device's grid.
        let kind = state.showing_kind();
        if let Some(card) = state.songs.card.as_mut() {
            request = banks::paint(ui, l.panel, s, card, session.as_deref(), state.dirty, kind);
        }
        if shield.clicked() {
            request = Some(banks::Request::Close);
        }
        match request {
            Some(banks::Request::Close) => state.songs.card = None,
            Some(other) => state.songs.request = Some(other),
            None => {}
        }
    }
    // The kit browser is the device card's sibling — the card's `KIT` key opens it — and it is painted before the device
    // card because only one card is ever up, and the card is what opens over everything else.
    if state.kits.open {
        let shield = modal::shield(ui, window, "kits-shield");
        let request = kits::paint(ui, state, shared, l.panel, s);
        if shield.clicked() {
            kits::close(state);
        } else {
            match request {
                Some(kits::Request::Close) => kits::close(state),
                Some(other) => state.kits_request = Some(other),
                None => {}
            }
        }
    }
    // Both device cards are painted last, the picker over the device card, so nothing draws over either. They are here
    // rather than inside the settings card's `draw_settings` because neither is a fold of anything: the card is about one
    // device, and the picker is the way to make another.
    device_card::paint(ui, state, l.panel, s);
    // The picker is the last thing painted of all: it is the card that adds a device, so it is the one that opens over
    // everything else and the one Escape closes first.
    device_card::paint_picker(ui, state, l.panel, s);
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
    // A focused text field owns every key on the keyboard: `wants_keyboard_input` is egui's own answer to "is something
    // being typed into", and it is asked here rather than at the call site so a later keyboard reader cannot forget it.
    // Without it, typing a capital into the device card's name field wrote a step and marked the session unsaved.
    if ui.ctx().text_edit_focused() {
        return;
    }
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
                    // A card over the face is modal, the same rule the step editor's arrows and `Enter` keep: behind a card a
                    // note key is an audition, not a step write. The key is still remembered (`fresh`) so it walks nothing.
                    let editing = (state.step_mode || shift) && !state.a_card_is_up();
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
                    if keys::release(&mut state.placed, key) && state.step_mode && !state.a_card_is_up() {
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
fn paint_switch(
    painter: &egui::Painter,
    rect: egui::Rect,
    scale: f32,
    label: &str,
    p: &theme::Palette,
) {
    painter.rect_filled(rect, 4.0 * scale, p.panel_raised);
    painter.rect_stroke(
        rect,
        4.0 * scale,
        p.silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );
    if !label.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            theme::legend(12.0 * scale),
            p.raised_ink,
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
fn paint_gear(painter: &egui::Painter, centre: egui::Pos2, scale: f32, p: &theme::Palette) {
    let stroke = p.silkscreen_stroke(scale);
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
    painter.circle_filled(centre, 2.0 * scale, p.silkscreen_dim);
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

/// Open the bank browser, or close it: the one rule for the card, whose two doors are the `BANKS` key and the session name.
///
/// Opening reads the songs directory, so it is not something to do twice a frame; nothing else remembers what is on disk.
fn open_banks(state: &mut PanelState) {
    if state.songs.card.is_some() {
        state.songs.card = None;
        return;
    }
    // One card at a time, in the one place that says so: the browser goes too, or Escape would give up a card nobody can see.
    state.put_cards_away();
    state.songs.card = Some(banks::Banks {
        names: crate::persist::list_songs(&crate::persist::data_dir()),
        // The demos are built into the binary, so the card only needs their names to list them.
        demos: crate::demo::songs()
            .iter()
            .map(|demo| demo.name.to_string())
            .collect(),
        name: state.songs.name.clone().unwrap_or_default(),
        open: state.songs.file.clone(),
        ..banks::Banks::default()
    });
}

/// The pattern on the panel, as the clipboard's text — the same text, `from` included, that `Ctrl+C` writes. A copy that did
/// not name its slot pastes as "a pattern", which is the notice a build that could not say would give.
fn pattern_text(shared: &SharedState, slot: &str) -> String {
    crate::clipboard::encode(&crate::clipboard::Clip::Pattern {
        pattern: *shared.pattern(),
        from: Some(slot.to_string()),
    })
}
