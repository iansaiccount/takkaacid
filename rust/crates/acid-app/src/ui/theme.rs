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
    /// The face of an *unlit* key — a transport key, an octave key, `RANDOM`. Its own role because in the amber scheme it
    /// is the one surface that is *lighter* than the face, where the wells are darker: the face is the subtle gold and the
    /// keys stand off it.
    pub key_face: Color32,
    /// Ink for captions and pointers drawn on `panel_raised`: a key's label, a knob's pointer line.
    ///
    /// The face's ink will not do: it is chosen to read against the face, and a raised control is the *other* family's
    /// surface in one of the two schemes, so a caption painted in the silkscreen would be near-black on a near-black knob
    /// the moment the face inverts.
    pub raised_ink: Color32,
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
    /// Accent indicator: ink on the *face*, and on the cream a mark is drawn over.
    pub led_accent: Color32,
    /// The legend of a lit key, drawn on that key's `panel_raised` face. Not `led_accent`: that one is ink for the face,
    /// and the two uses are disjoint in the amber scheme — a colour dark enough to read on the amber face is too dark to
    /// read on a near-black key.
    pub lit_ink: Color32,
    /// Display digits.
    pub text: Color32,
    /// Step keycap face.
    pub white_key: Color32,
    /// Ink for lettering and marks on the cream keycap, whose paper does not change between schemes.
    pub keycap_ink: Color32,
    /// The scrim under a card: the face behind it darkens, never disappears, so the panel still reads as the instrument
    /// rather than as a picture of one. Black in both schemes — it darkens whatever it is laid on.
    pub shade: Color32,
    /// The "you can pick this" colour while a learn is choosing.
    ///
    /// Not one of the panel's own lights, so it cannot be mistaken for a lit key — but it has to be *seen*, and on the
    /// amber face a cyan would be a hue difference with almost no luminance step (1.8:1, too weak for a cue). Each scheme
    /// therefore carries its own: cyan on the near-black face, a deep teal on the gold one.
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
    key_face: Color32::from_rgb(0x04, 0x04, 0x04),
    raised_ink: Color32::from_rgb(0xFF, 0xC3, 0x00),
    well: Color32::from_rgb(0x04, 0x04, 0x04),
    silkscreen: Color32::from_rgb(0xFF, 0xC3, 0x00),
    silkscreen_dim: Color32::from_rgb(0x7A, 0x5C, 0x00),
    led_on: Color32::from_rgb(0xFF, 0xE0, 0x66),
    led_off: Color32::from_rgb(0x2A, 0x22, 0x08),
    led_accent: Color32::from_rgb(0xFF, 0x95, 0x00),
    lit_ink: Color32::from_rgb(0xFF, 0x95, 0x00),
    text: Color32::from_rgb(0xF5, 0xF0, 0xE0),
    white_key: Color32::from_rgb(0xE8, 0xE2, 0xD0),
    keycap_ink: Color32::from_rgb(0x0B, 0x0B, 0x0B),
    shade: Color32::from_black_alpha(90),
    learn_target: Color32::from_rgb(0x3D, 0xE8, 0xF0),
};

/// The same two families the other way round: an amber face, near-black legends, dark knobs.
///
/// Hand-drawn rather than a byte inverse. An inversion would make the wells — the displays and knob recesses — the
/// brightest shapes on the face, and every reading rule the panel depends on ("a well is a hole in the face") would break.
/// What is exchanged is the two *families*: the face becomes the amber and the engraving becomes the near-black.
pub const YELLOW: Palette = Palette {
    panel: Color32::from_rgb(0xC9, 0x94, 0x00),
    panel_raised: Color32::from_rgb(0x14, 0x10, 0x08),
    // The old face's gold, now the keys': the face is the quieter gold below, so what a hand rests on stands out of it.
    key_face: Color32::from_rgb(0xFF, 0xC3, 0x00),
    raised_ink: Color32::from_rgb(0xFF, 0xE9, 0xB0),
    well: Color32::from_rgb(0x9E, 0x76, 0x00),
    silkscreen: Color32::from_rgb(0x0B, 0x0B, 0x0B),
    silkscreen_dim: Color32::from_rgb(0x3A, 0x2E, 0x00),
    // On a bright face nothing can be both brighter and contrasting, so the lamp rule inverts: a lit lamp is a *mark*
    // (dark enough to be seen on the amber) and an unlit one is invisible, exactly as black's `#2A2208` almost is on `#040404`.
    led_on: Color32::from_rgb(0x62, 0x16, 0x00),
    led_off: Color32::from_rgb(0xD9, 0xA6, 0x00),
    // Deep enough to outshout the dim legend: UNSAVED wears this colour and SAVED wears `silkscreen_dim`, and the quieter
    // of the two must be the one that is not shouting.
    led_accent: Color32::from_rgb(0x4E, 0x13, 0x00),
    lit_ink: Color32::from_rgb(0xFF, 0xC9, 0x7A),
    text: Color32::from_rgb(0x0B, 0x0B, 0x0B),
    white_key: Color32::from_rgb(0xE8, 0xE2, 0xD0),
    keycap_ink: Color32::from_rgb(0x0B, 0x0B, 0x0B),
    shade: Color32::from_black_alpha(90),
    learn_target: Color32::from_rgb(0x0B, 0x4E, 0x55),
};

/// The palette a device's face is painted in.
pub fn palette(scheme: crate::device::Scheme) -> &'static Palette {
    match scheme {
        crate::device::Scheme::Black => &BLACK,
        crate::device::Scheme::Yellow => &YELLOW,
    }
}

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
    fn the_amber_face_is_a_quieter_gold_than_the_keys_on_it() {
        assert!(
            luminance(BLACK.panel) < 0.02,
            "the black scheme's face is the darkest thing on screen"
        );
        // The amber face is the quieter of the two golds: bright enough to read as a lit machine, quiet enough that a whole
        // window of it does not glare. The gold a hand rests on — the keys — is the brighter one.
        assert!(
            luminance(YELLOW.panel) > 0.3 && luminance(YELLOW.panel) < luminance(YELLOW.key_face),
            "the amber face is a mid gold, and the keys stand out of it"
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
            // The dim legend is also drawn on the well — a key's border, the knob's unlit scale arc, the strip's shelf —
            // so it has to be seen there too.
            assert!(
                contrast(p.well, p.silkscreen_dim) >= 3.0,
                "{scheme:?}: the dim legend must read on the well as well as the face"
            );
            assert!(
                contrast(p.panel, p.silkscreen) > contrast(p.panel, p.silkscreen_dim),
                "{scheme:?}: the dim legend must be the quieter of the two"
            );
            // A lamp is a light on a dark face and a mark on a bright one, so the rule both schemes keep is contrast
            // against the surface: a lit lamp is seen, an unlit one disappears into the face.
            assert!(
                contrast(p.panel, p.led_on) >= 3.0,
                "{scheme:?}: a lit lamp must stand out from the face"
            );
            assert!(
                contrast(p.panel, p.led_off) < 2.0,
                "{scheme:?}: an unlit lamp must not shout from the face"
            );
            // The knob's lit scale arc is drawn on the well, not the face, and the strip's lamp sits in one.
            assert!(
                contrast(p.well, p.led_on) >= 3.0,
                "{scheme:?}: a lit lamp must stand out from the well too"
            );
            // The display digits read against their own well, not against the face.
            assert!(
                contrast(p.well, p.text) >= 4.5,
                "{scheme:?}: display digits need contrast against the well"
            );
            // Captions and pointers are drawn on a raised control, not on the face, so they need their own ink: the
            // face's would be a dark amber on a dark knob in the amber scheme.
            assert!(
                contrast(p.panel_raised, p.raised_ink) >= 4.5,
                "{scheme:?}: captions on a raised control need contrast against it"
            );
            // A lit key is a raised face with a legend on it, which is a different reading from the accent's on the face.
            assert!(
                contrast(p.panel_raised, p.lit_ink) >= 4.5,
                "{scheme:?}: a lit key's legend needs contrast against its own face"
            );
            // A key's legend is drawn on the key's own face, which in the amber scheme is lighter than the face around
            // it — so the pair to check is the key's, not the panel's.
            assert!(
                contrast(p.key_face, p.silkscreen) >= 4.5,
                "{scheme:?}: a key's legend needs contrast against the key"
            );
            // The keycap's paper is cream in both schemes, so its lettering is fixed and must be read on its own.
            assert!(
                contrast(p.white_key, p.keycap_ink) >= 4.5,
                "{scheme:?}: a keycap's lettering needs contrast against the cream"
            );
            // The learn target is a different kind of light, in both schemes.
            assert_ne!(p.learn_target, p.silkscreen);
            assert_ne!(p.learn_target, p.led_on);
            assert!(
                contrast(p.panel, p.learn_target) >= 3.0,
                "{scheme:?}: the learn outline is a cue, not text, but it must still be seen on the face"
            );
            // UNSAVED is the one that has to catch the eye, so the dirty colour is the louder of the two labels.
            assert!(
                contrast(p.panel, p.led_accent) > contrast(p.panel, p.silkscreen_dim),
                "{scheme:?}: UNSAVED must not read quieter than SAVED"
            );
            // The accent also rings a hovered, unlit key, whose face is a well.
            assert!(
                contrast(p.well, p.led_accent) >= 3.0,
                "{scheme:?}: the hover ring on an unlit key must be seen on the well"
            );
        }
        // Two schemes is two palettes: a `palette` that returned one of them twice would satisfy every rule above.
        assert_ne!(palette(Scheme::Black), palette(Scheme::Yellow));
    }

    #[test]
    fn the_two_families_are_exchanged() {
        // What was the face is now the engraving, and what was the engraving is now the gold a key is painted: the swap this
        // scheme exists for, with the bright gold moved one surface out so the face itself is the quieter of the two.
        assert_eq!(YELLOW.silkscreen, BLACK.panel);
        assert_eq!(YELLOW.key_face, BLACK.silkscreen);
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
        // The two inks black has always used: legends on a raised control are the silkscreen's amber, and the keycap's
        // lettering is the panel's near-black. Giving them their own fields must not move a single pixel of this scheme.
        assert_eq!(BLACK.raised_ink, BLACK.silkscreen);
        assert_eq!(BLACK.keycap_ink, BLACK.panel);
        // The accent's two roles were split in this round, not recoloured: a lit key's legend is still the amber it was.
        assert_eq!(BLACK.lit_ink, BLACK.led_accent);
    }
}
