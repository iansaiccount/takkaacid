//! When the output picker should close: a click outside it, or Escape.

use eframe::egui::{Pos2, Rect};

/// True when a click at `click` should close the picker without choosing a device.
///
/// The status line itself is protected: clicking it toggles the list, so it is never a "click outside".
pub fn should_close(click: Option<Pos2>, rows: &[Rect], status_line: Rect) -> bool {
    let Some(at) = click else {
        return false;
    };
    !status_line.contains(at) && !rows.iter().any(|row| row.contains(at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    fn row(y: f32) -> Rect {
        Rect::from_min_size(Pos2::new(100.0, y), vec2(360.0, 20.0))
    }

    #[test]
    fn a_click_outside_the_list_closes_the_picker() {
        let rows = [row(200.0), row(220.0)];
        let status = Rect::from_min_size(Pos2::new(100.0, 300.0), vec2(420.0, 18.0));
        assert!(should_close(Some(Pos2::new(600.0, 120.0)), &rows, status));
        assert!(!should_close(None, &rows, status), "no click, no close");
    }

    #[test]
    fn clicking_a_device_name_or_the_status_line_leaves_the_picker_alone() {
        let rows = [row(200.0), row(220.0)];
        let status = Rect::from_min_size(Pos2::new(100.0, 300.0), vec2(420.0, 18.0));
        assert!(!should_close(Some(Pos2::new(200.0, 225.0)), &rows, status));
        assert!(!should_close(Some(Pos2::new(200.0, 305.0)), &rows, status));
    }
}
