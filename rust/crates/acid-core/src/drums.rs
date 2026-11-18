//! The drum machine's voices, as one catalogue.
//!
//! Sixteen is not a round number chosen for the grid: it is exactly the width of a `u16`, and a step's `hits` mask has one bit
//! per voice. A seventeenth voice is a third mask, which is why the catalogue lives apart from the mask — that change is a type
//! here, not a search through the drawing, the sequencer and the save file.
//!
//! The voices are fixed for every kit. A kit that has no cowbell mutes the cowbell; it does not renumber the ones after it.

use std::sync::Arc;

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

/// The name of the kit a device holds before any kit is loaded. The card prints it, so it is a word and not an index.
pub const DEFAULT_KIT_NAME: &str = "Init";

/// The default voice's shape: a short percussive hit rather than a held note, which is what a drum is, and a filter as close to
/// the sample as recorded as the filter path allows — wide open, flat, low-pass.
///
/// Every one of these is a decision task 6's listening pass overturns per kit, which is why they are named here rather than spelled
/// in the constructor: changing a default is changing this list, not finding five literals.
const DEFAULT_TUNE: f32 = 0.0;
const DEFAULT_ATTACK_MS: f32 = 2.0;
const DEFAULT_DECAY_MS: f32 = 300.0;
const DEFAULT_SUSTAIN: f32 = 0.0;
const DEFAULT_RELEASE_MS: f32 = 20.0;
const DEFAULT_CUTOFF: f32 = 1.0;
const DEFAULT_RESONANCE: f32 = 0.0;

/// One voice of a kit: the sample it plays, what shapes it, and whether it starts silent.
///
/// The field order is the kit document's contract — task 6 writes `kit.json` from these names in this order, so a rename or a
/// reorder costs a migration. Provenance is deliberately not here: `file`, `source` and `credit` describe where a sample came
/// from, which the document records beside the file, not in the voice that plays it.
#[derive(Clone, Debug)]
pub struct KitVoice {
    /// Mono, at the engine's rate, already resampled. Empty means the voice has no sample: a silent voice, not a missing one.
    pub sample: Arc<[f32]>,
    /// Detune in semitones, ±12. This is the `TUNE` knob's range, not its position.
    pub tune: f32,
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
    /// Which side of the voice's filter the sample passes.
    pub filter: FilterKind,
    /// Knob positions in `0.0..=1.0`, mapped like the 303's by `param::cutoff_hz` and `param::resonance`.
    pub cutoff: f32,
    pub resonance: f32,
    /// The kit's starting mute. A kit with no cowbell mutes the cowbell; it does not renumber the voices after it.
    pub mute: bool,
}

impl Default for KitVoice {
    /// A voice with no sample, untouched: the default kit is sixteen of these. Written out rather than derived because every
    /// number is one of the decisions above.
    fn default() -> Self {
        // `Arc<[f32]>`'s `Default` is the empty slice, so this allocates nothing and cannot fail.
        Self {
            sample: Arc::default(),
            tune: DEFAULT_TUNE,
            attack_ms: DEFAULT_ATTACK_MS,
            decay_ms: DEFAULT_DECAY_MS,
            sustain: DEFAULT_SUSTAIN,
            release_ms: DEFAULT_RELEASE_MS,
            filter: FilterKind::Low,
            cutoff: DEFAULT_CUTOFF,
            resonance: DEFAULT_RESONANCE,
            mute: false,
        }
    }
}

/// A kit: the name the card prints, and one voice per drum in catalogue order.
///
/// A kit is the whole sound of the drum machine, so it is swapped whole behind an `ArcSwap` — the audio thread reads one kit or
/// the next, never a voice table with half a load applied to it.
#[derive(Clone, Debug)]
pub struct Kit {
    pub name: String,
    pub voices: [KitVoice; Drum::ALL.len()],
}

impl Kit {
    /// The kit's mutes packed in catalogue order: bit `Drum::index()` set means that voice starts silent.
    ///
    /// The pack and the unpack live here together and nowhere else, so the state's `drum_mute` — what the audio thread reads once
    /// per block — and the kit's own flags cannot disagree about which voice is bit five. `Drum::index` is the catalogue, so
    /// moving a voice in the grid moves its bit with it.
    pub fn mute_mask(&self) -> u16 {
        let mut mask = 0u16;
        for drum in Drum::ALL {
            if self.voices[drum.index()].mute {
                mask |= 1u16 << drum.index();
            }
        }
        mask
    }

    /// Set each voice's mute from a mask packed by [`Kit::mute_mask`] — the inverse, and the only other place a bit is decoded.
    pub fn set_mute_mask(&mut self, mask: u16) {
        for drum in Drum::ALL {
            self.voices[drum.index()].mute = mask & (1u16 << drum.index()) != 0;
        }
    }
}

/// The kit a device starts with: sixteen voices, none of them with a sample.
///
/// A device that has loaded no kit is silent, and that is the honest state — the kit is the sound, and there is nothing to play
/// before one arrives. Task 6 puts a factory kit in its place on the first run.
pub fn default_kit() -> Kit {
    Kit {
        name: DEFAULT_KIT_NAME.to_string(),
        voices: std::array::from_fn(|_| KitVoice::default()),
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

    #[test]
    fn a_default_kit_has_sixteen_voices_and_no_samples() {
        let kit = default_kit();
        assert_eq!(
            kit.voices.len(),
            Drum::ALL.len(),
            "one voice per drum, or the mask has a bit with nothing behind it"
        );
        assert_eq!(
            kit.name, "Init",
            "the card prints this for a device with no kit loaded"
        );

        let default = KitVoice::default();
        for drum in Drum::ALL {
            let voice = &kit.voices[drum.index()];
            assert!(
                voice.sample.is_empty(),
                "{drum:?} ships a sample, and the default kit has no source to ship"
            );
            assert!(
                !voice.mute,
                "{drum:?} starts silent, but a fresh device is audible"
            );
            assert_eq!(voice.tune, default.tune, "{drum:?} starts detuned");
            assert_eq!(
                voice.filter, default.filter,
                "{drum:?} starts on the wrong side"
            );
            assert_eq!(voice.attack_ms, default.attack_ms, "{drum:?}");
            assert_eq!(voice.decay_ms, default.decay_ms, "{drum:?}");
            assert_eq!(voice.sustain, default.sustain, "{drum:?}");
            assert_eq!(voice.release_ms, default.release_ms, "{drum:?}");
            assert_eq!(voice.cutoff, default.cutoff, "{drum:?}");
            assert_eq!(voice.resonance, default.resonance, "{drum:?}");
        }

        // The numbers themselves, pinned. A default that drifts — a decay so short the hit disappears, a filter closed onto the
        // sample — is a change to the machine's sound, and it should fail here rather than in a listening test nobody runs.
        assert_eq!(default.tune, 0.0, "TUNE at centre: no detune");
        assert_eq!(
            default.attack_ms, 2.0,
            "long enough not to click, short enough to be a hit"
        );
        assert_eq!(default.decay_ms, 300.0, "a hit, not a held note");
        assert_eq!(default.sustain, 0.0, "a drum decays to nothing");
        assert_eq!(default.release_ms, 20.0);
        assert_eq!(default.cutoff, 1.0, "wide open: the sample as recorded");
        assert_eq!(
            default.resonance, 0.0,
            "flat: no peak the kit did not ask for"
        );
    }

    #[test]
    fn a_kit_packs_its_mutes_in_catalogue_order() {
        let mut kit = default_kit();
        assert_eq!(kit.mute_mask(), 0, "a fresh kit mutes nothing");

        // Both ends of the catalogue and a middle voice: bit 0, bit 14 and bit 15. A reversed order, an off-by-one and a mask
        // packed from a hand-kept list all show up here.
        for drum in [Drum::Kick, Drum::Clave, Drum::Shaker] {
            kit.voices[drum.index()].mute = true;
        }
        assert_eq!(
            kit.mute_mask(),
            0b1100_0000_0000_0001,
            "bit `Drum::index()` is a voice's bit, in catalogue order"
        );

        // And the other direction: a mask from a saved kit or from the panel's mute has to land on the voices it names.
        let mut unpacked = default_kit();
        unpacked.set_mute_mask(kit.mute_mask());
        for drum in Drum::ALL {
            assert_eq!(
                unpacked.voices[drum.index()].mute,
                kit.voices[drum.index()].mute,
                "{drum:?} unpacked from the wrong bit"
            );
        }
        assert_eq!(
            unpacked.mute_mask(),
            kit.mute_mask(),
            "the pack and the unpack must be inverses"
        );
    }
}
