//! The save file: bank, knobs and transport as JSON, written atomically.
//!
//! `state.json` is the document the app reads back on launch; each non-empty pattern also gets its own
//! `patterns/<id>.json` so a human can read or hand-edit one pattern without the rest. Every write goes to
//! a `.tmp` file and is renamed into place, so a crash mid-save can never leave a half-written save behind.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use acid_core::bank::PatternBank;
use acid_core::engine::{DEFAULT_TUNING_KNOB, DEFAULT_VOLUME_KNOB};
use acid_core::oscillator::Waveform;
use acid_core::timing::DEFAULT_BPM;
use acid_core::voice::{
    DEFAULT_ACCENT_KNOB, DEFAULT_CUTOFF_KNOB, DEFAULT_DECAY_KNOB, DEFAULT_ENV_MOD_KNOB,
    DEFAULT_RESONANCE_KNOB,
};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::knobs::Knob;
use crate::state::SharedState;

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

/// The save-file format this build writes. A file without the field is version 0 and is upgraded on load.
pub const SAVE_VERSION: u32 = 9;

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
    /// The MIDI input the player chose, by port name; `None` means "the first one the system reports".
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
        }
    }
}

impl SavedState {
    /// Bring a file from an older build up to this one.
    ///
    /// Version 2 and older were written when the pitch lattice was rooted at MIDI 33; it is rooted at MIDI 12 now, so the
    /// same note is spelled 21 semitones higher. Shifting every step keeps the music exactly as it sounded — the
    /// alternative would drop every saved pattern by a twelfth and a half.
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

/// Write `bytes` to `path` without ever leaving a partial file there.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("renaming to {}", path.display()))?;
    Ok(())
}

/// Build the shared state the app runs on from a saved document.
pub fn saved_to_shared(state: &SavedState) -> Arc<SharedState> {
    let shared = SharedState::new();
    let mut bank = state.bank.clone();
    apply_saved(state, &shared, &mut bank);
    shared
}

/// Pour a saved document into a running app: the shared state the audio thread reads, and the bank the panel
/// navigates. Loading a song uses this, so a load and a restart cannot disagree about what a field means.
pub fn apply_saved(state: &SavedState, shared: &SharedState, bank: &mut PatternBank) {
    *bank = state.bank.clone();
    shared.tempo_bpm.set(state.bpm);
    shared.swing.set(state.swing);
    // The tone knobs load through the catalogue, so the file's field order cannot disagree with the panel's.
    for (knob, position) in Knob::ALL.iter().zip(state.knobs.positions()) {
        knob.publish(shared, position);
    }
    shared.volume.set(state.knobs.volume);
    // The learned MIDI mapping travels with the song, so a rig saved in one song comes back with it.
    for (slot, cc) in state.cc_map.iter().enumerate().take(shared.cc_map.len()) {
        shared.cc_map[slot].store(i32::from(*cc), std::sync::atomic::Ordering::Relaxed);
    }
    // A song written by an older build may have fewer slots than this one: fill the rest from the factory pairing, so a
    // control the song never knew about cannot inherit whatever the last session taught it.
    for slot in state.cc_map.len()..shared.cc_map.len() {
        shared.cc_map[slot].store(
            i32::from(crate::midi::cc::DEFAULT_CC[slot]),
            std::sync::atomic::Ordering::Relaxed,
        );
    }
    shared.set_pattern(Arc::new(*bank.current_pattern()));
    // A pattern controller reads this to know which bank it is pointing into, and a loaded song moves the bank.
    shared.set_pattern_index(bank.current());
}

/// Capture the running state for saving. The bank comes from `bank`, so callers keep the other 63 patterns.
pub fn shared_to_saved(shared: &SharedState, bank: PatternBank) -> SavedState {
    let snapshot = shared.snapshot();
    let mut bank = bank;
    let current = bank.current();
    bank.replace(current, *shared.pattern());
    SavedState {
        version: SAVE_VERSION,
        bank,
        knobs: Knobs {
            volume: snapshot.volume,
            ..Knobs::from_positions(&Knob::ALL.map(|knob| knob.read(shared)))
        },
        bpm: snapshot.tempo_bpm,
        swing: snapshot.swing,
        // The MIDI choice lives with the app, not with the shared state; `main` fills it in before saving.
        midi_input: None,
        // Which named song is loaded; `main` fills it in, like the MIDI choice.
        song: None,
        // The learned MIDI mapping is shared state, so it is read straight from it.
        cc_map: shared
            .cc_map
            .iter()
            .map(|mapped| {
                mapped
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .clamp(0, 127) as u8
            })
            .collect(),
        // The hover-help switch and the randomise dials live with the panel, not the shared state; `main` fills them in
        // like the MIDI choice.
        tooltips: true,
        density: crate::ui::randomise::DEFAULT_DENSITY,
        scale: 0,
        autosave: false,
    }
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
    // Four faces, not two: the save file stores the wave catalogue index, so clipping it to 1 would load SINE and TRI as
    // SQUARE — the bug the two-state switch left behind.
    state.knobs.waveform = state
        .knobs
        .waveform
        .min((acid_core::oscillator::Waveform::ALL.len() - 1) as u8);
    state.knobs.tuning = state.knobs.tuning.clamp(0.0, 1.0);
    state.knobs.cutoff = state.knobs.cutoff.clamp(0.0, 1.0);
    state.knobs.resonance = state.knobs.resonance.clamp(0.0, 1.0);
    state.knobs.env_mod = state.knobs.env_mod.clamp(0.0, 1.0);
    state.knobs.decay = state.knobs.decay.clamp(0.0, 1.0);
    state.knobs.accent = state.knobs.accent.clamp(0.0, 1.0);
    state.knobs.volume = state.knobs.volume.clamp(0.0, 1.0);
    state.density = state.density.clamp(0.0, 1.0);
    state.scale = state
        .scale
        .min((crate::ui::scale::Scale::ALL.len() - 1) as u8);
    // A map written by a build with more learnable knobs than this one cannot be honoured slot for slot, and guessing
    // which controller belonged to which knob would be worse than the factory pairing.
    if state.cc_map.len() > crate::midi::cc::LEARNABLE.len() {
        state.cc_map = crate::midi::cc::DEFAULT_CC.to_vec();
    }
    for cc in &mut state.cc_map {
        *cc = (*cc).min(127);
    }
    // A hand-edited file can hold fewer patterns than the box has. Every accessor assumes the full catalogue, and `get`
    // is on the audio path, so the bank is repaired before anything reads it.
    state.bank.normalise();
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
    let wanted = song_name(wanted).unwrap_or_else(|| "Untitled".to_string());
    if !song_path(dir, &wanted).exists() {
        return wanted;
    }
    for copy in 2..1000 {
        let suffix = format!(" ({copy})");
        // The suffix must not push the name past what a name may be, or the free name could not be saved at all.
        let room = 48usize.saturating_sub(suffix.chars().count());
        let stem: String = wanted.chars().take(room).collect();
        let candidate = format!("{stem}{suffix}");
        if !song_path(dir, &candidate).exists() {
            return candidate;
        }
    }
    format!("{wanted} (copy)")
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
