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
    let dir = persist::data_dir();
    let persist::Opened {
        state: saved,
        first_run,
        refused: _refused_version,
    } = persist::open(&dir);
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
    // Every drum machine's kit, by device index: the name is not resolved to samples until there is a running device to say
    // what rate to decode at. *Which* machines want one is `persist`'s answer, because it is a property of the document.
    let saved_kits: Vec<(usize, String)> = persist::wanted_kits(&saved);

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
                    // The file it was saved as, so the first SAVE after a restart keeps it instead of writing `Name (2)`. Only
                    // when that file is still there: a demo's or a `NEW` song's name is not a file, and claiming one would have
                    // the next save write over a song the player had under that name.
                    file: persist::session_song_file(&dir, song_name.as_deref()),
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
                kit_rate: None,
                kit_wanted: saved_kits,
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
    /// The rate the kit was decoded at, or `None` before there was a running device to ask. A kit's samples are
    /// resampled to this rate when the kit loads, so it is also the answer to "has the rate changed since the last load?".
    kit_rate: Option<f32>,
    /// The kits the session names, by device index, waiting for a rate to decode them at. A kit is a folder of WAVs and the
    /// rate is only known once the stream is open, so a load records what each machine wants and the first frame that knows
    /// the rate decodes it; the browser sets one device's entry when the player picks a kit.
    kit_wanted: Vec<(usize, String)>,
}

impl eframe::App for TakkaApp {
    /// Runs every frame, even when the window is hidden: audio supervision lives here.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rig.poll();
        // The transport belongs to the rack: the panel asks for a change rather than writing one device's atomic, which the
        // fan-out would immediately undo.
        if let Some(request) = self.panel.transport_request.take() {
            match request {
                panel_ui::transport::Request::Play => {
                    self.rack
                        .set_transport(acid_app::device::Transport::Playing);
                }
                panel_ui::transport::Request::Pause => {
                    // A pause with nothing to pause says so: the rack refuses it (the engine keeps its step through a stop),
                    // and a key that refuses in silence reads as broken.
                    if !panel_ui::transport::pause(&mut self.rack) {
                        self.panel.notify(panel_ui::transport::NOTHING_TO_PAUSE);
                    }
                }
                panel_ui::transport::Request::Stop => {
                    self.rack
                        .set_transport(acid_app::device::Transport::Stopped);
                }
                // The two performers: a direction this device's line runs in, and a semitone on this device. Both are
                // performance, neither is saved, and the engine takes the direction at the bar line.
                panel_ui::transport::Request::Reverse(reversed) => {
                    if let Some(device) = self.rack.devices().get(self.panel.strip.selected) {
                        device.state.set_reversed(reversed);
                    }
                }
                panel_ui::transport::Request::Transpose(semitones) => {
                    // The transpose is the device on the panel's: the row's readout, the engine and this write are one cell.
                    panel_ui::transport::apply_transpose(&self.rack, semitones);
                }
                panel_ui::transport::Request::NudgeTempo(direction) => {
                    // The tempo is in the save file, so a nudge changes the document. `apply_nudge` moves it and answers
                    // that, so the deck's stepper and the arrow keys cannot answer differently.
                    self.panel.dirty |= panel_ui::transport::apply_nudge(&mut self.rack, direction);
                }
            }
        }
        // The tempo is one cell for the whole rack, so a learned fader that moved it may have written its flag on a device
        // the panel is not showing. `draw_panel` drains the drawn device; this drains the rest, so no fader-moved tempo is left
        // unsaved because it was heard by a device off screen.
        self.panel.dirty |= panel_ui::transport::take_tempo_moved(&self.rack);
        // The rig owns the stream, so it owns the status: every device's state is told what it says, from the typed answer the
        // rig builds (the stream's own device name and rate, never the display line). This used to read the *state's* status and
        // write the same value back, which is why `main::sample_rate` answered `None` for ever and no kit ever loaded.
        let status = self.rig.audio_status();
        for device in self.rack.devices() {
            if device.state.status() != status {
                device.state.set_status(status.clone());
            }
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
        // `ui::dispatch`). A held arrow repeats — plain ones move the face's selection, `shift`ed ones nudge the tempo.
        //
        // The presses are *taken* out of the frame, not read: a logic-only tick (a minimised window) is handed the last
        // pass's events again, and the panel repaints every 200 ms, so a reader would replay the same Space or arrow until
        // the window came back. Nothing is taken while a text field has the keyboard — the field's letters are letters, and
        // its space bar is a space rather than the transport. `ui::mod`'s audition keys ask the same question, and both must,
        // or a name typed into a card plays notes and stops the transport.
        // The face is part of the question: `h` is a note on the 303 and `HIT` on a drum machine, so what the panel takes
        // from the frame depends on which face is showing.
        let kind = self.panel.showing_kind();
        let presses = ctx
            .input_mut(|input| panel_ui::dispatch::take_presses(&mut input.events, kind, typing));
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
                // The **playing** bar's count: `position` is that bar's published step, and a request held through
                // a thirty-two-step bar must not be released on its sixteenth.
                self.rack.state().pattern().len(),
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
                // The bank document's three doors. Saving and loading are the card's requests answered by the two methods
                // below; deleting is only the file, and the session's memory of it must go with it.
                panel_ui::banks::Request::SaveBank(typed) => self.save_bank_now(&dir, &typed),
                panel_ui::banks::Request::LoadBank(name) => self.load_bank_now(&dir, &name),
                panel_ui::banks::Request::DeleteBank(name) => {
                    match persist::delete_bank(&dir, &name) {
                        Ok(()) => {
                            // The next SAVE BANK must not write the deleted file back in place, so the session forgets the
                            // name as the file goes.
                            if self.panel.songs.bank_file.as_deref() == Some(name.as_str()) {
                                self.panel.songs.bank_file = None;
                            }
                            if let Some(card) = self.panel.songs.card.as_mut() {
                                card.bank_names = persist::list_banks(&dir);
                                card.delete_armed = None;
                                card.load_armed = None;
                                if card.bank_name == name {
                                    card.bank_name.clear();
                                }
                            }
                            self.notice(format!("deleted bank {name}"));
                        }
                        Err(error) => self.notice(format!("could not delete the bank: {error}")),
                    }
                }
                // A rack document is a later round: the keys are drawn and refuse, so the place on the face is found once and
                // a wiring round cannot rename their ids. Both notices say which round, because a player who pressed one is
                // owed that much.
                panel_ui::banks::Request::SaveRack => self.notice(
                    "a rack file is not saved yet: the rack round comes after this one".to_string(),
                ),
                panel_ui::banks::Request::LoadRack => self.notice(
                    "a rack file does not load yet: the rack round comes after this one"
                        .to_string(),
                ),
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
                        card.bank_name.clear();
                        card.open = None;
                        self.panel.songs.file = None;
                        // The selected device's bank is the starter phrase now, not any file the session came from.
                        self.panel.songs.bank_file = None;
                        card.names = persist::list_songs(&dir);
                        card.bank_names = persist::list_banks(&dir);
                        card.delete_armed = None;
                        card.load_armed = None;
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
                        // The export carries the name it was suggested under, so the file opens as the song it was.
                        let mut saved = self.session_for_disk();
                        saved.song = Some(suggested.clone());
                        match persist::export_song(&path, &saved) {
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
                    // The five rows are the demos' own names on either face, so what a row *does* is the device's kind —
                    // the one question `Device::kind` answers. Written out, so a third machine has to decide here.
                    match self.rack.device().kind() {
                        device::DeviceKind::Acid => {
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
                                // A demo is not a file: it arrives as a new song under a name, and nothing of it is on
                                // disk until the player renames and saves it.
                                self.panel.songs.name = Some(demo::NEW_SONG.to_string());
                                self.panel.songs.file = None;
                                // A demo is a bank too: the device's patterns are no longer a file of the player's.
                                self.panel.songs.bank_file = None;
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
                        device::DeviceKind::Drums => {
                            // The whole of what a demo on a drum machine is lives in `demo::load_drum_bank`: its patterns,
                            // its tempo and its shuffle, and nothing else — not the kit, not the session's name.
                            self.save();
                            if let Some(name) = demo::load_drum_bank(&mut self.rack, index) {
                                panel_ui::pattern_swap::adopt(
                                    &mut self.panel,
                                    self.rack.state(),
                                    self.rack.bank(),
                                );
                                self.pending_pattern = None;
                                self.panel.songs.card = None;
                                self.panel.dirty = true;
                                // The demo replaced this machine's whole bank, so the session is no longer on any bank file: a
                                // stale name would have the next SAVE BANK write the demo's grid over the player's file.
                                self.panel.songs.bank_file = None;
                                self.notice(format!("loaded the {name} drum bank"));
                            }
                        }
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
                    device.kind(),
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
            // The scheme and the kit are readings of the same device, refreshed the same way: a key on the card shows what the
            // rack holds, so a click on it — or an edit on the panel that copies a factory kit — moves the card on the next
            // frame and not before.
            self.panel.device_card.scheme_from(device.scheme);
            self.panel.device_card.kit_from(&device.state.kit().name);
        }
        if let Some(request) = self.panel.settings_request.take() {
            self.apply_settings_request(request);
        }
        // The kit's samples are resampled to the rate the running device plays at, and never to a default: a kit decoded at
        // the wrong rate plays every voice at the wrong pitch, and nothing in the face would say why. So the kit loads the
        // frame the device says what rate it runs at — and again if that rate ever changes. The renderer follows the state's
        // kit once a block, so a reload reaches the sound without the audio thread decoding anything.
        if let Some(rate) = self.sample_rate() {
            match self.kit_rate {
                None => {
                    self.kit_rate = Some(rate);
                    // The first frame that knows the rate decodes every kit the session asked for, one per drum machine.
                    self.load_wanted_kits(rate);
                }
                Some(loaded) if loaded != rate => {
                    // A rate change re-decodes every drum device's kit: a voice decoded at the old rate plays at the wrong
                    // pitch. An edit that has not been written is written **first, at the old rate** — the one its samples are
                    // at — because `save_kit` stamps that rate into every WAV it writes; adopting the new rate first would
                    // label old samples with a new rate and the reload would then play them sharp for ever. That is why the write
                    // comes before the new rate is adopted: the order is the fix, not a tidy-up's choice.
                    self.write_kit();
                    self.kit_rate = Some(rate);
                    let live: Vec<(usize, String)> = self
                        .rack
                        .devices()
                        .iter()
                        .enumerate()
                        .filter(|(_, device)| device.kind().has_kit())
                        .map(|(index, device)| (index, device.state.kit().name.clone()))
                        .collect();
                    for (index, name) in live {
                        self.load_kit_now(index, &name, rate);
                    }
                }
                Some(_) => {}
            }
            // A drum machine is built on the `Init` kit — sixteen voices and no samples — because a state cannot decode a WAV;
            // the rate is only known here. So every drum device that has never had a kit is given the default factory kit the
            // frame this runs: a device the player has just added is device 2 or later, and nothing else ever loads its kit.
            for device in self.rack.devices() {
                acid_app::kits::fill_if_silent(&device.state, rate);
            }
        }
        // The kits on offer, read when they are needed: on the first frame — the copy a factory edit makes has to know which
        // names are taken, and that is not a moment the card is open for — when the card opens, and after a copy is named.
        if self.panel.kits.refresh {
            self.panel.kits.refresh = false;
            self.panel.kits.names = persist::kit_names(&persist::kits_dir());
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
                // The face the panel draws is the kind's: a drum device gets the grid, an acid device the step row.
                kind: device.kind(),
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
        self.apply_kits_request();
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

    /// The rate the running audio device plays at, if there is one.
    ///
    /// A kit is decoded at this rate and never at a default: a voice decoded at the wrong rate plays at the wrong pitch, and
    /// nothing in the face would say why. While the device is still starting nothing is decoded, and a drum device is silent
    /// until its kit arrives — which is exactly what a device with no samples sounds like.
    ///
    /// The rig answers, not a device's state: the state mirrors the stream and can lag it, and this used to read that mirror —
    /// which nothing ever wrote, so the rate was `None` for ever and no kit was ever loaded. See the status block in `logic`.
    fn sample_rate(&self) -> Option<f32> {
        self.rig.sample_rate()
    }

    /// Publish the kit `name` into device `index`, decoding it at `rate`.
    ///
    /// A factory name comes out of the binary; anything else is a folder in the library. A kit that will not load is a
    /// notice and nothing more: the device keeps the kit it has, so a broken folder does not silence the machine.
    fn load_kit_now(&mut self, index: usize, name: &str, rate: f32) -> bool {
        let mut problems = Vec::new();
        let kit = if acid_app::kits::is_factory(name) {
            acid_app::kits::factory_kit(name, rate)
        } else if name == acid_core::drums::DEFAULT_KIT_NAME {
            // The default kit is not a folder and never was: a drum machine that has not picked one holds `Init`, and a
            // reload loop asking the library for it would report a kit that does not exist. Silence is the honest answer.
            Some(acid_core::drums::default_kit())
        } else {
            match persist::load_kit(&persist::kits_dir(), name, rate) {
                Ok(loaded) => {
                    problems = loaded.problems;
                    Some(loaded.kit)
                }
                Err(error) => {
                    self.kit_notice(format!("could not load the kit {name}: {error}"));
                    None
                }
            }
        };
        // One notice for the whole kit: `PanelState::notice` is one line, so a `notice` per voice would say only the last.
        // The voices are still named, in the decoder's own words, because a player holding a kit wants to know *which* file.
        if !problems.is_empty() {
            let named: Vec<String> = problems.iter().map(|problem| problem.to_string()).collect();
            self.kit_notice(format!("kit {name}: {}", named.join("; ")));
        }
        let Some(kit) = kit else {
            return false;
        };
        let Some(device) = self.rack.devices().get(index) else {
            return false;
        };
        // A load, not an edit: the kit's own mutes become the device's starting mask, so a kit that ships with its cowbell
        // off arrives with the cowbell off (`adopt_kit`, beside `set_kit`, says why they are two methods).
        device.state.adopt_kit(kit);
        true
    }

    /// Write the kit that was edited into the library, and only that one — the device the write is owed comes from the panel,
    /// by number.
    ///
    /// A kit is two things on disk — a name in the save file and a folder of samples — so a save writes the folder only when
    /// the kit changed. Rewriting two megabytes of WAV on every autosave tick is not a save, it is a disk burner; the name is
    /// written every time, because it is part of the document.
    ///
    /// The **edited** device is not always the selected one: a player can turn a dial on the second drum machine, click back to
    /// the bass, and save. So the panel records *which* device is owed the write (`PanelState::kit_dirty`), and this reads that
    /// number. The document still names only the saved device's kit (this phase's rule), so a kit edited on a second drum
    /// machine lands in the library where the browser lists it and is named by the rack round — but it is written.
    fn write_kit(&mut self) -> bool {
        let Some(number) = self.panel.kit_dirty else {
            return true;
        };
        let Some(rate) = self.kit_rate else {
            // No rate means no kit was ever decoded, so nothing was edited either.
            return true;
        };
        let Some(device) = self
            .rack
            .devices()
            .iter()
            .find(|device| device.number == number)
        else {
            // The device that was edited has been removed since; the kit went with it, and a number is never reused.
            self.panel.kit_dirty = None;
            return true;
        };
        let kit = device.state.kit();
        if acid_app::kits::is_reserved(&kit.name) {
            // A factory kit has no folder to write: an edit copies it before it lands. Reaching this is a copy that did not
            // happen, and writing into a folder named after a kit that ships is the one thing this must never do.
            self.panel.kit_dirty = None;
            return true;
        }
        match persist::save_kit(&kit, &persist::kits_dir(), rate) {
            Ok(_) => {
                self.panel.kit_dirty = None;
                true
            }
            Err(error) => {
                // The flag stays, so the next save tries again — and the caller must not report the session saved, or a full
                // disk would cost the player their kit with the status line saying everything was kept.
                self.kit_notice(format!("could not save the kit: {error}"));
                false
            }
        }
    }

    /// A load onto the selected device cancels the write *that device* was owed, and leaves every other device's alone: the
    /// edit a second drum machine is owed is not the selected device's to forgive.
    fn forgive_kit_write(&mut self) {
        if self.panel.kit_dirty == Some(self.rack.device().number) {
            self.panel.kit_dirty = None;
        }
    }

    /// Do what the kit browser asked for. It is the card's sibling, so its own list is refreshed after anything moves it.
    fn apply_kits_request(&mut self) {
        let Some(request) = self.panel.kits_request.take() else {
            return;
        };
        match request {
            panel_ui::kits::Request::Close => panel_ui::kits::close(&mut self.panel),
            panel_ui::kits::Request::Load(name) => {
                let Some(rate) = self.kit_rate else {
                    self.kit_notice(
                        "the audio device is not running yet, so no kit can be loaded".to_string(),
                    );
                    return;
                };
                // The kit lands on the device whose card opened the browser, which is the selected one. A load is not an
                // edit, so the write this device was owed is cancelled — another device's is not. The confirmation waits for
                // the load: a "could not load" notice must not be overwritten by a `kit: X` that never happened.
                let loaded = self.load_kit_now(self.rack.selected(), &name, rate);
                // Only a load that happened forgives the write: a kit that did not load leaves the device playing the edited
                // kit, so the folder is still owed it.
                if loaded {
                    self.forgive_kit_write();
                }
                self.want_kit(self.rack.selected(), name.clone());
                self.panel.kits.names = persist::kit_names(&persist::kits_dir());
                if loaded {
                    self.kit_notice(format!("kit: {name}"));
                }
            }
            panel_ui::kits::Request::Import => {
                let Some(rate) = self.kit_rate else {
                    self.kit_notice(
                        "the audio device is not running yet, so no kit can be imported"
                            .to_string(),
                    );
                    return;
                };
                let picked = rfd::FileDialog::new()
                    .set_title("Import a kit folder")
                    .pick_folder();
                let Some(from) = picked else {
                    return;
                };
                // A kit's folder is its name, so that is the name asked for; a taken one lands beside it.
                let wanted = from
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let library = persist::kits_dir();
                match persist::import_kit(&from, &wanted, &library, rate) {
                    Ok(imported) => {
                        let name = imported.name.clone();
                        self.panel.kits.names = persist::kit_names(&library);
                        let loaded = self.load_kit_now(self.rack.selected(), &name, rate);
                        if loaded {
                            self.forgive_kit_write();
                        }
                        self.want_kit(self.rack.selected(), name.clone());
                        // One notice, and it is the *last* one this frame, so it can say what to do: a voice that did not come
                        // with the folder is silent in the copy, and the file the document now names is the repair.
                        if imported.problems.is_empty() && loaded {
                            self.kit_notice(format!("imported the kit {name}"));
                        } else if !imported.problems.is_empty() {
                            let named: Vec<String> = imported
                                .problems
                                .iter()
                                .map(|problem| problem.to_string())
                                .collect();
                            self.kit_notice(format!(
                                "imported {name}, but {} - put that file in its folder and load it again",
                                named.join("; ")
                            ));
                        }
                    }
                    Err(error) => self.kit_notice(format!("could not import the kit: {error}")),
                }
            }
        }
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
                    // `bank_file` is about one device's bank, so it cannot outlive the selection it was about.
                    self.panel.songs.bank_file = None;
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
                    // The rack is what a save writes, so a device added here is a change to the document: the next save
                    // carries it, and the lamp says one is due.
                    self.panel.dirty = true;
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
                    // The session file holds every device, so losing any of them changes what a save would write: the
                    // survivors keep their numbers and their names, and the one that went is the whole difference.
                    self.panel.dirty = true;
                    // The card was about a device that is gone — and so is its mirror, which has to go with it or the next
                    // frame paints a name, a route and a removal for an index that is not there.
                    self.panel.close_device_card();
                    // Removing a device at or before the selection silently puts another machine on the panel, and that is a
                    // change of selection that never goes through `Request::Select` — so `bank_file`, which is about one
                    // device's bank, is cleared here too.
                    self.panel.songs.bank_file = None;
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
        let saved = self.session_for_disk();
        let dir = persist::data_dir();
        // The kit's folder, when it was edited: the name is in the document above, the samples are their own files. A folder
        // that would not write leaves the session unsaved, so SAVE is still the player's to press again.
        let kit_written = self.write_kit();
        match persist::save(&dir, &saved) {
            Ok(()) if kit_written => self.panel.dirty = false,
            Ok(()) => {}
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
        self.rack.replace_from_saved(&saved);
        // The panel pointed into a rack that is gone: an open device card, a held swap and the selection itself all reset
        // against slot 0, which is the one machine the file puts first.
        self.panel.close_device_card();
        panel_ui::pattern_swap::adopt(&mut self.panel, self.rack.state(), self.rack.bank());
        self.pending_pattern = None;
        self.panel.dirty = false;
        self.panel.songs.name = Some(name.to_string());
        self.panel.songs.file = Some(name.to_string());
        // A song's load replaces the rack, so the selected device's bank is the file's, not any bank file the session had.
        self.panel.songs.bank_file = None;
        // Every drum machine's kit is a folder of WAVs, decoded at the rate the running device plays at. Which machines want
        // one, and which kit, is `persist`'s to say: it is a property of the document, not of the load.
        self.kit_wanted = persist::wanted_kits(&saved);
        if let Some(rate) = self.kit_rate {
            self.load_wanted_kits(rate);
        }
        // A song from another machine names ports this one has not got. The devices keep the routing the file gave them — it
        // is worth keeping for the day the port comes back — and one line says which ports are missing.
        let missing = persist::unopened_ports(&saved, self.midi.enabled());
        if !missing.is_empty() {
            self.notice(format!(
                "ports this song names are not open: {}",
                missing.join(", ")
            ));
        }
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

    /// Save the selected device's bank under `typed`, or over the file it came from when the field was left blank.
    ///
    /// Where the file lands is `save_target`'s question — the same one the song's SAVE asks — and a new name that is taken lands
    /// beside its namesake, so one bank can never destroy another by being given its name.
    fn save_bank_now(&mut self, dir: &std::path::Path, typed: &str) {
        let name = match persist::save_target(self.panel.songs.bank_file.as_deref(), typed) {
            Some(persist::SaveTarget::InPlace(name)) => name,
            Some(persist::SaveTarget::NewFile(name)) => persist::free_bank_name(dir, &name),
            None => return,
        };
        let document = persist::BankDocument::of(self.rack.bank(), self.rack.device().kind());
        match persist::save_bank(dir, &name, &document) {
            Ok(()) => {
                self.panel.songs.bank_file = Some(name.clone());
                if let Some(card) = self.panel.songs.card.as_mut() {
                    card.bank_names = persist::list_banks(dir);
                    card.bank_name = name.clone();
                    card.load_armed = None;
                }
                self.notice(format!("saved bank {name}"));
            }
            Err(error) => self.notice(format!("could not save the bank: {error}")),
        }
    }

    /// Load a bank file over the selected device's whole bank — all sixty-four patterns, opening on the first — and say so once
    /// if it was written on another machine. Unsaved work is written first, as a song load does, so a click never throws away a
    /// session that only lived in memory.
    fn load_bank_now(&mut self, dir: &std::path::Path, name: &str) {
        let document = match persist::load_bank(dir, name) {
            Ok(document) => document,
            Err(error) => {
                self.notice(format!("could not load the bank: {error}"));
                return;
            }
        };
        self.save();
        self.rack.device_mut().load_bank(document.bank);
        // The panel was pointing into the bank that has just been replaced, so it re-reads the device's: a held swap belongs to
        // the patterns that are gone.
        panel_ui::pattern_swap::adopt(&mut self.panel, self.rack.state(), self.rack.bank());
        self.pending_pattern = None;
        self.panel.dirty = true;
        self.panel.songs.bank_file = Some(name.to_string());
        if let Some(card) = self.panel.songs.card.as_mut() {
            card.bank_names = persist::list_banks(dir);
            card.load_armed = None;
            card.bank_name = name.to_string();
        }
        match persist::bank_kind_notice(&document.kind, self.rack.device().kind()) {
            Some(said) => self.notice(said),
            None => self.notice(format!("loaded bank {name}")),
        }
    }

    /// Record that `index` wants `name` once the rate is known, replacing whatever it wanted before: one entry per machine, so
    /// a second kit picked on the same device does not leave the first one queued behind it.
    fn want_kit(&mut self, index: usize, name: String) {
        self.kit_wanted.retain(|(wanted, _)| *wanted != index);
        self.kit_wanted.push((index, name));
    }

    /// Decode every kit the session has asked for, at `rate`. The list is taken before the loads: a load publishes a kit and
    /// may leave a notice, and neither may run while the list is borrowed.
    fn load_wanted_kits(&mut self, rate: f32) {
        for (index, name) in std::mem::take(&mut self.kit_wanted) {
            self.load_kit_now(index, &name, rate);
        }
    }

    /// Write the session into the library under `name`, and make that song the session's own. `as_copy` only changes the
    /// words: a copy lands under a name that is free, so saving one never overwrites the song it came from.
    fn write_song(&mut self, dir: &std::path::Path, name: &str, as_copy: bool) {
        // The kit's folder travels with the song's name for it, so a song that names an edited kit brings its samples too. A
        // folder that would not write leaves the session unsaved, so SAVE is still the player's to press again.
        self.write_kit();
        let saved = self.session_for_disk();
        match persist::save_song(dir, name, &saved) {
            Ok(()) => {
                self.panel.songs.name = Some(name.to_string());
                self.panel.songs.file = Some(name.to_string());
                if self.panel.kit_dirty.is_none() {
                    self.panel.dirty = false;
                }
                // The session file remembers which song is open, or the next restart would open nothing.
                let _ = persist::save(dir, &self.session_for_disk());
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

    /// The session as it should be written to disk: every machine, and the panel state that travels with the file.
    fn session_for_disk(&self) -> persist::SavedState {
        // One builder for the rack's bytes: every machine, with device 1 mirrored in the flat fields an older build reads.
        let mut saved = persist::rack_to_saved(&self.rack);
        // One name is all the save file holds: the first port the player turned on.
        saved.midi_input = self.midi.enabled().first().cloned();
        saved.song = self.panel.songs.name.clone();
        // The hover-help switch is panel state, so it is carried here rather than in `rack_to_saved`.
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

    /// Say something about a kit: on the status line, and — while the browser is up — on the browser's own line, because the
    /// browser is the whole panel and a notice painted under a card is a notice nobody reads. One seam for every kit message:
    /// the load's failure, its per-voice problems, a copy's name, an import's hole, and the two "not running yet" refusals.
    fn kit_notice(&mut self, message: String) {
        if self.panel.kits.open {
            self.panel.kits.message = Some(message.clone());
        }
        self.notice(message);
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
