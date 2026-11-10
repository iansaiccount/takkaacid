//! Fitting one line of legend text into the room it is given.
//!
//! The label rect on the top strip holds the song's name or a notice, and a notice can be longer than the rest of the
//! face put together. Cutting it with a clip rect lost both ends; `ui::text::fit` cuts the end and says so.

use acid_app::ui::text;

/// A fake measurement: ten pixels a character, so the arithmetic in the test is readable.
fn ten_a_char(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

#[test]
fn a_line_that_fits_is_left_alone() {
    assert_eq!(
        text::fit_with("saved Acid 1", 200.0, ten_a_char),
        "saved Acid 1"
    );
}

#[test]
fn a_long_line_is_cut_and_says_so() {
    let line = text::fit_with("could not import: no such file", 100.0, ten_a_char);
    assert!(line.ends_with('…'), "{line}");
    assert!(
        ten_a_char(&line) <= 100.0,
        "the fitted line must fit: {line} is {}",
        ten_a_char(&line)
    );
    assert!(
        line.starts_with("could not"),
        "the beginning is the part worth keeping: {line}"
    );
}

#[test]
fn a_line_exactly_at_the_width_is_not_cut() {
    // Ten characters of ten pixels is exactly the room: cutting it would be the honest answer to a rounding error.
    assert_eq!(
        text::fit_with("0123456789", 100.0, ten_a_char),
        "0123456789"
    );
}

#[test]
fn no_room_at_all_gives_nothing_rather_than_an_ellipsis() {
    // An ellipsis alone says "there is something here" without saying what; on a strip too narrow for a word, nothing
    // is the truthful answer.
    assert_eq!(text::fit_with("something long", 0.0, ten_a_char), "");
    assert_eq!(text::fit_with("something long", 5.0, ten_a_char), "");
}

#[test]
fn the_ellipsis_counts_against_the_width() {
    // The cut line is "abcdef" plus "…": seven characters, so the room must be at least seventy pixels for six characters
    // to survive.
    let line = text::fit_with("abcdefghijklmno", 70.0, ten_a_char);
    assert_eq!(line, "abcdef…");
}
