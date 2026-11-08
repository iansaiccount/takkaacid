//! Panel geometry as pure `Rect` arithmetic — no painting, so it is testable without a window.
//!
//! Six horizontal regions, in the order the sketch stacks them: title bar, knob row, performance row, sixteen step
//! keys, bank/pattern switches, transport. Every band is a fraction of the panel, so the whole face scales as one
//! piece and nothing is ever positioned in absolute pixels.

use egui::{Pos2, Rect};

use super::theme;

/// Knobs across the panel.
pub const KNOB_COUNT: usize = 7;
/// Controls in the performance row: octave down, octave up, shuffle, randomise, density, scale.
pub const PERFORM_COUNT: usize = 6;
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

/// The five bands as fractions of the panel height, lifted from the design sketch: 16..72, 84..300,
/// The six bands as fractions of the panel height: 16..72, 84..300, 312..374, 386..490, 502..560, 570..614 on a
/// 620-tall panel. The first two keep the pixel heights they had at 560, so growing the panel for the performance row
/// cost nothing above it; the step keys and the two key rows below are untouched too.
const BANDS: [(f32, f32); 6] = [
    (16.0 / 620.0, 72.0 / 620.0),
    (84.0 / 620.0, 300.0 / 620.0),
    (312.0 / 620.0, 374.0 / 620.0),
    (386.0 / 620.0, 490.0 / 620.0),
    (502.0 / 620.0, 560.0 / 620.0),
    (570.0 / 620.0, 614.0 / 620.0),
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
            cells: knob_cells(perform_area, PERFORM_COUNT),
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
        assert!((r.width() / r.height() - 980.0 / 620.0).abs() < 0.01);
    }
}
