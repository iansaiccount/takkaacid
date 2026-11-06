mod audio;
mod ui;

use eframe::egui;
use ui::theme;

fn main() -> eframe::Result {
    // Silence for now — the voice arrives in Task 7.
    let handle = match audio::start_silent() {
        Ok(h) => Some(h),
        Err(err) => {
            eprintln!("takka-acid: no audio ({err}); running silently");
            None
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(theme::WINDOW_SIZE)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "takka-acid",
        options,
        Box::new(move |_cc| Ok(Box::new(TakkaApp { _audio: handle }))),
    )
}

struct TakkaApp {
    _audio: Option<audio::AudioHandle>,
}

impl eframe::App for TakkaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, theme::PANEL);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "takka-acid",
            theme::legend(24.0),
            theme::SILKSCREEN,
        );
    }
}
