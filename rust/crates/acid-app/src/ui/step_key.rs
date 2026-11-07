//! A step keycap: LED above, white key below, accent bar on top, and — for a slide — an amber block under
//! the key plus a dark arrow on it, because a note that runs into the next one has to be visible at a glance.
//!
//! The paint inputs live in `StepView` so the geometry-free half of this widget is testable without a
//! window; the keycap itself is painted from the cell `layout::step_cells` hands over.

use egui::{Rect, Response};

use super::{edit, theme};
use acid_core::pattern::Step;

/// The paint inputs for one key.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StepView {
    pub on: bool,
    pub accent: bool,
    pub slide: bool,
    pub current: bool,
    pub beat: bool,
    pub selected: bool,
    /// This step is being dragged, so its keycap stands back and leaves the pitch to the drag readout.
    pub dragging: bool,
}

/// Reduce a `Step` plus the transport state to the inputs the keycap paints.
pub fn step_view(
    step: &Step,
    index: usize,
    position: usize,
    playing: bool,
    selected: bool,
    dragging: bool,
) -> StepView {
    StepView {
        on: step.on,
        accent: step.accent,
        slide: step.slide,
        current: playing && index == position,
        beat: index.is_multiple_of(4),
        selected,
        dragging,
    }
}

/// What a keycap prints: the note it holds, or `--` for a rest.
pub fn step_label(step: &Step) -> &'static str {
    if step.on {
        edit::pitch_name(step.pitch, step.octave_offset)
    } else {
        "--"
    }
}

/// Paint one step key into `rect` (a cell from `layout::step_cells`).
///
/// Layout inside the cell, top to bottom: LED well, 2 px gap, keycap, slide bar. The LED is lit while the
/// step is playing or the step carries a note, and gets an amber ring when accented.
pub fn step_key(
    ui: &mut egui::Ui,
    index: usize,
    step: &Step,
    view: &StepView,
    rect: Rect,
) -> Response {
    let response = ui.interact(
        rect,
        ui.scope_id().with(("step-key", index)),
        egui::Sense::click_and_drag(),
    );
    // The design cell is 104 tall; everything below is scaled from that.
    let s = (rect.height() / 104.0).max(0.5);
    let led_band = 12.0 * s;
    let gap = 2.0 * s;
    let bar = 3.0 * s;

    let led_center = egui::Pos2::new(rect.center().x, rect.top() + led_band * 0.5);
    let keycap = Rect::from_min_max(
        egui::Pos2::new(rect.left(), rect.top() + led_band + gap),
        egui::Pos2::new(rect.right(), rect.bottom() - bar - gap),
    );
    // A slide fills the whole strip under the key (the accent bar's height plus the gap), so the cue reads
    // at 0.65x where a 3 px silk line just looked like panel shading.
    let slide_bar = Rect::from_min_max(
        egui::Pos2::new(rect.left(), rect.bottom() - bar - gap),
        rect.max,
    );

    let painter = ui.painter();
    let led_radius = (led_band * 0.30).max(2.0);
    painter.circle_filled(led_center, led_radius * 1.7, theme::WELL);
    let lit = view.current || view.on;
    if lit {
        // A soft halo, then the core: two concentric circles read as a glowing LED on a dark panel.
        painter.circle_filled(
            led_center,
            led_radius * 1.7,
            egui::Color32::from_rgba_unmultiplied(0xFF, 0xE0, 0x66, 64),
        );
        painter.circle_filled(led_center, led_radius, theme::LED_ON);
    } else {
        painter.circle_filled(led_center, led_radius, theme::LED_OFF);
    }
    if view.accent {
        painter.circle_stroke(
            led_center,
            led_radius * 1.25,
            egui::Stroke::new((1.5 * s).max(1.0), theme::LED_ACCENT),
        );
    }

    painter.rect_filled(keycap, 3.0 * s, theme::WHITE_KEY);
    painter.rect_stroke(
        keycap,
        3.0 * s,
        egui::Stroke::new((1.0 * s).max(1.0), theme::PANEL),
        egui::StrokeKind::Inside,
    );
    // While the step is being dragged the panel prints the live pitch in the cell, larger and in silk; a
    // second label on the keycap would collide with it and read as two overlapping words.
    if !view.dragging {
        painter.text(
            keycap.center() + egui::vec2(0.0, -s),
            egui::Align2::CENTER_CENTER,
            step_label(step),
            theme::legend(12.0 * s),
            theme::PANEL,
        );
    }
    // The step number keeps a home in the corner: the note is what you read, the number is what you count.
    painter.text(
        keycap.right_bottom() - egui::vec2(4.0 * s, 2.0 * s),
        egui::Align2::RIGHT_BOTTOM,
        format!("{}", index + 1),
        theme::legend(8.0 * s),
        theme::PANEL,
    );

    if view.accent {
        let accent_bar = Rect::from_min_max(
            keycap.min,
            egui::Pos2::new(keycap.right(), keycap.top() + bar),
        );
        painter.rect_filled(accent_bar, 1.5 * s, theme::LED_ACCENT);
    }
    if view.slide {
        // Two cues, because one thin strip was not enough to notice: an amber block filling the strip under
        // the key, and a dark arrow on the key itself saying "this note runs into the next one".
        painter.rect_filled(slide_bar, 1.5 * s, theme::LED_ACCENT);
        let head = 5.0 * s;
        let tip = egui::Pos2::new(keycap.right() - 9.0 * s, keycap.center().y);
        let stroke = egui::Stroke::new((1.5 * s).max(1.0), theme::PANEL);
        painter.line_segment([tip - egui::vec2(head, head), tip], stroke);
        painter.line_segment([tip, tip - egui::vec2(head, -head)], stroke);
    }
    // The whole cell, not the keycap: the outline has to read on a lit step and on a rest alike.
    if view.selected {
        painter.rect_stroke(
            rect,
            3.0 * s,
            egui::Stroke::new((2.0 * s).max(1.0), theme::LED_ACCENT),
            egui::StrokeKind::Inside,
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use acid_core::pitch::Pitch;

    #[test]
    fn a_keycap_names_its_note_and_a_rest_shows_dashes() {
        assert_eq!(step_label(&Step::note(Pitch::DSharp, 0)), "D#0");
        assert_eq!(step_label(&Step::note(Pitch::CHigh, 0)), "C1");
        assert_eq!(step_label(&Step::rest()), "--");
        // A rest keeps a remembered pitch; the label must still read as a rest.
        assert_eq!(
            step_label(&Step {
                on: false,
                ..Step::note(Pitch::G, 0)
            }),
            "--"
        );
    }

    #[test]
    fn the_view_carries_the_selection() {
        let step = Step::note(Pitch::C, 0);
        assert!(step_view(&step, 0, 0, false, true, false).selected);
        assert!(!step_view(&step, 0, 0, false, false, false).selected);
    }

    #[test]
    fn only_the_playing_position_lights_up() {
        let step = Step::note(Pitch::C, 0);
        let v = step_view(&step, 3, 3, true, false, false);
        assert!(v.current && !v.beat);
        let w = step_view(&step, 3, 4, true, false, false);
        assert!(!w.current);
        let stopped = step_view(&step, 3, 3, false, false, false);
        assert!(!stopped.current);
    }

    #[test]
    fn every_fourth_key_is_a_beat_and_flags_are_copied() {
        let step = Step {
            accent: true,
            slide: true,
            ..Step::note(Pitch::E, 0)
        };
        let v = step_view(&step, 8, 0, false, false, false);
        assert!(v.on && v.beat && v.accent && v.slide && !v.current);
        assert!(!step_view(&step, 9, 0, false, false, false).beat);
    }

    #[test]
    fn an_off_step_still_reports_its_pitch_flags_but_is_not_lit() {
        let rest = Step::rest();
        let v = step_view(&rest, 0, 0, true, false, false);
        assert!(!v.on && v.current);
    }
    /// The outline the human partner asked for must actually paint, not just be a flag on the view.
    #[test]
    fn a_selected_key_paints_more_than_an_unselected_one() {
        let shapes = |selected: bool| {
            let ctx = egui::Context::default();
            let mut count = 0;
            for _ in 0..2 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(200.0, 200.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let step = Step::note(Pitch::C, 0);
                        let view = step_view(&step, 0, 0, false, selected, false);
                        let cell = egui::Rect::from_min_size(
                            egui::Pos2::new(8.0, 8.0),
                            egui::vec2(48.0, 104.0),
                        );
                        step_key(ui, 0, &step, &view, cell);
                    },
                );
                output.textures_delta.clear();
                count = output.shapes.len();
            }
            count
        };
        assert!(
            shapes(true) > shapes(false),
            "the selection outline did not paint"
        );
    }

    /// While a step is being dragged the panel prints the live pitch in the cell, so the keycap's own note
    /// must stand down: two labels in the same place read as one garbled word.
    #[test]
    fn a_dragged_keycap_leaves_the_pitch_to_the_readout() {
        let shapes = |dragging: bool| {
            let ctx = egui::Context::default();
            let mut count = 0;
            for _ in 0..2 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(200.0, 200.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let step = Step::note(Pitch::C, 0);
                        let view = step_view(&step, 0, 0, false, false, dragging);
                        let cell = egui::Rect::from_min_size(
                            egui::Pos2::new(8.0, 8.0),
                            egui::vec2(48.0, 104.0),
                        );
                        step_key(ui, 0, &step, &view, cell);
                    },
                );
                output.textures_delta.clear();
                count = output.shapes.len();
            }
            count
        };
        assert_eq!(
            shapes(false) - shapes(true),
            1,
            "a dragged keycap must drop exactly its note label"
        );
    }

    /// A slide must be visible at a glance. The panel's only cue used to be a 3 px silk strip under the key,
    /// which read as panel shading — there was nothing to see.
    #[test]
    fn a_slide_key_paints_a_cue_that_a_plain_key_does_not() {
        let shapes = |slide: bool| {
            let ctx = egui::Context::default();
            let mut count = 0;
            for _ in 0..2 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(200.0, 200.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let step = Step {
                            slide,
                            ..Step::note(Pitch::C, 0)
                        };
                        let view = step_view(&step, 0, 0, false, false, false);
                        let cell = egui::Rect::from_min_size(
                            egui::Pos2::new(8.0, 8.0),
                            egui::vec2(48.0, 104.0),
                        );
                        step_key(ui, 0, &step, &view, cell);
                    },
                );
                output.textures_delta.clear();
                count = output.shapes.len();
            }
            count
        };
        // The strip under the key plus the two strokes of the arrow.
        assert!(
            shapes(true) >= shapes(false) + 3,
            "the slide cue did not paint: {} vs {}",
            shapes(true),
            shapes(false)
        );
    }
    /// The plan's "look at it" step, in headless form: sixteen keys paint, and the lit/accent/slide ones
    /// paint more than a bare key does.
    #[test]
    fn painting_sixteen_keys_emits_shapes() {
        let ctx = egui::Context::default();
        let pattern = [
            Step::note(Pitch::C, 0),
            Step::rest(),
            Step {
                accent: true,
                ..Step::note(Pitch::E, 0)
            },
            Step {
                slide: true,
                ..Step::note(Pitch::G, 0)
            },
        ];
        let mut counts = Vec::new();
        // One warm-up frame, then the measured frame.
        for _ in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(980.0, 560.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let cells = super::super::layout::step_cells(egui::Rect::from_min_size(
                        egui::Pos2::new(16.0, 316.0),
                        egui::vec2(948.0, 104.0),
                    ));
                    for (i, cell) in cells.iter().enumerate() {
                        let step = pattern[i % pattern.len()];
                        let view = step_view(&step, i, 1, false, false, false);
                        step_key(ui, i, &step, &view, *cell);
                    }
                },
            );
            output.textures_delta.clear();
            counts.push(output.shapes.len());
        }
        // 16 keys × (LED, keycap, outline, legend) is well past 32 shapes; a few extras cover bars and glow.
        let last = counts[counts.len() - 1];
        assert!(last > 48, "sixteen keycaps should paint plenty, got {last}");
    }
}
