//! The panel's palette and typography — the only place a colour literal may appear.
//!
//! Black and yellow: a near-black face, amber silkscreen, yellow LEDs. Nothing here is a stock widget
//! colour, and no other module may name a hex value.

use egui::{Color32, FontId};

/// The panel's design size in logical pixels; every layout dimension is a fraction of this.
/// The panel's design size in logical pixels; every layout dimension is a fraction of this. The strip band above the top
/// strip cost the panel 30 px; every band below it keeps the height it had.
pub const DESIGN_SIZE: egui::Vec2 = egui::vec2(980.0, 670.0);

/// Default window size — the panel at 1× scale.
pub const WINDOW_SIZE: [f32; 2] = [980.0, 670.0];

/// Inset from the panel edge to the first drawn element.
pub const MARGIN: f32 = 16.0;

/// Gap between neighbouring controls.
pub const GAP: f32 = 6.0;

/// Radius of the four corner screws at 1× scale.
pub const SCREW_RADIUS: f32 = 4.0;

/// The colours a face is painted in — one of these per scheme, and no colour literal anywhere else.
///
/// Value rather than consts now: the panel paints the face of the device it is showing, and that device may ask for the other
/// scheme. `Copy`, so a painter can take one by value without ceremony.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    /// The face: the largest surface on screen.
    pub panel: Color32,
    /// Knob bodies, raised key faces and the cards laid over the face.
    pub panel_raised: Color32,
    /// Recessed areas: display wells, knob recesses, the slab's own surround.
    pub well: Color32,
    /// Legends, frames, engraving.
    pub silkscreen: Color32,
    /// Secondary legends and unlit step numbers.
    pub silkscreen_dim: Color32,
    /// A lit LED's core.
    pub led_on: Color32,
    /// An unlit LED.
    pub led_off: Color32,
    /// Accent indicator.
    pub led_accent: Color32,
    /// Display digits.
    pub text: Color32,
    /// Step keycap face.
    pub white_key: Color32,
    /// The scrim under a card: the face behind it darkens, never disappears, so the panel still reads as the instrument
    /// rather than as a picture of one. Black in both schemes — it darkens whatever it is laid on.
    pub shade: Color32,
    /// The "you can pick this" colour while a learn is choosing.
    ///
    /// Cyan against amber: nearly complementary, so it reads as a different kind of light rather than a brighter one of the
    /// panel's own. Nothing else on the face is this colour, so it cannot be mistaken for a lit key — in either scheme.
    pub learn_target: Color32,
}

impl Palette {
    /// An engraving stroke: the dim legend, never pure white, never a shadow.
    pub fn silkscreen_stroke(&self, width: f32) -> egui::Stroke {
        egui::Stroke::new(width, self.silkscreen_dim)
    }
}

/// The instrument's own face: near black, amber silkscreen, yellow LEDs.
pub const BLACK: Palette = Palette {
    panel: Color32::from_rgb(0x0B, 0x0B, 0x0B),
    panel_raised: Color32::from_rgb(0x19, 0x19, 0x19),
    well: Color32::from_rgb(0x04, 0x04, 0x04),
    silkscreen: Color32::from_rgb(0xFF, 0xC3, 0x00),
    silkscreen_dim: Color32::from_rgb(0x7A, 0x5C, 0x00),
    led_on: Color32::from_rgb(0xFF, 0xE0, 0x66),
    led_off: Color32::from_rgb(0x2A, 0x22, 0x08),
    led_accent: Color32::from_rgb(0xFF, 0x95, 0x00),
    text: Color32::from_rgb(0xF5, 0xF0, 0xE0),
    white_key: Color32::from_rgb(0xE8, 0xE2, 0xD0),
    shade: Color32::from_black_alpha(90),
    learn_target: Color32::from_rgb(0x3D, 0xE8, 0xF0),
};

/// The same two families the other way round: an amber face, near-black legends, dark knobs.
///
/// Hand-drawn rather than a byte inverse. An inversion would make the wells — the displays and knob recesses — the
/// brightest shapes on the face, and every reading rule the panel depends on ("a well is a hole in the face") would break.
/// What is exchanged is the two *families*: the face becomes the amber and the engraving becomes the near-black.
pub const YELLOW: Palette = Palette {
    panel: Color32::from_rgb(0xFF, 0xC3, 0x00),
    panel_raised: Color32::from_rgb(0x14, 0x10, 0x08),
    well: Color32::from_rgb(0xC9, 0x94, 0x00),
    silkscreen: Color32::from_rgb(0x0B, 0x0B, 0x0B),
    silkscreen_dim: Color32::from_rgb(0x6B, 0x55, 0x00),
    led_on: Color32::from_rgb(0xFF, 0xFD, 0xF0),
    led_off: Color32::from_rgb(0x8A, 0x6A, 0x00),
    led_accent: Color32::from_rgb(0xB2, 0x3A, 0x00),
    text: Color32::from_rgb(0x0B, 0x0B, 0x0B),
    white_key: Color32::from_rgb(0xE8, 0xE2, 0xD0),
    shade: Color32::from_black_alpha(90),
    learn_target: Color32::from_rgb(0x3D, 0xE8, 0xF0),
};

/// The palette a device's face is painted in.
pub fn palette(scheme: crate::device::Scheme) -> &'static Palette {
    match scheme {
        crate::device::Scheme::Black => &BLACK,
        crate::device::Scheme::Yellow => &YELLOW,
    }
}

// The one palette's colours under their old names, while the painters are moved over one file at a time. Removed when the
// last of them takes a `Palette`; `BLACK` and `YELLOW` are the names afterwards.
/// Panel face — near black, never pure black.
pub const PANEL: Color32 = BLACK.panel;
/// Knob bodies and raised button faces.
pub const PANEL_RAISED: Color32 = BLACK.panel_raised;
/// Recessed areas: display wells, knob recesses.
pub const WELL: Color32 = BLACK.well;
/// Legends, frames, engraving.
pub const SILKSCREEN: Color32 = BLACK.silkscreen;
/// Secondary legends and unlit step numbers.
pub const SILKSCREEN_DIM: Color32 = BLACK.silkscreen_dim;
/// A lit LED's core.
pub const LED_ON: Color32 = BLACK.led_on;
/// The scrim under a card.
pub const SHADE: Color32 = BLACK.shade;
/// Accent indicator.
pub const LED_ACCENT: Color32 = BLACK.led_accent;
/// An unlit LED.
pub const LED_OFF: Color32 = BLACK.led_off;
/// Display digits.
pub const TEXT: Color32 = BLACK.text;
/// Step keycap face.
pub const WHITE_KEY: Color32 = BLACK.white_key;
/// The "you can pick this" colour while a learn is choosing.
pub const LEARN_TARGET: Color32 = BLACK.learn_target;

/// The silkscreen font at a given size.
pub fn legend(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Proportional)
}

/// An engraving stroke in the black scheme: amber-dim, never pure white, never a shadow.
pub fn silkscreen_stroke(width: f32) -> egui::Stroke {
    BLACK.silkscreen_stroke(width)
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
    use crate::device::Scheme;

    /// WCAG's contrast ratio: the pair of luminances, brighter over darker. What the eye reads, and what a scheme has to keep.
    fn contrast(a: Color32, b: Color32) -> f32 {
        let (bright, dark) = (luminance(a) + 0.05, luminance(b) + 0.05);
        if bright > dark {
            bright / dark
        } else {
            dark / bright
        }
    }

    #[test]
    fn the_face_is_dark_in_one_scheme_and_bright_in_the_other() {
        assert!(
            luminance(BLACK.panel) < 0.02,
            "the black scheme's face is the darkest thing on screen"
        );
        assert!(
            luminance(YELLOW.panel) > 0.5,
            "and the yellow scheme's is the brightest: that is the whole signal"
        );
    }

    #[test]
    fn both_schemes_hold_the_same_reading_rules() {
        for scheme in Scheme::ALL {
            let p = palette(scheme);
            // A recess is darker than the face in both schemes: that is what makes it read as a well.
            assert!(
                luminance(p.well) < luminance(p.panel),
                "{scheme:?}: the well must sit below the face"
            );
            // Engraving is legible against the face; dim engraving is legible but quieter.
            assert!(
                contrast(p.panel, p.silkscreen) >= 4.5,
                "{scheme:?}: legends need contrast against the face"
            );
            assert!(
                contrast(p.panel, p.silkscreen_dim) > 2.0,
                "{scheme:?}: a dim legend must still read"
            );
            assert!(
                contrast(p.panel, p.silkscreen) > contrast(p.panel, p.silkscreen_dim),
                "{scheme:?}: the dim legend must be the quieter of the two"
            );
            // A lit lamp outshines the face; an unlit one does not.
            assert!(
                luminance(p.led_on) > luminance(p.panel),
                "{scheme:?}: a lit lamp must be brighter than the face"
            );
            assert!(
                luminance(p.led_off) < luminance(p.led_on),
                "{scheme:?}: an unlit lamp is the darker of the pair"
            );
            // The display digits read against their own well, not against the face.
            assert!(
                contrast(p.well, p.text) >= 4.5,
                "{scheme:?}: display digits need contrast against the well"
            );
            // The learn target is a different kind of light, in both schemes.
            assert_ne!(p.learn_target, p.silkscreen);
            assert_ne!(p.learn_target, p.led_on);
        }
    }

    #[test]
    fn the_two_families_are_exchanged() {
        // What was the face is now the engraving, and the other way round: the swap this round exists for.
        assert_eq!(YELLOW.silkscreen, BLACK.panel);
        assert_eq!(YELLOW.panel, BLACK.silkscreen);
        // Black's face is near-black, not pure black; so is Yellow's engraving.
        assert_ne!(BLACK.panel, Color32::BLACK);
    }

    #[test]
    fn leds_read_as_on_and_off_in_the_black_scheme() {
        assert!(luminance(BLACK.led_on) > 0.5);
        assert!(luminance(BLACK.led_off) < 0.1);
        assert!(luminance(BLACK.led_accent) > 0.25);
        assert!(luminance(BLACK.text) > 0.8);
    }

    #[test]
    fn the_black_scheme_is_the_instrument_it_has_always_been() {
        assert!(luminance(BLACK.panel_raised) < luminance(BLACK.silkscreen));
        assert!(luminance(BLACK.well) < luminance(BLACK.panel));
        let c = BLACK.silkscreen;
        assert!(
            c.r() > 200 && c.g() > 150 && c.b() < 80,
            "expected amber, got {c:?}"
        );
    }
}
