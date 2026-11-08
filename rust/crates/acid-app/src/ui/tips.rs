//! Hover help: what every control does, and the key that does the same thing.
//!
//! The text lives here, not beside the widgets, for two reasons: the help card and the tooltips then describe the panel
//! in one voice, and there is a single place to change when a shortcut moves. Only controls a pointer can actually land on
//! are listed: the engravings (BANK, PATTERN, MODE) cannot be hovered, so a tip for them would be dead text. The sixteen
//! step keys are absent for the same reason the help card holds their gestures: they are the instrument, not a control.

use eframe::egui::Response;

use super::PanelState;

/// What a control does, and the key that does the same thing.
///
/// `shortcut` is `None` for anything with no key: the knobs are drag-and-wheel only, and inventing a key for them would
/// be a lie the player would try to use.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Tip {
    pub title: &'static str,
    pub what: &'static str,
    pub shortcut: Option<&'static str>,
}

impl Tip {
    /// The tooltip's text: title, then what it does, then the key when there is one.
    pub fn text(&self) -> String {
        match self.shortcut {
            Some(key) => format!("{}\n{}\n{key}", self.title, self.what),
            None => format!("{}\n{}", self.title, self.what),
        }
    }
}

/// Every control a pointer can hover, in panel order. `ALL` and [`TIPS`] are indexed by the same position.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Control {
    Waveform,
    Tuning,
    Cutoff,
    Resonance,
    EnvMod,
    Decay,
    Accent,
    Clear,
    Save,
    Songs,
    Step,
    Run,
    Stop,
    TempoDown,
    TempoUp,
    Learn,
    Tooltips,
    OctaveDown,
    OctaveUp,
    Shuffle,
    Randomise,
}

impl Control {
    /// Every control; the tests walk it, so a new one cannot ship without text.
    pub const ALL: [Control; 21] = [
        Control::Waveform,
        Control::Tuning,
        Control::Cutoff,
        Control::Resonance,
        Control::EnvMod,
        Control::Decay,
        Control::Accent,
        Control::Clear,
        Control::Save,
        Control::Songs,
        Control::Step,
        Control::Run,
        Control::Stop,
        Control::TempoDown,
        Control::TempoUp,
        Control::Learn,
        Control::Tooltips,
        Control::OctaveDown,
        Control::OctaveUp,
        Control::Shuffle,
        Control::Randomise,
    ];

    /// The knob in `index`'s slot, in the panel's knob order.
    pub fn knob(index: usize) -> Control {
        match index {
            0 => Control::Waveform,
            1 => Control::Tuning,
            2 => Control::Cutoff,
            3 => Control::Resonance,
            4 => Control::EnvMod,
            5 => Control::Decay,
            _ => Control::Accent,
        }
    }

    /// This control's help. The `match` is exhaustive, so a new control cannot reach a player without text.
    pub fn tip(self) -> Tip {
        match self {
            Control::Waveform => TIPS[0],
            Control::Tuning => TIPS[1],
            Control::Cutoff => TIPS[2],
            Control::Resonance => TIPS[3],
            Control::EnvMod => TIPS[4],
            Control::Decay => TIPS[5],
            Control::Accent => TIPS[6],
            Control::Clear => TIPS[7],
            Control::Save => TIPS[8],
            Control::Songs => TIPS[9],
            Control::Step => TIPS[10],
            Control::Run => TIPS[11],
            Control::Stop => TIPS[12],
            Control::TempoDown => TIPS[13],
            Control::TempoUp => TIPS[14],
            Control::Learn => TIPS[15],
            Control::Tooltips => TIPS[16],
            Control::OctaveDown => TIPS[17],
            Control::OctaveUp => TIPS[18],
            Control::Shuffle => TIPS[19],
            Control::Randomise => TIPS[20],
        }
    }
}

/// The help text, in `Control::ALL` order.
const TIPS: [Tip; 21] = [
    Tip {
        title: "WAVEFORM",
        what: "Chooses the oscillator's wave: saw, square, sine or triangle.",
        shortcut: None,
    },
    Tip {
        title: "TUNING",
        what: "Tunes the whole instrument, a semitone either side.",
        shortcut: None,
    },
    Tip {
        title: "CUT OFF",
        what: "Where the filter starts to close: low is dark and muffled, high is open.",
        shortcut: None,
    },
    Tip {
        title: "RESONANCE",
        what: "How hard the filter sings at its cut-off. High values whistle.",
        shortcut: None,
    },
    Tip {
        title: "ENV MOD",
        what: "How far the envelope sweeps the cut-off on every note.",
        shortcut: None,
    },
    Tip {
        title: "DECAY",
        what: "How long the filter takes to close after the attack.",
        shortcut: None,
    },
    Tip {
        title: "ACCENT",
        what: "How much an accented step pushes: louder, with a filter kick at the attack.",
        shortcut: None,
    },
    Tip {
        title: "CLEAR",
        what: "Empties the pattern on screen. Takes two clicks, because there is no undo.",
        shortcut: None,
    },
    Tip {
        title: "SAVE",
        what: "Writes the session now: patterns, knobs and the MIDI rig.",
        shortcut: None,
    },
    Tip {
        title: "SONGS",
        what: "Name, save, load, rename or delete a song.",
        shortcut: None,
    },
    Tip {
        title: "STEP",
        what: "Key down writes the selected step, key up walks to the next one.",
        shortcut: None,
    },
    Tip {
        title: "RUN",
        what: "Starts the transport from the pattern's first step.",
        shortcut: Some("SPACE: start or stop, same as RUN / STOP"),
    },
    Tip {
        title: "STOP",
        what: "Stops the transport and returns to the first step.",
        shortcut: Some("SPACE: start or stop, same as RUN / STOP"),
    },
    Tip {
        title: "TEMPO -",
        what: "One BPM slower.",
        shortcut: Some("down arrow: one BPM slower"),
    },
    Tip {
        title: "TEMPO +",
        what: "One BPM faster.",
        shortcut: Some("up arrow: one BPM faster"),
    },
    Tip {
        title: "LEARN",
        what: "Teaches a knob to follow a controller: click LEARN, then the knob, then move the fader.",
        shortcut: None,
    },
    Tip {
        title: "TIPS",
        what: "Turns these hover hints on and off. They are on unless you say otherwise.",
        shortcut: None,
    },
    Tip {
        title: "OCT -",
        what: "Moves the whole pattern an octave down. Notes below the range play as rests.",
        shortcut: None,
    },
    Tip {
        title: "OCT +",
        what: "Moves the whole pattern an octave up. Notes above the range play as rests.",
        shortcut: None,
    },
    Tip {
        title: "SHUFFLE",
        what: "Pushes every second step late: the groove loosens as the knob comes up.",
        shortcut: None,
    },
    Tip {
        title: "RANDOM",
        what: "Rerolls the whole pattern: which steps sound, and what they play. Accents and slides stay.",
        shortcut: None,
    },
];

/// The help for `control`, or nothing at all when the player has turned the hints off.
///
/// Split out from [`hover`] because an egui tooltip is a floating area, not something a test can read back: this is the
/// decision, and the live test covers the key that makes it.
pub fn text_for(state: &PanelState, control: Control) -> Option<String> {
    state.tooltips.then(|| control.tip().text())
}

/// Attach a control's help to its response, if the player wants help.
///
/// `Response::on_hover_text` is egui's own tooltip, so the hint appears after the usual hover delay, stays out of the
/// drag's way, and follows the panel's scale.
pub fn hover(response: &Response, state: &PanelState, control: Control) {
    if let Some(text) = text_for(state, control) {
        response.clone().on_hover_text(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_matches_the_panel_order_and_holds_every_control_once() {
        assert_eq!(Control::ALL.len(), TIPS.len());
        for (index, control) in Control::ALL.iter().enumerate() {
            assert_eq!(
                Control::ALL.iter().position(|c| c == control),
                Some(index),
                "{control:?} is listed twice or out of order"
            );
            assert_eq!(*control, Control::ALL[index]);
        }
        // The table and the exhaustive `match` in `tip` must agree, entry for entry.
        for (index, tip) in TIPS.iter().enumerate() {
            assert_eq!(Control::ALL[index].tip(), *tip);
        }
        // The knobs are the front of the panel, and the TIPS key is the last tip.
        assert_eq!(Control::knob(0), Control::ALL[0]);
        assert_eq!(Control::Tooltips, Control::ALL[16]);
    }

    #[test]
    fn every_control_says_what_it_does_in_ascii() {
        for control in Control::ALL {
            let tip = control.tip();
            assert!(!tip.title.is_empty(), "{control:?} has no title");
            assert!(!tip.what.is_empty(), "{control:?} has no description");
            // Words, not bytes: "One BPM slower." is the shortest honest sentence a control needs.
            assert!(
                tip.what.split_whitespace().count() >= 3,
                "{control:?} says too little"
            );
            assert!(tip.title.is_ascii(), "{control:?} title is not ASCII");
            assert!(tip.what.is_ascii(), "{control:?} text is not ASCII");
            if let Some(key) = tip.shortcut {
                assert!(key.is_ascii(), "{control:?} shortcut is not ASCII");
                assert!(!key.is_empty());
            }
        }
    }

    #[test]
    fn the_titles_are_distinct_so_two_controls_cannot_be_confused() {
        let mut titles: Vec<&str> = Control::ALL.iter().map(|c| c.tip().title).collect();
        titles.sort_unstable();
        let count = titles.len();
        titles.dedup();
        assert_eq!(titles.len(), count, "two controls share a title");
    }

    #[test]
    fn the_text_names_the_control_its_job_and_only_a_real_shortcut() {
        let text = Control::Run.tip().text();
        assert!(text.starts_with("RUN\n"));
        assert!(text.contains("transport"));
        assert!(text.ends_with("SPACE: start or stop, same as RUN / STOP"));

        // A knob has no key, and its text must not pretend otherwise.
        let knob = Control::Cutoff.tip();
        assert_eq!(knob.shortcut, None);
        assert_eq!(knob.text().lines().count(), 2);
    }

    #[test]
    fn every_knob_slot_names_its_own_control() {
        let knobbed: Vec<&str> = (0..7)
            .map(|index| Control::knob(index).tip().title)
            .collect();
        assert_eq!(
            knobbed,
            [
                "WAVEFORM",
                "TUNING",
                "CUT OFF",
                "RESONANCE",
                "ENV MOD",
                "DECAY",
                "ACCENT"
            ]
        );
    }

    #[test]
    fn the_hints_stay_silent_when_the_player_turned_them_off() {
        let on = PanelState::default();
        let off = PanelState {
            tooltips: false,
            ..PanelState::default()
        };
        assert_eq!(
            text_for(&on, Control::Cutoff),
            Some(Control::Cutoff.tip().text())
        );
        assert_eq!(
            text_for(&off, Control::Cutoff),
            None,
            "a silent panel stays silent"
        );
    }
}
