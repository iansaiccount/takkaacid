//! The save file: bank, knobs and transport as JSON, written atomically.
//!
//! `state.json` is the document the app reads back on launch; each non-empty pattern also gets its own
//! `patterns/<id>.json` so a human can read or hand-edit one pattern without the rest. Every write goes to
//! a `.tmp` file and is renamed into place, so a crash mid-save can never leave a half-written save behind.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use acid_core::bank::PatternBank;
use acid_core::drums::{Drum, FilterKind, Kit, default_kit};
use acid_core::engine::{DEFAULT_TUNING_KNOB, DEFAULT_VOLUME_KNOB};
use acid_core::oscillator::Waveform;
use acid_core::timing::DEFAULT_BPM;
use acid_core::voice::{
    DEFAULT_ACCENT_KNOB, DEFAULT_CUTOFF_KNOB, DEFAULT_DECAY_KNOB, DEFAULT_ENV_MOD_KNOB,
    DEFAULT_RESONANCE_KNOB,
};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::device::{Device, DeviceKind, MAX_DEVICES, Rack, Scheme};
use crate::knobs::Knob;
use crate::midi::route::{Channel, Ports, Role};
use crate::state::{AtomicF32, SharedState};

/// Directory name under the data home.
pub const APP_DIR: &str = "takka-acid";
/// Environment override for the whole data directory — tests and the user can sandbox it.
pub const ENV_DATA_DIR: &str = "TAKKA_ACID_DATA_DIR";
/// The document that holds everything but the pattern bodies.
const STATE_FILE: &str = "state.json";

/// Every knob, as the file stores it. `waveform` is `0` saw / `1` square.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Knobs {
    pub waveform: u8,
    pub tuning: f32,
    pub cutoff: f32,
    pub resonance: f32,
    pub env_mod: f32,
    pub decay: f32,
    pub accent: f32,
    pub volume: f32,
}

impl Default for Knobs {
    fn default() -> Self {
        Self {
            waveform: 0,
            tuning: DEFAULT_TUNING_KNOB,
            cutoff: DEFAULT_CUTOFF_KNOB,
            resonance: DEFAULT_RESONANCE_KNOB,
            env_mod: DEFAULT_ENV_MOD_KNOB,
            decay: DEFAULT_DECAY_KNOB,
            accent: DEFAULT_ACCENT_KNOB,
            volume: DEFAULT_VOLUME_KNOB,
        }
    }
}

impl Knobs {
    /// The seven tone knobs, in `Knob::ALL` order — the panel's order. The file's field names are the wire format; this
    /// pair of methods is the only place that says which field belongs to which knob, so a knob cannot be saved into a
    /// neighbour's slot, and a knob added to the catalogue fails to compile here instead of landing in a `_` arm.
    pub fn positions(&self) -> [f32; 7] {
        Knob::ALL.map(|knob| match knob {
            Knob::Waveform => {
                crate::ui::waveform_position(Waveform::from_index(usize::from(self.waveform)))
            }
            Knob::Tuning => self.tuning,
            Knob::Cutoff => self.cutoff,
            Knob::Resonance => self.resonance,
            Knob::EnvMod => self.env_mod,
            Knob::Decay => self.decay,
            Knob::Accent => self.accent,
        })
    }

    /// The file's knobs from seven positions in `Knob::ALL` order. Positions are clamped, and the switch snaps to its
    /// nearest face, so a hand-edited file still loads as a wave.
    ///
    /// `volume` is **not** one of the seven: it keeps [`Knobs::default`], and a caller holding a real volume overwrites it
    /// (see `shared_to_saved`). Volume is not in the knob row, so it is not in the catalogue.
    pub fn from_positions(positions: &[f32; 7]) -> Knobs {
        let mut knobs = Knobs::default();
        for (knob, position) in Knob::ALL.iter().zip(positions) {
            let position = position.clamp(0.0, 1.0);
            match knob {
                Knob::Waveform => {
                    knobs.waveform = crate::ui::waveform_from_position(position).index() as u8;
                }
                Knob::Tuning => knobs.tuning = position,
                Knob::Cutoff => knobs.cutoff = position,
                Knob::Resonance => knobs.resonance = position,
                Knob::EnvMod => knobs.env_mod = position,
                Knob::Decay => knobs.decay = position,
                Knob::Accent => knobs.accent = position,
            }
        }
        knobs
    }
}

/// One machine of a rack, as the file stores it.
///
/// Every field is something the strip, the face or the sound would otherwise forget: a name and a colour live only in
/// memory, and a bank, a patch, a shuffle, a learned rig, a set of ports and a kit are each their own thing to that machine.
/// A device's **kind** is a caption rather than an index — `DeviceKind::name()` written down — because a reader of the file
/// should be able to see which machine it is, and because `from_name` derives the mapping from the catalogue instead of
/// spelling the variants a second time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedDevice {
    /// Stable for the device's life, and never reused; the rack's next number is one above the highest it reads.
    pub number: usize,
    pub name: String,
    pub kind: String,
    pub scheme: String,
    pub inputs: SavedInputs,
    pub bank: PatternBank,
    pub knobs: Knobs,
    /// The panel's `SHUFFLE`, which is each machine's own.
    pub swing: f32,
    /// The controllers this machine has been taught, in `midi::cc::LEARNABLE` order.
    pub cc_map: Vec<u8>,
    /// The kit a drum machine plays, by name. `None` is the default kit, which is what an acid device always holds.
    pub kit: Option<String>,
}

/// What a device hears. The route types are written with serde's own names: they are open data — a port name is whatever
/// the system reports — not a catalogue the face spells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedInputs {
    pub ports: Ports,
    pub channel: Channel,
    pub role: Role,
}

impl SavedInputs {
    /// The factory filter: every port, every channel, notes and controllers. What a device has when the file says nothing,
    /// which is what a version-11 session does — it never saved one.
    pub fn everything() -> Self {
        Self {
            ports: Ports::All,
            channel: Channel::All,
            role: Role::PlayAndControl,
        }
    }
}

impl SavedDevice {
    /// The one machine a version-11 file describes, repaired from its flat mirror.
    ///
    /// Its **kind is inferred**, because a version-11 file records none: `DRUMS` when the file names a kit — only a machine
    /// with a kit ever writes a kit name — and `ACID` otherwise. A drum device that never left the `Init` kit cannot be
    /// recovered and opens as an acid; the file does not say, and a different guess would be worse than an honest one.
    pub fn from_mirror(state: &SavedState) -> SavedDevice {
        let kind = if state.kit.is_some() {
            DeviceKind::Drums
        } else {
            DeviceKind::Acid
        };
        SavedDevice {
            number: 1,
            name: Device::default_name(kind, 1),
            kind: kind.name().to_string(),
            scheme: Scheme::default().name().to_string(),
            inputs: SavedInputs::everything(),
            bank: state.bank.clone(),
            knobs: state.knobs,
            swing: state.swing,
            cc_map: state.cc_map.clone(),
            kit: state.kit.clone(),
        }
    }
}

/// The save-file format this build writes. A file without the field is version 0 and is upgraded on load.
pub const SAVE_VERSION: u32 = 12;

/// Everything that survives a restart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedState {
    /// The format this file was written in; `0` means it predates the field.
    #[serde(default)]
    pub version: u32,
    pub bank: PatternBank,
    pub knobs: Knobs,
    pub bpm: f32,
    pub swing: f32,
    /// One MIDI input name, kept for the frozen save format: the first port the session had open, or `None` when it had
    /// none. It cannot say "none on purpose" apart from "nothing chosen yet", so a session that closed every port opens one
    /// again on the next launch — the save-a-list round is where that gets a field.
    #[serde(default)]
    pub midi_input: Option<String>,
    /// The named song this session is working on, if any; `None` until it is first saved by name.
    #[serde(default)]
    pub song: Option<String>,
    /// The controller each learnable knob listens to, in `midi::cc::LEARNABLE` order; learned MIDI mappings, so a
    /// performance rig survives a restart. Absent means the factory pairing.
    #[serde(default = "default_cc_map")]
    pub cc_map: Vec<u8>,
    /// Whether hovering a control explains it. On for a file that predates the field: help should not have to be found.
    #[serde(default = "default_true")]
    pub tooltips: bool,
    /// How full RANDOM fills a pattern, `0.0..=1.0`. Panel-side like the TIPS switch, so `main` carries it.
    #[serde(default = "default_density")]
    pub density: f32,
    /// The scale RANDOM draws from, as a `ui::scale::Scale` index.
    #[serde(default)]
    pub scale: u8,
    /// Whether the session writes itself when it changes. Off unless the player turns it on: saving is a decision, and an
    /// autosave that silently overwrites the song you were about to discard is exactly the surprise this avoids.
    #[serde(default)]
    pub autosave: bool,
    /// The kit this device plays, by name. Absent means the default kit, which a session written before the drum machine
    /// has and which a drum device that never picked one keeps.
    ///
    /// **`main` is what loads it, not `apply_saved`**: decoding a kit needs the rate the running device plays at, and the
    /// kit is a folder of WAVs rather than a field of this document. It is appended after every other field, as the drum
    /// masks were appended to `Step`, so a version-10 file's prefix still reads.
    #[serde(default)]
    pub kit: Option<String>,
    /// Every machine in the rack, in rack order, with device 1 also written above as this document's flat fields. A file
    /// written before this field existed has none, and `sanitize` rebuilds one device from the mirror, so an older song loads
    /// as a one-device song and a newer file still names the machine an older build can read. Appended last, after `kit`, so
    /// a version-11 file's prefix is byte for byte what it was.
    #[serde(default)]
    pub devices: Vec<SavedDevice>,
}

/// The factory MIDI mapping, for a file that predates the field.
pub fn default_cc_map() -> Vec<u8> {
    crate::midi::cc::DEFAULT_CC.to_vec()
}

/// Hover help defaults on: a player should not have to find the switch to be told what things do.
fn default_true() -> bool {
    true
}

/// A file from before the density knob fills patterns half full, which is where the randomiser started.
fn default_density() -> f32 {
    crate::ui::randomise::DEFAULT_DENSITY
}

impl Default for SavedState {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            bank: PatternBank::factory(),
            knobs: Knobs::default(),
            bpm: DEFAULT_BPM,
            swing: 0.0,
            midi_input: None,
            song: None,
            cc_map: crate::midi::cc::DEFAULT_CC.to_vec(),
            tooltips: true,
            density: default_density(),
            scale: 0,
            autosave: false,
            kit: None,
            // One device is the empty list: the mirror above *is* that device, and `sanitize` builds it back from there.
            devices: Vec::new(),
        }
    }
}

impl SavedState {
    /// Bring a file from an older build up to this one.
    ///
    /// Version 2 and older were written when the pitch lattice was rooted at MIDI 33; it is rooted at MIDI 12 now, so the
    /// same note is spelled 21 semitones higher. Shifting every step keeps the music exactly as it sounded — the
    /// alternative would drop every saved pattern by a twelfth and a half.
    ///
    /// Version 9 and older predate the drum machine's masks: `serde`'s defaults zero them, which is what a pattern written
    /// before a voice existed should say — nothing struck. A bump with nothing to rewrite still bumps, because a build that
    /// knows 9 must not read a 10 as its own.
    pub fn upgraded(mut self) -> Self {
        if self.version < 3 {
            self.bank.map_patterns(|pattern| pattern.transposed(21));
        }
        // Version 4 and older were written before the waveform knob could be taught: its controller goes at the front of
        // the map, and every learned fader keeps the slot — and the knob — it had.
        if self.version < 5 {
            self.cc_map.insert(0, crate::midi::cc::DEFAULT_CC[0]);
        }
        // Version 5 and older predate the pattern selector and the learnable tempo. Both are appended, in slot order, so
        // again nothing that already had a slot moves.
        while self.cc_map.len() < crate::midi::cc::LEARNABLE.len() {
            let slot = self.cc_map.len();
            self.cc_map.push(crate::midi::cc::DEFAULT_CC[slot]);
        }
        // Version 12 is the rack document. A file written before it has no `devices`, which `sanitize` has already rebuilt
        // from the mirror, and a file written since carries the mirror too. Nothing to rewrite — the bump exists so that a
        // build which knows 11 does not read a 12 as its own.
        self.version = SAVE_VERSION;
        self
    }
}

/// Where the save file lives: the env override, else `$XDG_DATA_HOME`, else `~/.local/share`.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(ENV_DATA_DIR) {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir).join(APP_DIR);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".local").join("share").join(APP_DIR)
}

/// Write `state.json`, through a `.tmp` file and a rename. It is the only file the app reads, so it is the only
/// file it writes.
pub fn save(dir: &Path, state: &SavedState) -> Result<()> {
    let json = serde_json::to_vec_pretty(state)?;
    write_atomic(&dir.join(STATE_FILE), &json)
}

/// Read the save file. `Ok(None)` means there is nothing saved yet; a broken file is an error that names it.
pub fn load(dir: &Path) -> Result<Option<SavedState>> {
    let path = dir.join(STATE_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let state =
        serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(state))
}

/// Write `bytes` to `path` through a `.tmp` and a rename, which is the one place the app writes a file it could be
/// interrupted during. Both the save file and a kit's folder go through it, so there is one rule for "never half a file".
fn write_through(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Write `bytes` to `path` without ever leaving a partial file there.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    write_through(path, bytes).with_context(|| format!("writing {}", path.display()))
}

/// Build the shared state the app runs on from a saved document: device 1's machine.
pub fn saved_to_shared(state: &SavedState) -> Arc<SharedState> {
    let shared = SharedState::new();
    let mut bank = state.bank.clone();
    apply_saved(state, &shared, &mut bank);
    shared
}

/// One device's machine, built from its saved half.
pub fn saved_device_to_shared(saved: &SavedDevice, tempo: Arc<AtomicF32>) -> Arc<SharedState> {
    let kind = DeviceKind::from_name(&saved.kind).unwrap_or(DeviceKind::Acid);
    // The tempo cell is the rack's, so a machine loaded from a file follows the same clock every other one does.
    let shared = SharedState::with_tempo_for(kind, tempo);
    apply_device(saved, &shared);
    publish_pattern(&saved.bank, &shared);
    shared
}

/// Pour a saved document into a running app: the shared state the audio thread reads, and the bank the panel
/// navigates. Loading a song uses this, so a load and a restart cannot disagree about what a field means.
///
/// Device 1's half only — a document with a list is poured one device at a time by `Rack::replace_from_saved`, which owns the
/// rack. A document with no list is the document itself: the mirror is the machine.
pub fn apply_saved(state: &SavedState, shared: &SharedState, bank: &mut PatternBank) {
    *bank = state.bank.clone();
    shared.tempo_bpm.set(state.bpm);
    let device = state
        .devices
        .first()
        .cloned()
        .unwrap_or_else(|| SavedDevice::from_mirror(state));
    apply_device(&device, shared);
    publish_pattern(bank, shared);
}

/// One device's own half of a pour: its patch, its shuffle and its learned rig.
fn apply_device(saved: &SavedDevice, shared: &SharedState) {
    // The tone knobs load through the catalogue, so the file's field order cannot disagree with the panel's.
    for (knob, position) in Knob::ALL.iter().zip(saved.knobs.positions()) {
        knob.publish(shared, position);
    }
    shared.volume.set(saved.knobs.volume);
    shared.swing.set(saved.swing.clamp(0.0, 1.0));
    // The learned mapping travels with the machine, so a rig saved in a song comes back with it.
    for (slot, cc) in saved.cc_map.iter().enumerate().take(shared.cc_map.len()) {
        shared.cc_map[slot].store(i32::from(*cc), std::sync::atomic::Ordering::Relaxed);
    }
    // A file written by an older build may have fewer slots than this one: fill the rest from the factory pairing, so a
    // control the file never knew about cannot inherit whatever the last session taught it.
    for slot in saved.cc_map.len()..shared.cc_map.len() {
        shared.cc_map[slot].store(
            i32::from(crate::midi::cc::DEFAULT_CC[slot]),
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}

/// The pattern a bank is sitting on, and which one that is: a pattern controller reads the index to know where it points.
fn publish_pattern(bank: &PatternBank, shared: &SharedState) {
    shared.set_pattern(Arc::new(*bank.current_pattern()));
    shared.set_pattern_index(bank.current());
}

/// The parts of a document a machine owns: its patch, its shuffle and its learned rig.
fn saved_fields(shared: &SharedState) -> (Knobs, f32, Vec<u8>) {
    let knobs = Knobs {
        volume: shared.volume.get(),
        ..Knobs::from_positions(&Knob::ALL.map(|knob| knob.read(shared)))
    };
    let cc_map = shared
        .cc_map
        .iter()
        .map(|mapped| {
            mapped
                .load(std::sync::atomic::Ordering::Relaxed)
                .clamp(0, 127) as u8
        })
        .collect();
    (knobs, shared.swing.get(), cc_map)
}

/// One machine, as the file stores it.
pub fn saved_device_from(device: &Device) -> SavedDevice {
    let (knobs, swing, cc_map) = saved_fields(&device.state);
    SavedDevice {
        number: device.number,
        name: device.name.clone(),
        kind: device.kind().name().to_string(),
        scheme: device.scheme.name().to_string(),
        inputs: SavedInputs {
            ports: device.inputs.ports.clone(),
            channel: device.inputs.channel,
            role: device.inputs.role,
        },
        bank: device.bank.clone(),
        knobs,
        swing,
        cc_map,
        // A device still on the default kit has no name to write, and an absent field is what "the default kit" means.
        kit: Some(device.state.kit().name.clone())
            .filter(|name| name != acid_core::drums::DEFAULT_KIT_NAME),
    }
}

/// Capture the running state for saving. The bank comes from `bank`, so callers keep the other 63 patterns.
///
/// A **one-device** document: the flat fields are the machine and `devices` is empty, which `sanitize` turns back into a
/// list. A whole rack comes from [`rack_to_saved`], and that is what `main` saves.
pub fn shared_to_saved(shared: &SharedState, bank: PatternBank) -> SavedState {
    let (knobs, swing, cc_map) = saved_fields(shared);
    let mut bank = bank;
    let current = bank.current();
    bank.replace(current, *shared.pattern());
    SavedState {
        version: SAVE_VERSION,
        bank,
        knobs,
        bpm: shared.tempo_bpm.get(),
        swing,
        // The MIDI choice lives with the app, not with the shared state; `main` fills it in before saving.
        midi_input: None,
        // Which named song is loaded; `main` fills it in, like the MIDI choice.
        song: None,
        cc_map,
        // The hover-help switch and the randomise dials live with the panel, not the shared state; `main` fills them in
        // like the MIDI choice.
        tooltips: true,
        density: crate::ui::randomise::DEFAULT_DENSITY,
        scale: 0,
        autosave: false,
        // The kit the drawn device plays. Read from the kit itself, so a session names the kit it was saved with.
        kit: Some(shared.kit().name.clone())
            .filter(|name| name != acid_core::drums::DEFAULT_KIT_NAME),
        // One device is the empty list: the mirror above is the machine, and a reader turns it into a list.
        devices: Vec::new(),
    }
}

/// The kits a session names, by device index: one per drum machine, and the default factory kit for a machine that names
/// none. What `main` queues for the frame that learns the sample rate — a kit is a folder of WAVs and the rate is not known
/// when the file is read, so the decision of *which* kits to decode belongs here, where it can be tested.
pub fn wanted_kits(state: &SavedState) -> Vec<(usize, String)> {
    state
        .devices
        .iter()
        .enumerate()
        .filter(|(_, device)| DeviceKind::from_name(&device.kind) == Some(DeviceKind::Drums))
        .map(|(index, device)| {
            (
                index,
                device
                    .kit
                    .clone()
                    .unwrap_or_else(|| crate::kits::default_name().to_string()),
            )
        })
        .collect()
}

/// The ports a session names that `open` has not got, in the order they are first named and without repeats.
///
/// A song from another machine names ports by the system's own names, so a load can leave a device deaf. Nothing is rewritten
/// — the routing is worth keeping for the day the port comes back — and this is the list the status line says once.
pub fn unopened_ports(state: &SavedState, open: &[String]) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    for device in &state.devices {
        if let Ports::Named(names) = &device.inputs.ports {
            for port in names {
                if !open.iter().any(|have| have == port) && !missing.contains(port) {
                    missing.push(port.clone());
                }
            }
        }
    }
    missing
}

/// A whole rack, as the file stores it: every machine, and device 1 written down a second time as the flat mirror an older
/// build reads.
pub fn rack_to_saved(rack: &Rack) -> SavedState {
    let mut saved = SavedState {
        devices: rack.devices().iter().map(saved_device_from).collect(),
        ..SavedState::default()
    };
    // One rule for the mirror: a document leaving this build is repaired by the same function as one arriving.
    sanitize(&mut saved);
    saved.bpm = rack.tempo_bpm();
    saved
}

/// Never fails: a wrong save file means "start again", so the app always opens.
///
/// The second half of the answer is whether this was a first run — no session file at all — which `main` uses to mark the
/// session unsaved and to say so: a first run opens on a demo, and nothing about it is on disk yet.
pub fn open(dir: &Path) -> (SavedState, bool) {
    match load(dir) {
        Ok(Some(mut state)) => {
            sanitize(&mut state);
            (state.upgraded(), false)
        }
        Ok(None) => (crate::demo::seed(), true),
        Err(error) => {
            eprintln!("takka-acid: ignoring unreadable save file: {error}");
            (crate::demo::seed(), true)
        }
    }
}

/// The session with no demo in it: the factory bank. Kept for callers that want a blank machine rather than a first run.
pub fn load_or_default(dir: &Path) -> SavedState {
    match load(dir) {
        Ok(Some(mut state)) => {
            sanitize(&mut state);
            state.upgraded()
        }
        Ok(None) => SavedState::default(), // already the current version
        Err(error) => {
            eprintln!("takka-acid: ignoring unreadable save file: {error}");
            SavedState::default()
        }
    }
}

/// Whether a song of this name is already on disk: SAVE rewrites it when it is, and leaves the session alone when it is not.
pub fn song_exists(dir: &Path, name: &str) -> bool {
    list_songs(dir).iter().any(|listed| listed == name)
}

/// Clamp everything a hand-edited file could get wrong, so an edited JSON can never build an invalid
/// engine.
pub fn sanitize(state: &mut SavedState) {
    state.bpm = acid_core::timing::clamp_bpm(state.bpm);
    state.swing = state.swing.clamp(0.0, 1.0);
    sanitize_knobs(&mut state.knobs);
    state.density = state.density.clamp(0.0, 1.0);
    state.scale = state
        .scale
        .min((crate::ui::scale::Scale::ALL.len() - 1) as u8);
    sanitize_map(&mut state.cc_map);
    // A hand-edited file can hold fewer patterns than the box has. Every accessor assumes the full catalogue, and `get`
    // is on the audio path, so the bank is repaired before anything reads it.
    state.bank.normalise();
    sanitize_devices(state);
}

/// Every knob is a `0.0..=1.0` position except the waveform, which is a catalogue index. Four faces, not two: the save file
/// stores the wave catalogue index, so clipping it to 1 would load SINE and TRI as SQUARE — the bug the two-state switch left
/// behind.
fn sanitize_knobs(knobs: &mut Knobs) {
    knobs.waveform = knobs
        .waveform
        .min((acid_core::oscillator::Waveform::ALL.len() - 1) as u8);
    knobs.tuning = knobs.tuning.clamp(0.0, 1.0);
    knobs.cutoff = knobs.cutoff.clamp(0.0, 1.0);
    knobs.resonance = knobs.resonance.clamp(0.0, 1.0);
    knobs.env_mod = knobs.env_mod.clamp(0.0, 1.0);
    knobs.decay = knobs.decay.clamp(0.0, 1.0);
    knobs.accent = knobs.accent.clamp(0.0, 1.0);
    knobs.volume = knobs.volume.clamp(0.0, 1.0);
}

/// A map written by a build with more learnable knobs than this one cannot be honoured slot for slot, and guessing which
/// controller belonged to which knob would be worse than the factory pairing.
fn sanitize_map(map: &mut Vec<u8>) {
    if map.len() > crate::midi::cc::LEARNABLE.len() {
        *map = crate::midi::cc::DEFAULT_CC.to_vec();
    }
    for cc in map.iter_mut() {
        *cc = (*cc).min(127);
    }
}

/// The device list and the flat fields are one thing seen twice: the list is the rack, and the flat fields are device 1's
/// mirror, which is how a build that knows version 11 still opens a version-12 song. Repairing one from the other, in one
/// function, is what keeps a file this build writes from disagreeing with itself.
fn sanitize_devices(state: &mut SavedState) {
    // A version-11 file has no list at all; the mirror is the machine it describes.
    if state.devices.is_empty() {
        state.devices.push(SavedDevice::from_mirror(state));
    }
    // The strip shows four cells, so the first four are the ones that can be played; a fifth is a file that grew.
    state.devices.truncate(MAX_DEVICES);
    for device in &mut state.devices {
        // A caption this build does not know becomes the default one: a hand-edited word must not cost the player their song.
        if DeviceKind::from_name(&device.kind).is_none() {
            device.kind = DeviceKind::Acid.name().to_string();
        }
        if Scheme::from_name(&device.scheme).is_none() {
            device.scheme = Scheme::default().name().to_string();
        }
        device.swing = device.swing.clamp(0.0, 1.0);
        sanitize_knobs(&mut device.knobs);
        sanitize_map(&mut device.cc_map);
        device.bank.normalise();
    }
    // Numbers before names: the default a blank name falls back to is built from the number, so the number has to be settled
    // first. A duplicate is moved above the highest number in the file, which is the rule the rack's own counter keeps.
    let mut numbers: Vec<usize> = Vec::new();
    let mut next = state
        .devices
        .iter()
        .map(|device| device.number)
        .max()
        .unwrap_or(0)
        + 1;
    for device in &mut state.devices {
        while numbers.contains(&device.number) {
            device.number = next;
            next += 1;
        }
        numbers.push(device.number);
    }
    let mut taken: Vec<String> = Vec::new();
    for device in &mut state.devices {
        let kind = DeviceKind::from_name(&device.kind).unwrap_or(DeviceKind::Acid);
        let wanted = match device.name.trim() {
            "" => Device::default_name(kind, device.number),
            trimmed => trimmed.to_string(),
        };
        let name = distinct_name(&wanted, |candidate| {
            taken.iter().any(|held| held == candidate)
        });
        taken.push(name.clone());
        device.name = name;
    }
    // The mirror last, so one rule owns the equality. A file this build writes carries device 1 twice, identically.
    let Some(first) = state.devices.first().cloned() else {
        return;
    };
    state.bank = first.bank;
    state.knobs = first.knobs;
    state.swing = first.swing;
    state.cc_map = first.cc_map;
    state.kit = first.kit;
}

/// Where named songs live: one file each, under the data directory.
pub const SONGS_DIR: &str = "songs";

/// The file a named song lives in.
pub fn song_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(SONGS_DIR).join(format!("{name}.json"))
}

/// A name that can be a file name, or `None` when the typed one cannot: blank, too long, or shaped like a path.
/// The rules are deliberately conservative — a song name is typed into a panel, not a shell.
pub fn song_name(typed: &str) -> Option<String> {
    let name = typed.trim();
    if name.is_empty() || name.chars().count() > 48 {
        return None;
    }
    let forbidden = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    if name.chars().any(|c| forbidden.contains(&c)) || name.starts_with('.') || name.contains("..")
    {
        return None;
    }
    Some(name.to_string())
}

/// Every song already saved, sorted so the card's list does not jump about.
pub fn list_songs(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir.join(SONGS_DIR)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".json").map(str::to_string)
        })
        .collect();
    names.sort();
    names
}

/// Write a named song. Same format as the session file, so reading one is the same read.
pub fn save_song(dir: &Path, name: &str, state: &SavedState) -> Result<()> {
    let Some(name) = song_name(name) else {
        return Err(anyhow!("a song needs a name"));
    };
    let path = song_path(dir, &name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    write_atomic(&path, &serde_json::to_vec_pretty(state)?)
}

/// Read a named song back, upgrading it like the session file.
pub fn load_song(dir: &Path, name: &str) -> Result<SavedState> {
    let path = song_path(dir, name);
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut state: SavedState =
        serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))?;
    // A song is a save file like any other, and it may be hand-edited: sanitise before anyone reads its bank.
    sanitize(&mut state);
    Ok(state.upgraded())
}

/// Remove a saved song. The name is validated first, so a stray one can never point out of the songs directory.
pub fn delete_song(dir: &Path, name: &str) -> Result<()> {
    let Some(name) = song_name(name) else {
        return Err(anyhow!("that is not a song name"));
    };
    let path = song_path(dir, &name);
    std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))
}

// A song is never renamed in the app: a save under a new name writes a new file, and the file manager can move files.

/// Where a save should land: the file the session is on, or a new one under a typed name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveTarget {
    /// Overwrite the song the session came from.
    InPlace(String),
    /// Write a new song under this name, leaving the old file where it is.
    NewFile(String),
}

/// Decide where a save goes. A typed name equal to the session's song means "keep it current"; anything else is a new
/// file, so a save never silently moves a song. A blank name keeps the current one; blank with nothing open saves nothing.
pub fn save_target(current: Option<&str>, typed: &str) -> Option<SaveTarget> {
    let typed = song_name(typed);
    match (current, typed) {
        (Some(current), Some(typed)) if current == typed => Some(SaveTarget::InPlace(typed)),
        (_, Some(typed)) => Some(SaveTarget::NewFile(typed)),
        (Some(current), None) => Some(SaveTarget::InPlace(current.to_string())),
        (None, None) => None,
    }
}

/// Write the session to a path the player chose. The bytes are the library's own format, so an export can be dropped
/// straight into `songs/` and will load there.
pub fn export_song(to: &Path, state: &SavedState) -> Result<()> {
    if let Some(parent) = to.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        return Err(anyhow!("there is no folder at {}", parent.display()));
    }
    write_atomic(to, &serde_json::to_vec_pretty(state)?)
        .with_context(|| format!("writing {}", to.display()))
}

/// Copy a song from a path the player chose into the library, and answer the name it landed under.
///
/// A song is a save file like any other and may be hand-edited, so it is sanitised and upgraded before it is written; a
/// file that is not a song is refused before anything touches the library.
pub fn import_song(from: &Path, dir: &Path) -> Result<String> {
    let bytes = std::fs::read(from).with_context(|| format!("reading {}", from.display()))?;
    let mut state: SavedState =
        serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", from.display()))?;
    sanitize(&mut state);
    let state = state.upgraded();

    let stem = from
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let wanted = song_name(&stem).unwrap_or_else(|| "Imported song".to_string());
    let name = free_song_name(dir, &wanted);
    save_song(dir, &name, &state)?;
    Ok(name)
}

/// The name, or the first `X (2)`, `X (3)`, and so on that is free: a save or an import never overwrites a song nobody
/// opened. Public because `SAVE`, `SAVE A COPY` and `IMPORT` all land the same way, and one copy of this rule is enough.
pub fn free_song_name(dir: &Path, wanted: &str) -> String {
    free_name(wanted, |name| song_path(dir, name).exists())
}

/// The name, or the first `X (2)`, `X (3)` and so on that `taken` answers free.
///
/// One rule with three callers: a song (a file in `songs/`), a kit (a folder in `kits/`), and the copy an edit of a factory
/// kit makes, which is a name the panel holds in memory before anything is written. A second implementation of this, or of
/// the name rule it starts from, is the hand-kept second list this project has paid for twice.
pub fn free_name(wanted: &str, taken: impl Fn(&str) -> bool) -> String {
    let wanted = song_name(wanted).unwrap_or_else(|| "Untitled".to_string());
    distinct_name(&wanted, taken)
}

/// `wanted`, or the first `wanted (2)`, `wanted (3)` … that `taken` answers free.
///
/// The rule for telling two names of a kind apart, in one place: a device in a rack (`Rack::rename` and `sanitize`), a song
/// file, a kit folder, and the copy an edit of a factory kit makes. `free_name` puts a file name's validation in front of it
/// and is the only caller that needs to; a device's name is not a file name, so it comes here directly.
pub fn distinct_name(wanted: &str, taken: impl Fn(&str) -> bool) -> String {
    let wanted = match wanted.trim() {
        "" => "Untitled",
        trimmed => trimmed,
    };
    if !taken(wanted) {
        return wanted.to_string();
    }
    for copy in 2..1000 {
        let suffix = format!(" ({copy})");
        // The suffix must not push the name past what a name may be, or the free name could not be saved at all.
        let room = 48usize.saturating_sub(suffix.chars().count());
        let stem: String = wanted.chars().take(room).collect();
        let candidate = format!("{stem}{suffix}");
        if !taken(&candidate) {
            return candidate;
        }
    }
    format!("{wanted} (copy)")
}

/// Where the player's kits live: one folder each, under the data directory. Kept beside [`SONGS_DIR`] because a kit and a
/// song are the same kind of thing to a player — something on disk with a name.
pub const KITS_DIR: &str = "kits";

/// The document inside a kit's folder.
const KIT_FILE: &str = "kit.json";

/// Where kits live for this run: `<data>/kits`.
pub fn kits_dir() -> PathBuf {
    data_dir().join(KITS_DIR)
}

/// A kit's name, by the rule a song's name uses.
///
/// A name is typed into a panel and becomes a folder, so it has to be a name and never a path. This is deliberately the same
/// function rather than a second rule with its own list of forbidden characters: the two would drift, and the drift would be
/// a way out of `kits/`.
pub fn kit_name(typed: &str) -> Option<String> {
    song_name(typed)
}

/// The folder a kit of this name lives in.
pub fn kit_folder(dir: &Path, name: &str) -> PathBuf {
    dir.join(name)
}

/// The name a kit should land under: the one asked for, or the first `X (2)` free of both the player's folders and the names
/// that are reserved — the three that ship, because a player's folder called `808` would shadow the kit that ships, and `Init`,
/// because a kit of that name would be forgotten on the next save.
pub fn free_kit_name(dir: &Path, wanted: &str) -> String {
    free_name(wanted, |name| {
        crate::kits::is_reserved(name) || kit_folder(dir, name).exists()
    })
}

/// One voice of a kit, as its document stores it.
///
/// The playable fields are in [`KitVoice`](acid_core::drums::KitVoice)'s order and then the three provenance fields: `file`
/// is the WAV's name in the kit's folder, `source` and `credit` say where the sample came from. The order is the document's
/// contract, like [`SavedState`]'s — a rename or a reorder is a migration.
///
/// `file` empty means the voice has no sample, which is a silent voice and not a missing file. `filter` is the catalogue
/// index of a [`FilterKind`](acid_core::drums::FilterKind), the way [`Knobs::waveform`] is a waveform's index: the file
/// stores the position in a catalogue whose order the code owns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KitVoiceDoc {
    pub tune: f32,
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
    pub filter: u8,
    pub cutoff: f32,
    pub resonance: f32,
    pub mute: bool,
    pub file: String,
    pub source: String,
    pub credit: String,
}

/// A kit's document: `<data>/kits/<name>/kit.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KitDocument {
    /// The kit's name. It is written for a human reading the folder, and it never overrides the folder's name: the folder
    /// is what the browser lists, so a hand-edited name here must not rename a kit out from under the list.
    pub name: String,
    pub voices: [KitVoiceDoc; Drum::ALL.len()],
}

/// Why a kit would not load or would not save.
/// Every variant's [`Display`](std::fmt::Display) is a sentence the status line can print as it stands, naming the voice and
/// the file where there is one: a kit is a folder a player can own, damage and hand to a friend, so "this did not work" is
/// never the whole answer.
///
/// The two variants that carry a `std::io::ErrorKind` keep it for a caller that wants to tell "no permission" from "no space";
/// the sentence a player reads does not spell it, because an `ErrorKind`'s own words are for a programmer.
#[derive(Clone, Debug, PartialEq)]
pub enum KitError {
    /// There is no `kit.json` in the folder.
    NoDocument,
    /// There is one and it is not a kit this build can read.
    UnreadableDocument,
    /// The name is not a name, or it is one of the factory kits' and those are read-only.
    BadName,
    /// The folder could not be written.
    Write(std::io::ErrorKind),
    /// The document names a file that is not in the folder.
    Missing { voice: Drum, file: String },
    /// The file is there and could not be read off the disk.
    Unreadable {
        voice: Drum,
        file: String,
        kind: std::io::ErrorKind,
    },
    /// The file is there and is not a WAV this build reads. The decoder's own words, so a bad sample has one vocabulary.
    Undecodable {
        voice: Drum,
        file: String,
        error: crate::wav::WavError,
    },
    /// The document names something that is not a file name — a path, or a blank.
    NotAName { voice: Drum, file: String },
}

impl fmt::Display for KitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDocument => f.write_str("no kit document in that folder"),
            Self::UnreadableDocument => f.write_str("the kit's document will not read"),
            Self::BadName => f.write_str("that is not a name a kit can have"),
            Self::Write(_) => f.write_str("the kit could not be written"),
            Self::Missing { voice, file } => write!(f, "{}: {file} is missing", voice.label()),
            Self::Unreadable { voice, file, .. } => {
                write!(f, "{}: {file} could not be read", voice.label())
            }
            Self::Undecodable { voice, file, error } => {
                write!(f, "{}: {file} - {error}", voice.label())
            }
            Self::NotAName { voice, file } => {
                write!(f, "{}: {file:?} is not a file name", voice.label())
            }
        }
    }
}

impl std::error::Error for KitError {}

/// A kit that loaded, and the voices that did not.
///
/// A kit is sixteen files, and one of them being gone must not cost the player the other fifteen (Review Focus 2): what is
/// wrong arrives here as a [`KitError`] per voice, for the panel to say in a notice, while the kit itself is the one the
/// files that are there describe.
#[derive(Clone, Debug)]
pub struct LoadedKit {
    pub kit: Kit,
    pub problems: Vec<KitError>,
}

/// A kit that was copied into the library, and the voices that did not come with it.
///
/// An import is the one moment a player is holding a folder from somewhere else, so a kit with a broken voice lands anyway and
/// says which file to replace: refusing it whole would leave them nothing to fix, and a kit that is fifteen-sixteenths right is
/// worth more than no kit. `problems` is the same vocabulary [`LoadedKit`] uses, one entry per voice.
#[derive(Clone, Debug)]
pub struct ImportedKit {
    /// The folder the copy landed in, which is `<dir>/<name>` and never an existing one.
    pub folder: PathBuf,
    /// The name it landed under — the one asked for, or the first free `X (2)`.
    pub name: String,
    /// The voices that are silent in the copy, each naming the file to put in the folder.
    pub problems: Vec<KitError>,
}

/// Where one voice's sample came from, kept when a kit is written again so a save cannot forget a licence.
type Provenance = (String, String);

/// Write one kit folder: the document, and one WAV per voice that has a sample.
fn write_folder(
    folder: &Path,
    name: &str,
    kit: &Kit,
    carried: [Provenance; Drum::ALL.len()],
    named: [Option<String>; Drum::ALL.len()],
    rate: f32,
) -> Result<(), KitError> {
    std::fs::create_dir_all(folder).map_err(|error| KitError::Write(error.kind()))?;

    let mut voices: [KitVoiceDoc; Drum::ALL.len()] = std::array::from_fn(|slot| KitVoiceDoc {
        tune: kit.voices[slot].tune,
        attack_ms: kit.voices[slot].attack_ms,
        decay_ms: kit.voices[slot].decay_ms,
        sustain: kit.voices[slot].sustain,
        release_ms: kit.voices[slot].release_ms,
        filter: kit.voices[slot].filter.index() as u8,
        cutoff: kit.voices[slot].cutoff,
        resonance: kit.voices[slot].resonance,
        mute: kit.voices[slot].mute,
        file: String::new(),
        source: carried[slot].0.clone(),
        credit: carried[slot].1.clone(),
    });

    for drum in Drum::ALL {
        let voice = &kit.voices[drum.index()];
        if voice.sample.is_empty() {
            // A silent voice writes no file — but it may still be *named*, and one case needs that: the copy an import makes of
            // a kit whose KICK.wav was broken. A document that names the file is a kit with a hole a player can fill, and the
            // notice says which file; a document that names nothing is a kit whose voice is simply silent for ever.
            if let Some(file) = &named[drum.index()] {
                voices[drum.index()].file = file.clone();
            }
            continue;
        }
        let file = format!("{}.wav", drum.label());
        write_file(
            &folder.join(&file),
            &crate::wav::encode(&voice.sample, rate),
        )?;
        voices[drum.index()].file = file;
    }

    // A file this write did not name is a WAV the document does not reference any more, and the folder should not keep it: the
    // kit would carry a stale sample a player could hear by re-adding the name by hand. Only the names this module writes are
    // ever removed, so nothing else in the folder is touched.
    for drum in Drum::ALL {
        if voices[drum.index()].file.is_empty() {
            let stale = folder.join(format!("{}.wav", drum.label()));
            if stale.is_file() {
                std::fs::remove_file(&stale).map_err(|error| KitError::Write(error.kind()))?;
            }
        }
    }

    let document = KitDocument {
        name: name.to_string(),
        voices,
    };
    let json = serde_json::to_vec_pretty(&document)
        .map_err(|_| KitError::Write(std::io::ErrorKind::InvalidData))?;
    write_file(&folder.join(KIT_FILE), &json)
}

/// Read a kit's document, or say which of the two ways it is not there.
fn read_document(folder: &Path) -> Result<KitDocument, KitError> {
    match std::fs::read(folder.join(KIT_FILE)) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| KitError::UnreadableDocument),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(KitError::NoDocument),
        Err(_) => Err(KitError::UnreadableDocument),
    }
}

/// Read the kit in `folder`, decoding every voice at `rate`.
fn read_kit(folder: &Path, name: &str, rate: f32) -> Result<LoadedKit, KitError> {
    let document = read_document(folder)?;
    let mut kit = default_kit();
    // The folder names the kit, not the document: the browser lists folders, so a hand-edited name must not disagree with
    // the list the player picked from.
    kit.name = name.to_string();
    let mut problems = Vec::new();

    for drum in Drum::ALL {
        let entry = &document.voices[drum.index()];
        let voice = &mut kit.voices[drum.index()];
        // Everything a hand-edited document could get wrong is clamped, so an edited kit cannot build a voice the panel has
        // no way to draw and the engine no way to play.
        voice.tune = entry.tune.clamp(-12.0, 12.0);
        voice.attack_ms = entry.attack_ms.clamp(0.0, 10_000.0);
        voice.decay_ms = entry.decay_ms.clamp(0.0, 10_000.0);
        voice.sustain = entry.sustain.clamp(0.0, 1.0);
        voice.release_ms = entry.release_ms.clamp(0.0, 10_000.0);
        voice.filter = FilterKind::ALL[(entry.filter as usize).min(FilterKind::ALL.len() - 1)];
        voice.cutoff = entry.cutoff.clamp(0.0, 1.0);
        voice.resonance = entry.resonance.clamp(0.0, 1.0);
        voice.mute = entry.mute;

        if entry.file.is_empty() {
            continue;
        }
        let Some(file) = song_name(&entry.file) else {
            problems.push(KitError::NotAName {
                voice: drum,
                file: entry.file.clone(),
            });
            continue;
        };
        match std::fs::read(folder.join(&file)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                problems.push(KitError::Missing { voice: drum, file });
            }
            Err(error) => problems.push(KitError::Unreadable {
                voice: drum,
                file,
                kind: error.kind(),
            }),
            // The decoder is handed the rate the engine is running at, never a default: a kit decoded at the wrong rate
            // plays every voice at the wrong pitch (trap 1). Task 8 owns reloading when the rate changes.
            Ok(bytes) => match crate::wav::decode(&bytes, rate) {
                Ok(samples) => voice.sample = samples,
                Err(error) => problems.push(KitError::Undecodable {
                    voice: drum,
                    file,
                    error,
                }),
            },
        }
    }

    Ok(LoadedKit { kit, problems })
}

/// Write a kit into `<dir>/<kit.name>/`: one WAV per voice that has a sample, beside a `kit.json`.
///
/// A save writes in place — the folder is the kit — so which name a *new* kit takes is decided before this is called, by
/// [`free_kit_name`] or by the panel's own copy rule. A factory kit's name is refused: those are compiled in, and a folder
/// under one of their names would shadow the kit that ships.
///
/// `rate` is the rate the kit's samples are held at: a WAV records its own rate, and a kit written at a rate nobody is
/// running would come back at the wrong pitch.
///
/// Provenance the folder already held is carried over, per voice, when the file it describes is the file being written: the
/// player editing a decay must not erase where the sample came from.
pub fn save_kit(kit: &Kit, dir: &Path, rate: f32) -> Result<PathBuf, KitError> {
    let Some(name) = kit_name(&kit.name) else {
        return Err(KitError::BadName);
    };
    if crate::kits::is_reserved(&name) {
        return Err(KitError::BadName);
    }
    let folder = kit_folder(dir, &name);
    let before = read_document(&folder).ok();
    // What the folder already knew, kept per voice: where a sample came from, and — for a voice that has no sample now — the
    // file the document names, so a save cannot turn a kit with one missing WAV into a kit that has forgotten it ever had one.
    let carried: [Provenance; Drum::ALL.len()] = std::array::from_fn(|slot| {
        let Some(document) = &before else {
            return (String::new(), String::new());
        };
        let entry = &document.voices[slot];
        if entry.file != format!("{}.wav", Drum::ALL[slot].label()) {
            return (String::new(), String::new());
        }
        (entry.source.clone(), entry.credit.clone())
    });
    let named: [Option<String>; Drum::ALL.len()] = std::array::from_fn(|slot| {
        if !kit.voices[slot].sample.is_empty() {
            return None;
        }
        let document = before.as_ref()?;
        let file = song_name(&document.voices[slot].file)?;
        // The name is kept for every file this build cannot **play back**: gone, unreadable, or not a WAV. That is the hole a
        // notice tells the player to fill — `KICK: KICK.wav is missing` or `KICK: KICK.wav - not a RIFF file` — and keeping the
        // name (and the bytes) is what lets them fill it. Only a sample this build *can* decode is one the player deleted from
        // the kit: then the save deletes its file and the name with it, and the folder stops carrying a stale voice.
        let playable =
            crate::wav::decode(&std::fs::read(folder.join(&file)).unwrap_or_default(), rate)
                .is_ok();
        if playable { None } else { Some(file) }
    });
    write_folder(&folder, &name, kit, carried, named, rate)?;
    Ok(folder)
}

/// Read the kit `name` out of `dir`, decoding every voice at the engine's `rate`.
///
/// A voice whose file is gone, unreadable or not a WAV does not stop the kit loading: it arrives silent and is named in
/// `problems`. What the whole kit refuses is having no document at all, or a name that is a path.
pub fn load_kit(dir: &Path, name: &str, rate: f32) -> Result<LoadedKit, KitError> {
    let Some(name) = kit_name(name) else {
        return Err(KitError::BadName);
    };
    read_kit(&kit_folder(dir, &name), &name, rate)
}

/// Copy a kit folder into the library, and answer where it landed and what is wrong with it.
///
/// The copy is decoded and written again rather than copied byte for byte: a kit handed to the player may be 24-bit, at
/// another rate, or with a file that is not a WAV at all, and the library holds kits this build can play. Provenance travels
/// with it.
///
/// A kit with a broken voice **lands anyway**, with that voice silent and the problem named in `problems`: an import is the
/// moment a player is holding a folder from somewhere else, and refusing it whole would leave them nothing to fix. The voice's
/// slot in the document keeps its file name (`KICK.wav`) even though no file was written for it, so the notice says exactly
/// which file to put in the folder — and the kit's next save writes the rest.
pub fn import_kit(from: &Path, name: &str, dir: &Path, rate: f32) -> Result<ImportedKit, KitError> {
    let source = read_kit(from, name, rate)?;
    // A document that names something which is not a file name is the one problem an import cannot make repairable: there is
    // no file the player could put in the folder, and the name cannot be written into the copy's document either. That is a
    // broken document, not a broken sample, and it is refused whole — with the decoder's sentence naming the entry.
    if let Some(problem) = source
        .problems
        .iter()
        .find(|problem| matches!(problem, KitError::NotAName { .. }))
    {
        return Err(problem.clone());
    }
    let wanted = kit_name(name).unwrap_or_else(|| "Imported kit".to_string());
    let landed = free_kit_name(dir, &wanted);

    let document = read_document(from)?;
    let carried: [Provenance; Drum::ALL.len()] = std::array::from_fn(|slot| {
        (
            document.voices[slot].source.clone(),
            document.voices[slot].credit.clone(),
        )
    });
    // A voice that did not decode keeps the file name the source document gave it, so the copy is a kit *with a hole* rather
    // than a kit that never had the voice — and dropping the right WAV into that name is the whole repair.
    let named: [Option<String>; Drum::ALL.len()] = std::array::from_fn(|slot| {
        if !source.kit.voices[slot].sample.is_empty() {
            return None;
        }
        song_name(&document.voices[slot].file)
    });
    let folder = kit_folder(dir, &landed);
    write_folder(&folder, &landed, &source.kit, carried, named, rate)?;
    Ok(ImportedKit {
        folder,
        name: landed,
        problems: source.problems,
    })
}

/// Every kit on offer: the factory kits' names first, then the player's folders, each sorted.
///
/// A first run has no `kits/` directory at all — `TAKKA_ACID_DATA_DIR` on a fresh path is how the guide's screenshots are
/// taken — and that is the factory three, not an error. A plain file in the directory is not a kit: the folder with a
/// document in it is.
pub fn kit_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = crate::kits::factory_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    names.sort();
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut player: Vec<String> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().join(KIT_FILE).is_file())
            .filter_map(|entry| kit_name(&entry.file_name().to_string_lossy()))
            .filter(|name| !crate::kits::is_reserved(name))
            .collect();
        player.sort();
        player.dedup();
        names.extend(player);
    }
    names
}

/// Write a kit file through a `.tmp` and a rename, so a crash mid-kit cannot leave half a voice on disk.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), KitError> {
    write_through(path, bytes).map_err(|error| KitError::Write(error.kind()))
}

/// A brand-new song: the starter phrase in the first pattern, every other pattern empty, knobs and tempo at their
/// defaults — so a new song sounds like the panel intends before a hand touches it.
pub fn new_song() -> SavedState {
    let mut bank = PatternBank::new_empty();
    bank.replace(0, acid_core::factory::starter());
    SavedState {
        version: SAVE_VERSION,
        bank,
        ..SavedState::default()
    }
}
/// How long the panel may stay dirty before it is written.
pub const AUTOSAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Debounces saves: the first change saves at once, later ones wait for the interval.
#[derive(Debug)]
pub struct Autosave {
    last_save: Option<std::time::Instant>,
}

impl Default for Autosave {
    fn default() -> Self {
        Self::new()
    }
}

impl Autosave {
    pub fn new() -> Self {
        Self { last_save: None }
    }

    /// True when there are unsaved changes and either nothing has been saved yet or the interval has
    /// passed. Recording the save time here keeps every caller from having to.
    pub fn should_save(&mut self, dirty: bool, now: std::time::Instant) -> bool {
        if !dirty {
            return false;
        }
        let due = match self.last_save {
            None => true,
            Some(last) => now.duration_since(last) >= AUTOSAVE_INTERVAL,
        };
        if due {
            self.last_save = Some(now);
        }
        due
    }
}
