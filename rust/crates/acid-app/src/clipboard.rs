//! What the app puts on the operating system's clipboard, and what it takes back off it.
//!
//! A value plus a text form, not an in-process slot: text travels between two windows of the app and can be read in any
//! editor when a copy looks wrong. The envelope is a magic line (so a foreign clipboard is refused before any parsing) and a
//! version, then a tagged JSON payload — the round that copies device settings adds a *variant*, not a second format, and an
//! older build refuses a newer clipboard by the version instead of misreading it.

use acid_core::pattern::Pattern;
use serde::{Deserialize, Serialize};

/// The first word of every clipboard this app writes. Anything else is somebody else's.
pub const MAGIC: &str = "takka-acid";

/// The payload format this build writes, and the only one it reads.
pub const VERSION: u32 = 2;

/// One thing on the clipboard.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Clip {
    /// A pattern, and the slot it came from — `A1`..`H8` — for the notice a paste gives. `None` when the copy came from a
    /// build that did not say, or from a hand-written payload.
    Pattern {
        pattern: Pattern,
        from: Option<String>,
    },
}

/// The clipboard text for `clip`: the magic line, then the payload.
pub fn encode(clip: &Clip) -> String {
    let payload = serde_json::to_string(clip)
        .expect("a Clip always serialises: it is a pattern and an optional label");
    format!("{MAGIC} {VERSION}\n{payload}")
}

/// What a clipboard's text holds, or `None` if it is not ours: a foreign clipboard, a version this build does not know, or a
/// payload that does not parse.
pub fn decode(text: &str) -> Option<Clip> {
    let (header, payload) = text.split_once('\n')?;
    if header.trim() != format!("{MAGIC} {VERSION}") {
        return None;
    }
    serde_json::from_str(payload.trim()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    use acid_core::pattern::{Grid, PATTERN_SLOTS, Step};
    use acid_core::pitch::Pitch;

    /// A pattern with one of everything in it: sounding and resting steps, both ends of the octave range, accents, slides,
    /// and two velocities that are neither the default nor each other.
    fn varied() -> Pattern {
        let mut steps = [Step::rest(); PATTERN_SLOTS];
        steps[0] = Step::note(Pitch::C, -2);
        steps[1] = Step {
            accent: true,
            velocity: 35,
            ..Step::note(Pitch::G, 1)
        };
        steps[2] = Step {
            slide: true,
            ..Step::note(Pitch::A, 0)
        };
        steps[15] = Step {
            on: false,
            pitch: Pitch::DSharp,
            octave_offset: 3,
            accent: true,
            slide: true,
            velocity: 1,
            // The drum masks are part of the step a clipboard carries; a 303 fixture strikes no voice.
            hits: 0,
            accents: 0,
        };
        Pattern::from_steps(steps)
    }

    #[test]
    fn a_pattern_round_trips_through_the_clipboard() {
        let clip = Clip::Pattern {
            pattern: varied(),
            from: Some("C3".to_string()),
        };
        assert_eq!(decode(&encode(&clip)), Some(clip));
    }

    #[test]
    fn every_field_of_every_step_survives() {
        // The round trip above would hide a field the encoder dropped and the decoder defaulted, so each one is checked on the
        // step that carries it.
        let encoded = encode(&Clip::Pattern {
            pattern: varied(),
            from: None,
        });
        let Some(Clip::Pattern { pattern, .. }) = decode(&encoded) else {
            panic!("a pattern clip must decode as one: {encoded}");
        };
        assert!(pattern.steps[0].on && !pattern.steps[0].slide);
        assert_eq!(pattern.steps[1].velocity, 35);
        assert!(pattern.steps[1].accent);
        assert_eq!(pattern.steps[1].octave_offset, 1);
        assert!(pattern.steps[2].slide);
        assert_eq!(pattern.steps[15].octave_offset, 3);
        assert_eq!(pattern.steps[15].velocity, 1);
        assert_eq!(pattern.steps[15].pitch, Pitch::DSharp);
    }

    #[test]
    fn the_text_names_the_app_and_the_version_before_the_payload() {
        let encoded = encode(&Clip::Pattern {
            pattern: Pattern::empty(),
            from: None,
        });
        let header = encoded.lines().next().unwrap_or_default();
        assert_eq!(header, "takka-acid 2");
        assert!(
            encoded.contains(&format!("{MAGIC} {VERSION}")),
            "a reader must be able to tell what wrote it: {encoded}"
        );
    }

    #[test]
    fn a_foreign_clipboard_is_refused() {
        let payload = serde_json::to_string(&Clip::Pattern {
            pattern: varied(),
            from: None,
        })
        .expect("serialises");

        for text in [
            // Somebody else's text, at every shape a clipboard takes.
            "",
            "hello",
            "{\"steps\":[]}",
            // Our payload with no magic line: a bare payload is not ours, even when it parses.
            &payload,
            // A version this build does not know, in both directions.
            &format!("takka-acid 1\n{payload}"),
            &format!("takka-acid 3\n{payload}"),
            &format!("takka-acid 0\n{payload}"),
            // And a payload that does not parse, under the right header.
            &format!("takka-acid 2\n{{{payload}"),
            // A kind a newer build might write.
            "takka-acid 2\n{\"kind\":\"device\",\"name\":\"BASS\"}",
        ] {
            assert_eq!(decode(text), None, "should have been refused: {text:?}");
        }
    }

    #[test]
    fn a_clip_without_a_source_slot_still_round_trips() {
        let clip = Clip::Pattern {
            pattern: varied(),
            from: None,
        };
        assert_eq!(decode(&encode(&clip)), Some(clip));
    }

    /// A thirty-second bar travels with its grid: the payload is the pattern, so both the count and the caption have to
    /// be in it — and the envelope's version is what makes a version-1 build refuse it instead of reading half a bar.
    #[test]
    fn a_thirty_second_bar_round_trips_with_its_grid() {
        let mut steps = [Step::rest(); PATTERN_SLOTS];
        steps[0] = Step::note(Pitch::A, 1);
        steps[31] = Step::note(Pitch::C, 2);
        let wide = Clip::Pattern {
            pattern: Pattern::from_steps_in(Grid::ThirtySecond, steps),
            from: Some("A1".to_string()),
        };

        let Some(Clip::Pattern { pattern, .. }) = decode(&encode(&wide)) else {
            panic!("a pattern clip must decode as one");
        };
        assert_eq!(pattern.grid, Grid::ThirtySecond);
        assert_eq!(pattern.len(), 32, "thirty-two steps to the bar");
        assert!(
            pattern.steps[31].on,
            "the last step of the long bar came through"
        );

        let encoded = encode(&Clip::Pattern {
            pattern: Pattern::empty_in(Grid::EighthTriplet),
            from: None,
        });
        let Some(Clip::Pattern { pattern, .. }) = decode(&encoded) else {
            panic!("a triplet clip must decode as one");
        };
        assert_eq!(pattern.grid, Grid::EighthTriplet);
        assert_eq!(pattern.len(), 12);

        assert_eq!(VERSION, 2);
        assert_eq!(
            decode(&encoded.replacen("takka-acid 2", "takka-acid 1", 1)),
            None,
            "a version-1 header is refused, not misread into a twelve-step bar"
        );
    }
}
