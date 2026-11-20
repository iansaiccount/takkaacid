//! The transport row: a painted rectangular key like the hardware's RUN/STOP, the tempo nudges, and the catalogue that names
//! the row's cells.
//!
//! The key is drawn rather than styled from a stock widget because the panel has no stock widgets: a lit
//! key's legend is ink for its raised face, a dark one is engraving on black.

use egui::{Rect, Response};

use super::{layout, theme};

/// What the transport row asked the player for.
///
/// A request rather than a write: the rack owns the transport, and it fans a change out to every device. A row that wrote
/// the displayed device's own atomic instead would be undone by the next `sync_transport`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Request {
    Play,
    /// Stop the clock and keep the step. Play resumes from it; Stop returns to the bar.
    Pause,
    /// Run this device's line the other way. The engine takes it at the bar line, so a mid-bar press waits.
    Reverse(bool),
    /// Move the whole rack a semitone, as a performance control.
    Transpose(i32),
    Stop,
    /// One BPM, in the direction of the key.
    NudgeTempo(i8),
}

/// One BPM up or down, clamped to the panel's tempo range.
pub fn nudge_tempo(bpm: f32, direction: i8) -> f32 {
    acid_core::timing::clamp_bpm(bpm + f32::from(direction))
}

/// Move the rack's tempo one BPM in `direction`, and answer whether the session is now unsaved (it is: the tempo is in the
/// save file).
///
/// **The one place a `Request` moves the tempo.** The deck's stepper and the arrow keys both arrive here, so neither can
/// answer "is this a change worth writing" differently from the other. A learned fader reaches the same answer by a different
/// road: it writes the cell from the MIDI thread, which cannot touch panel state, and leaves `SharedState::tempo_written` for
/// the panel to take — `draw_panel` for the device on screen, `take_tempo_moved` for the rest of the rack.
pub fn apply_nudge(rack: &mut crate::device::Rack, direction: i8) -> bool {
    let next = nudge_tempo(rack.tempo_bpm(), direction);
    rack.set_tempo_bpm(next);
    // True even when the number did not move because it was already at the end of its range: the question this answers is
    // which control wrote, not whether the value differs — a file written for a clamped nudge is the smaller wrong.
    true
}

/// What a `PAUSE` says when there is nothing to pause. A key that refuses in silence reads as broken.
pub const NOTHING_TO_PAUSE: &str = "NOTHING TO PAUSE";

/// Pause the rack, or answer `false` because there is nothing to pause.
///
/// The rule is the rack's (`Rack::set_transport` refuses a pause from `Stopped`, because the engine keeps its step through a
/// stop and a pause taken there would resume mid-bar); this asks it rather than keeping a second copy of it, and the words
/// stay here, beside the `Request` they answer. In the library rather than in `main` so the answer has a test.
pub fn pause(rack: &mut crate::device::Rack) -> bool {
    rack.set_transport(crate::device::Transport::Paused)
}

/// Take the "a controller moved the tempo" flag from **every** device, and answer whether any of them had one.
///
/// The tempo is one cell for the whole rack, so the fader that moved it may have written its flag on a device the panel is not
/// showing — a second device hearing its own channel is enough. `draw_panel` drains the drawn device, which is the common
/// path; `main` calls this so the rest cannot leave a fader-moved tempo unsaved.
pub fn take_tempo_moved(rack: &crate::device::Rack) -> bool {
    let mut any = false;
    for device in rack.devices() {
        // `|=` and not `||`: both sides must run, or a later device's flag would stay set for another frame.
        any |= device.state.take_tempo_written();
    }
    any
}

/// One cell of the bottom row, left to right.
///
/// The row was a positional list — a `Vec<Rect>` destructured into seven names, and tests that clicked `cells[6]` with a
/// comment saying which key that was. A comment does not fail when a cell is inserted, so this is the name instead: the
/// layout's drawing and every test ask the catalogue by name, and adding a cell without it fails to compile.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Cell {
    Reverse,
    TransposeDown,
    /// The transpose's value, in a well between the two keys that move it. A cell, not a key: nothing presses it.
    Readout,
    TransposeUp,
    Tbc,
    /// The cell the player asked to keep labelled `TBC` beside the first: a spare, labelled honestly rather than given a job
    /// it does not have. The row is seven to match the switch row above it, and a spare that said nothing would be a hole.
    Spare,
    Learn,
}

impl Cell {
    /// Every cell, in drawing order.
    pub const ALL: [Cell; 7] = [
        Cell::Reverse,
        Cell::TransposeDown,
        Cell::Readout,
        Cell::TransposeUp,
        Cell::Tbc,
        Cell::Spare,
        Cell::Learn,
    ];

    /// Where the cell sits in the row.
    pub fn index(self) -> usize {
        match self {
            Cell::Reverse => 0,
            Cell::TransposeDown => 1,
            Cell::Readout => 2,
            Cell::TransposeUp => 3,
            Cell::Tbc => 4,
            Cell::Spare => 5,
            Cell::Learn => 6,
        }
    }
}

/// The rect of one cell of the bottom row.
pub fn cell(area: Rect, which: Cell) -> Rect {
    layout::knob_cells(area, Cell::ALL.len())[which.index()]
}

/// A key on one of the cards. Every card is painted in the black scheme whatever the face behind it does, so this is the one
/// place that says so — a card that took the face's palette would go amber over an amber device.
pub fn card_button(ui: &mut egui::Ui, id: &str, rect: Rect, label: &str, lit: bool) -> Response {
    transport_button(ui, id, rect, label, lit, &theme::BLACK, None)
}

/// A rectangular transport key. `lit` paints the legend in the palette's `lit_ink`, the way RUN glows while playing.
///
/// `mnemonic` is the letter that presses this key from the keyboard: it is underlined on the key, so a face says its own keys.
pub fn transport_button(
    ui: &mut egui::Ui,
    id: &str,
    rect: Rect,
    label: &str,
    lit: bool,
    p: &theme::Palette,
    mnemonic: Option<char>,
) -> Response {
    let response = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click());
    let scale = key_scale(rect);
    let face = if lit { p.panel_raised } else { p.key_face };
    // A lit key is a raised face and a legend on it, so its legend is `lit_ink`: `led_accent` is ink for the *face*, and in
    // the amber scheme a colour dark enough to read on amber is too dark to read on a near-black key.
    let legend = if lit { p.lit_ink } else { p.silkscreen };

    let painter = ui.painter();
    painter.rect_filled(rect, 3.0 * scale, face);
    painter.rect_stroke(
        rect,
        3.0 * scale,
        p.silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );
    // The letter that presses the key is underlined on it; a key with no key has only its caption.
    match mnemonic {
        Some(letter) => {
            super::text::with_mnemonic(
                painter,
                rect,
                label,
                key_font(rect),
                legend,
                letter,
                legend,
            );
        }
        None => {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                key_font(rect),
                legend,
            );
        }
    }
    if response.hovered() {
        // The ring follows the legend's rule, because it is drawn on the same face: a lit key's is `panel_raised`, where
        // the face's accent is unreadable in the amber scheme, and an unlit key's well takes the accent.
        painter.rect_stroke(
            rect,
            3.0 * scale,
            egui::Stroke::new(1.5 * scale, if lit { p.lit_ink } else { p.led_accent }),
            egui::StrokeKind::Inside,
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A key's own scale, from its height: the corner radius, the stroke and the legend all follow it.
///
/// Public because the transpose's well sits in the same row and scales off its own rect too — a second spelling of this
/// arithmetic would let the well and the keys beside it drift apart at a small window.
pub fn key_scale(rect: Rect) -> f32 {
    (rect.height() / 44.0).clamp(0.65, 1.6)
}

/// The font a key's caption is drawn at. Public because a card that fits a label into a key — a port's name belongs to the
/// machine and can be as long as it likes — has to measure with the font that will draw it, and a second spelling of this
/// arithmetic would drift from this one.
pub fn key_font(rect: Rect) -> egui::FontId {
    theme::legend(13.0 * key_scale(rect))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::switches::Switch;
    use eframe::egui::{Pos2, vec2};

    /// The bottom band at the design size: 980 wide less the 16 px side inset, 44 tall.
    fn area() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(948.0, 44.0))
    }

    #[test]
    fn the_cells_are_numbered_and_drawn_in_order() {
        for (index, cell) in Cell::ALL.iter().enumerate() {
            assert_eq!(cell.index(), index, "{cell:?} has the wrong slot");
        }
    }

    #[test]
    fn every_cell_is_inside_the_row_and_clear_of_its_neighbour() {
        let cells: Vec<Rect> = Cell::ALL.iter().map(|c| cell(area(), *c)).collect();
        assert!(
            (cells[0].left() - area().left()).abs() < 0.01,
            "the row does not start at the wall"
        );
        assert!(
            (cells[cells.len() - 1].right() - area().right()).abs() < 0.01,
            "the row does not end at the wall"
        );
        for (index, rect) in cells.iter().enumerate() {
            assert!(area().contains_rect(*rect), "{index} escapes the row");
            if index > 0 {
                assert!(
                    cells[index - 1].right() <= rect.left() + 0.01,
                    "{index} overlaps"
                );
            }
        }
    }

    #[test]
    fn the_bottom_row_is_as_wide_as_the_switch_row_above_it() {
        // The player's choice from plan 01: two rows of seven share one column grid rather than meeting only at the right
        // wall. A cell added to one row and not the other is a face that reads as two grids again.
        assert_eq!(Cell::ALL.len(), Switch::ALL.len());
    }

    #[test]
    fn a_pause_with_nothing_to_pause_answers_so() {
        let mut rack = crate::device::Rack::from_saved(&crate::persist::SavedState::default());
        // A stopped rack: the engine keeps its step through a stop, so a pause taken here would resume mid-bar. It is refused,
        // and that refusal is what `main` turns into a notice — a key that refuses in silence reads as broken.
        assert!(!pause(&mut rack), "there is nothing to pause");
        assert_eq!(rack.transport(), crate::device::Transport::Stopped);

        rack.set_transport(crate::device::Transport::Playing);
        assert!(pause(&mut rack), "a playing rack pauses");
        assert_eq!(rack.transport(), crate::device::Transport::Paused);
        // A pause of a pause is a pause: there is still something there to pause.
        assert!(pause(&mut rack));
        assert_eq!(rack.transport(), crate::device::Transport::Paused);
        assert!(
            NOTHING_TO_PAUSE.is_ascii(),
            "the face may not print a glyph"
        );
    }

    #[test]
    fn a_tempo_nudge_moves_the_rack_and_marks_the_session_unsaved() {
        let mut rack = crate::device::Rack::from_saved(&crate::persist::SavedState::default());
        let before = rack.tempo_bpm();
        assert!(
            apply_nudge(&mut rack, 1),
            "a nudge changes the tempo, and the tempo is in the save file"
        );
        assert_eq!(rack.tempo_bpm(), nudge_tempo(before, 1));
        assert!(
            apply_nudge(&mut rack, -1),
            "and so does the other direction"
        );
        assert_eq!(rack.tempo_bpm(), before);

        // At the end of the range the number cannot move; the answer is about which control wrote, not whether it moved, so
        // a file written for a clamped nudge is the smaller wrong.
        rack.set_tempo_bpm(300.0);
        assert!(apply_nudge(&mut rack, 1), "a nudge at the top still wrote");
        assert_eq!(rack.tempo_bpm(), 300.0);
    }
    #[test]
    fn the_row_reads_left_to_right_as_the_face_does() {
        // The face's order, written out: REVERSE, TRANS −, the readout between the two keys that move it, TBC, the spare, then
        // LEARN. That `ALL` and `index()` agree is not the same claim — both could be reordered together, and every other test
        // here, and every live test, would follow them: the drawing asks the same catalogue the clicker does, so a swapped
        // pair moves the key on the face rather than landing a click on its neighbour.
        let ordered = [
            Cell::Reverse,
            Cell::TransposeDown,
            Cell::Readout,
            Cell::TransposeUp,
            Cell::Tbc,
            Cell::Spare,
            Cell::Learn,
        ];
        assert_eq!(
            ordered.len(),
            Cell::ALL.len(),
            "a cell is missing from the face's order in this test"
        );
        for pair in ordered.windows(2) {
            assert!(
                cell(area(), pair[0]).left() < cell(area(), pair[1]).left(),
                "{:?} does not sit left of {:?}",
                pair[0],
                pair[1]
            );
        }
    }
}
