//! The performance row: the controls in it, and the two clusters they hang off.
//!
//! The row was a positional list — `l.perform.cells[..]` destructured into seven names in the drawing, and three test files
//! that clicked `cells[2]`, `cells[3]` and `cells[4]` with a comment saying which control that was. A comment does not fail when
//! a control is inserted, so this is the name instead, and the row's two cluster counts are read off the catalogue rather than
//! kept beside it.
//!
//! The split is part of the catalogue and not of the geometry: which controls are *played* and which *shape the next roll* is a
//! fact about the row, and `layout::perform_cells` should not have to know a control's name to count a cluster.

use eframe::egui::Rect;

use super::layout;

/// One control in the performance row, in drawing order: the played keys on the left, the dials on the right.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Perform {
    OctaveDown,
    OctaveUp,
    Clear,
    Randomise,
    Shuffle,
    Density,
    Scale,
}

impl Perform {
    /// The left cluster: the things played while the pattern runs.
    pub const LEFT: [Perform; 4] = [
        Perform::OctaveDown,
        Perform::OctaveUp,
        Perform::Clear,
        Perform::Randomise,
    ];

    /// The right cluster: the dials that shape the next roll.
    pub const RIGHT: [Perform; 3] = [Perform::Shuffle, Perform::Density, Perform::Scale];

    /// Every control, in drawing order: `LEFT` then `RIGHT`.
    ///
    /// Written out rather than built from the two arrays at compile time — a const concatenation reads worse than the test that
    /// pins the two together, and the test is `the_left_cluster_is_the_played_keys_and_the_right_the_dials`.
    pub const ALL: [Perform; 7] = [
        Perform::OctaveDown,
        Perform::OctaveUp,
        Perform::Clear,
        Perform::Randomise,
        Perform::Shuffle,
        Perform::Density,
        Perform::Scale,
    ];

    /// Where the control sits in the row.
    pub fn index(self) -> usize {
        match self {
            Perform::OctaveDown => 0,
            Perform::OctaveUp => 1,
            Perform::Clear => 2,
            Perform::Randomise => 3,
            Perform::Shuffle => 4,
            Perform::Density => 5,
            Perform::Scale => 6,
        }
    }

    /// Whether this control hangs off the row's left wall. Written out rather than read off `index()`: two hand-kept facts
    /// that could disagree are worse than one, and a match here fails to compile when a control is added.
    pub fn in_the_left_cluster(self) -> bool {
        match self {
            Perform::OctaveDown | Perform::OctaveUp | Perform::Clear | Perform::Randomise => true,
            Perform::Shuffle | Perform::Density | Perform::Scale => false,
        }
    }
}

/// The rect of one control in the performance row.
pub fn cell(area: Rect, which: Perform) -> Rect {
    layout::perform_cells(area)[which.index()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Pos2;
    use eframe::egui::vec2;

    /// The performance band at the design size: 980 wide less the 16 px side inset, 62 tall.
    fn area() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(948.0, 62.0))
    }

    #[test]
    fn the_controls_are_numbered_and_drawn_in_order() {
        for (index, control) in Perform::ALL.iter().enumerate() {
            assert_eq!(control.index(), index, "{control:?} has the wrong slot");
        }
    }

    #[test]
    fn every_control_is_inside_the_row_and_clear_of_its_neighbour() {
        let cells: Vec<Rect> = Perform::ALL.iter().map(|c| cell(area(), *c)).collect();
        for (index, rect) in cells.iter().enumerate() {
            assert!(area().contains_rect(*rect), "{index} escapes the row");
            if index > 0 {
                assert!(
                    cells[index - 1].right() <= rect.left() + 0.01,
                    "{index} overlaps"
                );
            }
        }
        // The two clusters hang off their own walls, so the break between them is wider than the gap inside them — that
        // break is the row's whole shape.
        let inside = cells[1].left() - cells[0].right();
        let between = cells[Perform::LEFT.len()].left() - cells[Perform::LEFT.len() - 1].right();
        assert!(
            between > inside + 1.0,
            "the clusters must read as two groups: {inside} inside, {between} between"
        );
    }

    #[test]
    fn the_left_cluster_is_the_played_keys_and_the_right_the_dials() {
        // `ALL` is `LEFT` then `RIGHT`: a control added to one cluster and not to the other fails here rather than drawing
        // in the wrong group.
        for (index, control) in Perform::ALL.iter().enumerate() {
            let side = if index < Perform::LEFT.len() {
                Perform::LEFT[index]
            } else {
                Perform::RIGHT[index - Perform::LEFT.len()]
            };
            assert_eq!(*control, side, "ALL is LEFT then RIGHT at {index}");
            assert_eq!(
                control.in_the_left_cluster(),
                index < Perform::LEFT.len(),
                "{control:?} is on the wrong side of the break"
            );
        }
        // And the geometry hears the catalogue's counts: one falsifiable line, because the three `layout::PERFORM_*` constants
        // *are* these arrays' lengths and comparing them would restate the derivation rather than test it.
        assert_eq!(
            layout::perform_cells(area()).len(),
            Perform::ALL.len(),
            "the row draws one cell per control"
        );
    }
}
