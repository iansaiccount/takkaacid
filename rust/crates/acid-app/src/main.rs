use std::sync::Arc;
use std::time::Duration;

use acid_app::audio::{self, Rig};
use acid_app::state::{self, AudioStatus};
use acid_app::ui as panel_ui;

use acid_core::pattern::{Pattern, Step};
use eframe::egui;

fn main() -> eframe::Result {
    let state = state::SharedState::new();
    // Temporary demo pattern until the step editor lands in Phase 08: the RUN button needs something to
    // play. Task 42's factory bank takes over from here.
    state.set_pattern(Arc::new(demo_pattern()));

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
                pattern: Arc::new(demo_pattern()),
                ..Default::default()
            };
            Ok(Box::new(TakkaApp { rig, state, panel }))
        }),
    )
}

struct TakkaApp {
    rig: Rig,
    state: Arc<state::SharedState>,
    panel: panel_ui::PanelState,
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
        panel_ui::draw_panel(&mut self.panel, &self.state, ui);


        ui.painter().text(
            rect.center_bottom() - egui::vec2(0.0, 6.0),
            egui::Align2::CENTER_CENTER,
            self.rig.status(),
            panel_ui::theme::legend(11.0),
            panel_ui::theme::SILKSCREEN_DIM,
        );
    }
}

/// Temporary demo pattern for the RUN button until the step editor arrives (Phase 06/07).
///
/// It is deliberately placed here rather than in `Engine::new`: a default pattern inside the library would
/// make every engine test noisy, and Task 26's factory bank is where a default really belongs.
fn demo_pattern() -> Pattern {
    use acid_core::pitch::Pitch;
    let mut steps = [Step::rest(); 16];
    steps[0] = Step::note(Pitch::A, -1);
    steps[2] = Step {
        accent: true,
        ..Step::note(Pitch::A, -1)
    };
    steps[4] = Step::note(Pitch::C, 0);
    steps[7] = Step {
        slide: true,
        ..Step::note(Pitch::E, 0)
    };
    steps[8] = Step::note(Pitch::A, -1);
    steps[10] = Step {
        accent: true,
        ..Step::note(Pitch::G, 0)
    };
    steps[12] = Step::note(Pitch::A, 0);
    steps[15] = Step {
        slide: true,
        ..Step::note(Pitch::C, 1)
    };
    Pattern::from_steps(steps)
}

#[cfg(test)]
mod tests {
    use super::demo_pattern;
    use acid_core::engine::Engine;
    use std::sync::Arc;

    /// Offline stand-in for the plan's "listen to the loop" step: nothing in the render may jump hard
    /// enough to click, and that includes the wrap from step 15 back to step 0.
    #[test]
    fn the_demo_pattern_loops_without_a_click_at_the_wrap() {
        let sr = 48_000.0;
        let mut e = Engine::new(sr);
        e.set_pattern(Arc::new(demo_pattern()));
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
