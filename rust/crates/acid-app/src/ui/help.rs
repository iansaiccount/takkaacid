//! The help card: every gesture, key and controller the instrument answers to, on the panel itself.
//!
//! The rule for this phase is that nothing needs a manual, so the card is written in the panel's own words:
//! short, ASCII, and short enough to fit at 0.65x scale.

use eframe::egui::{Align2, Painter, Pos2, Rect};

use super::theme;

/// Four sections: what the step keys do, what the keyboard does, what MIDI does, and the small print.
pub fn help_sections() -> [(&'static str, &'static [&'static str]); 4] {
    [
        (
            "STEPS",
            &[
                "click: select the step",
                "ctrl+click: note on or off",
                "shift+click: accent (louder, brighter)",
                "alt+click: slide (dim = next step rests)",
                "drag up or down: change pitch",
                "double click: clear the step",
                "CLEAR button: empty the pattern (twice)",
            ],
        ),
        (
            "KEYBOARD",
            &[
                "A W S E D F T G Y H U J K: play notes",
                "Z / X: computer keyboard octave down / up",
                "SPACE: start or stop, same as RUN / STOP",
                "left / right: previous or next pattern",
                "up / down: tempo by 1 BPM",
                "shift+left / right: move the selection",
                "shift+up / down: note up or down",
                "enter: note on or off",
                "F1 or shift+?: this card",
            ],
        ),
        (
            "MIDI",
            &[
                "MIDI in: notes play over the pattern",
                "click the MIDI line: choose an input port",
                "clock: tempo follows a 24 ppqn source",
                "CC 3 tuning, 5 accent, 7 volume",
                "CC 71 resonance, 72 tempo, 73 swing",
                "CC 74 cutoff, 75 decay, 76 env mod",
            ],
        ),
        (
            "NOTES",
            &[
                "the top strip names the audio device",
                "MIDI status sits beside the pattern display",
                "one voice: MIDI and the keyboard share it",
                "saved on exit and 2 s after a change",
            ],
        ),
    ]
}

/// How many lines the card has, headings excluded.
pub fn help_line_count() -> usize {
    help_sections().iter().map(|(_, lines)| lines.len()).sum()
}

/// Height the card's contents need at this scale — the card has to fit the panel at 0.65x and 1.6x.
pub fn help_content_height(scale: f32) -> f32 {
    let heading_size = 13.0 * scale;
    let line_height = 15.0 * scale;
    let padding = 12.0 * scale;
    let mut height = padding;
    for (_, lines) in help_sections() {
        height += heading_size + line_height * 0.4;
        height += line_height * lines.len() as f32;
        height += line_height * 0.5;
    }
    height + padding
}

/// Draw the card inside `panel`, scaled. Everything is sized from `scale`, so it fits at 0.65x and 1.6x.
pub fn paint_help(painter: &Painter, panel: Rect, scale: f32) {
    let card = panel.shrink(theme::MARGIN * scale);
    painter.rect_filled(card, 6.0 * scale, theme::PANEL_RAISED);
    painter.rect_stroke(
        card,
        6.0 * scale,
        theme::silkscreen_stroke(scale),
        eframe::egui::StrokeKind::Inside,
    );

    let heading_size = 13.0 * scale;
    let body_size = 11.0 * scale;
    let line_height = 15.0 * scale;
    let padding = 12.0 * scale;
    let mut y = card.top() + padding;
    let left = card.left() + padding;

    for (heading, lines) in help_sections() {
        painter.text(
            Pos2::new(left, y),
            Align2::LEFT_TOP,
            heading,
            theme::legend(heading_size),
            theme::SILKSCREEN,
        );
        y += heading_size + line_height * 0.4;
        for line in lines {
            painter.text(
                Pos2::new(left, y),
                Align2::LEFT_TOP,
                line,
                theme::legend(body_size),
                theme::TEXT,
            );
            y += line_height;
        }
        y += line_height * 0.5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_help_card_names_every_gesture_and_shortcut() {
        let text: String = help_sections()
            .iter()
            .flat_map(|(_, lines)| lines.iter())
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "click",
            "ctrl+click",
            "shift+click",
            "alt+click",
            "drag",
            "double",
            "SPACE",
            "RUN",
            "F1",
            "Z",
            "X",
            "shift+left",
            "shift+up",
            "enter",
            "CLEAR",
            "MIDI",
            "clock",
            "CC",
        ] {
            assert!(text.contains(needle), "help is missing {needle:?}");
        }
        // The card is the only manual, so the two gestures that matter most are pinned word for word: a
        // stale "click: note on or off" would satisfy a loose needle and then mislead the user.
        assert!(
            text.contains("click: select the step"),
            "the card must say what a plain click does"
        );
        assert!(
            text.contains("ctrl+click: note on or off"),
            "the card must name the gesture that edits a note"
        );
        assert!(help_line_count() >= 12);
    }

    #[test]
    fn help_text_is_plain_ascii_so_the_panel_font_can_draw_it() {
        for (_, lines) in help_sections() {
            for line in lines {
                assert!(line.is_ascii(), "non-ascii help line: {line}");
                assert!(line.len() <= 64, "help line too long: {line}");
            }
        }
    }

    #[test]
    fn every_cc_mapping_is_documented() {
        let midi: String = help_sections()[2].1.join("\n");
        for (cc, _) in crate::midi::cc::CC_MAP {
            assert!(midi.contains(&cc.to_string()), "CC {cc} is undocumented");
        }
    }

    #[test]
    fn the_card_fits_inside_the_panel_at_both_scale_extremes() {
        for scale in [0.65f32, 1.0, 1.6] {
            let panel = Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE * scale);
            let card = panel.shrink(theme::MARGIN * scale);
            let needed = help_content_height(scale);
            assert!(
                needed <= card.height(),
                "at {scale}x the card needs {needed} but has {}",
                card.height()
            );
        }
    }

    /// The card must actually paint: a frame with it showing emits the frame, the headings and the lines.
    #[test]
    fn painting_the_card_emits_shapes() {
        let ctx = eframe::egui::Context::default();
        let mut output = ctx.run_ui(
            eframe::egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    eframe::egui::vec2(980.0, 560.0),
                )),
                ..Default::default()
            },
            |ui| {
                let panel = ui.max_rect();
                paint_help(ui.painter(), panel, 1.0);
            },
        );
        output.textures_delta.clear();
        let shapes = output.shapes.len();
        assert!(shapes > 20, "the card painted only {shapes} shapes");
    }
}
