//! The panel's palettes and typography — the only place a colour literal may appear.
//!
//! Twenty-two faces: the instrument's own black, and twenty-one derived from the five-colour palettes on
//! <https://www.media.io/color-palette/music-color-palette.html>. A theme is five colours; [`derive`] turns them into the
//! eighteen roles a face needs, and the reading rules in this module's tests are what stops a theme being unreadable. No
//! other module may name a hex value.

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

/// Which end of its palette a theme's face is taken from.
///
/// A theme is five colours; which of them is the face is a decision, not an ordering, and it is the one decision the rule
/// cannot make: `Vinyl Noir` is a dark room with a brass lamp, `Choir Pastel` is a light room with charcoal type.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Polar {
    /// The face is the darkest colour; legends and lamps are light.
    Dark,
    /// The face is the lightest colour; legends and lamps are dark. The page says "canvas" when it means this.
    Light,
}

/// A role a theme's five colours could not satisfy, set by hand.
///
/// Visible in the table on purpose: this is where taste overruled the rule, and each one carries the rule it was needed for.
#[derive(Copy, Clone, Default)]
pub struct Tune {
    /// The face, when the page's anchor is one another theme also uses. Two themes with one face is a list of identical
    /// squares; the anchor is a starting point, not a vow.
    pub panel: Option<Color32>,
    pub well: Option<Color32>,
    pub key_face: Option<Color32>,
    pub silkscreen: Option<Color32>,
    pub silkscreen_dim: Option<Color32>,
    pub led_on: Option<Color32>,
    pub led_accent: Option<Color32>,
    pub learn_target: Option<Color32>,
    pub raised_ink: Option<Color32>,
    pub lit_ink: Option<Color32>,
}

/// One theme's source: the five colours the page prints, which end the face comes from, and its hand-set roles.
#[derive(Copy, Clone)]
pub struct Row {
    pub polar: Polar,
    pub colours: [Color32; 5],
    pub tune: Tune,
}

/// A fixed colour with no theme's fingerprints on it: the keycap's cream, its near-black lettering, and the scrim.
const CREAM: Color32 = Color32::from_rgb(0xE8, 0xE2, 0xD0);
const KEYCAP_INK: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0B);
const SCRIM: Color32 = Color32::from_black_alpha(90);

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// The page's twenty-one palettes, in its order. `Scheme::ALL`'s arms line up with this table after `Black`, which is a
/// `const` of its own (`BLACK`) because it must never move.
pub const ROWS: [Row; THEMES - 1] = [
    // 1 — Vinyl Noir
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x0B, 0x0B, 0x0D),
            rgb(0x1F, 0x29, 0x33),
            rgb(0xB0, 0x8D, 0x57),
            rgb(0xE6, 0xE1, 0xD8),
            rgb(0x6B, 0x72, 0x80),
        ],
        tune: Tune::NONE,
    },
    // 2 — Stage Spotlight
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x11, 0x18, 0x27),
            rgb(0xF5, 0x9E, 0x0B),
            rgb(0xFD, 0xE6, 0x8A),
            rgb(0xEF, 0x44, 0x44),
            rgb(0xF9, 0xFA, 0xFB),
        ],
        tune: Tune::NONE,
    },
    // 3 — Jazz Lounge. The page's own darkest is the espresso Bassline Ember also uses, so the face is that espresso warmed
    // towards this theme's burnt orange: the same room, a different lamp.
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x2B, 0x1D, 0x1A),
            rgb(0x7C, 0x2D, 0x12),
            rgb(0xD9, 0x77, 0x06),
            rgb(0xF3, 0xE8, 0xD4),
            rgb(0x1C, 0x19, 0x17),
        ],
        tune: Tune {
            panel: Some(rgb(0x34, 0x1E, 0x16)),
            ..Tune::NONE
        },
    },
    // 4 — Synth Neon. Orchestral Ice has the page's midnight, so this one is the same midnight tinted towards its own violet.
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x0F, 0x17, 0x2A),
            rgb(0x7C, 0x3A, 0xED),
            rgb(0x22, 0xD3, 0xEE),
            rgb(0xF4, 0x72, 0xB6),
            rgb(0xF8, 0xFA, 0xFC),
        ],
        tune: Tune {
            panel: Some(rgb(0x1F, 0x1C, 0x47)),
            ..Tune::NONE
        },
    },
    // 5 — Acoustic Sunrise
    Row {
        polar: Polar::Light,
        colours: [
            rgb(0xFD, 0xE2, 0xC2),
            rgb(0xF5, 0x9E, 0x0B),
            rgb(0xA1, 0x62, 0x07),
            rgb(0x3F, 0x62, 0x12),
            rgb(0xFF, 0xFB, 0xEB),
        ],
        tune: Tune::NONE,
    },
    // 6 — Festival Citrus. Orchestral Ice has the page's navy; this one is the same navy with the green in it.
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x0E, 0xA5, 0xE9),
            rgb(0x22, 0xC5, 0x5E),
            rgb(0xF9, 0x73, 0x16),
            rgb(0xFA, 0xCC, 0x15),
            rgb(0x0F, 0x17, 0x2A),
        ],
        tune: Tune {
            panel: Some(rgb(0x11, 0x2C, 0x30)),
            ..Tune::NONE
        },
    },
    // 7 — Indie Dusk
    Row {
        polar: Polar::Light,
        colours: [
            rgb(0x2D, 0x1B, 0x69),
            rgb(0x4B, 0x55, 0x63),
            rgb(0xFC, 0xA5, 0xA5),
            rgb(0xF3, 0xF4, 0xF6),
            rgb(0x11, 0x18, 0x27),
        ],
        tune: Tune::NONE,
    },
    // 8 — Opera Velvet
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x4C, 0x05, 0x19),
            rgb(0x9F, 0x12, 0x39),
            rgb(0xF1, 0xC4, 0x0F),
            rgb(0xF5, 0xF3, 0xFF),
            rgb(0x1F, 0x29, 0x37),
        ],
        tune: Tune::NONE,
    },
    // 9 — Lo Fi Rain
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x0B, 0x13, 0x20),
            rgb(0x33, 0x41, 0x55),
            rgb(0x94, 0xA3, 0xB8),
            rgb(0xC7, 0xD2, 0xFE),
            rgb(0xE2, 0xE8, 0xF0),
        ],
        tune: Tune::NONE,
    },
    // 10 — Drumline Concrete. Same anchor as Stage Spotlight, so its face is its own concrete grey instead: two themes with
    // one face would be a list of identical squares.
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x11, 0x18, 0x27),
            rgb(0x37, 0x41, 0x51),
            rgb(0x9C, 0xA3, 0xAF),
            rgb(0xF9, 0x73, 0x16),
            rgb(0xF3, 0xF4, 0xF6),
        ],
        tune: Tune {
            panel: Some(rgb(0x37, 0x41, 0x51)),
            ..Tune::NONE
        },
    },
    // 11 — Choir Pastel
    Row {
        polar: Polar::Light,
        colours: [
            rgb(0xFC, 0xE7, 0xF3),
            rgb(0xDD, 0xD6, 0xFE),
            rgb(0xC7, 0xF9, 0xCC),
            rgb(0xBA, 0xE6, 0xFD),
            rgb(0x1F, 0x29, 0x37),
        ],
        tune: Tune::NONE,
    },
    // 12 — Bassline Ember
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x1C, 0x19, 0x17),
            rgb(0x7F, 0x1D, 0x1D),
            rgb(0xEA, 0x58, 0x0C),
            rgb(0xFD, 0xBA, 0x74),
            rgb(0xFA, 0xFA, 0xF9),
        ],
        tune: Tune::NONE,
    },
    // 13 — Pop Bubblegum
    Row {
        polar: Polar::Light,
        colours: [
            rgb(0xFF, 0x4D, 0x8D),
            rgb(0xFF, 0xB3, 0xC7),
            rgb(0x7C, 0x3A, 0xED),
            rgb(0xFD, 0xE0, 0x47),
            rgb(0xFF, 0xF7, 0xED),
        ],
        tune: Tune::NONE,
    },
    // 14 — Orchestral Ice
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x0F, 0x17, 0x2A),
            rgb(0x1D, 0x4E, 0xD8),
            rgb(0x93, 0xC5, 0xFD),
            rgb(0xE0, 0xF2, 0xFE),
            rgb(0xF8, 0xFA, 0xFC),
        ],
        tune: Tune::NONE,
    },
    // 15 — Garage Grunge
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x0A, 0x0A, 0x0A),
            rgb(0x3F, 0x3F, 0x46),
            rgb(0xA3, 0xA3, 0xA3),
            rgb(0x84, 0xCC, 0x16),
            rgb(0xF5, 0xF5, 0xF5),
        ],
        tune: Tune::NONE,
    },
    // 16 — Disco Mirrorball. The navy is the third theme to use it; #111827 tinted 12 % towards its own violet is the same
    // room with the mirrorball in it, and it keeps three faces apart.
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x11, 0x18, 0x27),
            rgb(0xA8, 0x55, 0xF7),
            rgb(0xF9, 0x73, 0x16),
            rgb(0xFD, 0xE0, 0x47),
            rgb(0xE5, 0xE7, 0xEB),
        ],
        tune: Tune {
            panel: Some(rgb(0x23, 0x1F, 0x40)),
            ..Tune::NONE
        },
    },
    // 17 — Ambient Moss
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x0B, 0x1F, 0x17),
            rgb(0x14, 0x53, 0x2D),
            rgb(0x86, 0xEF, 0xAC),
            rgb(0xD1, 0xFA, 0xE5),
            rgb(0xF8, 0xFA, 0xFC),
        ],
        tune: Tune::NONE,
    },
    // 18 — K Pop Candy. Choir Pastel already owns this blush; this one is the same blush mixed 12 % into its own pink.
    Row {
        polar: Polar::Light,
        colours: [
            rgb(0x0E, 0xA5, 0xE9),
            rgb(0xA7, 0x8B, 0xFA),
            rgb(0xFB, 0x71, 0x85),
            rgb(0xFC, 0xE7, 0xF3),
            rgb(0x0F, 0x17, 0x2A),
        ],
        tune: Tune {
            panel: Some(rgb(0xFC, 0xD9, 0xE6)),
            ..Tune::NONE
        },
    },
    // 19 — Studio Sandalwood
    Row {
        polar: Polar::Light,
        colours: [
            rgb(0x3B, 0x2F, 0x2A),
            rgb(0x8B, 0x5E, 0x34),
            rgb(0xD6, 0xB9, 0x8C),
            rgb(0xF3, 0xED, 0xE2),
            rgb(0x6B, 0x72, 0x80),
        ],
        tune: Tune::NONE,
    },
    // 20 — EDM Laser
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x05, 0x08, 0x16),
            rgb(0x22, 0xC5, 0x5E),
            rgb(0x06, 0xB6, 0xD4),
            rgb(0xA8, 0x55, 0xF7),
            rgb(0xE2, 0xE8, 0xF0),
        ],
        tune: Tune::NONE,
    },
    // 21 — Retro Cassette
    Row {
        polar: Polar::Dark,
        colours: [
            rgb(0x2A, 0x2E, 0x45),
            rgb(0xF9, 0x73, 0x16),
            rgb(0xFB, 0x71, 0x85),
            rgb(0xFD, 0xE6, 0x8A),
            rgb(0xFA, 0xFA, 0xF9),
        ],
        tune: Tune::NONE,
    },
];

impl Tune {
    const NONE: Tune = Tune {
        panel: None,
        well: None,
        key_face: None,
        silkscreen: None,
        silkscreen_dim: None,
        led_on: None,
        led_accent: None,
        learn_target: None,
        raised_ink: None,
        lit_ink: None,
    };
}

/// A colour mixed `t` of the way from `a` to `b`, per channel.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let channel = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}

/// How far apart two colours are in hue-and-saturation terms: the spread of their channels.
fn chroma(c: Color32) -> u8 {
    let (r, g, b) = (c.r(), c.g(), c.b());
    r.max(g).max(b) - r.min(g).min(b)
}

/// WCAG's contrast ratio: the pair of luminances, brighter over darker.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (bright, dark) = (luminance(a) + 0.05, luminance(b) + 0.05);
    if bright > dark {
        bright / dark
    } else {
        dark / bright
    }
}

/// The face of a row: the anchor end, less an off-white when a cream is the real canvas.
fn face(row: &Row, sorted: &[Color32; 5]) -> Color32 {
    match row.polar {
        Polar::Dark => sorted[0],
        Polar::Light => {
            if luminance(sorted[4]) > 0.93 {
                sorted[3]
            } else {
                sorted[4]
            }
        }
    }
}

/// The end of the scale that is further from `surface` — measured, not guessed from a 0.5 threshold: a mid-tone surface can be
/// further from white than from black, and the guess sent captions on a mid-brown key towards white at 2.9:1.
fn away_from(surface: Color32) -> Color32 {
    if contrast(surface, Color32::WHITE) >= contrast(surface, Color32::BLACK) {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

/// The colour of the five that reads best on `surface`.
fn ink_on(surface: Color32, colours: &[Color32; 5]) -> Color32 {
    // *Of the five*: a theme's legends are its own colours, never a stock white. Only when none of them clears 4.5:1 is the
    // best of them mixed towards black or white until it does. The first derivation offered white and black as candidates and
    // every dark theme came out pure white, which is a palette with the theme taken out of it.
    let best = colours
        .iter()
        .copied()
        .max_by(|a, b| contrast(surface, *a).total_cmp(&contrast(surface, *b)))
        .expect("a palette has five colours");
    if contrast(surface, best) >= 4.5 {
        return best;
    }
    let away = away_from(surface);
    for step in 1..=10 {
        let candidate = mix(best, away, step as f32 * 0.1);
        if contrast(surface, candidate) >= 4.5 {
            return candidate;
        }
    }
    away
}

/// A dim legend: the legend mixed into the face until it is quiet but still legible — on the **well** as well as the face,
/// which is where a key's border, the knob's unlit arc and the strip's shelf are drawn.
fn dim_legend(silkscreen: Color32, panel: Color32, well: Color32) -> Color32 {
    // The **quietest** legend that still reads on the well and clears 2.5:1 on the face: walk from nearly the face back
    // towards the legend and take the first that passes. Walking the other way gives a "dim" legend one shade off the full
    // one, which is what the first derivation did.
    // Steps of 5 %: Pop Bubblegum's first acceptable dim lands at 2.97:1 on a 10 % grid and clears it on a finer one.
    let mut fallback = mix(silkscreen, panel, 0.05);
    for step in (1..=19).rev() {
        let candidate = mix(silkscreen, panel, step as f32 * 0.05);
        fallback = candidate;
        if contrast(panel, candidate) >= 2.5 && contrast(well, candidate) >= 3.0 {
            return candidate;
        }
    }
    fallback
}

/// The lit lamp: the most chromatic of the five that clears 3:1 against the face **and** the well, or that colour pushed
/// away from the face until it does. On a bright face that is a dark mark, exactly as the amber scheme concluded.
fn lit_lamp(colours: &[Color32; 5], panel: Color32, well: Color32) -> Color32 {
    let clears = |c: Color32| contrast(panel, c) >= 3.0 && contrast(well, c) >= 3.0;
    let brightest_chroma = |candidates: &mut dyn Iterator<Item = Color32>| {
        candidates.fold(None, |best: Option<Color32>, c| match best {
            Some(b) if chroma(b) >= chroma(c) => Some(b),
            _ => Some(c),
        })
    };
    if let Some(c) = brightest_chroma(&mut colours.iter().copied().filter(|c| clears(*c))) {
        return c;
    }
    // Nothing in the palette reads on both surfaces: take the most chromatic and push it towards black or white, whichever
    // is further from the face, in steps until it clears.
    let seed = brightest_chroma(&mut colours.iter().copied()).expect("a palette has five colours");
    let away = away_from(panel);
    for step in 1..=10 {
        let candidate = mix(seed, away, step as f32 * 0.1);
        if clears(candidate) {
            return candidate;
        }
    }
    away
}

/// The loud ink for UNSAVED: louder than the dim legend against the face, but not the legend itself.
fn loud_ink(colours: &[Color32; 5], panel: Color32, dim: Color32, silkscreen: Color32) -> Color32 {
    let dim_ratio = contrast(panel, dim);
    let mut candidates: Vec<Color32> = colours
        .iter()
        .copied()
        .filter(|c| *c != silkscreen && contrast(panel, *c) > dim_ratio)
        .collect();
    candidates.sort_by(|a, b| contrast(panel, *b).total_cmp(&contrast(panel, *a)));
    if let Some(c) = candidates.first() {
        return *c;
    }
    // Push the legend away from the face until it is louder than the dim one.
    let away = away_from(panel);
    for step in 1..=10 {
        let candidate = mix(silkscreen, away, step as f32 * 0.1);
        if contrast(panel, candidate) > dim_ratio && contrast(panel, candidate) >= 4.5 {
            return candidate;
        }
    }
    away
}

/// The learn cue: a colour the other two lamps left, clearing 3:1 against the face — pushed away from the face if no source
/// colour manages it.
fn learn_cue(
    colours: &[Color32; 5],
    panel: Color32,
    lit: Color32,
    loud: Color32,
    silkscreen: Color32,
) -> Color32 {
    // A fourth kind of light, not the legend and not either lamp: a cue that matches the legend is a cue nobody sees, which is
    // what the first derivation produced for six themes whose palettes are mostly neutrals.
    let clears = |c: Color32| contrast(panel, c) >= 3.0 && c != lit && c != loud && c != silkscreen;
    let mut left: Vec<Color32> = colours.iter().copied().filter(|c| clears(*c)).collect();
    left.sort_by_key(|c| std::cmp::Reverse(chroma(*c)));
    if let Some(c) = left.first() {
        return *c;
    }
    // Nothing left: take the lamp and move it *towards the face* until it is still a cue and no longer the legend. Moving it
    // away would land on white or black, which is the legend on half these palettes.
    for step in 1..=9 {
        let candidate = mix(lit, panel, step as f32 * 0.1);
        if contrast(panel, candidate) >= 3.0 && candidate != silkscreen && candidate != loud {
            return candidate;
        }
    }
    mix(lit, panel, 0.5)
}

/// One row of the table, turned into a face.
pub fn derive(row: &Row) -> Palette {
    let mut sorted = row.colours;
    sorted.sort_by(|a, b| luminance(*a).total_cmp(&luminance(*b)));

    let panel = row.tune.panel.unwrap_or_else(|| face(row, &sorted));
    let silkscreen = row
        .tune
        .silkscreen
        .unwrap_or_else(|| ink_on(panel, &sorted));
    // A well is the recess the display and the name field sit in. On a dark face that is a hole, so it goes darker; on a light
    // face the readout is the brightest thing on the instrument, so it goes lighter — and that is also what gives the dim
    // legend room to be quiet, because a quiet legend reads *better* against a lighter well than against a mid-tone one.
    let well = row.tune.well.unwrap_or_else(|| match row.polar {
        Polar::Dark => mix(panel, Color32::BLACK, 0.18),
        Polar::Light => mix(panel, Color32::WHITE, 0.55),
    });
    let panel_raised = match row.polar {
        Polar::Dark => mix(panel, Color32::WHITE, 0.08),
        Polar::Light => mix(panel, Color32::BLACK, 0.35),
    };
    let key_face = row
        .tune
        .key_face
        .unwrap_or_else(|| mix(panel, Color32::WHITE, 0.22));
    let silkscreen_dim = row
        .tune
        .silkscreen_dim
        .unwrap_or_else(|| dim_legend(silkscreen, panel, well));
    let led_on = row
        .tune
        .led_on
        .unwrap_or_else(|| lit_lamp(&sorted, panel, well));
    let led_accent = row
        .tune
        .led_accent
        .unwrap_or_else(|| loud_ink(&sorted, panel, silkscreen_dim, silkscreen));
    let learn_target = row
        .tune
        .learn_target
        .unwrap_or_else(|| learn_cue(&sorted, panel, led_on, led_accent, silkscreen));
    let raised_ink = row
        .tune
        .raised_ink
        .unwrap_or_else(|| ink_on(panel_raised, &sorted));
    let lit_ink = row
        .tune
        .lit_ink
        .unwrap_or_else(|| ink_on(panel_raised, &sorted));

    Palette {
        panel,
        panel_raised,
        key_face,
        raised_ink,
        well,
        silkscreen,
        silkscreen_dim,
        led_on,
        led_off: mix(panel, silkscreen, 0.1),
        led_accent,
        lit_ink,
        // The digits sit in the well, not on the face: on a light theme the well is lighter than the face and the legend is
        // dark, so copying the legend would leave the digits at 3.6:1.
        text: ink_on(well, &sorted),
        white_key: CREAM,
        keycap_ink: KEYCAP_INK,
        shade: SCRIM,
        learn_target,
    }
}

/// How many themes there are, taken from the catalogue rather than written down twice.
pub const THEMES: usize = crate::device::Scheme::ALL.len();

fn build() -> [Palette; THEMES] {
    let mut out = [BLACK; THEMES];
    for (index, row) in ROWS.iter().enumerate() {
        out[index + 1] = derive(row);
    }
    out
}

/// The palette a device's face is painted in.
pub fn palette(scheme: crate::device::Scheme) -> &'static Palette {
    use std::sync::OnceLock;
    static CACHE: OnceLock<[Palette; THEMES]> = OnceLock::new();
    // Derived once, on the first frame that paints a device: pure arithmetic on constants, no allocation after the first
    // call and nothing for the audio thread to see.
    &CACHE.get_or_init(build)[scheme.index()]
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

    /// Polarity is real work, not a label: a dark face wears lights, a light face wears marks. If the branch were removed
    /// the light themes would keep the dark themes' lamps and every one of them would be invisible on its own face.
    #[test]
    fn a_light_face_gets_dark_lamps_and_a_dark_face_light_ones() {
        for (index, row) in ROWS.iter().enumerate() {
            let scheme = Scheme::ALL[index + 1];
            let p = palette(scheme);
            match row.polar {
                Polar::Dark => assert!(
                    luminance(p.led_on) > luminance(p.panel),
                    "{scheme:?}: a dark face's lit lamp is a light on it"
                ),
                Polar::Light => assert!(
                    luminance(p.led_on) < luminance(p.panel),
                    "{scheme:?}: a light face's lit lamp is a mark on it"
                ),
            }
        }
    }

    #[test]
    fn every_theme_holds_the_reading_rules() {
        for scheme in Scheme::ALL {
            let p = palette(scheme);
            // A well is the surface the legends read *best* against, which is what makes it read as a recess: on a dark face
            // that is darker than the face, and on a light face it is the bright readout. Written as "further from the legends"
            // rather than "darker", because the second is only true of half the catalogue.
            assert!(
                contrast(p.well, p.silkscreen) > contrast(p.panel, p.silkscreen),
                "{scheme:?}: the well must be further from the legends than the face is"
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
    }

    /// A list of names with eleven identical squares on it is a lie, so every theme must have its own face.
    #[test]
    fn every_theme_has_its_own_face() {
        for (index, scheme) in Scheme::ALL.iter().enumerate() {
            for other in &Scheme::ALL[index + 1..] {
                assert_ne!(
                    palette(*scheme).panel,
                    palette(*other).panel,
                    "{scheme:?} and {other:?} paint the same face"
                );
            }
        }
    }

    /// The table is the truth: `palette(scheme)` must be *that theme's* row, not a neighbour's. A swapped pair of arms in the
    /// cache, or an index that drifted, fails here.
    #[test]
    fn every_theme_is_derived_from_its_row() {
        assert_eq!(
            ROWS.len(),
            Scheme::ALL.len() - 1,
            "one row per theme but BLACK"
        );
        assert_eq!(
            *palette(Scheme::Black),
            BLACK,
            "BLACK is a const, not a row"
        );
        for (index, row) in ROWS.iter().enumerate() {
            let scheme = Scheme::ALL[index + 1];
            assert_eq!(
                palette(scheme),
                &derive(row),
                "{scheme:?} is not its own row"
            );
        }
    }

    /// Each face is one of its own five colours, unless its row says otherwise — the row's `tune.panel` is the only place the
    /// derivation may be overruled, and it is visible there. The page's palette is where a theme came from.
    #[test]
    fn every_face_is_one_of_its_rows_colours_or_says_so() {
        for (index, row) in ROWS.iter().enumerate() {
            let scheme = Scheme::ALL[index + 1];
            let panel = palette(scheme).panel;
            assert!(
                row.colours.contains(&panel) || row.tune.panel == Some(panel),
                "{scheme:?}: the face {panel:?} is neither the page's nor its row's"
            );
        }
    }

    /// The legends are one of the five colours the page gave, not a stock white.
    ///
    /// This is the bug the first derivation shipped: `ink_on` offered white and black among its candidates, so every dark
    /// theme came out with pure-white legends and the catalogue was twenty-one themes with the theme taken out of them. The
    /// reading rules all passed — white is the most readable thing there is — which is why the rule needed a test of its own.
    #[test]
    fn a_theme_legends_are_one_of_its_own_colours() {
        for (index, row) in ROWS.iter().enumerate() {
            let scheme = Scheme::ALL[index + 1];
            let legend = palette(scheme).silkscreen;
            assert!(
                (legend != Color32::WHITE && legend != Color32::BLACK)
                    || row.colours.contains(&legend),
                "{scheme:?}: the legend {legend:?} is a stock white or black the page never gave"
            );
            assert!(
                row.colours
                    .iter()
                    .any(|of| of.r().abs_diff(legend.r()) <= 60
                        && of.g().abs_diff(legend.g()) <= 60
                        && of.b().abs_diff(legend.b()) <= 60),
                "{scheme:?}: the legend {legend:?} is not a shade of any of the page's five"
            );
        }
    }

    /// The dim legend is **quiet by measure**, not one shade off the full one.
    ///
    /// The first `dim_legend` walked from quiet to loud and returned the first candidate that passed, which is the loudest
    /// acceptable colour: a "dim" legend 10 % away from the full one. It walked the wrong way round.
    #[test]
    fn every_theme_legend_has_a_quieter_second_weight() {
        for scheme in Scheme::ALL {
            let p = palette(scheme);
            let full = contrast(p.panel, p.silkscreen);
            let dim = contrast(p.panel, p.silkscreen_dim);
            assert!(
                dim <= full * 0.85,
                "{scheme:?}: the dim legend reads at {dim:.2}:1 against the legend's {full:.2}:1, which is not dimmed"
            );
            assert!(
                contrast(p.well, p.silkscreen_dim) >= 3.0,
                "{scheme:?}: and still has to read in the well it is drawn in"
            );
        }
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
