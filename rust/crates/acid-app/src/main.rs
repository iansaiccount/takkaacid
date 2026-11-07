use std::sync::Arc;
use std::time::Duration;

use acid_app::audio::{self, Rig};
use acid_app::state::{self, AudioStatus};
use acid_app::midi;
use acid_app::persist;
use acid_app::ui as panel_ui;
use acid_core::bank::PatternBank;

use eframe::egui;

fn main() -> eframe::Result {
    // Restore the last session; a first run opens on the factory line.
    let saved = persist::load_or_default(&persist::data_dir());
    let state = persist::saved_to_shared(&saved);
    let bank = saved.bank.clone();

    // One engine per stream: rebuilt at the device's rate whenever the stream reopens.
    let rig = Rig::new({
        let state = Arc::clone(&state);
        move |sample_rate| {
            let state = Arc::clone(&state);
            // Mono out: `Opened::spawn` duplicates every sample to every channel.
            let mut renderer = audio::renderer::AudioRenderer::new(
                sample_rate,
                1,
                state,
                audio::MAX_BLOCK,
            );
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
                ..Default::default()
            };
            Ok(Box::new(TakkaApp {
                rig,
                state,
                panel,
                bank,
                autosave: persist::Autosave::new(),
                midi: Midi::new(),
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
        let presses: Vec<(egui::Key, bool)> = ctx.input(|input| {
            [
                egui::Key::Space,
                egui::Key::ArrowUp,
                egui::Key::ArrowDown,
                egui::Key::ArrowRight,
                egui::Key::ArrowLeft,
                egui::Key::F1,
                egui::Key::Questionmark,
            ]
            .into_iter()
            .filter(|key| input.key_pressed(*key))
            .map(|key| (key, input.modifiers.shift))
            .collect()
        });
        for (key, shift) in presses {
            if let Some(shortcut) = panel_ui::shortcuts::shortcut_for(key, shift) {
                panel_ui::shortcuts::apply(shortcut, &self.state, &mut self.bank);
                // The help card is panel state, so it is toggled here rather than in `apply`.
                if matches!(shortcut, panel_ui::shortcuts::Shortcut::ToggleHelp) {
                    self.panel.help_visible = !self.panel.help_visible;
                }
            }
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
        if ui
            .interact(
                status_rect,
                ui.scope_id().with("audio-status"),
                egui::Sense::click(),
            )
            .clicked()
        {
            self.panel.device_list = Some(audio::list_output_devices());
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
            for (index, name) in devices.iter().enumerate() {
                let row = egui::Rect::from_min_size(
                    egui::Pos2::new(
                        rect.center().x - width / 2.0,
                        base - (index as f32 + 1.0) * row_height,
                    ),
                    egui::vec2(width, row_height),
                );
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
                    .interact(row, ui.scope_id().with(("device", index)), egui::Sense::click())
                    .clicked()
                {
                    chosen = Some(name.clone());
                }
            }
            if let Some(name) = chosen {
                self.panel.device_list = None;
                self.panel.device_choice = Some(name);
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
        let saved = persist::shared_to_saved(&self.state, self.bank.clone());
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
}

impl Midi {
    fn new() -> Self {
        Self {
            input: None,
            supervisor: midi::supervisor::MidiSupervisor::new(),
            pending: None,
            name: None,
        }
    }

    /// Start a probe unless one is already running or the supervisor is waiting.
    fn probe_if_due(&mut self, state: Arc<state::SharedState>) {
        if self.pending.is_some() || !self.supervisor.should_probe(std::time::Instant::now()) {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        std::thread::spawn(move || {
            let mut follower = midi::tempo::TempoFollower::new();
            let result = midi::ports::MidiIn::open(None, move |_, bytes| {
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
                        midi::message::handle(other, &state);
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
                self.supervisor.on_success();
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
        self.supervisor.on_success();
    }
}


#[cfg(test)]
mod tests {
    use acid_core::factory::launcher;
    use acid_core::engine::Engine;
    use std::sync::Arc;

    /// Offline stand-in for the plan's "listen to the loop" step: nothing in the render may jump hard
    /// enough to click, and that includes the wrap from step 15 back to step 0.
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

        let biggest = buf
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);

        assert!(biggest < 0.25, "a click-sized jump: {biggest}");
        assert!(
            buf[loop_len - 1_000..loop_len + 1_000].iter().any(|s| s.abs() > 0.01),
            "the loop wrap should be in the middle of a phrase, not in silence"
        );
    }
}
