//! The panel's palette and typography — the only place a colour literal may appear.
//!
//! Black and yellow: a near-black face, amber silkscreen, yellow LEDs. Nothing here is a stock widget
//! colour, and no other module may name a hex value.

use egui::{Color32, FontId};

/// The panel's design size in logical pixels; every layout dimension is a fraction of this.
pub const DESIGN_SIZE: egui::Vec2 = egui::vec2(980.0, 560.0);

/// Default window size — the panel at 1× scale.
pub const WINDOW_SIZE: [f32; 2] = [980.0, 560.0];

/// Inset from the panel edge to the first drawn element.
pub const MARGIN: f32 = 16.0;

/// Gap between neighbouring controls.
pub const GAP: f32 = 6.0;

/// Panel face — near black, never pure black.
pub const PANEL: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0B);
/// Knob bodies and raised button faces.
pub const PANEL_RAISED: Color32 = Color32::from_rgb(0x19, 0x19, 0x19);
/// Recessed areas: display wells, knob recesses.
pub const WELL: Color32 = Color32::from_rgb(0x04, 0x04, 0x04);
/// Legends, frames, engraving.
pub const SILKSCREEN: Color32 = Color32::from_rgb(0xFF, 0xC3, 0x00);
/// Secondary legends and unlit step numbers.
pub const SILKSCREEN_DIM: Color32 = Color32::from_rgb(0x7A, 0x5C, 0x00);
/// A lit LED's core.
pub const LED_ON: Color32 = Color32::from_rgb(0xFF, 0xE0, 0x66);
/// Accent indicator.
pub const LED_ACCENT: Color32 = Color32::from_rgb(0xFF, 0x95, 0x00);
/// An unlit LED.
pub const LED_OFF: Color32 = Color32::from_rgb(0x2A, 0x22, 0x08);
/// Display digits.
pub const TEXT: Color32 = Color32::from_rgb(0xF5, 0xF0, 0xE0);
/// Step keycap face.
pub const WHITE_KEY: Color32 = Color32::from_rgb(0xE8, 0xE2, 0xD0);

/// The silkscreen font at a given size.
pub fn legend(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Proportional)
}

/// Rec. 709 relative luminance in `0.0..=1.0`, computed on linearised sRGB so dark colours separate
/// the way the eye sees them. Tests use it to keep the panel dark and the LEDs bright.
pub fn luminance(c: Color32) -> f32 {
    fn linear(v: u8) -> f32 {
        let v = v as f32 / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_is_the_darkest_thing_on_screen() {
        assert!(luminance(PANEL) < 0.02);
        assert!(luminance(PANEL_RAISED) < luminance(SILKSCREEN));
        assert!(luminance(WELL) < luminance(PANEL));
    }

    #[test]
    fn leds_read_as_on_and_off() {
        assert!(luminance(LED_ON) > 0.5);
        assert!(luminance(LED_OFF) < 0.1);
        assert!(luminance(LED_ACCENT) > 0.25);
        assert!(luminance(TEXT) > 0.8);
    }

    #[test]
    fn silkscreen_is_yellow_not_white() {
        let c = SILKSCREEN;
        assert!(
            c.r() > 200 && c.g() > 150 && c.b() < 80,
            "expected amber, got {c:?}"
        );
    }
}
