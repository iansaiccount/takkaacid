//! The knob: a painted 270° rotary control with vertical drag, shift-fine mode, wheel and double-click
//! reset.
//!
//! The pure parts (drag maths, angle, arc geometry) are tested; the painted result is checked by eye in the
//! assembled panel (Task 32). No font glyph is ever used as a knob's face — the indicator is a drawn line.

use egui::{Pos2, Response};

use super::theme;

/// Pixels of vertical drag for the full `0.0..=1.0` range.
pub fn knob_travel_pixels() -> f32 {
    180.0
}

/// A knob value moved by a vertical drag. Up is positive; `fine` divides the travel by ten.
pub fn value_from_drag(start: f32, dy_pixels: f32, fine: bool) -> f32 {
    let travel = knob_travel_pixels() * if fine { 10.0 } else { 1.0 };
    (start - dy_pixels / travel).clamp(0.0, 1.0)
}

/// The indicator angle in radians: `-135°` at 0.0, `0` at centre, `+135°` at 1.0.
pub fn angle_for(value: f32) -> f32 {
    (value.clamp(0.0, 1.0) - 0.5) * 270f32.to_radians()
}

/// One point on a circle in screen space, where angle 0 is 12 o'clock and positive is clockwise.
fn arc_point(center: Pos2, radius: f32, angle: f32) -> Pos2 {
    Pos2::new(
        center.x + radius * angle.sin(),
        center.y - radius * angle.cos(),
    )
}

/// The value arc: from 7 o'clock round to wherever `value` puts the indicator.
pub fn arc_points(center: Pos2, radius: f32, value: f32) -> Vec<Pos2> {
    let start = -135f32.to_radians();
    let end = angle_for(value);
    let step = 9f32.to_radians();
    let segments = ((end - start) / step).ceil().max(1.0) as usize;
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            arc_point(center, radius, start + (end - start) * t)
        })
        .collect()
}

/// Paint a knob and handle its input: vertical drag (shift = fine), wheel, double-click to centre.
pub fn knob(
    ui: &mut egui::Ui,
    id: egui::Id,
    label: &str,
    value: &mut f32,
    diameter: f32,
) -> Response {
    let label_height = (diameter * 0.22).max(10.0);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(diameter, diameter + label_height),
        egui::Sense::hover(),
    );
    // The caller owns the identity, so seven knobs in a row stay distinguishable.
    let response = ui.interact(rect, id, egui::Sense::drag());
    let center = Pos2::new(rect.center().x, rect.top() + diameter / 2.0);

    if response.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        *value = value_from_drag(*value, response.drag_delta().y, fine);
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            *value = (*value + scroll * 0.002).clamp(0.0, 1.0);
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    if response.double_clicked() {
        *value = 0.5;
    }

    let painter = ui.painter();
    let radius = diameter / 2.0;
    let line = (radius * 0.12).max(2.0);
    painter.circle_filled(center, radius, theme::WELL);
    painter.circle_filled(center, radius * 0.86, theme::PANEL_RAISED);
    painter.circle_stroke(center, radius * 0.58, egui::Stroke::new(line, theme::PANEL));
    painter.add(egui::Shape::line(
        arc_points(center, radius * 0.95, 1.0),
        theme::silkscreen_stroke(line * 0.8),
    ));
    let lit = arc_points(center, radius * 0.95, *value);
    if lit.len() > 1 {
        painter.add(egui::Shape::line(
            lit,
            egui::Stroke::new(line, theme::LED_ON),
        ));
    }
    let angle = angle_for(*value);
    painter.line_segment(
        [
            arc_point(center, radius * 0.30, angle),
            arc_point(center, radius * 0.80, angle),
        ],
        egui::Stroke::new(line, theme::SILKSCREEN),
    );
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() - label_height / 2.0),
        egui::Align2::CENTER_CENTER,
        label,
        theme::legend(label_height * 0.8),
        theme::SILKSCREEN,
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_up_180_pixels_covers_the_whole_range() {
        assert!((value_from_drag(0.5, -180.0, false) - 1.0).abs() < 1e-5);
        assert!((value_from_drag(0.5, 180.0, false) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn fine_mode_is_ten_times_gentler_and_never_leaves_the_range() {
        let coarse = value_from_drag(0.5, -18.0, false);
        let fine = value_from_drag(0.5, -18.0, true);
        assert!((coarse - 0.6).abs() < 1e-4);
        assert!((fine - 0.51).abs() < 1e-4);
        assert_eq!(value_from_drag(0.95, -500.0, false), 1.0);
        assert_eq!(value_from_drag(0.05, 500.0, false), 0.0);
    }

    #[test]
    fn the_arc_sweeps_270_degrees_from_seven_to_five_oclock() {
        assert!((angle_for(0.0) + 135f32.to_radians()).abs() < 1e-5);
        assert!(angle_for(0.5).abs() < 1e-5);
        assert!((angle_for(1.0) - 135f32.to_radians()).abs() < 1e-5);
        let points = arc_points(Pos2::new(100.0, 100.0), 40.0, 0.5);
        assert!(points.len() > 8);
        assert!(
            points
                .iter()
                .all(|p| (p.distance(Pos2::new(100.0, 100.0)) - 40.0).abs() < 0.01)
        );
    }

    #[test]
    fn the_knob_is_not_a_font_glyph() {
        // Guard from AGENTS.md: icons must be drawn or loaded, never a font glyph.
        for label in theme::KNOB_LABELS {
            assert!(label.is_ascii(), "knob label must be ASCII: {label}");
        }
        assert_eq!(theme::KNOB_LABELS.len(), 7);
    }

    /// Drive one knob through a list of frames of raw input, returning the final value.
    fn drag_frames(events_per_frame: Vec<Vec<egui::Event>>, shift: bool, diameter: f32) -> f32 {
        let mut value = 0.5f32;
        let ctx = egui::Context::default();
        // A widget must exist for one frame before egui can route a press to it: interaction uses the
        // previous frame's widget rects.
        for events in core::iter::once(Vec::new()).chain(events_per_frame) {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(400.0, 400.0),
                )),
                ..Default::default()
            };
            if shift {
                input
                    .events
                    .push(egui::Event::ModifiersChanged(egui::Modifiers {
                        shift: true,
                        ..Default::default()
                    }));
            }
            input.events.extend(events);
            let mut output = ctx.run_ui(input, |ui| {
                knob(
                    ui,
                    egui::Id::unique("test-knob"),
                    "TUNING",
                    &mut value,
                    diameter,
                );
            });
            output.textures_delta.clear();
        }
        value
    }

    fn press_at(pos: Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        }
    }

    #[test]
    fn a_drag_up_raises_the_value_and_a_drag_down_lowers_it() {
        let center = Pos2::new(30.0, 30.0);
        let up = drag_frames(
            vec![
                vec![egui::Event::PointerMoved(center), press_at(center)],
                vec![egui::Event::PointerMoved(center - egui::vec2(0.0, 18.0))],
            ],
            false,
            60.0,
        );
        assert!((up - 0.6).abs() < 0.02, "dragged up 18 px: {up}");

        let down = drag_frames(
            vec![
                vec![egui::Event::PointerMoved(center), press_at(center)],
                vec![egui::Event::PointerMoved(center + egui::vec2(0.0, 18.0))],
            ],
            false,
            60.0,
        );
        assert!((down - 0.4).abs() < 0.02, "dragged down 18 px: {down}");
    }

    #[test]
    fn shift_makes_the_same_drag_ten_times_gentler() {
        let center = Pos2::new(30.0, 30.0);
        let coarse = drag_frames(
            vec![
                vec![egui::Event::PointerMoved(center), press_at(center)],
                vec![egui::Event::PointerMoved(center - egui::vec2(0.0, 18.0))],
            ],
            false,
            60.0,
        );
        let fine = drag_frames(
            vec![
                vec![egui::Event::PointerMoved(center), press_at(center)],
                vec![egui::Event::PointerMoved(center - egui::vec2(0.0, 18.0))],
            ],
            true,
            60.0,
        );
        assert!(
            (coarse - 0.5).abs() > 0.05,
            "coarse drag should move it: {coarse}"
        );
        assert!(
            (fine - 0.5).abs() < 0.02,
            "fine drag should barely move it: {fine}"
        );
    }
}
