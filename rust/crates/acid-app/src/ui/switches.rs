//! The switches row: the engraved legends and the live keys, in one place so their number exists once.
//!
//! Every live key was previously found by indexing `layout::knob_cells(area, N)`, which meant that adding one key
//! silently broke the tests that pass a different `N`. They all ask this module now.

use eframe::egui::Rect;

use super::layout;

/// One key of the switches row, left to right.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Switch {
    Bank,
    Pattern,
    Mode,
    Clear,
    Save,
    Songs,
    Step,
}

impl Switch {
    /// Every key, in drawing order.
    pub const ALL: [Switch; 7] = [
        Switch::Bank,
        Switch::Pattern,
        Switch::Mode,
        Switch::Clear,
        Switch::Save,
        Switch::Songs,
        Switch::Step,
    ];

    /// Where the key sits in the row.
    pub fn index(self) -> usize {
        match self {
            Switch::Bank => 0,
            Switch::Pattern => 1,
            Switch::Mode => 2,
            Switch::Clear => 3,
            Switch::Save => 4,
            Switch::Songs => 5,
            Switch::Step => 6,
        }
    }

    /// What the key prints.
    pub fn label(self) -> &'static str {
        match self {
            Switch::Bank => "BANK",
            Switch::Pattern => "PATTERN",
            Switch::Mode => "MODE",
            Switch::Clear => "CLEAR",
            Switch::Save => "SAVE",
            Switch::Songs => "SONGS",
            Switch::Step => "STEP",
        }
    }

    /// Whether the key does anything: BANK, PATTERN and MODE are engraving for now.
    pub fn is_live(self) -> bool {
        !matches!(self, Switch::Bank | Switch::Pattern | Switch::Mode)
    }
}

/// The rect of one key in the switches row.
pub fn cell(area: Rect, switch: Switch) -> Rect {
    layout::knob_cells(area, Switch::ALL.len())[switch.index()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Pos2, Rect, vec2};

    fn area() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(980.0, 58.0))
    }

    #[test]
    fn the_keys_are_numbered_and_labeled_in_order() {
        for (index, switch) in Switch::ALL.iter().enumerate() {
            assert_eq!(switch.index(), index, "{switch:?} has the wrong slot");
            assert!(!switch.label().is_empty());
            assert!(switch.label().is_ascii(), "{switch:?} is not ASCII");
        }
    }

    #[test]
    fn a_cell_is_inside_the_row_and_clear_of_its_neighbours() {
        let cells: Vec<Rect> = Switch::ALL.iter().map(|s| cell(area(), *s)).collect();
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
    fn only_the_first_three_are_engraving() {
        assert!(!Switch::Bank.is_live());
        assert!(!Switch::Pattern.is_live());
        assert!(!Switch::Mode.is_live());
        for switch in [Switch::Clear, Switch::Save, Switch::Songs, Switch::Step] {
            assert!(switch.is_live(), "{switch:?} should be live");
        }
    }
}
