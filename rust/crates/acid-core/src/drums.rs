//! The drum machine's voices, as one catalogue.
//!
//! Sixteen is not a round number chosen for the grid: it is exactly the width of a `u16`, and a step's `hits` mask has one bit
//! per voice. A seventeenth voice is a third mask, which is why the catalogue lives apart from the mask — that change is a type
//! here, not a search through the drawing, the sequencer and the save file.
//!
//! The voices are fixed for every kit. A kit that has no cowbell mutes the cowbell; it does not renumber the ones after it.

/// One voice of the drum machine, in grid order — the order the face draws and the mask's bits count in.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Drum {
    Kick,
    Snare,
    /// The 808's rimshot and the 909's rim click share the slot: one sound per voice, chosen by the kit.
    Rim,
    Clap,
    /// A short hat. Choked by [`Drum::OpenHat`] and choking it — see [`Drum::choke_group`].
    ClosedHat,
    /// A long hat, cut off when a closed hat lands.
    OpenHat,
    LowTom,
    MidTom,
    /// `HTOM` rather than `HIT`: the knob row's audition key is already called `HIT`, and one face cannot carry two.
    HighTom,
    Crash,
    Ride,
    Cowbell,
    /// The 808's four extras. A 909 kit mutes them; the catalogue keeps the slots so a kit can grow into them.
    LowConga,
    HighConga,
    Clave,
    Shaker,
}

impl Drum {
    /// Every voice, in grid order.
    pub const ALL: [Drum; 16] = [
        Drum::Kick,
        Drum::Snare,
        Drum::Rim,
        Drum::Clap,
        Drum::ClosedHat,
        Drum::OpenHat,
        Drum::LowTom,
        Drum::MidTom,
        Drum::HighTom,
        Drum::Crash,
        Drum::Ride,
        Drum::Cowbell,
        Drum::LowConga,
        Drum::HighConga,
        Drum::Clave,
        Drum::Shaker,
    ];

    /// Which bit of a step's `hits`/`accents` mask this voice is, and which row of the grid.
    pub fn index(self) -> usize {
        match self {
            Drum::Kick => 0,
            Drum::Snare => 1,
            Drum::Rim => 2,
            Drum::Clap => 3,
            Drum::ClosedHat => 4,
            Drum::OpenHat => 5,
            Drum::LowTom => 6,
            Drum::MidTom => 7,
            Drum::HighTom => 8,
            Drum::Crash => 9,
            Drum::Ride => 10,
            Drum::Cowbell => 11,
            Drum::LowConga => 12,
            Drum::HighConga => 13,
            Drum::Clave => 14,
            Drum::Shaker => 15,
        }
    }

    /// The caption on the grid's row. Four characters at most, ASCII: the gutter is narrow and the face may not print glyphs.
    pub fn label(self) -> &'static str {
        match self {
            Drum::Kick => "KICK",
            Drum::Snare => "SNR",
            Drum::Rim => "RIM",
            Drum::Clap => "CLAP",
            Drum::ClosedHat => "CHH",
            Drum::OpenHat => "OHH",
            Drum::LowTom => "LOT",
            Drum::MidTom => "MID",
            Drum::HighTom => "HTOM",
            Drum::Crash => "CRS",
            Drum::Ride => "RDE",
            Drum::Cowbell => "CWB",
            Drum::LowConga => "LCO",
            Drum::HighConga => "HCO",
            Drum::Clave => "CLV",
            Drum::Shaker => "SHK",
        }
    }

    /// Which voices a hit on this one silences. `0` is "chokes nothing".
    ///
    /// This is the whole of a drum machine's realism in one number: a hat pair that rings together sounds like a synth pad,
    /// and one that cuts sounds like a drummer.
    pub fn choke_group(self) -> u8 {
        match self {
            Drum::ClosedHat | Drum::OpenHat => 1,
            Drum::Kick
            | Drum::Snare
            | Drum::Rim
            | Drum::Clap
            | Drum::LowTom
            | Drum::MidTom
            | Drum::HighTom
            | Drum::Crash
            | Drum::Ride
            | Drum::Cowbell
            | Drum::LowConga
            | Drum::HighConga
            | Drum::Clave
            | Drum::Shaker => 0,
        }
    }
}

/// Which side of the filter a voice passes. The knob row's three-way key prints these.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FilterKind {
    Low,
    Band,
    High,
}

impl FilterKind {
    pub const ALL: [FilterKind; 3] = [FilterKind::Low, FilterKind::Band, FilterKind::High];

    pub fn index(self) -> usize {
        match self {
            FilterKind::Low => 0,
            FilterKind::Band => 1,
            FilterKind::High => 2,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FilterKind::Low => "LP",
            FilterKind::Band => "BP",
            FilterKind::High => "HP",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sixteen_voices_are_labelled_numbered_and_chase_in_one_group() {
        assert_eq!(Drum::ALL.len(), 16, "one voice per bit of the step mask");

        for (index, drum) in Drum::ALL.iter().enumerate() {
            assert_eq!(drum.index(), index, "{drum:?} has the wrong slot");
            assert_eq!(
                Drum::ALL[drum.index()],
                *drum,
                "{drum:?} does not round trip"
            );
            assert!(drum.label().is_ascii(), "{drum:?} is not ASCII");
            assert!(
                !drum.label().is_empty() && drum.label().len() <= 4,
                "{drum:?} has a label the face has no room for"
            );
        }

        // The captions themselves, in grid order: the gutter prints these, so a rename, a duplicate or the old `HIT` for
        // `HighTom` is a face change and must fail here rather than in a screenshot.
        assert_eq!(
            Drum::ALL.map(Drum::label),
            [
                "KICK", "SNR", "RIM", "CLAP", "CHH", "OHH", "LOT", "MID", "HTOM", "CRS", "RDE",
                "CWB", "LCO", "HCO", "CLV", "SHK"
            ],
            "the grid's gutter prints these, in this order"
        );

        // One choke group, and it is the hats: a closed hat cuts an open one, which is what makes a hat pattern a pattern.
        let chasing: Vec<Drum> = Drum::ALL
            .iter()
            .copied()
            .filter(|drum| drum.choke_group() != 0)
            .collect();
        assert_eq!(
            chasing,
            vec![Drum::ClosedHat, Drum::OpenHat],
            "the hats share a choke group and nothing else does"
        );
        assert_eq!(
            Drum::ClosedHat.choke_group(),
            Drum::OpenHat.choke_group(),
            "the two hats must name the same group"
        );
    }

    #[test]
    fn the_filter_has_three_faces_with_ascii_names() {
        assert_eq!(FilterKind::ALL.len(), 3);
        for (index, kind) in FilterKind::ALL.iter().enumerate() {
            assert_eq!(kind.index(), index, "{kind:?} has the wrong slot");
            assert!(kind.label().is_ascii(), "{kind:?} is not ASCII");
        }
        assert_eq!(
            FilterKind::ALL.map(FilterKind::label),
            ["LP", "BP", "HP"],
            "the face prints these three, in this order"
        );
    }
}
