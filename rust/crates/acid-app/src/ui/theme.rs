//! The panel's palette and typography — the only place a colour literal may appear.
//!
//! Black and yellow: a near-black face, amber silkscreen, yellow LEDs. Nothing here is a stock widget
//! colour, and no other module may name a hex value.

use egui::{Color32, FontId};

/// The panel's design size in logical pixels; every layout dimension is a fraction of this.
pub const DESIGN_SIZE: egui::Vec2 = egui::vec2(980.0, 640.0);

/// Default window size — the panel at 1× scale.
pub const WINDOW_SIZE: [f32; 2] = [980.0, 640.0];

/// Inset from the panel edge to the first drawn element.
pub const MARGIN: f32 = 16.0;

/// Gap between neighbouring controls.
pub const GAP: f32 = 6.0;

/// Radius of the four corner screws at 1× scale.
pub const SCREW_RADIUS: f32 = 4.0;

/// The seven knob legends, left to right. ASCII only: a knob has no glyph face to fall back on.
pub const KNOB_LABELS: [&str; 7] = [
    "WAVEFORM",
    "TUNING",
    "CUT OFF",
    "RESONANCE",
    "ENV MOD",
    "DECAY",
    "ACCENT",
];

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

/// The "you can pick this" colour while a learn is choosing.
///
/// Cyan against amber: nearly complementary, so it reads as a different kind of light rather than a brighter one of the
/// panel's own. Nothing else on the face is this colour, so it cannot be mistaken for a lit key.
pub const LEARN_TARGET: Color32 = Color32::from_rgb(0x3D, 0xE8, 0xF0);

/// The silkscreen font at a given size.
pub fn legend(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Proportional)
}

/// An engraving stroke: amber-dim, never pure white, never a shadow.
pub fn silkscreen_stroke(width: f32) -> egui::Stroke {
    egui::Stroke::new(width, SILKSCREEN_DIM)
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
