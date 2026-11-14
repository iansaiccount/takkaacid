//! The strip: one cell per device above the top strip — its name, its play lamp, its MUTE key, and a border on the one the
//! panel is showing — plus the `+` that adds another.
//!
//! The strip paints and asks; it never changes the rack. `main` fills `PanelState::strip` from the rack every frame and
//! drains `PanelState::device_request` after the frame, so the panel cannot index a device `main` has already removed.

use eframe::egui::{self, Align2, Pos2, Rect, Sense};

use super::{PanelState, layout, text, theme, tips, transport};
use crate::device::{DeviceKind, Request};

/// One device, as the strip shows it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StripCell {
    pub name: String,
    /// That device's transport, not the rack's: one lamp per machine.
    pub running: bool,
    pub muted: bool,
}

/// What the panel knows about the rack without owning it. `main` fills it every frame; the strip reads it and asks.
#[derive(Clone, Default, Debug)]
pub struct Strip {
    pub cells: Vec<StripCell>,
    /// Which cell is on the panel. `draw_panel` clamps it into range before anything draws, because `main` owns the list
    /// and the panel may be a frame behind a removal.
    pub selected: usize,
}

/// A cell's padding, as a fraction of its height, so the strip's geometry is the same at every scale.
const PAD: f32 = 0.16;

/// The play lamp: between the name and the MUTE key, vertically centred. A circle, because the face may not use a font
/// glyph for "this one is running".
pub fn lamp_rect(cell: Rect) -> Rect {
    let pad = cell.height() * PAD;
    let radius = cell.height() * PAD;
    let right = mute_rect(cell).left() - pad;
    Rect::from_center_size(
        Pos2::new(right - radius, cell.center().y),
        egui::vec2(radius * 2.0, radius * 2.0),
    )
}

/// The MUTE key: the cell's right end. Narrower than it is tall, so a long name always loses the argument.
pub fn mute_rect(cell: Rect) -> Rect {
    let pad = cell.height() * PAD;
    let width = (cell.width() * 0.26).min(cell.height() * 1.2);
    Rect::from_min_max(
        Pos2::new(cell.right() - pad - width, cell.top() + pad),
        Pos2::new(cell.right() - pad, cell.bottom() - pad),
    )
}

/// The caret at the selected cell's left end, before the name: the way into that device's card. Two strokes, not a font
/// glyph, and the space is reserved on every cell so a name never shifts when the selection moves.
pub fn caret_rect(cell: Rect) -> Rect {
    let pad = cell.height() * PAD;
    let width = pad * 2.0;
    Rect::from_min_max(
        Pos2::new(cell.left() + pad * 0.5, cell.top() + pad * 1.5),
        Pos2::new(cell.left() + pad * 0.5 + width, cell.bottom() - pad * 1.5),
    )
}

/// The name: everything from the caret to the lamp. A long name is cut by `text::fit` and never in the store.
pub fn name_rect(cell: Rect) -> Rect {
    let pad = cell.height() * PAD;
    Rect::from_min_max(
        Pos2::new(caret_rect(cell).right() + pad * 0.5, cell.top()),
        Pos2::new(lamp_rect(cell).left() - pad, cell.bottom()),
    )
}

/// Paint the band, and record what a click asked for in `state.device_request`.
pub fn paint(ui: &mut egui::Ui, state: &mut PanelState, area: Rect, s: f32) {
    if area.height() <= 0.0 {
        return;
    }
    // The shelf: a recessed well with a hairline under it, so the strip reads as sitting above the face rather than as
    // part of the top strip.
    {
        let painter = ui.painter();
        painter.rect_filled(area, 4.0 * s, theme::WELL);
        painter.line_segment(
            [area.left_bottom(), area.right_bottom()],
            theme::silkscreen_stroke(s),
        );
    }

    let font = theme::legend(11.0 * s);
    for (index, cell_rect) in layout::strip_cells(area, state.strip.cells.len())
        .into_iter()
        .enumerate()
    {
        let Some(cell) = state.strip.cells.get(index).cloned() else {
            continue;
        };
        let selected = index == state.strip.selected;
        {
            let painter = ui.painter();
            painter.rect_filled(cell_rect, 3.0 * s, theme::PANEL_RAISED);
            if selected {
                painter.rect_stroke(
                    cell_rect,
                    3.0 * s,
                    egui::Stroke::new(1.5 * s, theme::LED_ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            let name = text::fit(
                painter,
                &cell.name,
                font.clone(),
                name_rect(cell_rect).width(),
            );
            painter.text(
                name_rect(cell_rect).left_center(),
                Align2::LEFT_CENTER,
                name,
                font.clone(),
                if selected {
                    theme::SILKSCREEN
                } else {
                    theme::SILKSCREEN_DIM
                },
            );
            let lamp = lamp_rect(cell_rect);
            painter.circle_filled(
                lamp.center(),
                lamp.width() * 0.5,
                if cell.running {
                    theme::LED_ON
                } else {
                    theme::LED_OFF
                },
            );
        }

        // The cell's own hit area, so a click cannot be stolen by the neighbour it overlaps by a pixel.
        let response = ui.interact(
            cell_rect,
            ui.scope_id().with(format!("device-cell-{index}")),
            Sense::click(),
        );
        tips::hover(&response, state.tooltips, tips::Control::DeviceCell);
        if response.clicked() {
            state.device_request = Some(Request::Select(index));
        }

        let mute = transport::transport_button(
            ui,
            &format!("device-mute-{index}"),
            mute_rect(cell_rect),
            "M",
            cell.muted,
        );
        tips::hover(&mute, state.tooltips, tips::Control::DeviceMute);
        if mute.clicked() {
            // The index it was drawn for, not the selected one: muting the device on the panel when another was clicked
            // is the bug this carries the index to avoid.
            state.device_request = Some(Request::SetMute(index, !cell.muted));
        }
        if selected {
            // The caret is drawn and registered after the cell's own hit area, so a click on the pixels they share is the
            // caret's: a later widget wins a hit-test tie. It writes `Open`, not `Select` — the device is already on the
            // panel, and what the click asks for is its card.
            let caret = caret_rect(cell_rect);
            let stroke = egui::Stroke::new(1.5 * s, theme::LED_ACCENT);
            let middle = Pos2::new(caret.right(), caret.center().y);
            ui.painter()
                .line_segment([caret.left_top(), middle], stroke);
            ui.painter()
                .line_segment([middle, caret.left_bottom()], stroke);
            let response = ui.interact(
                caret,
                ui.scope_id().with(format!("device-caret-{index}")),
                Sense::click(),
            );
            tips::hover(&response, state.tooltips, tips::Control::DeviceCaret);
            if response.clicked() {
                state.device_request = Some(Request::Open(index));
            }
        }
    }

    paint_add(ui, state, area, s);
}

/// The `+` at the strip's right wall: two strokes, greyed at the cap. It stays clickable when the rack is full — the
/// refusal is `Rack::add`'s to say, and a dead key would say nothing at all.
fn paint_add(ui: &mut egui::Ui, state: &mut PanelState, area: Rect, s: f32) {
    let add = layout::strip_add_rect(area);
    // The cap is the catalogue's answer, not a second comparison here: `+` greys when no kind can be added.
    let full = !DeviceKind::ALL
        .iter()
        .any(|kind| kind.enabled(state.strip.cells.len()));
    let colour = if full {
        theme::LED_OFF
    } else {
        theme::SILKSCREEN_DIM
    };
    {
        let painter = ui.painter();
        let arm = add.width().min(add.height()) * 0.28;
        let middle = add.center();
        let stroke = egui::Stroke::new(2.0 * s, colour);
        painter.line_segment(
            [
                Pos2::new(middle.x - arm, middle.y),
                Pos2::new(middle.x + arm, middle.y),
            ],
            stroke,
        );
        painter.line_segment(
            [
                Pos2::new(middle.x, middle.y - arm),
                Pos2::new(middle.x, middle.y + arm),
            ],
            stroke,
        );
    }
    let response = ui.interact(add, ui.scope_id().with("device-add"), Sense::click());
    tips::hover(&response, state.tooltips, tips::Control::DeviceAdd);
    if response.clicked() {
        // The picker offers every kind there is; `+` only opens it. It opens even at the cap — a dead key would say
        // nothing at all, and the picker's greyed rows are what say the rack is full.
        state.device_request = Some(Request::OpenPicker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::MAX_DEVICES;

    fn area() -> Rect {
        Rect::from_min_size(Pos2::new(16.0, 8.0), egui::vec2(948.0, 30.0))
    }

    #[test]
    fn a_cell_is_wide_enough_for_a_name_and_the_lamp_never_moves() {
        for count in 1..=MAX_DEVICES {
            let cells = layout::strip_cells(area(), count);
            assert_eq!(cells.len(), count);
            // The lamp and the MUTE key are at the cell's right edge, so a wide name eats the name, never the lamp.
            let lamp = lamp_rect(cells[0]);
            let mute = mute_rect(cells[0]);
            assert!(lamp.right() <= cells[0].right());
            assert!(
                mute.left() >= lamp.right(),
                "the lamp and the key do not overlap"
            );
            assert!(
                name_rect(cells[0]).right() <= lamp.left(),
                "a name can never be drawn over the lamp"
            );
            assert!(name_rect(cells[0]).width() > 0.0, "a cell needs a name");
            assert!(cells[0].contains_rect(mute));
        }
    }
}
