//! Panel geometry as pure `Rect` arithmetic — no painting, so it is testable without a window.
//!
//! Six horizontal regions, in the order the sketch stacks them: title bar, knob row, performance row, sixteen step
//! keys, bank/pattern switches, transport. Every band is a fraction of the panel, so the whole face scales as one
//! piece and nothing is ever positioned in absolute pixels.

use egui::{Pos2, Rect};

use super::theme;

/// Knobs across the panel: the row is the catalogue, so a knob added there gets a cell instead of being dropped by the
/// drawing loop's `zip`.
pub const KNOB_COUNT: usize = crate::knobs::Knob::ALL.len();
/// Controls in the performance row: octave down, octave up, shuffle, randomise, density, scale.
pub const PERFORM_COUNT: usize = PERFORM_LEFT + PERFORM_RIGHT;
/// Its left group: octave down, octave up, shuffle — the things played while the pattern runs.
pub const PERFORM_LEFT: usize = 3;
/// Its right group: randomise, density, scale — the things that shape the next roll.
pub const PERFORM_RIGHT: usize = 3;
/// How wide a performance cell is, as a multiple of the row's height: enough for a knob and the caption under it, tight
/// enough that the three in a cluster read as one group rather than a third of the panel each.
const PERFORM_CELL_WIDTH: f32 = 1.35;
/// Step keys in the row.
pub const STEP_COUNT: usize = 16;

/// One control row: the band it occupies and one cell per control.
pub struct Row {
    pub area: Rect,
    pub cells: Vec<Rect>,
}

/// Where every region of the panel sits, in window coordinates.
pub struct PanelLayout {
    pub panel: Rect,
    pub top: Rect,
    pub knobs: Row,
    /// The performance row under the knobs: octave down/up, shuffle, randomise.
    pub perform: Row,
    pub steps: Rect,
    pub switches: Rect,
    pub transport: Rect,
}

/// The six bands as fractions of the panel height: 16..72, 84..300, 312..374, 386..490, 502..560, 570..614 on a
/// 640-tall panel. The first two keep the pixel heights they had at 560, so growing the panel for the performance row
/// cost nothing above it; the step keys and the two key rows below are untouched too.
const BANDS: [(f32, f32); 6] = [
    (16.0 / 640.0, 72.0 / 640.0),
    (84.0 / 640.0, 300.0 / 640.0),
    (312.0 / 640.0, 374.0 / 640.0),
    (386.0 / 640.0, 490.0 / 640.0),
    (502.0 / 640.0, 560.0 / 640.0),
    (570.0 / 640.0, 614.0 / 640.0),
];

/// Side inset as a fraction of the panel width (16 of 980).
const INSET: f32 = 16.0 / 980.0;

/// How much bigger the panel is drawn than its design size, clamped so a tiny window stays readable and
/// a huge one does not turn the silkscreen into a wall of pixels.
pub fn scale(window: Rect) -> f32 {
    let fit = (window.width() / theme::DESIGN_SIZE.x).min(window.height() / theme::DESIGN_SIZE.y);
    fit.clamp(0.65, 1.6)
}

/// The panel rectangle: design aspect, centred, and as large as the window allows — but never larger.
///
/// This deliberately uses the *unclamped* fit rather than `scale`: the clamp exists to keep text legible,
/// and applying it here would let the panel overflow a window smaller than 65 % of the design size.
pub fn design_rect(window: Rect) -> Rect {
    let fit = (window.width() / theme::DESIGN_SIZE.x).min(window.height() / theme::DESIGN_SIZE.y);
    Rect::from_center_size(window.center(), theme::DESIGN_SIZE * fit)
}

/// Place all five regions inside `window`. Pure arithmetic: no painting, no egui context.
pub fn layout(window: Rect) -> PanelLayout {
    let panel = design_rect(window);
    let band = |i: usize| {
        let (top, bottom) = BANDS[i];
        Rect::from_min_max(
            Pos2::new(
                panel.left() + panel.width() * INSET,
                panel.top() + panel.height() * top,
            ),
            Pos2::new(
                panel.right() - panel.width() * INSET,
                panel.top() + panel.height() * bottom,
            ),
        )
    };
    let knobs_area = band(1);
    let perform_area = band(2);

    PanelLayout {
        panel,
        top: band(0),
        knobs: Row {
            area: knobs_area,
            cells: knob_cells(knobs_area, KNOB_COUNT),
        },
        perform: Row {
            area: perform_area,
            cells: perform_cells(perform_area),
        },
        steps: band(3),
        switches: band(4),
        transport: band(5),
    }
}

/// `count` equal cells across `area`, `theme::GAP` apart.
pub fn knob_cells(area: Rect, count: usize) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let gap = theme::GAP;
    let cell_width = (area.width() - gap * (count as f32 - 1.0)) / count as f32;
    let stride = cell_width + gap;
    (0..count)
        .map(|i| {
            let left = area.left() + i as f32 * stride;
            // The last cell snaps to the row's right edge: float rounding must never leave a cell a
            // fraction of a pixel outside the band it belongs to.
            let right = if i + 1 == count {
                area.right()
            } else {
                left + cell_width
            };
            Rect::from_min_max(Pos2::new(left, area.top()), Pos2::new(right, area.bottom()))
        })
        .collect()
}

/// The performance row: two clusters, one against each wall, with open face between them.
///
/// Octave and shuffle are played while the pattern runs; randomise, density and scale shape the next roll. Spreading six
/// equal cells across the whole row would put a play control next to a dice dial and make the row read as one run, so
/// each group is a tight cluster — a knob's width plus its caption — and the space left over *is* the gap.
pub fn perform_cells(area: Rect) -> Vec<Rect> {
    // A cell wide enough for a knob and its caption, but never so wide that six of them plus the space between the two
    // clusters would not fit side by side.
    let roomy = area.height() * PERFORM_CELL_WIDTH;
    let squeeze = (area.width() - theme::GAP * (PERFORM_COUNT as f32 + 2.0)) / PERFORM_COUNT as f32;
    let width = roomy.min(squeeze).max(0.0);
    let stride = width + theme::GAP;

    let cell = |left: f32| {
        Rect::from_min_max(
            Pos2::new(left, area.top()),
            Pos2::new(left + width, area.bottom()),
        )
    };

    let mut cells = Vec::with_capacity(PERFORM_COUNT);
    for index in 0..PERFORM_LEFT {
        cells.push(cell(area.left() + index as f32 * stride));
    }
    // The right cluster hangs from the right wall, so the two groups hug their own ends and the middle stays clear.
    let right_start =
        area.right() - (PERFORM_RIGHT as f32 * width + (PERFORM_RIGHT - 1) as f32 * theme::GAP);
    for index in 0..PERFORM_RIGHT {
        let mut rect = cell(right_start + index as f32 * stride);
        // The last cell snaps to the wall: float rounding must never leave a cell a fraction of a pixel outside the band
        // it belongs to.
        if index + 1 == PERFORM_RIGHT {
            rect.max.x = area.right().max(rect.min.x);
        }
        cells.push(rect);
    }
    cells
}

/// Sixteen keycap cells across the step band.
pub fn step_cells(area: Rect) -> [Rect; STEP_COUNT] {
    let mut cells = [Rect::NOTHING; STEP_COUNT];
    cells.copy_from_slice(&knob_cells(area, STEP_COUNT));
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Vec2;

    /// The window grown by half a pixel: the tolerance for float error in a region that must merely *fit*.
    fn expand_rect(size: Vec2) -> Rect {
        Rect::from_min_size(Pos2::ZERO, size).expand(0.51)
    }

    #[test]
    fn regions_are_ordered_and_do_not_overlap() {
        for size in [
            egui::vec2(500.0, 300.0),
            theme::DESIGN_SIZE,
            egui::vec2(1600.0, 900.0),
        ] {
            let l = layout(Rect::from_min_size(Pos2::ZERO, size));
            assert!(l.top.bottom() <= l.knobs.area.top() + 0.01);
            assert!(l.knobs.area.bottom() <= l.perform.area.top() + 0.01);
            assert!(l.perform.area.bottom() <= l.steps.top() + 0.01);
            assert!(l.steps.bottom() <= l.switches.top() + 0.01);
            assert!(l.switches.bottom() <= l.transport.top() + 0.01);
            for r in [
                l.top,
                l.knobs.area,
                l.perform.area,
                l.steps,
                l.switches,
                l.transport,
            ] {
                assert!(
                    expand_rect(size).contains_rect(r),
                    "region escapes the window: {r:?}"
                );
            }
        }
    }

    #[test]
    fn every_band_sits_inside_the_frame_lines() {
        // The silkscreen frame is inset from the panel, and everything drawn on the face must stay inside it. The
        // transport row bled past the bottom line for a round because the panel grew for the performance row and the
        // last band was still measured against the old height; this is the guard that would have caught it.
        for size in [
            egui::vec2(500.0, 340.0),
            theme::DESIGN_SIZE,
            egui::vec2(1600.0, 1040.0),
        ] {
            let l = layout(Rect::from_min_size(Pos2::ZERO, size));
            let [_, inner] = crate::ui::panel::frame_lines(l.panel);
            for (name, band) in [
                ("top", l.top),
                ("knobs", l.knobs.area),
                ("perform", l.perform.area),
                ("steps", l.steps),
                ("switches", l.switches),
                ("transport", l.transport),
            ] {
                assert!(
                    inner.contains_rect(band),
                    "{name} bleeds past the frame: {band:?} vs {inner:?}"
                );
            }
        }
    }

    #[test]
    fn the_performance_row_sits_between_the_knobs_and_the_steps() {
        let l = layout(Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE));
        assert_eq!(l.perform.cells.len(), PERFORM_COUNT);
        assert!(l.perform.area.top() >= l.knobs.area.bottom());
        assert!(l.perform.area.bottom() <= l.steps.top());
        for cell in &l.perform.cells {
            assert!(l.perform.area.contains_rect(*cell));
            // Tall enough to hold a knob and its caption at 1x.
            assert!(cell.height() >= 40.0, "row too shallow: {}", cell.height());
        }
        // The step keys keep their height: the row was paid for by growing the panel, not by squeezing the instrument.
        assert!(step_cells(l.steps)[0].height() > 100.0);
    }

    #[test]
    fn the_performance_row_is_two_clusters_against_each_wall() {
        let l = layout(Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE));
        assert_eq!(l.perform.cells.len(), PERFORM_LEFT + PERFORM_RIGHT);

        let between = |a: Rect, b: Rect| b.left() - a.right();
        let inside = between(l.perform.cells[0], l.perform.cells[1]);
        let split = between(
            l.perform.cells[PERFORM_LEFT - 1],
            l.perform.cells[PERFORM_LEFT],
        );
        // Inside a cluster the cells sit one gap apart; between the clusters there is most of the row.
        assert!(
            (inside - theme::GAP).abs() < 0.01,
            "cells in a cluster should be one gap apart: {inside}"
        );
        assert!(
            split > l.perform.area.width() / 3.0,
            "the clusters should be far apart: {split} of {}",
            l.perform.area.width()
        );

        // Each cluster hugs its own wall, so the clear face between them *is* the gap.
        assert!((l.perform.cells[0].left() - l.perform.area.left()).abs() < 0.01);
        assert!(
            (l.perform.cells[PERFORM_COUNT - 1].right() - l.perform.area.right()).abs() < 0.01,
            "the randomiser cluster should reach the right wall"
        );
        for cell in &l.perform.cells {
            assert!(l.perform.area.contains_rect(*cell));
        }
        for group in [
            &l.perform.cells[..PERFORM_LEFT],
            &l.perform.cells[PERFORM_LEFT..],
        ] {
            let width = group[0].width();
            assert!(group.iter().all(|cell| (cell.width() - width).abs() < 0.5));
        }
        // A cluster is a knob's worth of room, not a third of the panel each.
        assert!(l.perform.cells[0].width() < l.perform.area.width() / 4.0);
    }

    #[test]
    fn seven_knobs_are_evenly_spaced_and_sixteen_keys_fit() {
        let l = layout(Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE));
        assert_eq!(l.knobs.cells.len(), 7);
        let widths: Vec<f32> = l.knobs.cells.iter().map(|r| r.width()).collect();
        let first = widths[0];
        assert!(
            widths.iter().all(|w| (w - first).abs() < 0.5),
            "uneven knobs: {widths:?}"
        );
        for pair in l.knobs.cells.windows(2) {
            assert!(pair[0].right() <= pair[1].left());
        }
        let keys = step_cells(l.steps);
        assert_eq!(keys.len(), 16);
        assert!(keys.iter().all(|k| k.width() > 12.0 && k.height() > 20.0));
        for pair in keys.windows(2) {
            assert!(
                (pair[1].left() - pair[0].left() - (keys[1].left() - keys[0].left())).abs() < 0.5
            );
        }
    }

    #[test]
    fn the_panel_keeps_its_aspect_and_scales_within_bounds() {
        assert!((scale(Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE)) - 1.0).abs() < 0.01);
        assert!(scale(Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 100.0))) >= 0.65);
        assert!(scale(Rect::from_min_size(Pos2::ZERO, egui::vec2(4000.0, 3000.0))) <= 1.6);
        let r = design_rect(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 900.0)));
        assert!((r.width() / r.height() - 980.0 / 640.0).abs() < 0.01);
    }
}
