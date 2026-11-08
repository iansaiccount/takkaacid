use std::sync::Arc;
use std::time::Duration;

use acid_app::audio::{self, Rig};
use acid_app::midi;
use acid_app::persist;
use acid_app::state::{self, AudioStatus};
use acid_app::ui as panel_ui;
use acid_core::bank::PatternBank;

use eframe::egui;

fn main() -> eframe::Result {
    // `--midi-map` answers "is it my keyboard or the app?" without opening a window or an audio device: it prints what
    // the app makes of every note number, so a player can compare it with the note they pressed.
    if std::env::args().any(|argument| argument == "--midi-map") {
        print_midi_map();
        return Ok(());
    }
    // Restore the last session; a first run opens on the factory line.
    let saved = persist::load_or_default(&persist::data_dir());
    let state = persist::saved_to_shared(&saved);
    let bank = saved.bank.clone();
    // The MIDI thread reads this to know which bank a pattern controller is pointing into.
    state.set_pattern_index(bank.current());
    // The MIDI input the last session used, if any: a missing port is reported, never swapped for another.
    let midi_preference = match saved.midi_input.as_deref() {
        Some(name) => midi::ports::PortChoice::Named(name.to_string()),
        None => midi::ports::PortChoice::Any,
    };
    let song_name = saved.song.clone();
    // Copied out before the closure takes its other fields: a `bool` needs no ceremony to travel.
    let saved_tooltips = saved.tooltips;
    // The randomise dials are panel state too, so they travel the same way.
    let saved_density = saved.density.clamp(0.0, 1.0);
    let saved_scale = panel_ui::scale::Scale::from_index(saved.scale as usize);

    // One engine per stream: rebuilt at the device's rate whenever the stream reopens.
    let rig = Rig::new({
        let state = Arc::clone(&state);
        move |sample_rate| {
            let state = Arc::clone(&state);
            // Mono out: `Opened::spawn` duplicates every sample to every channel.
            let mut renderer =
                audio::renderer::AudioRenderer::new(sample_rate, 1, state, audio::MAX_BLOCK);
            Box::new(move |buf: &mut [f32]| renderer.render_mono(buf))
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(panel_ui::theme::WINDOW_SIZE)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "takka-acid",
        options,
        Box::new(move |_cc| {
            let panel = panel_ui::PanelState {
                pattern: state.pattern(),
                // The saved session may have been left on another pattern, so the display starts from the bank.
                pattern_id: PatternBank::id_of(bank.current()),
                // The song the last session was working on, if it had a name.
                song_name: song_name.clone(),
                // The hover-help switch and the randomise dials the last session was left with.
                tooltips: saved_tooltips,
                density: saved_density,
                scale: saved_scale,
                scale_knob: panel_ui::scale::scale_position(saved_scale),
                ..Default::default()
            };
            Ok(Box::new(TakkaApp {
                rig,
                state,
                panel,
                bank,
                autosave: persist::Autosave::new(),
                midi: Midi::new(midi_preference),
            }))
        }),
    )
}

struct TakkaApp {
    rig: Rig,
    state: Arc<state::SharedState>,
    panel: panel_ui::PanelState,
    /// The bank as loaded from disk; only the playing pattern is kept in `state`.
    bank: PatternBank,
    autosave: persist::Autosave,
    midi: Midi,
}

impl eframe::App for TakkaApp {
    /// Runs every frame, even when the window is hidden: audio supervision lives here.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rig.poll();
        let status = self.state.status();
        let text = match &status {
            AudioStatus::Starting => "starting".to_string(),
            AudioStatus::Running { .. } => self.rig.status().to_string(),
            AudioStatus::Failed { reason } => format!("failed: {reason}"),
        };
        if text != self.rig.status() {
            self.state.set_status(match &status {
                AudioStatus::Running {
                    device,
                    sample_rate,
                } => AudioStatus::Running {
                    device: device.clone(),
                    sample_rate: *sample_rate,
                },
                other => other.clone(),
            });
        }
        // Keyboard shortcuts. F1 and `?` join this list when the help card lands (Task 54).
        // Escape closes whatever is open, help first: it is the thing covering the panel.
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.panel.help_visible {
                self.panel.help_visible = false;
            } else if self.panel.songs.is_some() {
                self.panel.songs = None;
            } else if self.panel.learn != panel_ui::learn::Step::Idle {
                // Escape gives up on a learn in progress; the shared flag must fall with it.
                self.panel.learn = panel_ui::learn::Step::Idle;
                panel_ui::learn::cancel(&self.state);
            } else {
                self.panel.device_list = None;
            }
        }
        // One press, one action: the step editor first, then the transport and pattern shortcuts (see
        // `ui::dispatch`). A held key repeats only for the tempo nudges.
        let presses =
            ctx.input(|input| panel_ui::dispatch::presses(&input.events, input.modifiers.shift));
        for press in presses {
            panel_ui::dispatch::handle(press, &mut self.panel, &self.state, &mut self.bank);
        }

        // A learned pattern controller cannot move the bank from the MIDI thread, so it leaves a request and the panel
        // applies it here, through the same path the shift+arrow shortcut uses.
        if let Some(index) = self.state.take_pattern_request() {
            panel_ui::shortcuts::select_pattern(&self.state, &mut self.bank, index);
            self.panel.pattern = self.state.pattern();
            self.panel.pattern_id = PatternBank::id_of(self.bank.current());
            self.state.set_pattern_index(self.bank.current());
            self.panel.dirty = true;
        }
        // MIDI: collect a finished probe, then start another one if one is due. Probing never blocks this
        // thread — it runs on a spawned thread and reports through a channel.
        // SAVE asks for a write; `save` clears `dirty` when it succeeds and prints when it does not.
        if self.panel.save_requested {
            self.panel.save_requested = false;
            self.save();
        }

        // The songs card: keep the session under a name, or load one. Loading saves what is on screen first, so a
        // load never throws away work that only lived in memory.
        if let Some(request) = self.panel.song_request.take() {
            let dir = persist::data_dir();
            match request {
                panel_ui::songs::Request::Close => {}
                panel_ui::songs::Request::Save(name) => {
                    let mut saved = persist::shared_to_saved(&self.state, self.bank.clone());
                    saved.midi_input = self.midi.preference_name();
                    saved.song = Some(name.clone());
                    saved.tooltips = self.panel.tooltips;
                    saved.density = self.panel.density;
                    saved.scale = self.panel.scale.index() as u8;
                    match persist::save_song(&dir, &name, &saved) {
                        Ok(()) => {
                            self.panel.song_name = Some(name.clone());
                            self.panel.dirty = false;
                            // The session file remembers which song is open.
                            let _ = persist::save(&dir, &saved);
                            if let Some(card) = self.panel.songs.as_mut() {
                                card.names = persist::list_songs(&dir);
                                card.name = name.clone();
                                card.open = Some(name.clone());
                                card.message = Some(format!("saved {name}"));
                            }
                        }
                        Err(error) => {
                            if let Some(card) = self.panel.songs.as_mut() {
                                card.message = Some(format!("could not save: {error}"));
                            }
                        }
                    }
                }
                panel_ui::songs::Request::New => {
                    self.save();
                    let saved = persist::new_song();
                    persist::apply_saved(&saved, &self.state, &mut self.bank);
                    self.panel.pattern = self.state.pattern();
                    self.panel.pattern_id = PatternBank::id_of(self.bank.current());
                    self.state.set_pattern_index(self.bank.current());
                    self.panel.selected = 0;
                    self.panel.song_name = None;
                    self.panel.dirty = true;
                    if let Some(card) = self.panel.songs.as_mut() {
                        card.name.clear();
                        card.open = None;
                        card.names = persist::list_songs(&dir);
                        card.delete_armed = None;
                        card.message = Some("new song".to_string());
                    }
                }
                panel_ui::songs::Request::Rename(to) => {
                    if let Some(from) = self.panel.song_name.clone() {
                        match persist::rename_song(&dir, &from, &to) {
                            Ok(()) => {
                                self.panel.song_name = Some(to.clone());
                                // The session file remembers the new name, or the next restart would look for the old one.
                                self.save();
                                if let Some(card) = self.panel.songs.as_mut() {
                                    card.names = persist::list_songs(&dir);
                                    card.name = to.clone();
                                    card.open = Some(to.clone());
                                    card.message = Some(format!("renamed {from} to {to}"));
                                }
                            }
                            Err(error) => {
                                if let Some(card) = self.panel.songs.as_mut() {
                                    card.message = Some(format!("could not rename: {error}"));
                                }
                            }
                        }
                    }
                }
                panel_ui::songs::Request::Delete(name) => {
                    match persist::delete_song(&dir, &name) {
                        Ok(()) => {
                            // The song is gone; the session is not, but it is no longer that song.
                            if self.panel.song_name.as_deref() == Some(name.as_str()) {
                                self.panel.song_name = None;
                            }
                            if let Some(card) = self.panel.songs.as_mut() {
                                card.names = persist::list_songs(&dir);
                                card.delete_armed = None;
                                if card.open.as_deref() == Some(name.as_str()) {
                                    card.open = None;
                                }
                                card.message = Some(format!("deleted {name}"));
                            }
                        }
                        Err(error) => {
                            if let Some(card) = self.panel.songs.as_mut() {
                                card.message = Some(format!("could not delete: {error}"));
                            }
                        }
                    }
                }
                panel_ui::songs::Request::Load(name) => match persist::load_song(&dir, &name) {
                    Ok(saved) => {
                        self.save();
                        persist::apply_saved(&saved, &self.state, &mut self.bank);
                        self.panel.pattern = self.state.pattern();
                        self.panel.pattern_id = PatternBank::id_of(self.bank.current());
                        self.state.set_pattern_index(self.bank.current());
                        self.panel.selected = 0;
                        self.panel.dirty = false;
                        self.panel.song_name = Some(name.clone());
                        // The randomise dials are a song's own settings, so they come back with it.
                        self.panel.set_density(saved.density);
                        self.panel
                            .set_scale(panel_ui::scale::Scale::from_index(saved.scale as usize));
                        if let Some(port) = saved.midi_input.clone() {
                            self.midi.choose(midi::ports::PortChoice::Named(port));
                        }
                        if let Some(card) = self.panel.songs.as_mut() {
                            card.names = persist::list_songs(&dir);
                            card.name = name.clone();
                            card.open = Some(name.clone());
                            card.message = Some(format!("loaded {name}"));
                        }
                    }
                    Err(error) => {
                        if let Some(card) = self.panel.songs.as_mut() {
                            card.message = Some(format!("could not load: {error}"));
                        }
                    }
                },
            }
        }
        // A port chosen on the panel: drop the old connection and probe the new one now.
        if let Some(choice) = self.panel.midi_port_choice.take() {
            self.midi.choose(choice);
        }
        // MIDI: collect a finished probe, then start another one if one is due. Probing never blocks this
        // thread — it runs on a spawned thread and reports through a channel.
        self.midi.poll();
        if self.panel.reprobe_requested {
            self.panel.reprobe_requested = false;
            self.midi.reprobe();
        }
        self.midi.probe_if_due(Arc::clone(&self.state));
        self.panel.midi_status = midi::supervisor::status_line(
            self.midi.name.as_deref(),
            self.midi.supervisor.retries(),
        );
        self.panel.midi_given_up = self.midi.name.is_none()
            && self.midi.supervisor.retries() >= midi::supervisor::MIDI_MAX_RETRIES;
        ctx.request_repaint_after(Duration::from_millis(200));
        // Write the session when something changed and the interval has passed.
        if self
            .autosave
            .should_save(self.panel.dirty, std::time::Instant::now())
        {
            self.save();
        }
    }

    /// Closing the window is one last chance to keep whatever the user just did.
    fn on_exit(&mut self) {
        self.save();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        panel_ui::draw_panel(&mut self.panel, &self.state, ui);

        // The audio status doubles as the device picker: click the line to list the outputs, click a name
        // to move the stream there.
        let status_rect = egui::Rect::from_center_size(
            rect.center_bottom() - egui::vec2(0.0, 8.0),
            egui::vec2(rect.width().min(420.0), 18.0),
        );
        // The status line toggles the list, so a second click on it is a dismissal and not a reopen.
        if ui
            .interact(
                status_rect,
                ui.scope_id().with("audio-status"),
                egui::Sense::click(),
            )
            .clicked()
        {
            self.panel.device_list = if self.panel.device_list.is_some() {
                None
            } else {
                Some(audio::list_output_devices())
            };
        }
        ui.painter().text(
            status_rect.center(),
            egui::Align2::CENTER_CENTER,
            self.rig.status(),
            panel_ui::theme::legend(11.0),
            panel_ui::theme::SILKSCREEN_DIM,
        );
        if let Some(devices) = self.panel.device_list.clone() {
            let row_height = 20.0;
            let width = 360.0;
            let base = status_rect.top() - 6.0;
            let mut chosen = None;
            let mut rows = Vec::with_capacity(devices.len());
            for (index, name) in devices.iter().enumerate() {
                let row = egui::Rect::from_min_size(
                    egui::Pos2::new(
                        rect.center().x - width / 2.0,
                        base - (index as f32 + 1.0) * row_height,
                    ),
                    egui::vec2(width, row_height),
                );
                rows.push(row);
                ui.painter()
                    .rect_filled(row, 2.0, panel_ui::theme::PANEL_RAISED);
                ui.painter().text(
                    row.center(),
                    egui::Align2::CENTER_CENTER,
                    audio::status::device_label(name, 44),
                    panel_ui::theme::legend(11.0),
                    panel_ui::theme::SILKSCREEN,
                );
                if ui
                    .interact(
                        row,
                        ui.scope_id().with(("device", index)),
                        egui::Sense::click(),
                    )
                    .clicked()
                {
                    chosen = Some(name.clone());
                }
            }
            if let Some(name) = chosen {
                self.panel.device_list = None;
                self.panel.device_choice = Some(name);
            } else {
                // A click outside the list takes the list away without touching the stream.
                let click = ui.input(|i| {
                    if i.pointer.any_click() {
                        i.pointer.interact_pos()
                    } else {
                        None
                    }
                });
                if panel_ui::picker::should_close(click, &rows, status_rect) {
                    self.panel.device_list = None;
                }
            }
        }
        if let Some(name) = self.panel.device_choice.take() {
            self.panel.device_list = None;
            self.rig.request_device(Some(name));
        }
    }
}

impl TakkaApp {
    /// Write the session, keeping the other 63 patterns from the bank as loaded.
    fn save(&mut self) {
        let mut saved = persist::shared_to_saved(&self.state, self.bank.clone());
        saved.midi_input = self.midi.preference_name();
        saved.song = self.panel.song_name.clone();
        // The hover-help switch is panel state, so it is carried here rather than in `shared_to_saved`.
        saved.tooltips = self.panel.tooltips;
        saved.density = self.panel.density;
        saved.scale = self.panel.scale.index() as u8;
        match persist::save(&persist::data_dir(), &saved) {
            Ok(()) => self.panel.dirty = false,
            Err(error) => eprintln!("takka-acid: could not save: {error}"),
        }
    }
}

/// Owns the MIDI connection and the probe machinery.
///
/// A probe runs on its own thread and reports through a channel, so a missing or unplugged interface can
/// never block a frame. The message handler runs on midir's thread and only writes atomics.
struct Midi {
    input: Option<midi::ports::MidiIn>,
    supervisor: midi::supervisor::MidiSupervisor,
    pending: Option<std::sync::mpsc::Receiver<Result<midi::ports::MidiIn, String>>>,
    name: Option<String>,
    /// Which port to open. `Any` is the first the system reports; a name never falls back to another port.
    preference: midi::ports::PortChoice,
}

impl Midi {
    fn new(preference: midi::ports::PortChoice) -> Self {
        Self {
            input: None,
            supervisor: midi::supervisor::MidiSupervisor::new(),
            pending: None,
            name: None,
            preference,
        }
    }

    /// Switch ports: drop the open connection, remember the choice and probe straight away.
    fn choose(&mut self, choice: midi::ports::PortChoice) {
        self.input = None;
        self.name = None;
        self.pending = None;
        self.preference = choice;
        self.supervisor.probe_now();
    }

    /// The port name the session should remember, `None` for "any".
    fn preference_name(&self) -> Option<String> {
        self.preference.name().map(str::to_string)
    }

    /// Start a probe unless one is already running or the supervisor is waiting.
    fn probe_if_due(&mut self, state: Arc<state::SharedState>) {
        let now = std::time::Instant::now();
        if self.pending.is_some() || !self.supervisor.should_probe(now) {
            return;
        }
        // While a port is open, only ask whether it is still there. Opening a second connection to find out would make
        // every note arrive twice, which is exactly the "one press writes two steps" report.
        if let Some(name) = self.name.clone() {
            if midi::ports::is_listed(&name) {
                self.supervisor.on_success(now);
                return;
            }
            self.input = None;
            self.name = None;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        let choice = self.preference.clone();
        std::thread::spawn(move || {
            let mut follower = midi::tempo::TempoFollower::new();
            let mut gate = midi::gate::NoteGate::new();
            let result = midi::ports::MidiIn::open(&choice, move |_, bytes| {
                // The raw bytes are the ground truth: they are what the keyboard actually sent, before the app reads
                // anything into them. Looking at them is how a player tells a transposed keyboard from a wrong mapping.
                acid_app::trace::note("midi raw", || {
                    let hex: Vec<String> = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
                    format!("{} ({} bytes)", hex.join(" "), bytes.len())
                });
                let Some(event) = midi::message::parse(bytes) else {
                    return;
                };
                match event {
                    midi::message::MidiEvent::Clock => {
                        if let Some(bpm) = follower.on_clock(std::time::Instant::now()) {
                            midi::tempo::follow_clock(&state, bpm);
                        }
                    }
                    midi::message::MidiEvent::ControlChange { cc, value } => {
                        midi::cc::apply_cc(&state, cc, value);
                    }
                    other => {
                        // One note-on per press: a note already down is the same note delivered twice, not a second
                        // press, and it must not write a second step.
                        if midi::gate::passes(&mut gate, other) {
                            midi::message::handle(other, &state);
                        }
                    }
                }
            })
            .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
    }

    /// Collect a finished probe, if any, and update the supervisor.
    fn poll(&mut self) {
        let Some(rx) = &self.pending else {
            return;
        };
        let Ok(result) = rx.try_recv() else {
            return;
        };
        self.pending = None;
        match result {
            Ok(input) => {
                self.name = Some(input.port_name().to_string());
                self.input = Some(input);
                self.supervisor.on_success(std::time::Instant::now());
            }
            Err(message) => {
                if self.name.is_some() {
                    // The port went away: drop the connection so nothing is left holding it.
                    self.input = None;
                    self.name = None;
                }
                let action = self.supervisor.on_error(std::time::Instant::now());
                if matches!(action, midi::supervisor::MidiAction::GiveUp) {
                    eprintln!("takka-acid: {message} — press the amber dot to retry");
                }
            }
        }
    }

    /// Try again now, whatever the supervisor thinks.
    fn reprobe(&mut self) {
        self.supervisor.probe_now();
    }
}

/// Print what the app makes of each MIDI note, so a player can compare it with the note they pressed.
///
/// Two names appear per line on purpose. `keyboard` is what the note is called on a keyboard (MIDI 60 is C4); `app` is
/// what the panel stores and shows, which comes from the instrument's own register — its C sits at MIDI 33, nine
/// semitones below a keyboard's, so the app's name runs three semitones sharp of the keyboard's. Seeing both is the only
/// way to tell a transposed keyboard from a mismatched register.
fn print_midi_map() {
    println!("note  keyboard  app writes  app shows  sounds");
    for note in 21..=108u8 {
        let (pitch, octave) = acid_app::ui::edit::from_midi_note(note);
        let label = acid_app::ui::edit::pitch_name(pitch, octave);
        let hz = acid_core::pitch::step_hz(pitch, octave);
        let solid = if note % 12 == 0 { " <-- a C key" } else { "" };
        println!(
            "{note:>4}  {:>8}  {pitch:>9?} {octave:>2}  {label:>9}  {hz:>7.1} Hz{solid}",
            midi::message::concert_name(note)
        );
    }
}

#[cfg(test)]
mod tests {
    use acid_core::engine::Engine;
    use acid_core::factory::launcher;
    use std::sync::Arc;

    /// Offline stand-in for the plan's "listen to the loop" step: nothing in the render may jump hard
    /// Offline stand-in for the plan's "listen to the loop" step: the wrap from step 15 back to step 0 must not jump.
    ///
    /// The measurement is taken *at the wrap*, not over the whole buffer: an accent's filter kick is a deliberately
    /// steep, bright attack, and a global maximum would report that as a click. A discontinuity at the wrap would show up
    /// as a step between adjacent samples no other part of the phrase produces.
    #[test]
    fn the_demo_pattern_loops_without_a_click_at_the_wrap() {
        let sr = 48_000.0;
        let mut e = Engine::new(sr);
        e.set_pattern(Arc::new(launcher()));
        e.set_bpm(130.0);
        e.start();
        // 16 steps at 130 bpm.
        let loop_len = (sr * 60.0 / (130.0 * 4.0) * 16.0).round() as usize;
        let mut buf = vec![0.0; loop_len * 2];
        e.render(&mut buf);

        // The steepest step anywhere in the phrase, and the steepest one right at the wrap.
        let steepest = |slice: &[f32]| {
            slice
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).abs())
                .fold(0.0f32, f32::max)
        };
        let anywhere = steepest(&buf);
        let at_wrap = steepest(&buf[loop_len - 64..=loop_len + 64]);
        let elsewhere = steepest(&buf[..loop_len - 64]);

        assert!(
            anywhere < 0.6,
            "the phrase should not clip or tear: {anywhere}"
        );
        assert!(
            at_wrap <= elsewhere * 1.1,
            "the wrap is a discontinuity the phrase never makes elsewhere: {at_wrap} vs {elsewhere}"
        );
        assert!(
            buf[loop_len - 1_000..loop_len + 1_000]
                .iter()
                .any(|s| s.abs() > 0.01),
            "the loop wrap should be in the middle of a phrase, not in silence"
        );
    }
}
