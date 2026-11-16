//! Hover help: what every control does, and the key that does the same thing.
//!
//! The text lives here, not beside the widgets, for two reasons: the help card and the tooltips then describe the panel
//! in one voice, and there is a single place to change when a shortcut moves. Only controls a pointer can actually land on
//! are listed: the engravings (BANK, PATTERN, MODE) cannot be hovered, so a tip for them would be dead text. The sixteen
//! step keys are absent for the same reason the help card holds their gestures: they are the instrument, not a control.

use eframe::egui::Response;

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
    Density,
    Scale,
    /// The BPM readout. Appended, so no existing control's position in the table moves.
    Tempo,
    Pattern,
    /// The AUTOSAVE switch in the settings card: whether the session saves itself.
    Autosave,
    /// A device's cell in the strip: put that device on the panel.
    DeviceCell,
    /// That cell's MUTE key.
    DeviceMute,
    /// The strip's `+`: another device in the rack.
    DeviceAdd,
    /// The caret on the selected cell: the way into that device's card. Appended, so no existing position in the table
    /// moves — the cell says which device is on the panel, and the caret says how to open it, which are two questions.
    DeviceCaret,
    /// The deck's BACK key: a seek backwards. Appended, like the strip's own three, so no position moves.
    TapeBack,
    /// The deck's PAUSE key: silent, with the bar kept.
    TapePause,
    /// The deck's FORWARD key: a seek forwards.
    TapeForward,
    /// The deck's RECORD key: nothing to record into yet, and its tip says so.
    TapeRecord,
    /// The deck's PLAY key. The transport row's `RUN` is a different key with a different job, so it has its own words; the
    /// deck's STOP is the same key's job as the row's and shares its row.
    TapePlay,
    /// The row's REVERSE key: this device's line, backwards, from the next bar.
    Reverse,
    /// The row's TBC key: a cell kept for the player, with nothing decided behind it.
    Tbc,
    /// The row's TRANS -: the whole rack down a semitone, as a performance control.
    TransposeDown,
    /// The row's TRANS +: the whole rack up a semitone.
    TransposeUp,
}

impl Control {
    /// Every control; the tests walk it, so a new one cannot ship without text.
    pub const ALL: [Control; 39] = [
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
        Control::Density,
        Control::Scale,
        Control::Tempo,
        Control::Pattern,
        Control::Autosave,
        Control::DeviceCell,
        Control::DeviceMute,
        Control::DeviceAdd,
        Control::DeviceCaret,
        Control::TapeBack,
        Control::TapePause,
        Control::TapeForward,
        Control::TapeRecord,
        Control::TapePlay,
        Control::Reverse,
        Control::Tbc,
        Control::TransposeDown,
        Control::TransposeUp,
    ];

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
            Control::TapeBack => TIPS[30],
            Control::TapePause => TIPS[31],
            Control::TapeForward => TIPS[32],
            Control::TapeRecord => TIPS[33],
            Control::TapePlay => TIPS[34],
            Control::Reverse => TIPS[35],
            Control::Tbc => TIPS[36],
            Control::TransposeDown => TIPS[37],
            Control::TransposeUp => TIPS[38],
            Control::OctaveDown => TIPS[17],
            Control::OctaveUp => TIPS[18],
            Control::Shuffle => TIPS[19],
            Control::Randomise => TIPS[20],
            Control::Density => TIPS[21],
            Control::Scale => TIPS[22],
            Control::Tempo => TIPS[23],
            Control::Pattern => TIPS[24],
            Control::Autosave => TIPS[25],
            Control::DeviceCell => TIPS[26],
            Control::DeviceMute => TIPS[27],
            Control::DeviceAdd => TIPS[28],
            Control::DeviceCaret => TIPS[29],
        }
    }

    /// The help for a tone knob. The mapping lives here, not on `Knob`, so the catalogue stays free of the view: it is the
    /// same one exhaustive `match` plan 30 deleted the index version of.
    pub fn for_knob(knob: crate::knobs::Knob) -> Control {
        use crate::knobs::Knob;

        match knob {
            Knob::Waveform => Control::Waveform,
            Knob::Tuning => Control::Tuning,
            Knob::Cutoff => Control::Cutoff,
            Knob::Resonance => Control::Resonance,
            Knob::EnvMod => Control::EnvMod,
            Knob::Decay => Control::Decay,
            Knob::Accent => Control::Accent,
        }
    }
}

/// The help text, in `Control::ALL` order.
const TIPS: [Tip; 39] = [
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
        title: "BANKS",
        what: "Browse, load, save, copy, import, export or delete a bank. A bank is one device's patterns and knobs.",
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
        shortcut: None,
    },
    Tip {
        title: "STOP",
        what: "Stops the transport and returns to the first step.",
        shortcut: None,
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
        title: "TOOLTIPS",
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
        what: "Pushes every second step late as the knob comes up. LEARN can give it a controller.",
        shortcut: None,
    },
    Tip {
        title: "RANDOM",
        what: "Rerolls the whole pattern: which steps sound, and what they play. Accents and slides stay.",
        shortcut: None,
    },
    Tip {
        title: "DENSITY",
        what: "How full RANDOM makes a pattern: a handful of steps at the left, nearly all of them at the right.",
        shortcut: None,
    },
    Tip {
        title: "SCALE",
        what: "Which notes RANDOM may use, rooted on C. Turning it never changes the pattern in front of you.",
        shortcut: None,
    },
    Tip {
        title: "TEMPO",
        what: "How fast the sequencer runs, in BPM. Click it while LEARN is picking to teach a controller.",
        shortcut: Some("up / down arrow: one BPM faster or slower"),
    },
    Tip {
        title: "PATTERN",
        what: "Which pattern is playing, bank and slot. Click it while LEARN is picking to teach a controller; the swap waits for the bar.",
        shortcut: Some("shift + left / right arrow: the previous or next pattern, at once"),
    },
    Tip {
        title: "AUTOSAVE",
        what: "Whether the session writes itself as you work. Off by default: the SAVE key and the lamp are how you know where you stand.",
        shortcut: None,
    },
    Tip {
        title: "DEVICE",
        what: "Puts that device on the panel: its knobs, its pattern and the step you were editing.",
        shortcut: Some("ctrl + 1..4: the same, from the keyboard"),
    },
    Tip {
        title: "MUTE",
        what: "Silences that device without stopping it, so unmuting rejoins on the beat.",
        shortcut: None,
    },
    Tip {
        title: "ADD DEVICE",
        what: "Another machine in the rack, up to four. Names and the rack are not saved yet.",
        shortcut: None,
    },
    Tip {
        title: "DEVICE CARD",
        what: "The device you are looking at: its name, what it hears, and the way to remove it.",
        shortcut: Some("F3: the same, from the keyboard"),
    },
    Tip {
        title: "BACK",
        what: "Step the playhead back a bar. Not wired yet: the key is drawn so its place is settled.",
        shortcut: Some(","),
    },
    Tip {
        title: "PAUSE",
        what: "Stop the clock and keep the bar. A play carries on from the step it kept.",
        shortcut: Some("space"),
    },
    Tip {
        title: "FORWARD",
        what: "Step the playhead on a bar. Not wired yet: the key is drawn so its place is settled.",
        shortcut: Some("."),
    },
    Tip {
        title: "RECORD",
        what: "Record what the instrument plays. Not wired yet: nothing exists to record into.",
        shortcut: Some("R"),
    },
    Tip {
        title: "PLAY",
        what: "Start the bar again from a stop, or carry on from the step a pause kept.",
        shortcut: Some("SPACE: play or pause, same as PLAY / PAUSE"),
    },
    Tip {
        title: "REVERSE",
        what: "Runs this device's line backwards. It takes effect at the next bar, so the bar keeps its own time.",
        shortcut: None,
    },
    Tip {
        title: "TBC",
        what: "A cell kept for the player: nothing is decided for it yet.",
        shortcut: None,
    },
    Tip {
        title: "TRANS -",
        what: "Moves the whole rack down a semitone. A performance control: the TUNING knobs are untouched and nothing is saved.",
        shortcut: None,
    },
    Tip {
        title: "TRANS +",
        what: "Moves the whole rack up a semitone. A performance control: the TUNING knobs are untouched and nothing is saved.",
        shortcut: None,
    },
];

/// The help for `control`, or nothing at all when the player has turned the hints off.
///
/// `wants_help` rather than `&PanelState`: the settings card is painted with the two flags it needs, not with the whole
/// panel, and its two switches are the controls the deleted TIPS and AUTO keys were. One mapping, `Control::tip`, serves
/// every surface.
pub fn text_for(wants_help: bool, control: Control) -> Option<String> {
    wants_help.then(|| control.tip().text())
}

/// Attach a control's help to its response, if the player wants help.
///
/// `Response::on_hover_text` is egui's own tooltip, so the hint appears after the usual hover delay, stays out of the
/// drag's way, and follows the panel's scale.
pub fn hover(response: &Response, wants_help: bool, control: Control) {
    if let Some(text) = text_for(wants_help, control) {
        response.clone().on_hover_text(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knobs::Knob;

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
        // The knobs are the front of the panel; the last entries are the settings card's two switches.
        assert_eq!(Control::for_knob(Knob::Waveform), Control::ALL[0]);
        assert_eq!(Control::Tooltips, Control::ALL[16]);
        assert_eq!(Control::Autosave, Control::ALL[25]);
    }

    #[test]
    fn the_two_readouts_have_their_own_help() {
        // The pattern and tempo readouts can be hovered and can be taught, so both need a tip. They are appended to the
        // table so no existing control's position moves.
        assert_eq!(Control::ALL[23], Control::Tempo);
        assert_eq!(Control::ALL[24], Control::Pattern);
        assert_eq!(Control::Tempo.tip().title, "TEMPO");
        assert_eq!(Control::Pattern.tip().title, "PATTERN");
        assert!(
            Control::Tempo.tip().shortcut.is_some(),
            "the arrows nudge it"
        );
        assert!(
            Control::Pattern.tip().shortcut.is_some(),
            "shift + arrows move the pattern"
        );
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
        // RUN's key has no shortcut any more: the deck's PLAY and PAUSE are the pair that own the space bar, so a tip that
        // still claimed it would be a lie the player would try to use.
        let run = Control::Run.tip();
        assert!(run.text().starts_with("RUN\n"));
        assert!(run.text().contains("transport"));
        assert_eq!(run.shortcut, None);

        // And the space bar is claimed by the key that has it.
        let play = Control::TapePlay.tip();
        assert!(play.text().starts_with("PLAY\n"));
        assert!(
            play.shortcut.is_some_and(|key| key.starts_with("SPACE")),
            "PLAY owns the space bar: {:?}",
            play.shortcut
        );

        // A knob has no key, and its text must not pretend otherwise.
        let knob = Control::Cutoff.tip();
        assert_eq!(knob.shortcut, None);
        assert_eq!(knob.text().lines().count(), 2);
    }

    #[test]
    fn every_knob_slot_names_its_own_control() {
        let knobbed: Vec<&str> = Knob::ALL
            .iter()
            .map(|knob| Control::for_knob(*knob).tip().title)
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
        // And the catalogue's slots are the table's knob prefix, so the two orders cannot drift apart.
        for (index, knob) in Knob::ALL.iter().enumerate() {
            assert_eq!(
                Control::for_knob(*knob),
                Control::ALL[index],
                "{knob:?} is in the wrong slot"
            );
        }
    }

    #[test]
    fn the_hints_stay_silent_when_the_player_turned_them_off() {
        // The switch, not the panel, is the question: the settings card paints with the two flags it needs, so this is
        // the whole contract between "help is on" and a tip.
        assert_eq!(
            text_for(true, Control::Cutoff),
            Some(Control::Cutoff.tip().text())
        );
        assert_eq!(
            text_for(false, Control::Cutoff),
            None,
            "a silent panel stays silent"
        );
    }
}
