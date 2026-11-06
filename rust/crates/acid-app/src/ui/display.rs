//! The LED window: pattern id and tempo as seven-segment characters.
//!
//! Everything here is formatting or geometry, so it is all testable headless. `Bank B` is the reason the
//! letter table carries `B` even though the plan skipped it: bank letters run A..H, and `B1` must draw.

use acid_core::pattern::PatternId;
use acid_core::timing::clamp_bpm;
use egui::{Painter, Pos2, Rect};

use super::theme;

/// Segment order used by every table here: `a` top, `b` upper-right, `c` lower-right, `d` bottom,
/// `e` lower-left, `f` upper-left, `g` middle.
const A: usize = 0;
const B: usize = 1;
const C: usize = 2;
const D: usize = 3;
const E: usize = 4;
const F: usize = 5;
const G: usize = 6;

/// The pattern's name as the window shows it: `A1`..`H8`.
pub fn format_pattern(id: PatternId) -> String {
    id.to_string()
}

/// Three characters of tempo, clamped to the panel's range and rounded to whole BPM.
pub fn format_tempo(bpm: f32) -> String {
    let bpm = clamp_bpm(bpm).round() as i32;
    format!("{bpm:>3}")
}

/// The same digits, for call sites that want to reason about the label: the unit is *silkscreened beside*
/// the window, never squeezed into its three character cells.
pub fn format_tempo_with_unit(bpm: f32) -> String {
    format_tempo(bpm)
}

/// Seven-segment truth table for one character. Anything without a glyph is blank.
pub fn segments_for(ch: char) -> [bool; 7] {
    let mut s = [false; 7];
    let on: &[usize] = match ch.to_ascii_uppercase() {
        '0' => &[A, B, C, D, E, F],
        '1' => &[B, C],
        '2' => &[A, B, G, E, D],
        '3' => &[A, B, G, C, D],
        '4' => &[F, G, B, C],
        '5' => &[A, F, G, C, D],
        '6' => &[A, F, G, E, C, D],
        '7' => &[A, B, C],
        '8' => &[A, B, C, D, E, F, G],
        '9' => &[A, B, C, D, F, G],
        // Bank letters A..H. `B` is the lower-case shape, so it cannot be read as 8 or 6.
        'A' => &[A, B, C, E, F, G],
        'B' => &[C, D, E, F, G],
        'C' => &[A, D, E, F],
        'D' => &[B, C, D, E, G],
        'E' => &[A, D, E, F, G],
        'F' => &[A, E, F, G],
        'G' => &[A, C, D, E, F],
        'H' => &[B, C, E, F, G],
        _ => &[],
    };
    for i in on {
        s[*i] = true;
    }
    s
}

/// Paint `text` as segments in a dark well, one character per cell.
pub fn paint_display(painter: &Painter, rect: Rect, text: &str, scale: f32) {
    painter.rect_filled(rect, 3.0 * scale, theme::WELL);
    painter.rect_stroke(
        rect,
        3.0 * scale,
        theme::silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );
    let count = text.chars().count().max(1);
    let cell_width = rect.width() / count as f32;
    let pad = 4.0 * scale;
    for (i, ch) in text.chars().enumerate() {
        let cell = Rect::from_min_max(
            Pos2::new(rect.left() + i as f32 * cell_width + pad, rect.top() + pad),
            Pos2::new(
                rect.left() + (i + 1) as f32 * cell_width - pad,
                rect.bottom() - pad,
            ),
        );
        paint_segments(painter, cell, segments_for(ch), scale);
    }
}

/// One character's segments: a bar for each lit segment, drawn twice — a wide dim halo under a bright core.
fn paint_segments(painter: &Painter, cell: Rect, segments: [bool; 7], scale: f32) {
    let thickness = (cell.height() * 0.12).max(1.5 * scale);
    let half = thickness * 0.5;
    let (l, r, t, b) = (cell.left(), cell.right(), cell.top(), cell.bottom());
    let mid_y = cell.center().y;
    // Bars are inset by the mitre allowance so uprights and horizontals meet without overlapping corners.
    let bar = |painter: &Painter, min: Pos2, max: Pos2, color: egui::Color32| {
        painter.rect_filled(Rect::from_min_max(min, max), half, color);
    };
    let horiz = |painter: &Painter, y: f32, thick: f32, color: egui::Color32| {
        bar(
            painter,
            Pos2::new(l + thick, y - thick * 0.5),
            Pos2::new(r - thick, y + thick * 0.5),
            color,
        );
    };
    let vert =
        |painter: &Painter, x: f32, top: f32, bottom: f32, thick: f32, color: egui::Color32| {
            bar(
                painter,
                Pos2::new(x - thick * 0.5, top + thick),
                Pos2::new(x + thick * 0.5, bottom - thick),
                color,
            );
        };

    let halo = egui::Color32::from_rgba_unmultiplied(0xFF, 0xE0, 0x66, 102); // 40 % LED_ON
    if segments[A] {
        horiz(painter, t + half, thickness * 1.8, halo);
        horiz(painter, t + half, thickness, theme::TEXT);
    }
    if segments[G] {
        horiz(painter, mid_y, thickness * 1.8, halo);
        horiz(painter, mid_y, thickness, theme::TEXT);
    }
    if segments[D] {
        horiz(painter, b - half, thickness * 1.8, halo);
        horiz(painter, b - half, thickness, theme::TEXT);
    }
    for (lit, x, top, bottom) in [
        (segments[F], l + half, t, mid_y),
        (segments[B], r - half, t, mid_y),
        (segments[E], l + half, mid_y, b),
        (segments[C], r - half, mid_y, b),
    ] {
        if lit {
            vert(painter, x, top, bottom, thickness * 1.8, halo);
            vert(painter, x, top, bottom, thickness, theme::TEXT);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use acid_core::timing::{MAX_BPM, MIN_BPM};

    #[test]
    fn tempo_is_three_plain_digits_within_the_panel_range() {
        assert_eq!(format_tempo(130.0), "130");
        assert_eq!(format_tempo(99.4), " 99");
        assert_eq!(format_tempo(1000.0), "300");
        assert_eq!(format_tempo(10.0), " 40");
        assert_eq!(format_tempo(99.6), "100");
        assert_eq!(format_tempo(MIN_BPM), " 40");
        assert_eq!(format_tempo(MAX_BPM), "300");
    }

    #[test]
    fn pattern_ids_and_hex_letters_both_render() {
        assert_eq!(format_pattern(PatternId { bank: 0, number: 0 }), "A1");
        assert_eq!(format_pattern(PatternId { bank: 7, number: 7 }), "H8");
        assert!(segments_for('H').iter().any(|s| *s));
        assert_eq!(segments_for('Z'), [false; 7]);
        assert_eq!(segments_for(' '), [false; 7]);
    }

    #[test]
    fn every_digit_uses_a_different_segment_set() {
        let sets: Vec<[bool; 7]> = "0123456789".chars().map(segments_for).collect();
        for (i, a) in sets.iter().enumerate() {
            for (j, b) in sets.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "digits {} and {} share a pattern", i, j);
                }
            }
        }
    }

    /// Bank `B` exists, so the display must be able to draw it — and it must not be confusable with 8.
    #[test]
    fn bank_b_renders_and_differs_from_eight() {
        let b = segments_for('B');
        assert!(b.iter().any(|s| *s), "bank B must have segments");
        assert_ne!(b, segments_for('8'), "B and 8 must not look identical");
        assert_ne!(b, segments_for('6'), "B and 6 must not look identical");
        for letter in ['A', 'C', 'D', 'E', 'F', 'G', 'H', 'B'] {
            assert!(
                segments_for(letter).iter().any(|s| *s),
                "bank letter {letter} must have segments"
            );
        }
    }
}
