//! The panel face: recessed black slab, engraved silkscreen frame, four screws, wordmark.
//!
//! Everything here is geometry plus a few `Painter` calls. The geometry is tested; the look is not
//! testable, though `painting_the_panel_emits_shapes` at least proves the painting path runs and stays
//! inside the palette.

use egui::{Painter, Pos2, Rect};

use super::layout::PanelLayout;
use super::theme;

/// The four corner screws, inset `MARGIN` from each corner, clockwise from top-left.
///
/// The inset scales with the panel like the frame it decorates: a fixed pixel inset would leave the screws proportionally
/// further from the corner as the face shrinks.
pub fn screw_positions(panel: Rect) -> [Pos2; 4] {
    let inset = theme::MARGIN * panel.width() / theme::DESIGN_SIZE.x;
    [
        Pos2::new(panel.left() + inset, panel.top() + inset),
        Pos2::new(panel.right() - inset, panel.top() + inset),
        Pos2::new(panel.left() + inset, panel.bottom() - inset),
        Pos2::new(panel.right() - inset, panel.bottom() - inset),
    ]
}

/// The engraved frame: an outer rule and an inner rule, both inside the panel edge.
///
/// The rules are drawn in design pixels scaled with the panel, like every other measurement here: a fixed inset would
/// leave the frame the same thickness while the face inside it shrank, and on a small window the bands would reach past
/// it — which is exactly what happened when the panel grew for the performance row and the last band was still measured
/// against the old height.
pub fn frame_lines(panel: Rect) -> [Rect; 2] {
    let scale = panel.width() / theme::DESIGN_SIZE.x;
    let outer = panel.shrink(theme::MARGIN * scale / 2.0);
    [outer, outer.shrink(theme::GAP * scale / 2.0)]
}

/// The volume knob's corner: the top strip's far left, before the wordmark. Size is proportional to the strip,
/// so the knob scales with the panel like every other measurement here.
pub fn volume_knob_rect(top: Rect) -> Rect {
    let size = top.height() * 0.92;
    Rect::from_min_size(
        Pos2::new(top.left() + theme::GAP, top.center().y - size * 0.5),
        egui::vec2(size, size),
    )
}

/// The wordmark's corner of the top strip: right of the volume knob, so the two never collide.
pub fn brand_rect(top: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(volume_knob_rect(top).right() + theme::GAP, top.top()),
        Pos2::new(top.left() + top.width() * 0.40, top.bottom()),
    )
}

/// The MIDI status line: between the wordmark and the pattern display, right-aligned. It is also the picker's
/// handle, so it is the same rect the status text is painted in.
pub fn midi_status_rect(top: Rect, scale: f32) -> Rect {
    Rect::from_min_max(
        Pos2::new(top.left() + top.width() * 0.42, top.top()),
        Pos2::new(
            pattern_display_rect(top).left() - theme::GAP * scale,
            top.bottom(),
        ),
    )
}
/// The song label: the left half of the status band, so the name the session is working on sits beside the pattern
/// display while the status — or a notice — keeps the right half.
pub fn song_label_rect(top: Rect, scale: f32) -> Rect {
    let band = midi_status_rect(top, scale);
    Rect::from_min_max(
        band.min,
        Pos2::new(band.left() + band.width() * 0.45, band.max.y),
    )
}

/// Outline a control that a learn can pick, while a learn is choosing.
///
/// Drawn just outside the control's own rect so it never covers the face it is pointing at, in a colour no other light on
/// the panel uses: the player is being asked "which one?", and the answer has to be visible at a glance.
pub fn learn_highlight(painter: &Painter, rect: Rect, scale: f32) {
    painter.rect_stroke(
        learn_highlight_bounds(rect, scale),
        6.0 * scale,
        egui::Stroke::new(2.0 * scale, theme::LEARN_TARGET),
        egui::StrokeKind::Outside,
    );
}

/// The outline's rect: the control's own, grown so the stroke sits outside it and never covers the face it points at.
pub fn learn_highlight_bounds(rect: Rect, scale: f32) -> Rect {
    rect.expand(2.5 * scale)
}

/// The saved-or-unsaved word: right end of the wordmark's corner, clear of the `?` that opens the help card.
pub fn save_indicator_rect(top: Rect, scale: f32) -> Rect {
    let brand = brand_rect(top);
    Rect::from_min_max(
        Pos2::new(brand.right() - 96.0 * scale, top.top()),
        Pos2::new(brand.right() - 28.0 * scale, top.bottom()),
    )
}

/// The pattern display well: right of the wordmark, left of the tempo display.
pub fn pattern_display_rect(top: Rect) -> Rect {
    let right = top.right() - tempo_display_rect(top).width() - theme::GAP;
    Rect::from_min_max(
        Pos2::new(right - top.width() * 0.12, top.top()),
        Pos2::new(right, top.bottom()),
    )
}

/// The tempo display well: hard against the right end of the top strip.
pub fn tempo_display_rect(top: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(top.right() - top.width() * 0.14, top.top()),
        Pos2::new(top.right(), top.bottom()),
    )
}

/// Paint the face: recess, slab, engraved frame, screws and wordmark. Displays come in Task 31.
pub fn paint_panel(painter: &Painter, layout: &PanelLayout) {
    let panel = layout.panel;
    // What the panel is set *into*: a well slightly larger than the slab, so the face looks recessed
    // rather than printed on the window.
    let s = panel.width() / theme::DESIGN_SIZE.x;
    painter.rect_filled(panel.expand(theme::GAP * s), 10.0 * s, theme::WELL);
    painter.rect_filled(panel, 8.0 * s, theme::PANEL);

    let [outer, inner] = frame_lines(panel);
    painter.rect_stroke(
        outer,
        6.0 * s,
        theme::silkscreen_stroke(2.0 * s),
        egui::StrokeKind::Inside,
    );
    painter.rect_stroke(
        inner,
        5.0 * s,
        theme::silkscreen_stroke(1.0 * s),
        egui::StrokeKind::Inside,
    );

    for pos in screw_positions(panel) {
        let r = theme::SCREW_RADIUS * s;
        painter.circle_filled(pos, r, theme::PANEL_RAISED);
        painter.circle_stroke(pos, r, theme::silkscreen_stroke(1.0 * s));
        // A single slot, so the screw reads as a screw and not a rivet.
        painter.line_segment(
            [
                pos - egui::vec2(r * 0.6, 0.0),
                pos + egui::vec2(r * 0.6, 0.0),
            ],
            theme::silkscreen_stroke(1.0 * s),
        );
    }

    let brand = brand_rect(layout.top);
    painter.text(
        brand.left_center(),
        egui::Align2::LEFT_CENTER,
        "takka-acid",
        theme::legend(20.0 * s),
        theme::SILKSCREEN,
    );
    // A thin rule closing off the top strip, so the knobs sit under a line like silkscreen does.
    painter.line_segment(
        [
            Pos2::new(layout.top.left(), layout.top.bottom()),
            Pos2::new(layout.top.right(), layout.top.bottom()),
        ],
        theme::silkscreen_stroke(1.0 * s),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout;

    #[test]
    fn screws_sit_inside_each_corner() {
        let panel = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(980.0, 560.0));
        let screws = screw_positions(panel);
        assert_eq!(screws.len(), 4);
        assert!(screws.iter().all(|p| panel.contains(*p)));
        assert!(screws[0].x < panel.center().x && screws[0].y < panel.center().y);
        assert!(screws[3].x > panel.center().x && screws[3].y > panel.center().y);
    }

    #[test]
    fn top_strip_regions_are_ordered_left_to_right() {
        let l = layout::layout(Rect::from_min_size(Pos2::ZERO, egui::vec2(980.0, 560.0)));
        let volume = volume_knob_rect(l.top);
        let brand = brand_rect(l.top);
        let pattern = pattern_display_rect(l.top);
        let tempo = tempo_display_rect(l.top);
        // VOLUME sits before the wordmark, and the two must not overlap.
        assert!(
            volume.right() <= brand.left(),
            "the knob must clear the wordmark"
        );
        assert!(volume.left() >= l.top.left(), "and stay on the panel");
        assert!(brand.left() < pattern.left());
        assert!(pattern.right() <= tempo.left());
        assert!(tempo.right() <= l.top.right());
        for r in [brand, pattern, tempo] {
            assert!(
                l.top.contains_rect(r),
                "region escapes the top strip: {r:?}"
            );
        }
    }

    #[test]
    fn the_engraved_frame_is_inside_the_panel() {
        let panel = Rect::from_min_size(Pos2::ZERO, egui::vec2(980.0, 560.0));
        let [outer, inner] = frame_lines(panel);
        assert!(panel.contains_rect(outer));
        assert!(outer.contains_rect(inner));
    }

    /// A stand-in for the plan's "look at it" step: the painting path must run on a real `Painter` and
    /// emit shapes. Nobody's eyes are on this one, so a smoke test is the floor.
    #[test]
    fn painting_the_panel_emits_shapes() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let l = layout::layout(ui.max_rect());
            paint_panel(ui.painter(), &l);
        });
        // Nothing renders this frame's textures, and `FullOutput` panics if its deltas are dropped
        // unhandled — clear them before the frame goes out of scope.
        output.textures_delta.clear();
        let shapes = output.shapes.len();
        assert!(
            shapes > 4,
            "expected a recess, frame, screws and a wordmark, got {} shapes",
            shapes
        );
    }
}
