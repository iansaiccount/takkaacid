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

/// How wide the wordmark's corner is, as a multiple of the band's height. The wordmark is one constant word at one size, so
/// this is *its* width and not a share of the band: a fraction left a corner wider than the word in it, and the player saw the
/// gap as space between the logo and everything else.
const BRAND_WIDTH: f32 = 2.15;
/// How much of the band the session name's corridor takes, measured from the wordmark's corner. Fixed, not fitted to the
/// name: a corridor that grew with its contents would move the deck every time a song was named, and the fit inside a fixed
/// corridor is what the tooltip exists to complete.
const NAME_FRACTION: f32 = 0.10;
/// How big the settings gear and the help dot are, as a multiple of the band's height — the size the two round buttons have
/// always been drawn at (18 of 56), kept band-relative so every piece of this band is measured one way.
const CLUSTER_BUTTON: f32 = 18.0 / 56.0;
/// How wide the tempo's number is, as a fraction of the band. Narrower than it was: the section now also holds the two nudge
/// keys, and every pixel the number gives up is a pixel the deck keeps.
const TEMPO_WELL: f32 = 0.13;

/// The wordmark's corner of the top strip: right of the volume knob, so the two never collide.
pub fn brand_rect(top: Rect) -> Rect {
    let left = volume_knob_rect(top).right() + theme::GAP;
    Rect::from_min_max(
        Pos2::new(left, top.top()),
        Pos2::new(left + top.height() * BRAND_WIDTH, top.bottom()),
    )
}

/// The settings gear: first of the session's two round buttons, hung off the name's corridor. The cluster is anchored on the
/// left — name, gear, help — because the name's corridor is a fraction of the band while the gaps shrink with the panel; an
/// anchor from the right let them collide at a small window.
pub fn gear_rect(top: Rect, _scale: f32) -> Rect {
    // Sized from the band, like every key beside it: `tape_cells` measures this rect to know where the deck starts, and a
    // scale-dependent size would have the deck start inside the dot at any window but the design one.
    let size = top.height() * CLUSTER_BUTTON;
    let centre = brand_rect(top).right() + theme::GAP + size * 0.5;
    Rect::from_center_size(Pos2::new(centre, top.center().y), egui::vec2(size, size))
}

/// The help dot: the gear's twin, one gap to its right, and the last thing before the deck.
pub fn help_rect(top: Rect, scale: f32) -> Rect {
    let gear = gear_rect(top, scale);
    Rect::from_center_size(
        Pos2::new(
            gear.right() + theme::GAP + gear.width() * 0.5,
            top.center().y,
        ),
        gear.size(),
    )
}

/// The session name's corridor: the left cluster's third element, between the wordmark and the settings gear.
///
/// Fixed width, left-aligned: the name is fitted into it and the whole of it is in the tooltip. It used to be centred on the
/// band, then a corridor to the deck; now the deck owns the middle of the band and the session's own cluster (name, settings,
/// help) sits to its left, whole.
pub fn song_label_rect(top: Rect, scale: f32) -> Rect {
    let left = help_rect(top, scale).right() + theme::GAP;
    Rect::from_min_max(
        Pos2::new(left, top.top()),
        Pos2::new(left + top.width() * NAME_FRACTION, top.bottom()),
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

/// Keys on the tape deck, derived from the deck's own catalogue: a seventh key is a variant, and gets a cell without this
/// line moving. The order is `TapeKey::ALL`'s, and the rects follow it positionally.
pub const TAPE_COUNT: usize = crate::ui::tape::TapeKey::ALL.len();

/// Cells in the tempo stepper: the nudge down, the readout, the nudge up.
pub const TEMPO_STEPPER_CELLS: usize = 3;

/// The widest a key of the deck may be, as a multiple of the band's height. The deck spreads across the band (task 3a) and
/// the keys grow with it, but a deck is still a row of keys: past this the shapes inside them start to look lost.
const TAPE_KEY_MAX: f32 = 0.62;
/// The narrowest, for a window so small that the band has almost nothing to give: below this the shapes stop reading.
const TAPE_KEY_MIN: f32 = 0.18;

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
    // From the help dot to the pattern readout: the room the saved word and the name's tail used to spend, now the deck's.
    let left = song_label_rect(top, 1.0).right() + theme::GAP;
    // The stepper sits between the deck and the pattern readout, and the record key needs air before the section: two
    // gaps, not one, or the tempo's outline reads as part of the deck.
    let right = tempo_stepper_cells(top)[0].left() - theme::GAP * 2.0;
    let span = (right - left).max(1.0);
    let tallest = top.height() * TAPE_KEY_MAX;
    let shortest = (top.height() * TAPE_KEY_MIN).max(1.0);
    // Six keys and five gaps: the keys take what the cap allows, and the gaps share the rest, so the row fills the band
    // instead of huddling at one end of it.
    let per = span / TAPE_COUNT as f32;
    // The floor is a floor only while it fits: at a tiny window six minimum keys are wider than the band.
    let key = (per * 0.82).min(tallest).max(shortest).min(per);
    let gap = ((span - TAPE_COUNT as f32 * key) / (TAPE_COUNT as f32 - 1.0)).max(0.0);
    let y = top.center().y - key * 0.5;
    let stride = key + gap;
    let mut cells = [Rect::NOTHING; TAPE_COUNT];
    // One accumulated stride, not `index * stride` per cell: two neighbours then agree about the edge they share, which a
    // multiplication and an addition need not do in f32 — the last pair straddled by a hardware ulp and a test caught it.
    let mut x = left;
    for cell in cells.iter_mut() {
        *cell = Rect::from_min_size(Pos2::new(x, y), egui::vec2(key, key));
        x += stride;
    }
    cells
}

/// The tempo section: one plate holding the number and the two nudge keys, stacked with `+` above `−` at its left.
///
/// The pair used to flank the number, one on each side, and at the band's left edge of the deck they read as two loose keys
/// rather than as the tempo's. Stacked inside the section they cannot: the two half-band keys add up to the section's own
/// height, and the plate behind all three says they belong together.
pub fn tempo_stepper_cells(top: Rect) -> [Rect; TEMPO_STEPPER_CELLS] {
    // Each key is half the band, so the pair is exactly the section's height.
    let half = (top.height() * 0.5).max(1.0);
    let right = pattern_display_rect(top).left() - theme::GAP;
    let well = Rect::from_min_max(
        Pos2::new(right - top.width() * TEMPO_WELL, top.top()),
        Pos2::new(right, top.bottom()),
    );
    let x = well.left() - theme::GAP - half;
    let up = Rect::from_min_size(Pos2::new(x, top.top()), egui::vec2(half, half));
    let down = Rect::from_min_size(Pos2::new(x, top.bottom() - half), egui::vec2(half, half));
    [down, well, up]
}

/// The plate the three share: the pair and the number, with a little air around them.
pub fn tempo_section_rect(top: Rect) -> Rect {
    let cells = tempo_stepper_cells(top);
    Rect::from_min_max(
        Pos2::new(cells[2].left() - 2.0 * top.height() / 56.0, top.top()),
        Pos2::new(cells[1].right(), top.bottom()),
    )
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

            let brand = brand_rect(top);
            let name = song_label_rect(top, s);
            let gear = gear_rect(top, s);
            let help = help_rect(top, s);
            let tape = tape_cells(top);
            let stepper = tempo_stepper_cells(top);
            let pattern = pattern_display_rect(top);

            // The band reads as one sentence: wordmark, settings, help, the session's name, then the deck across what is left.
            assert!(
                brand.right() <= gear.left(),
                "wordmark then gear at {size:?}"
            );
            assert!(gear.right() <= help.left(), "gear then help at {size:?}");
            assert!(help.right() <= name.left(), "help then name at {size:?}");
            assert!(name.right() <= tape[0].left(), "name then deck at {size:?}");
            assert!(
                tape[TAPE_COUNT - 1].right() <= stepper[0].left(),
                "deck then tempo at {size:?}"
            );
            assert!(
                stepper[2].right() <= pattern.left(),
                "tempo then readout at {size:?}"
            );
            assert!(
                (pattern.right() - top.right()).abs() < 0.5,
                "the pattern readout keeps the band's right wall at {size:?}"
            );

            for rect in tape.iter().chain(stepper.iter()) {
                assert!(
                    top.contains_rect(*rect),
                    "escapes the top strip at {size:?}: {rect:?}"
                );
                assert!(rect.width() > 0.0 && rect.height() > 0.0);
            }
            // The deck spreads: one width for every key, and the gaps share what the cap leaves.
            let width = tape[0].width();
            for pair in tape.windows(2) {
                assert!(
                    (pair[1].width() - width).abs() < 0.5,
                    "the deck's keys are one size at {size:?}"
                );
                assert!(pair[0].right() <= pair[1].left(), "no overlap at {size:?}");
            }
            assert_eq!(
                stepper[1],
                tempo_display_rect(top),
                "the readout is the stepper's middle"
            );
        }

        // At the reference size the keys are bigger than the old fixed 0.36 of the band, which is what the round is for.
        let l = layout::layout(Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE));
        let tape = tape_cells(l.top);
        assert!(
            tape[0].width() > l.top.height() * 0.36,
            "the deck should have grown: {:.1} against {:.1}",
            tape[0].width(),
            l.top.height() * 0.36
        );
        assert!(
            song_label_rect(l.top, 1.0).width() > 80.0,
            "the name still needs a corridor to be read in"
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
