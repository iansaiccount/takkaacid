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
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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

/// The save-file format this build writes. A file without the field is version 0 and is upgraded on load.
pub const SAVE_VERSION: u32 = 1;

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
        }
    }
}

impl SavedState {
    /// Bring a file from an older build up to this one. Version 0 predates the field: every field it can carry
    /// still means what it meant, so the only work is to stamp it, and the next save writes it out.
    pub fn upgraded(mut self) -> Self {
        if self.version == 0 {
            self.version = SAVE_VERSION;
        }
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
    shared.tempo_bpm.set(state.bpm);
    shared.swing.set(state.swing);
    shared.set_waveform(if state.knobs.waveform == 1 {
        Waveform::Square
    } else {
        Waveform::Saw
    });
    shared.tuning.set(state.knobs.tuning);
    shared.cutoff.set(state.knobs.cutoff);
    shared.resonance.set(state.knobs.resonance);
    shared.env_mod.set(state.knobs.env_mod);
    shared.decay.set(state.knobs.decay);
    shared.accent.set(state.knobs.accent);
    shared.volume.set(state.knobs.volume);
    shared.set_pattern(Arc::new(*state.bank.current_pattern()));
    shared
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
            waveform: if snapshot.waveform == Waveform::Square {
                1
            } else {
                0
            },
            tuning: snapshot.tuning,
            cutoff: snapshot.cutoff,
            resonance: snapshot.resonance,
            env_mod: snapshot.env_mod,
            decay: snapshot.decay,
            accent: snapshot.accent,
            volume: snapshot.volume,
        },
        bpm: snapshot.tempo_bpm,
        swing: snapshot.swing,
        // The MIDI choice lives with the app, not with the shared state; `main` fills it in before saving.
        midi_input: None,
    }
}

/// Never fails: a missing or broken save file means "first run", so the app always opens.
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

/// Clamp everything a hand-edited file could get wrong, so an edited JSON can never build an invalid
/// engine.
pub fn sanitize(state: &mut SavedState) {
    state.bpm = acid_core::timing::clamp_bpm(state.bpm);
    state.swing = state.swing.clamp(0.0, 1.0);
    state.knobs.waveform = state.knobs.waveform.min(1);
    state.knobs.tuning = state.knobs.tuning.clamp(0.0, 1.0);
    state.knobs.cutoff = state.knobs.cutoff.clamp(0.0, 1.0);
    state.knobs.resonance = state.knobs.resonance.clamp(0.0, 1.0);
    state.knobs.env_mod = state.knobs.env_mod.clamp(0.0, 1.0);
    state.knobs.decay = state.knobs.decay.clamp(0.0, 1.0);
    state.knobs.accent = state.knobs.accent.clamp(0.0, 1.0);
    state.knobs.volume = state.knobs.volume.clamp(0.0, 1.0);
    let current = state.bank.current();
    state.bank.select(current);
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
