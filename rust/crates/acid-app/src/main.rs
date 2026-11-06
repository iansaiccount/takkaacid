use std::sync::Arc;
use std::time::Duration;

use acid_app::audio::Rig;
use acid_app::state::{self, AudioStatus};
use acid_app::ui::theme;
use acid_core::engine::Engine;
use eframe::egui;

fn main() -> eframe::Result {
    let state = state::SharedState::new();

    // One engine per stream: rebuilt at the device's rate whenever the stream reopens.
    let rig = Rig::new({
        let state = Arc::clone(&state);
        move |sample_rate| {
            let state = Arc::clone(&state);
            let mut engine = Engine::new(sample_rate);
            Box::new(move |buf: &mut [f32]| {
                let want = state.is_playing();
                if want != engine.is_playing() {
                    if want {
                        engine.start();
                    } else {
                        engine.stop();
                    }
                }
                engine.render(buf);
            })
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(theme::WINDOW_SIZE)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "takka-acid",
        options,
        Box::new(move |_cc| Ok(Box::new(TakkaApp { rig, state }))),
    )
}

struct TakkaApp {
    rig: Rig,
    state: Arc<state::SharedState>,
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
        ctx.request_repaint_after(Duration::from_millis(200));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, theme::PANEL);
        ui.painter().text(
            rect.center_top() + egui::vec2(0.0, 80.0),
            egui::Align2::CENTER_CENTER,
            "takka-acid",
            theme::legend(24.0),
            theme::SILKSCREEN,
        );

        // Temporary transport until the panel lands in Phase 06.
        let button = egui::Rect::from_center_size(
            rect.center() + egui::vec2(0.0, 40.0),
            egui::vec2(120.0, 40.0),
        );
        let playing = self.state.is_playing();
        let label = if playing { "STOP" } else { "RUN" };
        if ui.put(button, egui::Button::new(label)).clicked() {
            self.state.toggle_playing();
        }

        ui.painter().text(
            rect.center_bottom() - egui::vec2(0.0, 24.0),
            egui::Align2::CENTER_CENTER,
            self.rig.status(),
            theme::legend(12.0),
            theme::SILKSCREEN_DIM,
        );
    }
}
