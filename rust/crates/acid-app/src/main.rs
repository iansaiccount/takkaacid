use std::sync::Arc;
use std::time::Duration;

use acid_app::audio::{self, Rig};
use acid_app::demo;
use acid_app::device;
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
    // Restore the last session. A first run opens on a demo, under the name a new song starts with, and nothing about it
    // is on disk yet — so the first thing the player sees is a line that plays, and a lamp that says "keep me".
    let (saved, first_run) = persist::open(&persist::data_dir());
    let rack = device::Rack::from_saved(&saved);
    // The MIDI thread reads this to know which bank a pattern controller is pointing into.
    rack.state().set_pattern_index(rack.bank().current());
    // The MIDI inputs the last session had open, if any: a missing port is reported, never swapped for another. A session
    // that saved no name opens the first port the machine reports, filled in on the first probe.
    let midi_enabled: Vec<String> = saved.midi_input.clone().into_iter().collect();
    let song_name = saved.song.clone();
    // Copied out before the closure takes its other fields: a `bool` needs no ceremony to travel.
    let saved_tooltips = saved.tooltips;
    // Saving is manual unless the player turned autosave on, so the switch is carried like the rest.
    let saved_autosave = saved.autosave;
    // The randomise dials are panel state too, so they travel the same way.
    let saved_density = saved.density.clamp(0.0, 1.0);
    let saved_scale = panel_ui::scale::Scale::from_index(saved.scale as usize);

    // One mix bus per stream: rebuilt at the device's rate whenever the stream reopens. Every device's engine lives in it, so
    // the rack can add or remove one without the stream noticing.
    let bus = rack.bus();
    let rig = Rig::new(move |sample_rate| {
        let mut mixer = audio::mixer::Mixer::new(sample_rate, audio::MAX_BLOCK, Arc::clone(&bus));
        // Mono out: `Opened::spawn` duplicates every sample to every channel.
        Box::new(move |buf: &mut [f32]| mixer.render_mono(buf))
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
                pattern: rack.state().pattern(),
                // The saved session may have been left on another pattern, so the display starts from the bank.
                pattern_id: PatternBank::id_of(rack.bank().current()),
                // The song the last session was working on, if it had a name.
                songs: panel_ui::SongState {
                    name: song_name.clone(),
                    ..panel_ui::SongState::default()
                },
                // A first run has nothing on disk, so the lamp says "keep me" until the player does.
                dirty: first_run,
                // The hover-help switch and the randomise dials the last session was left with.
                tooltips: saved_tooltips,
                autosave: saved_autosave,
                density: saved_density,
                scale: saved_scale,
                scale_knob: panel_ui::scale::scale_position(saved_scale),
                ..Default::default()
            };
            // The MIDI side takes the two cells it needs before the rack moves into the app: a router writes the tempo cell
            // and loads the published bus, and never sees the rack itself.
            let tempo_cell = rack.tempo_cell();
            let midi_bus = rack.bus();
            Ok(Box::new(TakkaApp {
                rig,
                rack,
                panel,
                autosave: persist::Autosave::new(),
                settings_was_open: false,
                card_filled: None,
                midi: Midi::new(midi_enabled, tempo_cell, midi_bus),
                pending_pattern: None,
                last_position: 0,
            }))
        }),
    )
}

struct TakkaApp {
    rig: Rig,
    /// The devices, the selection and the transport. Only the selected device's pattern lives in its own `state`.
    rack: device::Rack,
    panel: panel_ui::PanelState,
    autosave: persist::Autosave,
    /// Whether the settings card was open last frame, so its device lists are filled on the rising edge and not per frame.
    settings_was_open: bool,
    /// Which device the card was filled for, or `None` while it is closed. The name and the port list are the round trip —
    /// typing into the name must not be interrupted by a re-read, and `list_ports()` costs a system call — so they are read
    /// the frame the card opens, whichever of the caret and F3 opened it. The *route* is not: `logic` re-reads it every
    /// frame from the rack, because a tick or a channel is the store's answer and must follow a click immediately.
    card_filled: Option<usize>,
    midi: Midi,
    /// A pattern a learned controller asked for, waiting for the bar to come round.
    pending_pattern: Option<usize>,
    /// The transport step seen last frame, so a bar crossed between frames is still noticed.
    last_position: usize,
}

impl eframe::App for TakkaApp {
    /// Runs every frame, even when the window is hidden: audio supervision lives here.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rig.poll();
        // The transport belongs to the rack: the panel asks for a change rather than writing one device's atomic, which the
        // fan-out would immediately undo.
        if let Some(request) = self.panel.transport_request.take() {
            match request {
                panel_ui::transport::Request::Play => self
                    .rack
                    .set_transport(acid_app::device::Transport::Playing),
                panel_ui::transport::Request::Pause => {
                    self.rack.set_transport(acid_app::device::Transport::Paused)
                }
                panel_ui::transport::Request::Stop => self
                    .rack
                    .set_transport(acid_app::device::Transport::Stopped),
                // The two performers: a direction this device's line runs in, and a semitone for the whole rack. Both are
                // performance, neither is saved, and the engine takes the direction at the bar line.
                panel_ui::transport::Request::Reverse(reversed) => {
                    if let Some(device) = self.rack.devices().get(self.panel.strip.selected) {
                        device.state.set_reversed(reversed);
                    }
                }
                panel_ui::transport::Request::Transpose(semitones) => {
                    let next = (self.rack.transpose() + semitones).clamp(-12, 12);
                    self.rack.set_transpose(next);
                }
                panel_ui::transport::Request::NudgeTempo(direction) => {
                    let next = panel_ui::transport::nudge_tempo(self.rack.tempo_bpm(), direction);
                    self.rack.set_tempo_bpm(next);
                }
            }
        }
        let status = self.rack.state().status();
        let text = match &status {
            AudioStatus::Starting => "starting".to_string(),
            AudioStatus::Running { .. } => self.rig.status().to_string(),
            AudioStatus::Failed { reason } => format!("failed: {reason}"),
        };
        if text != self.rig.status() {
            self.rack.state().set_status(match &status {
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
        // Escape gives up on one thing at a time; the order lives in `ui::dispatch` so it can be tested. It is *taken* out of
        // the frame for the same reason the presses are: `key_pressed` counts presses over the event buffer, and a logic-only
        // tick is handed the last pass's buffer again, so a read would close one card per 200 ms.
        let typing = ctx.text_edit_focused();
        let escape =
            ctx.input_mut(|input| panel_ui::dispatch::take_escape(&mut input.events, typing));
        if escape {
            panel_ui::dispatch::escape(&mut self.panel, self.rack.state());
        }
        // One press, one action: the step editor first, then the transport and pattern shortcuts (see
        // `ui::dispatch`). A held key repeats only for the tempo nudges.
        //
        // The presses are *taken* out of the frame, not read: a logic-only tick (a minimised window) is handed the last
        // pass's events again, and the panel repaints every 200 ms, so a reader would replay the same Space or arrow until
        // the window came back. Nothing is taken while a text field has the keyboard — the field's letters are letters, and
        // its space bar is a space rather than the transport. `ui::mod`'s audition keys ask the same question, and both must,
        // or a name typed into a card plays notes and stops the transport.
        let presses =
            ctx.input_mut(|input| panel_ui::dispatch::take_presses(&mut input.events, typing));
        let pattern_before = self.rack.bank().current();
        for press in presses {
            let device = self.rack.device_mut();
            panel_ui::dispatch::handle(press, &mut self.panel, &device.state, &mut device.bank);
        }
        // The keyboard is deliberate and immediate: a move it made wins over a swap that was waiting for the bar.
        if self.rack.bank().current() != pattern_before {
            self.pending_pattern = None;
        }
        // The clipboard, which is not a key: egui-winit turns Ctrl+C/Ctrl+V into Copy/Paste events and never delivers the
        // press, so both gestures are answered here. The rule about a focused text field is the one the keys use — the
        // card's name field is a field, and its clipboard is its own — and `dispatch::clipboard` is where the behaviour
        // lives, because `main` is a binary no test can call.
        //
        // The events are *taken*, not read: a logic-only tick (a minimised window) is handed the last pass's events again,
        // and the panel repaints every 200 ms, so a reader would re-dispatch the same paste until the window came back.
        let clip_events =
            ctx.input_mut(|input| panel_ui::dispatch::take_clipboard(&mut input.events, typing));
        let copying = {
            let device = self.rack.device_mut();
            panel_ui::dispatch::clipboard(
                &clip_events,
                typing,
                &mut self.panel,
                &device.state,
                &mut device.bank,
            )
        };
        if let Some(text) = copying {
            ctx.copy_text(text);
        }

        // A learned pattern controller cannot move the bank from the MIDI thread, so it leaves a request and this is
        // where it lands. A pattern is a bar long: the swap waits for the transport to come round, so cycling with the
        // knob rides the groove instead of restarting it. The readout follows the held selection, so the player can see
        // where they are going before it is heard.
        if let Some(index) = self.rack.state().take_pattern_request() {
            if index == self.rack.bank().current() {
                // A fader resting inside the zone it is already on asks for nothing; treating it as a swap would
                // re-publish the pattern and mark the session unsaved every bar.
                self.pending_pattern = None;
                self.rack.state().clear_pattern_selected();
            } else {
                // The selection is what the controller is holding from now on, even though the sound is still the old
                // pattern: the pickup measures against this, or the next message would look like a hand on the knob.
                self.rack.state().set_pattern_selected(index);
                self.pending_pattern = Some(index);
                self.panel.pattern_id = PatternBank::id_of(index);
            }
        }
        let position = self
            .state()
            .position
            .load(std::sync::atomic::Ordering::Relaxed);
        if let Some(index) = self.pending_pattern
            && panel_ui::pattern_swap::swap_now(
                self.rack.state().is_playing(),
                position,
                self.last_position,
            )
        {
            let device = self.rack.device_mut();
            panel_ui::shortcuts::select_pattern(&device.state, &mut device.bank, index);
            panel_ui::pattern_swap::adopt(&mut self.panel, self.rack.state(), self.rack.bank());
            self.panel.dirty = true;
            self.pending_pattern = None;
        }
        self.last_position = position;
        // thread — it runs on a spawned thread and reports through a channel.
        // SAVE asks for a write; `save` clears `dirty` when it succeeds and prints when it does not.
        if self.panel.save_requested {
            self.panel.save_requested = false;
            self.save();
        }

        // The songs card: keep the session under a name, or load one. Loading saves what is on screen first, so a
        // load never throws away work that only lived in memory.
        if let Some(request) = self.panel.songs.request.take() {
            let dir = persist::data_dir();
            match request {
                panel_ui::banks::Request::Close => {}
                panel_ui::banks::Request::Save(typed) => {
                    // One button: an unchanged name keeps the song, a new name writes a new file and leaves the old one
                    // where it is, so a save can never move a song out from under its own name.
                    match persist::save_target(self.panel.songs.file.as_deref(), &typed) {
                        Some(persist::SaveTarget::InPlace(name)) => {
                            self.write_song(&dir, &name, false);
                        }
                        Some(persist::SaveTarget::NewFile(name)) => {
                            // A new name must not land on a song nobody opened: it lands beside it, the way an import
                            // does, so SAVE can never destroy a song by being given its name.
                            let name = persist::free_song_name(&dir, &name);
                            self.write_song(&dir, &name, true);
                        }
                        None => {}
                    }
                }
                panel_ui::banks::Request::New => {
                    self.save();
                    let saved = persist::new_song();
                    let device = self.rack.device_mut();
                    persist::apply_saved(&saved, &device.state, &mut device.bank);
                    self.rack.set_tempo_bpm(saved.bpm);
                    panel_ui::pattern_swap::adopt(
                        &mut self.panel,
                        self.rack.state(),
                        self.rack.bank(),
                    );
                    // A held swap belongs to the song that was playing, not to this one.
                    self.pending_pattern = None;
                    self.panel.songs.name = None;
                    self.panel.dirty = true;
                    if let Some(card) = self.panel.songs.card.as_mut() {
                        card.name.clear();
                        card.open = None;
                        self.panel.songs.file = None;
                        card.names = persist::list_songs(&dir);
                        card.delete_armed = None;
                    }
                    self.panel.notify("new song");
                }
                panel_ui::banks::Request::SaveCopy(typed) => {
                    let name = persist::free_song_name(&dir, &typed);
                    self.write_song(&dir, &name, true);
                }
                panel_ui::banks::Request::Import => {
                    let picked = rfd::FileDialog::new()
                        .add_filter("takka-acid song", &["json"])
                        .set_title("Import a song")
                        .pick_file();
                    if let Some(path) = picked {
                        match persist::import_song(&path, &dir) {
                            Ok(name) => {
                                // An import is a load with a copy step in front of it, so it lands the same way a click on
                                // the row would: the library is refreshed and the song becomes the session's own.
                                self.load_song_now(&dir, &name);
                            }
                            Err(error) => self.notice(format!("could not import: {error}")),
                        }
                    }
                }
                panel_ui::banks::Request::Export => {
                    let suggested = self
                        .panel
                        .songs
                        .name
                        .clone()
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| "takka-acid song".to_string());
                    let picked = rfd::FileDialog::new()
                        .add_filter("takka-acid song", &["json"])
                        .set_title("Export the session")
                        .set_file_name(format!("{suggested}.json"))
                        .save_file();
                    if let Some(path) = picked {
                        match persist::export_song(&path, &self.session_for_disk(&suggested)) {
                            Ok(()) => self.notice(format!("exported to {}", path.display())),
                            Err(error) => self.notice(format!("could not export: {error}")),
                        }
                    }
                }
                panel_ui::banks::Request::Delete(name) => {
                    match persist::delete_song(&dir, &name) {
                        Ok(()) => {
                            // The song is gone; the session is not, but it is no longer that song.
                            let was_open = self.panel.songs.file.as_deref() == Some(name.as_str());
                            if self.panel.songs.name.as_deref() == Some(name.as_str()) {
                                self.panel.songs.name = None;
                            }
                            if let Some(card) = self.panel.songs.card.as_mut() {
                                card.names = persist::list_songs(&dir);
                                card.delete_armed = None;
                                if card.open.as_deref() == Some(name.as_str()) {
                                    card.open = None;
                                }
                                if self.panel.songs.file.as_deref() == Some(name.as_str()) {
                                    self.panel.songs.file = None;
                                }
                            }
                            if was_open {
                                // The session outlives its file, but nothing of it is on disk now.
                                self.panel.dirty = true;
                            }
                            self.panel.notify(format!("deleted {name}"));
                        }
                        Err(error) => {
                            self.notice(format!("could not delete: {error}"));
                        }
                    }
                }
                panel_ui::banks::Request::Load(name) => self.load_song_now(&dir, &name),
                panel_ui::banks::Request::LoadDemo(index) => {
                    if let Some(demo) = demo::songs().into_iter().nth(index) {
                        self.save();
                        let device = self.rack.device_mut();
                        persist::apply_saved(&demo.state, &device.state, &mut device.bank);
                        self.rack.set_tempo_bpm(demo.state.bpm);
                        panel_ui::pattern_swap::adopt(
                            &mut self.panel,
                            self.rack.state(),
                            self.rack.bank(),
                        );
                        self.pending_pattern = None;
                        // A demo is not a file: it arrives as a new song under a name, and nothing of it is on disk until
                        // the player renames and saves it.
                        self.panel.songs.name = Some(demo::NEW_SONG.to_string());
                        self.panel.songs.file = None;
                        self.panel.songs.card = None;
                        self.panel.dirty = true;
                        self.panel.tooltips = demo.state.tooltips;
                        self.panel.autosave = demo.state.autosave;
                        self.panel.set_density(demo.state.density);
                        self.panel.set_scale(panel_ui::scale::Scale::from_index(
                            demo.state.scale as usize,
                        ));
                    }
                }
            }
        }
        // A port the card turned on or off has already been opened or closed: `apply_settings_request` does it, because a
        // row is a switch and not a choice waiting for the next frame.

        // The card's two lists come from the machine, so they are read the frame it opens rather than every frame:
        // enumerating ports and outputs costs a round trip each, and a hot-plugged device shows on the next opening.
        if self.panel.settings_open && !self.settings_was_open {
            self.panel.settings.offer(
                midi::ports::list_ports().unwrap_or_default(),
                audio::list_output_devices(),
            );
        }
        self.settings_was_open = self.panel.settings_open;
        // The device card is filled the frame it opens, and not per frame: the machine's port list is a round trip, and the
        // name is being typed into. Whichever of the caret and F3 opened it, this is the one place that reads — the open flag
        // is compared, not the click that set it, so no blank card is ever painted.
        if self.panel.device_card.open != self.card_filled {
            if let Some(index) = self.panel.device_card.open
                && let Some(device) = self.rack.devices().get(index)
            {
                self.panel.device_card.offer(
                    &device.name,
                    &device.inputs,
                    midi::ports::list_ports().unwrap_or_default(),
                );
            }
            self.card_filled = self.panel.device_card.open;
        }
        // Its route is a mirror, though, and read every frame while the card is up: a tick, a channel or a role is the
        // rack's answer, so the card shows what the store holds from the frame after the click that changed it. The name is
        // the one field that is not a mirror, because it is the one field a hand is typing into.
        if let Some(index) = self.panel.device_card.open
            && let Some(device) = self.rack.devices().get(index)
        {
            self.panel.device_card.route_from(&device.inputs);
            // The scheme is a reading of the same device, refreshed the same way: the key on the card shows what the rack
            // holds, so a click on it moves the key on the next frame and not before.
            self.panel.device_card.scheme_from(device.scheme);
        }
        if let Some(request) = self.panel.settings_request.take() {
            self.apply_settings_request(request);
        }
        // MIDI: collect a finished probe, then start another one if one is due. Probing never blocks this
        // thread — it runs on a spawned thread and reports through a channel.
        self.midi.poll();
        if self.panel.reprobe_requested {
            self.panel.reprobe_requested = false;
            self.midi.reprobe();
        }
        self.midi.probe_if_due();
        // The supervisor's own words, and whether it has given up: the settings card shows both, and nowhere else does.
        let connected = self.midi.connected();
        let midi_status = midi::supervisor::status_line(
            self.midi.enabled().len(),
            &connected,
            self.midi.supervisor.retries(),
        );
        // Given up means nothing answered and nothing is being tried: a player who turned every port off is not stuck.
        // Given up means a port was wanted, nothing answered, and nothing is being tried any more. A player who turned
        // every port off is not stuck, so RETRY has nothing to offer them.
        let wanted = !self.midi.enabled().is_empty();
        self.panel.midi_given_up = wanted
            && connected.is_empty()
            && self.midi.supervisor.retries() >= midi::supervisor::MIDI_MAX_RETRIES;
        // Everything else on the settings card is a mirror of the machine, so it is copied while the card is up rather
        // than when it opened: an output the player just chose, a device unplugged, a stream that failed or a supervisor
        // that gives up 30 s later must all show on the card that is already standing. The port named is the connection,
        // not the set held: a controller with its cable out keeps its row, and the line above the list names what answers.
        if self.panel.settings_open {
            let enabled: Vec<&str> = self.midi.enabled().iter().map(String::as_str).collect();
            let status = self.rack.state().status();
            let live = panel_ui::settings::Live {
                midi_current: &connected,
                midi_enabled: &enabled,
                output_current: match &status {
                    AudioStatus::Running { device, .. } => Some(device.as_str()),
                    _ => None,
                },
                midi_status: &midi_status,
                midi_given_up: self.panel.midi_given_up,
                audio_status: self.rig.status(),
            };
            self.panel.settings.follow(live);
        }
        ctx.request_repaint_after(Duration::from_millis(200));
        // Write the session when the player turned autosave on, something changed, and the interval has passed.
        if self.panel.autosave
            && self
                .autosave
                .should_save(self.panel.dirty, std::time::Instant::now())
        {
            self.save();
        }
        // The strip is a mirror like `pattern` is: what the rack holds, copied for the painter. Filled last, after every
        // drain in this frame that could change the rack, so the list the panel draws and the list a click is answered
        // against are the same list.
        self.panel.strip.cells = self
            .rack
            .devices()
            .iter()
            .map(|device| panel_ui::strip::StripCell {
                name: device.name.clone(),
                // The panel's per-device editor slots are keyed by this, so a removal cannot hand one device's step,
                // step mode and randomise dials to its neighbour.
                number: device.number,
                // Each cell wears its own device's colours: the strip is the legend of the rack.
                scheme: device.scheme,
                running: device.state.is_playing(),
                muted: device.state.is_muted(),
            })
            .collect();
        self.panel.strip.selected = self.rack.selected();
        // And the face is the selected device's: the panel paints one machine at a time, in that machine's colours.
        self.panel.scheme = self.rack.device().scheme;
    }

    /// Closing the window is one last chance to keep whatever the user just did.
    fn on_exit(&mut self) {
        self.save();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The panel is the whole surface. Device names used to be written at the bottom of the window and doubled as the
        // output picker; both live in the settings card now, so nothing is painted outside the panel.
        panel_ui::draw_panel(&mut self.panel, self.rack.state(), ui);
        self.apply_device_request();
        if let Some(name) = self.panel.picker.output_choice.take() {
            self.rig.request_device(Some(name));
        }
    }
}

impl TakkaApp {
    /// The selected device's state, as the panel, the shortcuts and the save file want it.
    fn state(&self) -> &state::SharedState {
        self.rack.state()
    }

    /// Do what the settings card asked for. The folds and CLOSE never arrive here — they are the card's own state — and
    /// every case below is reachable, because `Request` only names the work `main` can do.
    fn apply_settings_request(&mut self, request: panel_ui::settings::Request) {
        use panel_ui::settings::Action;
        match request.action() {
            Action::ToggleTooltips => {
                self.panel.tooltips = !self.panel.tooltips;
                self.panel.dirty = true;
            }
            Action::ToggleAutosave => {
                self.panel.autosave = !self.panel.autosave;
                self.panel.dirty = true;
            }
            Action::Retry => self.panel.reprobe_requested = true,
            // A port row opens or closes one input. `enable` probes at once, because the player asked for it.
            Action::TogglePort(name, on) => {
                if on {
                    self.midi.enable(name);
                } else {
                    self.midi.disable(&name);
                }
            }
            Action::SoundOut(name) => self.panel.picker.output_choice = Some(name),
        }
    }

    /// Answer a device request: select, mute, add, rename, route or remove one. The four arms that wait for the device
    /// card are marked and do nothing yet.
    ///
    /// The match is written out in full because `Request` is a catalogue — a new request must fail to compile here — and
    /// because `Rack::add`'s refusal has to be said somewhere: the panel cannot see the rack, so `main` is the only place
    /// that can tell the player the rack is full.
    fn apply_device_request(&mut self) {
        let Some(request) = self.panel.device_request.take() else {
            return;
        };
        match request {
            device::Request::Select(index) => {
                if self.rack.select(index) {
                    // The display follows the device's own bank, but not its pattern's first step: the step the hand was
                    // on comes back from this panel's edit slots in `draw_panel` on the next frame, so resetting it here
                    // would save the *leaving* device at step 1.
                    panel_ui::pattern_swap::follow(
                        &mut self.panel,
                        self.rack.state(),
                        self.rack.bank(),
                    );
                    // A swap a controller asked for belongs to the device it was asked of, not to the one now on screen.
                    self.pending_pattern = None;
                } else {
                    // Only the keyboard can name a device the rack does not hold: `Ctrl+3` on a two-device rack. A cell
                    // click cannot, because the cell was drawn for a device that was there. Saying so beats a dead key.
                    self.notice("no device there".to_string());
                }
            }
            // The index the key was drawn for, not the one on the panel: `SetMute` carries its own.
            device::Request::SetMute(index, muted) => {
                if let Some(device) = self.rack.devices().get(index) {
                    device.state.set_mute(muted);
                }
            }
            device::Request::Add(kind) => match self.rack.add(kind) {
                Ok(index) => {
                    self.rack.select(index);
                    panel_ui::pattern_swap::follow(
                        &mut self.panel,
                        self.rack.state(),
                        self.rack.bank(),
                    );
                    // A held swap belonged to the device that was on screen, not to the new one.
                    self.pending_pattern = None;
                    let name = self.rack.device().name.clone();
                    // Nothing on disk changed: the session saves the first device, and a device added here is not in the
                    // save format yet.
                    self.notice(format!("added {name}"));
                }
                Err(device::AddError::AtCapacity) => {
                    self.notice("four devices is the limit".to_string())
                }
            },
            // The name is the rack's to decide: it trims, falls back to `ACID <number>`, and suffixes a collision. The
            // panel shows the answer, never the typed string, so a `BASS` typed twice reads `BASS (2)` on the card.
            device::Request::Rename(index, typed) => {
                let landed = self.rack.rename(index, &typed);
                // The card shows what the store answered, never what was typed: `BASS` typed twice lands as `BASS (2)`, and
                // a field that kept saying `BASS` would look like a rename that did nothing.
                if self.panel.device_card.open == Some(index) {
                    self.panel.device_card.name = landed;
                }
            }
            // A routing edit replaces one field of the device's filter and keeps the other two: the filter is the only
            // scope a message is filtered by, so nothing else may be rewritten on the way.
            device::Request::SetRole(index, role) => {
                if let Some(mut inputs) = self.rack.inputs(index).cloned() {
                    inputs.role = role;
                    self.rack.set_inputs(index, inputs);
                }
            }
            device::Request::SetPorts(index, ports) => {
                if let Some(mut inputs) = self.rack.inputs(index).cloned() {
                    inputs.ports = ports;
                    self.rack.set_inputs(index, inputs);
                }
            }
            device::Request::SetChannel(index, channel) => {
                if let Some(mut inputs) = self.rack.inputs(index).cloned() {
                    inputs.channel = channel;
                    self.rack.set_inputs(index, inputs);
                }
            }
            // A colour is the panel's business alone: nothing on the bus carries it, so nothing is published.
            device::Request::SetScheme(index, scheme) => self.rack.set_scheme(index, scheme),
            // The last device cannot go, and the panel cannot see the rack, so the refusal is said here.
            device::Request::ConfirmRemove(index) => match self.rack.remove(index) {
                Ok(()) => {
                    // A save writes the first device, so only losing *that* one changes what the file would hold; a
                    // survivor keeps its number and its name, and nothing else a save reads has moved.
                    if index == 0 {
                        self.panel.dirty = true;
                    }
                    // The card was about a device that is gone — and so is its mirror, which has to go with it or the next
                    // frame paints a name, a route and a removal for an index that is not there.
                    self.panel.close_device_card();
                }
                Err(device::RemoveError::LastDevice) => {
                    self.notice("the last device cannot go".to_string())
                }
                Err(device::RemoveError::OutOfRange) => {}
            },
            // The question is the card's own state — `armed_remove` is the two-click switch CLEAR uses — so there is nothing
            // for the rack to do with the first click. The request is still sent, so the queue names everything the card can
            // say and `main`'s drain stays the whole catalogue of it.
            device::Request::AskRemove(_) => {}
            // The caret on the selected cell asks for this. The fill — the device's name, its route and the machine's
            // ports — happens below, on the frame the card opens, whichever of the two opened it.
            device::Request::Open(index) => {
                if index < self.rack.devices().len() {
                    self.panel.open_device_card(index);
                }
            }
            // The card's CLOSE key and its shield call this. `open` is a flag the card could clear itself, but the mirror it
            // is about is `main`'s reading of a device, so closing drops both in the one place that filled them.
            device::Request::Close => self.panel.close_device_card(),
            // The `+` asked for the picker: it is the card that offers the kinds there are, and it closes itself once a row
            // has answered.
            device::Request::OpenPicker => self.panel.open_device_picker(),
        }
    }
    /// Write the session, keeping the other 63 patterns from the bank as loaded.
    fn save(&mut self) {
        let first = self.rack.saved_device();
        let mut saved = persist::shared_to_saved(&first.state, first.bank.clone());
        // One name is all the save file holds until the format lands: the first port the player turned on.
        saved.midi_input = self.midi.enabled().first().cloned();
        saved.song = self.panel.songs.name.clone();
        // The hover-help switch is panel state, so it is carried here rather than in `shared_to_saved`.
        saved.tooltips = self.panel.tooltips;
        saved.autosave = self.panel.autosave;
        saved.density = self.panel.density;
        saved.scale = self.panel.scale.index() as u8;
        let dir = persist::data_dir();
        match persist::save(&dir, &saved) {
            Ok(()) => self.panel.dirty = false,
            Err(error) => eprintln!("takka-acid: could not save: {error}"),
        }
        // A session that came from a song keeps that file current: SAVE means "keep this", not "keep half of it".
        if let Some(name) = self.panel.songs.file.clone()
            && let Err(error) = persist::save_song(&dir, &name, &saved)
        {
            eprintln!("takka-acid: could not save the song {name}: {error}");
        }
    }

    /// Load a song from the library and make it the session's own: the panel's switches are a song's settings, so they
    /// come back with it. Anything unsaved is written first, so a load never throws work away.
    fn load_song_now(&mut self, dir: &std::path::Path, name: &str) {
        let Ok(saved) = persist::load_song(dir, name) else {
            let error = persist::load_song(dir, name).unwrap_err();
            self.notice(format!("could not load: {error}"));
            return;
        };
        self.save();
        let device = self.rack.device_mut();
        persist::apply_saved(&saved, &device.state, &mut device.bank);
        self.rack.set_tempo_bpm(saved.bpm);
        panel_ui::pattern_swap::adopt(&mut self.panel, self.rack.state(), self.rack.bank());
        // A held swap belongs to the song that was playing, not to the one just loaded.
        self.pending_pattern = None;
        self.panel.dirty = false;
        self.panel.songs.name = Some(name.to_string());
        self.panel.songs.file = Some(name.to_string());
        self.panel.tooltips = saved.tooltips;
        self.panel.autosave = saved.autosave;
        self.panel.set_density(saved.density);
        self.panel
            .set_scale(panel_ui::scale::Scale::from_index(saved.scale as usize));
        if let Some(port) = saved.midi_input.clone() {
            // Added to the set rather than replacing it: the file names one port, and dropping the player's other
            // controllers because a song remembered one of them would be a surprise.
            self.midi.enable(port);
        }
        if let Some(card) = self.panel.songs.card.as_mut() {
            card.names = persist::list_songs(dir);
            card.name = name.to_string();
            card.open = Some(name.to_string());
        }
        self.panel.notify(format!("loaded {name}"));
    }

    /// Write the session into the library under `name`, and make that song the session's own. `as_copy` only changes the
    /// words: a copy lands under a name that is free, so saving one never overwrites the song it came from.
    fn write_song(&mut self, dir: &std::path::Path, name: &str, as_copy: bool) {
        let saved = self.session_for_disk(name);
        match persist::save_song(dir, name, &saved) {
            Ok(()) => {
                self.panel.songs.name = Some(name.to_string());
                self.panel.songs.file = Some(name.to_string());
                self.panel.dirty = false;
                // The session file remembers which song is open, or the next restart would open nothing.
                let _ = persist::save(dir, &self.session_for_disk(name));
                if let Some(card) = self.panel.songs.card.as_mut() {
                    card.names = persist::list_songs(dir);
                    card.name = name.to_string();
                    card.open = Some(name.to_string());
                }
                self.notice(if as_copy {
                    format!("saved a copy as {name}")
                } else {
                    format!("saved {name}")
                });
            }
            Err(error) => self.notice(format!("could not save: {error}")),
        }
    }

    /// The session as it should be written to disk, under `name` when the save is landing in a file.
    fn session_for_disk(&self, name: &str) -> persist::SavedState {
        let first = self.rack.saved_device();
        let mut saved = persist::shared_to_saved(&first.state, first.bank.clone());
        saved.midi_input = self.midi.enabled().first().cloned();
        saved.song = Some(name.to_string());
        // The hover-help switch is panel state, so it is carried here rather than in `shared_to_saved`.
        saved.tooltips = self.panel.tooltips;
        saved.autosave = self.panel.autosave;
        saved.density = self.panel.density;
        saved.scale = self.panel.scale.index() as u8;
        saved
    }

    /// Say something on the status line. It survives the card that asked for it, and it clears itself.
    fn notice(&mut self, message: String) {
        self.panel.notify(message);
    }
}

/// What one port's probe found: the name and the connection it opened, or the name and why it did not open.
type ProbeResult = Result<(String, midi::ports::MidiIn), (String, String)>;

/// Whether a probe result should be installed. A port turned off while the probe was in flight must not come back on its
/// own: the liveness check only drops ports the machine stopped reporting, so nothing would ever turn it off again.
fn still_wanted(enabled: &[String], name: &str) -> bool {
    enabled.iter().any(|wanted| wanted == name)
}

/// Owns the MIDI connections and the probe machinery.
///
/// A probe runs on its own thread and reports through a channel, so a missing or unplugged interface can never block a
/// frame. Each connection's handler runs on midir's thread and only writes atomics.
struct Midi {
    /// The ports to open, by name. A name whose port has gone away stays here, in the order the player turned them on: the
    /// device it feeds stays silent until that is decided, rather than quietly listening to whatever is left.
    enabled: Vec<String>,
    /// The live connections, one per enabled port that answered.
    inputs: Vec<(String, midi::ports::MidiIn)>,
    /// One for the app, not one per port: it answers "is there a MIDI input at all", and that question did not change.
    supervisor: midi::supervisor::MidiSupervisor,
    /// The probe in flight, if any: one result per port it tried, and the channel closes when the thread is done.
    pending: Option<std::sync::mpsc::Receiver<ProbeResult>>,
    /// Whether `enabled` has ever been filled from the machine. A session with no saved port opens the first the machine
    /// reports, which is the one the app opened on its own before it could hold more than one; a player who then turns
    /// every port off gets silence for the rest of the session, because the fill happens once.
    filled: bool,
    /// The rack's tempo cell: the same atomic every device's `SharedState` holds, shared with every router this app opens,
    /// so one clock — or one fader — is every engine's tempo at its next block.
    tempo: Arc<state::AtomicF32>,
    /// The published rack, for each router to load once per message: a clone of the swap, never a copy of its contents.
    bus: Arc<arc_swap::ArcSwap<device::BusState>>,
}

impl Midi {
    fn new(
        enabled: Vec<String>,
        tempo: Arc<state::AtomicF32>,
        bus: Arc<arc_swap::ArcSwap<device::BusState>>,
    ) -> Self {
        Self {
            filled: !enabled.is_empty(),
            enabled,
            inputs: Vec::new(),
            supervisor: midi::supervisor::MidiSupervisor::new(),
            pending: None,
            tempo,
            bus,
        }
    }

    /// The ports the app is holding open, by name: what the card ticks, and what the session remembers.
    fn enabled(&self) -> &[String] {
        &self.enabled
    }

    /// The ports that answered. Not the same list as `enabled`: an unplugged port is still held.
    fn connected(&self) -> Vec<&str> {
        self.inputs.iter().map(|(name, _)| name.as_str()).collect()
    }

    /// Open a port, and probe straight away: the player asked for this one.
    fn enable(&mut self, port: String) {
        if self.enabled.iter().any(|name| name == &port) {
            return;
        }
        self.enabled.push(port);
        self.supervisor.probe_now();
    }

    /// Close a port and forget it. Dropping the connection here is what stops it delivering, at once.
    fn disable(&mut self, port: &str) {
        self.enabled.retain(|name| name != port);
        self.inputs.retain(|(name, _)| name != port);
    }

    /// Start a probe unless one is already running or the supervisor is waiting.
    fn probe_if_due(&mut self) {
        let now = std::time::Instant::now();
        if self.pending.is_some() || !self.supervisor.should_probe(now) {
            return;
        }
        // A session with no saved port opens the first the machine reports. Once: a player who turns every port off gets
        // silence for the rest of the session, which is what the card's unticked rows say.
        if !self.filled {
            match midi::ports::list_ports() {
                Ok(names) => {
                    self.enabled = midi::ports::opened_by_default(&names);
                    self.filled = true;
                }
                // Nothing to enumerate yet: count it like any other failed probe and try again in a moment.
                Err(_) => {
                    self.supervisor.on_error(now);
                    return;
                }
            }
        }
        // Ask the port list, once, whether the connections are still there. Opening a second connection to find out would
        // make every note arrive twice, which is exactly the "one press writes two steps" report. An enumeration that
        // fails counts as "still there": dropping a working connection over a transient error is the worse mistake.
        if let Ok(listed) = midi::ports::list_ports() {
            self.inputs
                .retain(|(name, _)| listed.iter().any(|listed| listed == name));
        }
        let missing: Vec<String> = self
            .enabled
            .iter()
            .filter(|name| !self.inputs.iter().any(|(open, _)| open == *name))
            .cloned()
            .collect();
        if missing.is_empty() {
            // Everything it wants is open — or it wants nothing, which is a settled state and not a retry.
            self.supervisor.on_success(now);
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        // The two cells the MIDI thread needs, cloned once per probe round: the router never sees the rack.
        let tempo = Arc::clone(&self.tempo);
        let bus = Arc::clone(&self.bus);
        std::thread::spawn(move || {
            for name in missing {
                // One router per connection, because a note gate and a clock follower belong to a port: two ports sharing
                // an "already down" set would swallow a press on the second keyboard.
                let bus = Arc::clone(&bus);
                let mut router = midi::route::Router::new(name.as_str(), Arc::clone(&tempo));
                let result = midi::ports::MidiIn::open_named(&name, move |_, bytes| {
                    router.on_message(&bus.load(), bytes)
                })
                .map(|input| (name.clone(), input))
                .map_err(|error| (name, error.to_string()));
                // A receiver that has gone away means the app is closing: drop the connection and stop opening more.
                if tx.send(result).is_err() {
                    return;
                }
            }
        });
    }

    /// Collect what the probe found, if anything, and update the supervisor.
    fn poll(&mut self) {
        // Taken out so the drain below can push into `inputs`; put back if the thread is still opening ports, because a
        // result sent after the channel was dropped would close the connection it carries.
        let Some(rx) = self.pending.take() else {
            return;
        };
        let mut opened = 0;
        let mut failed = 0;
        let mut first_error: Option<String> = None;
        let mut done = false;
        loop {
            match rx.try_recv() {
                Ok(Ok((name, input))) => {
                    // A port the player turned off while the probe was in flight must not come back on its own: nothing
                    // else would take it away again, so the result is dropped here and the connection with it.
                    if !still_wanted(&self.enabled, &name) {
                        continue;
                    }
                    self.inputs.push((name, input));
                    opened += 1;
                }
                Ok(Err((name, message))) => {
                    failed += 1;
                    if first_error.is_none() {
                        first_error = Some(format!("{name}: {message}"));
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    done = true;
                    break;
                }
            }
        }
        if !done {
            self.pending = Some(rx);
        }
        let now = std::time::Instant::now();
        if opened > 0 {
            self.supervisor.on_success(now);
        } else if failed > 0
            && matches!(
                self.supervisor.on_error(now),
                midi::supervisor::MidiAction::GiveUp
            )
        {
            // Said once, when the retries run out, instead of once per probe round. The name stays held regardless: a
            // controller that comes back is opened again, and one that does not is the player's to turn off.
            let reason = first_error.unwrap_or_else(|| "no MIDI input answered".to_string());
            eprintln!("takka-acid: {reason} — giving up on the MIDI input (RETRY in settings, F2)");
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

    use super::{Midi, still_wanted};
    /// A `Midi` with no rack behind it: the roster is all these tests touch, and a real `MidiIn` would need a device.
    fn midi_for_test(enabled: Vec<String>) -> Midi {
        Midi::new(
            enabled,
            Arc::new(acid_app::state::AtomicF32::new(130.0)),
            Arc::new(arc_swap::ArcSwap::from_pointee(
                acid_app::device::BusState {
                    slots: Vec::new(),
                    routes: Vec::new(),
                },
            )),
        )
    }
    /// Each port is one entry in a set, and turning one off cannot disturb another. No device is needed: the roster is
    /// bookkeeping, and the connections it drops are only ever the ones it was handed.
    #[test]
    fn ports_are_a_set_and_turning_one_off_leaves_the_rest() {
        let mut midi = midi_for_test(Vec::new());
        midi.enable("KeyStep".to_string());
        midi.enable("nanoKONTROL".to_string());
        midi.enable("KeyStep".to_string());
        assert_eq!(midi.enabled(), ["KeyStep", "nanoKONTROL"], "once, in order");

        midi.disable("KeyStep");
        assert_eq!(midi.enabled(), ["nanoKONTROL"]);
        assert!(midi.connected().is_empty(), "nothing has been opened");
    }

    /// The window this closes: a probe round computes what to open, the player unticks one of those ports while the thread
    /// is still opening them, and its success lands afterwards. Installing it would play a port the card shows as off, and
    /// nothing would ever take it away again — the liveness check only drops ports the machine stopped reporting.
    #[test]
    fn a_probe_result_is_kept_only_while_its_port_is_still_wanted() {
        let wanted = ["KeyStep".to_string()];
        assert!(still_wanted(&wanted, "KeyStep"));
        assert!(!still_wanted(&wanted, "nanoKONTROL"));
        assert!(!still_wanted(&[], "KeyStep"), "everything off means off");
    }

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
