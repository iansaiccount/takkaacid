//! Fitting one line of legend text into the room it is given.
//!
//! The face paints text at absolute positions with no layout engine behind it, so a line that is too long has to be cut
//! by hand. A clip rect cuts both ends — the verb at the front and the detail at the back — so the line is measured and
//! cut at the end instead, where a reader stops anyway.

use eframe::egui::{Color32, FontId, Painter, Pos2, Rect, Stroke};

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
    fit_with(text, width, |candidate| advance(painter, candidate, &font))
}

/// The width the painter gives `text` at `font`: the measure a line or a glyph is placed by.
fn advance(painter: &Painter, text: &str, font: &FontId) -> f32 {
    painter
        .layout_no_wrap(text.to_string(), font.clone(), Color32::PLACEHOLDER)
        .rect
        .width()
}

/// Paint `label` centred in `rect` and underline the `letter` that presses it.
///
/// The face prints the key under each control — `H` on `HIT`, `L` on the filter's low side — and this is that mark. It is an
/// egui line under the one glyph, not a font style and not a glyph of its own: a symbol drawn from a system font renders as a
/// box on some machines, and a line takes the palette's ink and the face's scale like every other shape.
///
/// The mark's weight and its distance below the text follow the glyph's own point size against the legend's 13 px design
/// size: one design pixel at 1.0x for a control drawn at 13, a lighter and tighter mark for a smaller caption. So the mark
/// sits under the letters rather than across the key at every size. A label that does not print `letter` loses its mark
/// rather than getting one in the wrong place.
pub fn with_mnemonic(
    painter: &Painter,
    rect: Rect,
    label: &str,
    font: FontId,
    colour: Color32,
    letter: char,
    ink: Color32,
) {
    let galley = painter.layout_no_wrap(label.to_string(), font.clone(), colour);
    let text = Rect::from_center_size(rect.center(), galley.size());
    painter.galley(text.min, galley, colour);
    let Some(start) = label.find(letter) else {
        // A face that prints a key's letter nowhere is a face with a key nothing says: in a debug build that is worth
        // stopping on, and in a release one the caption still prints, unmarked.
        debug_assert!(
            false,
            "{label:?} does not print {letter:?}, the letter that presses it"
        );
        return;
    };
    // The glyph's own box: everything the label prints before the letter, then the letter. The bundled font is set without
    // kerning, so a prefix and a glyph measured apart add up to the box the letter occupies.
    let end = start + letter.len_utf8();
    let from = text.left() + advance(painter, &label[..start], &font);
    let to = from + advance(painter, &label[start..end], &font);
    let scale = font.size / 13.0;
    let y = text.bottom() - 1.0 * scale;
    painter.line_segment(
        [Pos2::new(from, y), Pos2::new(to, y)],
        Stroke::new(1.0 * scale, ink),
    );
}
