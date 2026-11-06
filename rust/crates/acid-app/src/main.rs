mod ui;

use eframe::egui;
use ui::theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(theme::WINDOW_SIZE)
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "takka-acid",
        options,
        Box::new(|_cc| Ok(Box::new(TakkaApp))),
    )
}

#[derive(Default)]
struct TakkaApp;

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
