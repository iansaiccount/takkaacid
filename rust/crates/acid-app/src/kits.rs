//! The three kits that ship: acoustic, `808` and `909`, compiled into the binary.
//!
//! A kit is a folder on disk for the player's own (see [`crate::persist`]); the factory three are different in one way —
//! they are `include_bytes!`'d, so they cannot be edited, they cannot go missing, and a player who edits one is making a
//! copy under a free name rather than changing what ships. That is why their names are **reserved**: a folder called `808`
//! in the player's library would shadow the kit that ships, so [`is_factory`] is what both the writer and the free-name rule
//! ask, and there is no second list of them anywhere.
//!
//! The samples are CC0 — `kits/CREDITS.md` records every file's source and licence, and `kits/refresh.py` is the script that
//! fetched them from the same table that generated that page.
//!
//! [`factory_kits`] decodes all forty-eight WAVs, so it is not something to call per frame: it caches its answer per rate.
//! Decoding is UI-thread work, once, when the app learns what rate its device runs at — and again if that rate ever changes,
//! which is why the rate is a parameter and never a constant.

use std::sync::{Mutex, OnceLock};

use acid_core::drums::{Drum, FilterKind, Kit, KitVoice};

use crate::wav;

/// One voice of a factory kit: the WAV it plays, and how the kit shapes it.
///
/// Four numbers, not ten: the kit's character is its tuning, how long a hit rings, and how open its filter is. The rest of
/// [`KitVoice`] keeps [`KitVoice::default`], so a voice that does not name a number here is the same shape in every kit and
/// the difference between the kits is the sound the files already carry.
struct FactoryVoice {
    wav: &'static [u8],
    tune: f32,
    decay_ms: f32,
    cutoff: f32,
}

/// `wav`, tuned and shaped: the whole of what a kit says about a voice beyond the file.
const fn shaped(wav: &'static [u8], tune: f32, decay_ms: f32, cutoff: f32) -> FactoryVoice {
    FactoryVoice {
        wav,
        tune,
        decay_ms,
        cutoff,
    }
}

/// One kit that ships.
struct FactoryKit {
    name: &'static str,
    voices: [FactoryVoice; Drum::ALL.len()],
}

/// The three kits, in the order a browser lists them.
///
/// **The numbers are the first guess of a listening pass nobody has run.** They are set by intent — `808` long and dark,
/// `909` short and bright, `acoustic` as recorded — and the files themselves are what make each kit what it is. An ear with
/// the drum machine in front of it is what turns these into a sound; see `EAR_TESTING.md`.
const FACTORY: [FactoryKit; 3] = [
    FactoryKit {
        name: "808",
        voices: [
            shaped(include_bytes!("../kits/808/KICK.wav"), -2.0, 500.0, 0.80),
            shaped(include_bytes!("../kits/808/SNR.wav"), 0.0, 260.0, 0.92),
            shaped(include_bytes!("../kits/808/RIM.wav"), 0.0, 60.0, 1.0),
            shaped(include_bytes!("../kits/808/CLAP.wav"), 0.0, 220.0, 0.95),
            shaped(include_bytes!("../kits/808/CHH.wav"), 0.0, 90.0, 1.0),
            shaped(include_bytes!("../kits/808/OHH.wav"), 0.0, 380.0, 1.0),
            shaped(include_bytes!("../kits/808/LOT.wav"), -3.0, 500.0, 0.75),
            shaped(include_bytes!("../kits/808/MID.wav"), -1.0, 420.0, 0.82),
            shaped(include_bytes!("../kits/808/HTOM.wav"), 2.0, 360.0, 0.90),
            shaped(include_bytes!("../kits/808/CRS.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/808/RDE.wav"), 0.0, 600.0, 0.95),
            shaped(include_bytes!("../kits/808/CWB.wav"), 0.0, 300.0, 1.0),
            shaped(include_bytes!("../kits/808/LCO.wav"), -2.0, 450.0, 0.80),
            shaped(include_bytes!("../kits/808/HCO.wav"), 0.0, 400.0, 0.85),
            shaped(include_bytes!("../kits/808/CLV.wav"), 0.0, 120.0, 1.0),
            shaped(include_bytes!("../kits/808/SHK.wav"), 0.0, 150.0, 1.0),
        ],
    },
    FactoryKit {
        name: "909",
        voices: [
            shaped(include_bytes!("../kits/909/KICK.wav"), 0.0, 300.0, 1.0),
            shaped(include_bytes!("../kits/909/SNR.wav"), 0.0, 190.0, 1.0),
            shaped(include_bytes!("../kits/909/RIM.wav"), 2.0, 70.0, 1.0),
            shaped(include_bytes!("../kits/909/CLAP.wav"), 0.0, 200.0, 1.0),
            shaped(include_bytes!("../kits/909/CHH.wav"), 0.0, 70.0, 1.0),
            shaped(include_bytes!("../kits/909/OHH.wav"), 0.0, 300.0, 1.0),
            shaped(include_bytes!("../kits/909/LOT.wav"), -2.0, 350.0, 0.95),
            shaped(include_bytes!("../kits/909/MID.wav"), 0.0, 320.0, 1.0),
            shaped(include_bytes!("../kits/909/HTOM.wav"), 3.0, 300.0, 1.0),
            shaped(include_bytes!("../kits/909/CRS.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/909/RDE.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/909/CWB.wav"), 1.0, 260.0, 1.0),
            shaped(include_bytes!("../kits/909/LCO.wav"), -1.0, 420.0, 0.90),
            shaped(include_bytes!("../kits/909/HCO.wav"), 2.0, 380.0, 0.95),
            shaped(include_bytes!("../kits/909/CLV.wav"), 3.0, 110.0, 1.0),
            shaped(include_bytes!("../kits/909/SHK.wav"), 2.0, 160.0, 1.0),
        ],
    },
    FactoryKit {
        name: "Acoustic",
        voices: [
            shaped(include_bytes!("../kits/acoustic/KICK.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/SNR.wav"), 0.0, 500.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/RIM.wav"), 0.0, 300.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/CLAP.wav"), 0.0, 400.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/CHH.wav"), 0.0, 200.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/OHH.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/LOT.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/MID.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/HTOM.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/CRS.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/RDE.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/CWB.wav"), 0.0, 500.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/LCO.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/HCO.wav"), 0.0, 600.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/CLV.wav"), 0.0, 250.0, 1.0),
            shaped(include_bytes!("../kits/acoustic/SHK.wav"), 0.0, 200.0, 1.0),
        ],
    },
];

/// The factory kits' names, in the order they ship. Reserved: a player's kit may not take one.
pub fn factory_names() -> Vec<&'static str> {
    FACTORY.iter().map(|kit| kit.name).collect()
}

/// How many bytes of WAV the three kits put in the binary — the spec's ~2 MB, as a number a test can hold a build to.
pub fn factory_sample_bytes() -> usize {
    FACTORY
        .iter()
        .flat_map(|kit| kit.voices.iter())
        .map(|voice| voice.wav.len())
        .sum()
}

/// Whether this is the kit a drum machine arrives on rather than a kit anybody chose: `Init` is reserved, and it holds sixteen
/// voices and **no samples at all**. A *player's* kit can also decode to nothing — a folder whose WAVs are missing loads with a
/// problem per voice, by design — so "all samples empty" alone would take that kit away from them; the name is what tells the
/// two apart.
pub fn needs_a_kit(kit: &acid_core::drums::Kit) -> bool {
    kit.name == acid_core::drums::DEFAULT_KIT_NAME
        && kit.voices.iter().all(|voice| voice.sample.is_empty())
}

/// Give the default factory kit to a drum machine that has never had one, decoding at `rate`. Answers whether it filled it.
///
/// A drum device is built on `default_kit()` — sixteen voices and no samples — because a `SharedState` cannot decode a WAV: the
/// rate is only known when the stream opens, which is `main`'s to say. So a device added from the picker arrives silent, and
/// this is the half that makes it audible. `main` asks once a frame, and the answer is `false` for every device that already
/// has samples, which is all of them after the first frame.
pub fn fill_if_silent(state: &crate::state::SharedState, rate: f32) -> bool {
    if !state.kind.has_kit() || !needs_a_kit(&state.kit()) {
        return false;
    }
    let Some(kit) = factory_kit(default_name(), rate) else {
        return false;
    };
    state.adopt_kit(kit);
    true
}

/// Whether `name` is one of the kits that ships — the question a wall and a free name both ask.
pub fn is_factory(name: &str) -> bool {
    FACTORY.iter().any(|kit| kit.name == name)
}

/// Whether `name` is a name no player's kit may take: the three that ship, and the default kit's own word.
///
/// `Init` is on the list because `SavedState::kit: None` *means* "the default kit", and the default kit is called `Init`.
/// A folder by that name would load, play, and then be forgotten on the next save — the session would write nothing rather
/// than the name — so it is reserved rather than made unrememberable. One wall, asked by the writer and the free-name rule.
pub fn is_reserved(name: &str) -> bool {
    is_factory(name) || name == acid_core::drums::DEFAULT_KIT_NAME
}

/// The kits that ship, decoded at `rate` so a voice plays at the pitch its recording had.
///
/// The result is cached per rate: `factory_kits` decodes about two megabytes of WAV the first time it is asked, which is
/// once at startup and once more if the device's rate changes. The cache is a `Mutex` because the rate can change between
/// two callers; it is never held anywhere near the audio path — this is UI-thread work, and the audio thread reads the
/// `ArcSwap<Kit>` the panel publishes.
///
/// A kit whose name is not one of [`factory_names`] is not on offer here; the player's own kits live in their own folder
/// and are read by [`crate::persist::load_kit`].
pub fn factory_kits(rate: f32) -> Vec<Kit> {
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    let mut slot = match cache.lock() {
        Ok(slot) => slot,
        // A panic while decoding cannot have left a half-built kit: the slot is only written once the kits exist.
        Err(poisoned) => poisoned.into_inner(),
    };
    let key = rate.to_bits();
    if let Some((cached, kits)) = slot.as_ref()
        && *cached == key
    {
        return kits.clone();
    }
    let kits = decode(rate);
    *slot = Some((key, kits.clone()));
    kits
}

/// One kit that ships, by the name the browser and the save file use, decoded at `rate`. The one lookup of "which factory kit is
/// this name": the panel's load path and the fill for a silent drum machine both ask it, so a name cannot mean two kits.
pub fn factory_kit(name: &str, rate: f32) -> Option<Kit> {
    factory_kits(rate).into_iter().find(|kit| kit.name == name)
}

/// The kit a drum device with no kit of its own plays, **by name**: the decision the default is, stated once.
///
/// It used to be `FACTORY[0]`, which was the 808 by an accident of that table's order — reordering the table would have
/// changed what a new machine sounds like with no test to say so. The player's ruling is the 808, so the 808 is written down.
pub const DEFAULT_FACTORY_KIT: &str = "808";

/// The name of the kit a drum device with no kit of its own plays, and the answer to "which kit does `None` mean?".
///
/// A name, not a `Kit`: the kits cannot be decoded until a device says what rate it runs at, so what a session's `None`
/// resolves to is a *name* everywhere it is read — `main` loads `factory_kits(rate)` under it, the browser lists it, and the
/// file writes it. One spelling, so the three cannot disagree about which kit a fresh session sounds like.
pub fn default_name() -> &'static str {
    DEFAULT_FACTORY_KIT
}

/// What the cache holds, keyed by the rate its samples were resampled to: the kits are decoded once per rate, never per
/// frame, because they are two megabytes of WAV.
type Decoded = (u32, Vec<Kit>);

/// The cache itself. A `Mutex` because the rate can change between two callers; it is held for the length of one lookup on
/// the UI thread and never anywhere near the audio path, which reads the `ArcSwap<Kit>` the panel publishes.
static CACHE: OnceLock<Mutex<Option<Decoded>>> = OnceLock::new();

/// Decode every voice of every factory kit at `rate`.
fn decode(rate: f32) -> Vec<Kit> {
    FACTORY
        .iter()
        .map(|kit| Kit {
            name: kit.name.to_string(),
            voices: std::array::from_fn(|slot| {
                let voice = &kit.voices[slot];
                // The files are compiled in and `tests/kits.rs` proves every one decodes, so this cannot fail in a build
                // that shipped. A silent voice is the honest answer if it ever does: a panel that panics over a bad sample
                // is worse than one voice that does not sound.
                let sample = wav::decode(voice.wav, rate).unwrap_or_default();
                KitVoice {
                    sample,
                    tune: voice.tune,
                    decay_ms: voice.decay_ms,
                    sustain: KitVoice::default().sustain,
                    attack_ms: KitVoice::default().attack_ms,
                    release_ms: KitVoice::default().release_ms,
                    filter: FilterKind::Low,
                    cutoff: voice.cutoff,
                    resonance: KitVoice::default().resonance,
                    mute: false,
                }
            }),
        })
        .collect()
}
