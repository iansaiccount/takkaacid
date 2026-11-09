//! Fitting one line of legend text into the room it is given.
//!
//! The face paints text at absolute positions with no layout engine behind it, so a line that is too long has to be cut
//! by hand. A clip rect cuts both ends — the verb at the front and the detail at the back — so the line is measured and
//! cut at the end instead, where a reader stops anyway.

use eframe::egui::{Color32, FontId, Painter};

/// The mark that says a line was cut. It counts against the width like any other character.
const ELLIPSIS: char = '…';

/// One line cut to fit `width`, marked with an ellipsis when it was cut. `width_of` measures a candidate line, so the
/// cut lands where the text really stops rather than at a guess in characters; the rule is pure, so a test can drive it
/// with arithmetic instead of a font.
pub fn fit_with(text: &str, width: f32, width_of: impl Fn(&str) -> f32) -> String {
    if width_of(text) <= width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    // The longest prefix that still fits with the ellipsis on the end. A width too narrow for even that gives nothing:
    // a lone ellipsis says there is something here without saying what, which is worse than saying nothing.
    for keep in (0..chars.len()).rev() {
        let mut candidate: String = chars[..keep].iter().collect();
        candidate.push(ELLIPSIS);
        if width_of(&candidate) <= width {
            return candidate;
        }
    }
    String::new()
}

/// The same, measured by the painter at `font`: whatever the face is about to draw, this is what will fit.
pub fn fit(painter: &Painter, text: &str, font: FontId, width: f32) -> String {
    fit_with(text, width, |candidate| {
        painter
            .layout_no_wrap(candidate.to_string(), font.clone(), Color32::PLACEHOLDER)
            .rect
            .width()
    })
}
