mod audio;
mod ui;

use std::time::Duration;

use eframe::egui;
use ui::theme;

fn main() -> eframe::Result {
    // Silence for now — the voice arrives when the engine is wired to the stream (Task 9).
    let rig = audio::Rig::new(|| Box::new(|buf: &mut [f32]| buf.fill(0.0)));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(theme::WINDOW_SIZE)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "takka-acid",
        options,
        Box::new(move |_cc| Ok(Box::new(TakkaApp { rig }))),
    )
}

struct TakkaApp {
    rig: audio::Rig,
}

impl eframe::App for TakkaApp {
    /// Runs every frame, even when the window is hidden: the place for audio supervision.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rig.poll();
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
        ui.painter().text(
            rect.center_bottom() - egui::vec2(0.0, 24.0),
            egui::Align2::CENTER_CENTER,
            self.rig.status(),
            theme::legend(12.0),
            theme::SILKSCREEN_DIM,
        );
    }
}
