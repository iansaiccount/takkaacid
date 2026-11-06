//! The assembled panel: nothing overlaps anything at any size someone would actually use.
//!
//! The plan's version of the first test compared *every* rect against every other, which includes a band
//! against the cells it contains — an intersection that is the whole point of a band. This version checks
//! the pairs an overlap would be a bug in: bands against each other, cells within a row, and the knob row
//! against the step row.

use acid_app::ui::layout::{layout, step_cells};
use egui::{Pos2, Rect};

#[test]
fn the_panel_never_overlaps_itself_at_any_reasonable_size() {
    for (w, h) in [
        (700.0, 400.0),
        (980.0, 560.0),
        (1280.0, 720.0),
        (1920.0, 1080.0),
    ] {
        let l = layout(Rect::from_min_size(Pos2::ZERO, egui::vec2(w, h)));

        // Bands must stack without touching each other.
        let bands = [l.top, l.knobs.area, l.steps, l.switches, l.transport];
        for (i, a) in bands.iter().enumerate() {
            for b in bands.iter().skip(i + 1) {
                assert!(!a.intersects(*b), "bands {} overlap at {w}x{h}", i);
            }
        }

        // Cells inside a row must not overlap their neighbours.
        for (i, a) in l.knobs.cells.iter().enumerate() {
            for b in l.knobs.cells.iter().skip(i + 1) {
                assert!(!a.intersects(*b), "knob cells overlap at {w}x{h}");
            }
            assert!(
                l.knobs.area.contains_rect(*a),
                "knob cell escapes its row at {w}x{h}"
            );
        }
        let keys = step_cells(l.steps);
        for (i, a) in keys.iter().enumerate() {
            for b in keys.iter().skip(i + 1) {
                assert!(!a.intersects(*b), "step keys overlap at {w}x{h}");
            }
            assert!(
                l.steps.contains_rect(*a),
                "step key escapes its row at {w}x{h}"
            );
        }

        // The two dense rows must never reach into each other.
        for knob in &l.knobs.cells {
            for key in &keys {
                assert!(
                    !knob.intersects(*key),
                    "a knob cell reaches into the step row at {w}x{h}"
                );
            }
        }
    }
}

#[test]
fn knob_hit_targets_stay_usable_at_the_smallest_size() {
    let l = layout(Rect::from_min_size(Pos2::ZERO, egui::vec2(700.0, 400.0)));
    assert!(
        l.knobs
            .cells
            .iter()
            .all(|r| r.width() >= 40.0 && r.height() >= 40.0)
    );
}
