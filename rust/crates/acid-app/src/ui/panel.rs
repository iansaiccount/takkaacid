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

/// The help dot: at the right end of the wordmark's corner, clear of the saves lamp beside it.
pub fn help_rect(top: Rect, scale: f32) -> Rect {
    Rect::from_center_size(
        Pos2::new(brand_rect(top).right() - 10.0 * scale, top.center().y),
        egui::vec2(18.0 * scale, 18.0 * scale),
    )
}

/// The settings gear: the help dot's twin, one step left, so help and settings read as one pair of round buttons and the
/// gear never crowds the saves lamp.
pub fn gear_rect(top: Rect, scale: f32) -> Rect {
    let help = help_rect(top, scale);
    Rect::from_center_size(
        egui::Pos2::new(brand_rect(top).right() - 34.0 * scale, help.center().y),
        help.size(),
    )
}

/// The song label: the corridor between the wordmark's corner and the tape deck.
///
/// It used to be centred on the strip, symmetric about `top.center().x`, and it is not any more: the deck, the tempo stepper
/// and the song's two document icons took the room that symmetry spent, and a centred label ending at the deck would be a few
/// pixels wide. The name fills the corridor it has, and a line cut by `text::fit` loses its tail rather than both ends.
pub fn song_label_rect(top: Rect, scale: f32) -> Rect {
    let gap = theme::GAP * scale;
    let left = brand_rect(top).right() + gap;
    // The corridor to the song's document icons, and the whole of it. The name used to be centred on the strip, symmetric
    // about `top.center().x`; the tape deck and the two icons took the room that symmetry spent, and a centred label with its
    // right edge at the document corner would be a few pixels wide.
    let right = tape_cells(top)[0].left() - gap;
    // A window narrow enough that the corners meet or cross: clamp rather than invert — a negative-width rect paints mirrored
    // and reads as a bug.
    Rect::from_min_max(
        Pos2::new(left, top.top()),
        Pos2::new(right.max(left), top.bottom()),
    )
}

/// Outline a control that a learn can pick, while a learn is choosing.
///
/// Drawn just outside the control's own rect so it never covers the face it is pointing at, in a colour no other light on
/// the panel uses: the player is being asked "which one?", and the answer has to be visible at a glance.
pub fn learn_highlight(painter: &Painter, rect: Rect, scale: f32, p: &theme::Palette) {
    painter.rect_stroke(
        learn_highlight_bounds(rect, scale),
        6.0 * scale,
        egui::Stroke::new(2.0 * scale, p.learn_target),
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
        Pos2::new(brand.right() - 118.0 * scale, top.top()),
        Pos2::new(brand.right() - 56.0 * scale, top.bottom()),
    )
}

/// Keys on the tape deck: back, play, pause, stop, forward, record — in that order, left to right, the way a deck reads.
pub const TAPE_COUNT: usize = 6;

/// Cells in the tempo stepper: the nudge down, the readout, the nudge up.
pub const TEMPO_STEPPER_CELLS: usize = 3;

/// How wide one key of the deck is, as a multiple of the band's height: square-ish, so a triangle fits inside it.
const TAPE_KEY: f32 = 0.36;
/// The daylight between two keys of the deck. Small enough to read as one block of keys, wide enough that two shapes never
/// touch at 1× — a deck's keys abut, they do not merge.
const TAPE_GAP: f32 = 2.0;

/// One key of the top band's controls: square-ish, sized from the band so the whole strip scales as one piece.
fn tape_key(top: Rect) -> f32 {
    (top.height() * TAPE_KEY).max(1.0)
}

/// The deck: `TAPE_COUNT` square keys, left of the tempo stepper.
///
/// Everything on this side of the band is anchored to the right wall and built leftwards in one chain — the pattern readout
/// takes the wall, the stepper hangs off it, the deck off the stepper — so no piece can drift away from its neighbour whatever
/// the session name does.
///
/// The song's `LOAD`/`SAVE` icons were going to sit between the stepper and the readout. They are not drawn in this round: two
/// unwired icons cost the session-name corridor — which is also the status line — about forty pixels, and at the smallest
/// window that left the line able to say twelve characters of a notice. The space is not reserved either: when the song
/// document exists it will need its own answer, and a layout that pauses for it now would be a layout that lies about what fits.
pub fn tape_cells(top: Rect) -> [Rect; TAPE_COUNT] {
    let size = tape_key(top);
    let gap = TAPE_GAP;
    let total = TAPE_COUNT as f32 * size + (TAPE_COUNT as f32 - 1.0) * gap;
    let right = tempo_stepper_cells(top)[0].left() - theme::GAP;
    let left = right - total;
    let y = top.center().y - size * 0.5;
    let mut cells = [Rect::NOTHING; TAPE_COUNT];
    for (index, cell) in cells.iter_mut().enumerate() {
        let x = left + index as f32 * (size + gap);
        *cell = Rect::from_min_size(Pos2::new(x, y), egui::vec2(size, size));
    }
    cells
}

/// The tempo stepper: `−`, the readout well, `+`, between the deck and the pattern readout.
///
/// The readout is the middle cell, so the display code and the learn highlight keep the rect they always had; the two nudge
/// keys sit outside it, and no key is scaled by the panel's text scale — a cell gap is not stroke thickness (see
/// `layout::knob_cells`, which does the same at every size).
pub fn tempo_stepper_cells(top: Rect) -> [Rect; TEMPO_STEPPER_CELLS] {
    let size = tape_key(top);
    let y = top.center().y - size * 0.5;
    let plus_right = pattern_display_rect(top).left() - theme::GAP;
    let well_right = plus_right - size - TAPE_GAP;
    let well_left = well_right - top.width() * 0.14;
    let down_left = well_left - TAPE_GAP - size;
    [
        Rect::from_min_size(Pos2::new(down_left, y), egui::vec2(size, size)),
        Rect::from_min_max(
            Pos2::new(well_left, top.top()),
            Pos2::new(well_right, top.bottom()),
        ),
        Rect::from_min_size(Pos2::new(plus_right - size, y), egui::vec2(size, size)),
    ]
}

/// The pattern display well: the band's right wall, where it has always been.
pub fn pattern_display_rect(top: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(top.right() - top.width() * 0.12, top.top()),
        Pos2::new(top.right(), top.bottom()),
    )
}

/// The tempo display well: the middle cell of the stepper.
///
/// It is a readout, not a key: the two nudge keys sit outside it, so nothing that paints or learns a display had to move.
pub fn tempo_display_rect(top: Rect) -> Rect {
    tempo_stepper_cells(top)[1]
}

/// Paint the face: recess, slab, engraved frame, screws and wordmark. Displays come in Task 31.
pub fn paint_panel(painter: &Painter, layout: &PanelLayout, p: &theme::Palette) {
    let panel = layout.panel;
    // What the panel is set *into*: a well slightly larger than the slab, so the face looks recessed
    // rather than printed on the window.
    let s = panel.width() / theme::DESIGN_SIZE.x;
    painter.rect_filled(panel.expand(theme::GAP * s), 10.0 * s, p.well);
    painter.rect_filled(panel, 8.0 * s, p.panel);

    let [outer, inner] = frame_lines(panel);
    painter.rect_stroke(
        outer,
        6.0 * s,
        p.silkscreen_stroke(2.0 * s),
        egui::StrokeKind::Inside,
    );
    painter.rect_stroke(
        inner,
        5.0 * s,
        p.silkscreen_stroke(1.0 * s),
        egui::StrokeKind::Inside,
    );

    for pos in screw_positions(panel) {
        let r = theme::SCREW_RADIUS * s;
        painter.circle_filled(pos, r, p.panel_raised);
        painter.circle_stroke(pos, r, p.silkscreen_stroke(1.0 * s));
        // A single slot, so the screw reads as a screw and not a rivet.
        painter.line_segment(
            [
                pos - egui::vec2(r * 0.6, 0.0),
                pos + egui::vec2(r * 0.6, 0.0),
            ],
            p.silkscreen_stroke(1.0 * s),
        );
    }

    let brand = brand_rect(layout.top);
    painter.text(
        brand.left_center(),
        egui::Align2::LEFT_CENTER,
        "takka-acid",
        theme::legend(20.0 * s),
        p.silkscreen,
    );
    // A thin rule closing off the top strip, so the knobs sit under a line like silkscreen does.
    painter.line_segment(
        [
            Pos2::new(layout.top.left(), layout.top.bottom()),
            Pos2::new(layout.top.right(), layout.top.bottom()),
        ],
        p.silkscreen_stroke(1.0 * s),
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
        assert!(
            tempo_stepper_cells(l.top)[2].right() <= pattern_display_rect(l.top).left(),
            "the tempo stepper stops short of the pattern readout"
        );
        assert!(tempo.right() <= l.top.right());
        for r in [brand, pattern, tempo] {
            assert!(
                l.top.contains_rect(r),
                "region escapes the top strip: {r:?}"
            );
        }
    }

    /// The tape deck, the tempo stepper and the song's two document icons all live in the top band, in the order the player
    /// approved: the session name, the deck, the tempo, the documents, and the pattern readout at the wall.
    ///
    /// Three window sizes, because the band's arithmetic is in fractions of the panel and a single size hides whatever only
    /// works there: 500 × 340 is the smallest window the shipped app can be, the design size is the reference, and 1600 × 1040
    /// is a big one. Pressability and a readable name are asserted at the design size only — below it the whole face is small
    /// by nature, and a floor there would be a test of the window, not the layout.
    #[test]
    fn the_tape_and_its_neighbours_share_the_top_strip_in_order() {
        for size in [
            egui::vec2(500.0, 340.0),
            theme::DESIGN_SIZE,
            egui::vec2(1600.0, 1040.0),
        ] {
            let window = Rect::from_min_size(Pos2::ZERO, size);
            let l = layout::layout(window);
            let top = l.top;
            let s = layout::scale(window);

            let label = song_label_rect(top, s);
            let tape = tape_cells(top);
            let stepper = tempo_stepper_cells(top);
            let pattern = pattern_display_rect(top);

            // Left to right, nothing crossing: the name, the deck, the tempo, the pattern readout at the wall.
            assert!(
                label.right() <= tape[0].left(),
                "label then deck at {size:?}"
            );
            assert!(
                tape[TAPE_COUNT - 1].right() <= stepper[0].left(),
                "deck then tempo at {size:?}"
            );
            assert!(
                stepper[2].right() <= pattern.left(),
                "tempo then the pattern readout at {size:?}"
            );
            assert!(
                (pattern.right() - top.right()).abs() < 0.5,
                "the pattern readout keeps the band's right wall at {size:?}: {} vs {}",
                pattern.right(),
                top.right()
            );

            // Every piece is on the band, and each one has room to exist at all.
            for rect in tape.iter().chain(stepper.iter()) {
                assert!(
                    top.contains_rect(*rect),
                    "escapes the top strip at {size:?}: {rect:?}"
                );
                assert!(rect.width() > 0.0 && rect.height() > 0.0);
            }
            // A deck's keys abut, they do not float: one continuous block read as a tape deck.
            for pair in tape.windows(2) {
                assert!(
                    pair[0].right() <= pair[1].left(),
                    "tape keys overlap at {size:?}"
                );
                assert!(
                    pair[1].left() - pair[0].right() <= 3.0,
                    "a deck's keys abut: {} of daylight at {size:?}",
                    pair[1].left() - pair[0].right()
                );
            }
            // The number keeps its own well between the nudge keys.
            assert_eq!(
                stepper[1],
                tempo_display_rect(top),
                "the readout is the stepper's middle"
            );
            assert!(stepper[0].right() <= stepper[1].left());
            assert!(stepper[1].right() <= stepper[2].left());
        }

        // At the reference size: keys a finger can hit, and a name with room to be a name.
        let l = layout::layout(Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE));
        let tape = tape_cells(l.top);
        assert!(
            tape[0].width() >= 18.0 && tape[0].height() >= 18.0,
            "a key of the deck is a key: {:.1} × {:.1}",
            tape[0].width(),
            tape[0].height()
        );
        let label = song_label_rect(l.top, 1.0);
        assert!(
            label.width() > 60.0,
            "the session name needs room at the reference size: {:.1}",
            label.width()
        );
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
            paint_panel(ui.painter(), &l, &theme::BLACK);
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
