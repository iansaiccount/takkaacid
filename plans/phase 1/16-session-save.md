# Session Save Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the panel an explicit SAVE key and a visible saved/unsaved state, backed by one versioned file that is the only thing the app reads.

**Architecture:** `persist::SavedState` gains a `version` field and stops writing the per-pattern files it never reads. The panel gains a SAVE key in the switches row and a one-word indicator in the top strip; `main.rs` writes on request the way it already writes on exit.

**Tech Stack:** Rust 2024, egui/eframe 0.37, `serde`, `serde_json`, `tempfile` (dev).

**Spec:** this plan — the scope decisions below are the spec.

## Scope decisions

- **One artefact: `state.json`.** `persist::load` only ever reads `state.json`; the `patterns/*.json` files are write-only today, so the writer goes and the files already on disk are left alone (harmless, and deletable by hand).
- **`version: 1` is written first.** A file without the field is read as version 1 — there is nothing to migrate yet — and is rewritten with the field on the next save.
- **SAVE is a single click.** It is not destructive, so unlike CLEAR it needs no arming.
- **Autosave stays**: on exit, and 2 s after a change stops. The button is for the person who does not trust that, not a replacement for it.
- **Indicator**: `SAVED` in dim silk, `UNSAVED` in amber, in the top strip beside the wordmark.
- **Not in this plan**: named song files, a file browser, export/import. Those need a dialog and a naming UI; this plan makes the existing file trustworthy first.

## Global Constraints

- Saving stays atomic: write a `.tmp` beside the target and rename it, so a crash never leaves half a file.
- A corrupt or foreign file is still ignored with one printed line (`load_or_default`), never a panic.
- The audio thread is untouched; `cargo test -p acid-app --test no_alloc` stays green.
- Panel legends are ASCII only; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` stay clean.

## Review Focus

1. **Quit straight after an edit.** The exit save still writes; nothing about the session depends on the button having been pressed.
2. **SAVE twice in a row.** The second write is harmless: same bytes, no growing file, no error.
3. **A file written before this change (no `version`).** It loads with every setting intact and comes back versioned after the next save.
4. **A data directory that cannot be written.** SAVE prints the error on stderr and leaves `dirty` set, so the indicator keeps saying UNSAVED rather than lying.
5. **Save and restart with a MIDI input chosen** (plan 15). The choice survives, along with the bank, knobs, tempo and swing.

---

### Task 1: `version` in the save file

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`SavedState`)
- Test: `rust/crates/acid-app/src/persist.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub const SAVE_VERSION: u32 = 1;`
  - `SavedState::version: u32` — first field, `#[serde(default)]`, so a file without it reads as `0`.
  - `SavedState::upgraded(self) -> Self` — sets `version` to `SAVE_VERSION` when it is `0`; called by `load_or_default`.

- [x] **Step 1: Write the failing tests**

```rust
#[test]
fn a_saved_file_names_its_version_and_an_old_one_upgrades_to_it() {
    let json = serde_json::to_string(&SavedState::default()).unwrap();
    let versioned: SavedState = serde_json::from_str(&json).unwrap();
    assert_eq!(versioned.version, SAVE_VERSION, "a written file carries its version");

    // A file from before the field existed has no `version`: it must load, and be rewritten with one.
    let mut legacy: serde_json::Value = serde_json::from_str(&json).unwrap();
    legacy.as_object_mut().unwrap().remove("version");
    let old: SavedState = serde_json::from_value(legacy).unwrap();
    assert_eq!(old.version, 0);
    assert_eq!(old.upgraded().version, SAVE_VERSION);
}
```

- [x] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app persist`
Expected: FAIL — `no field version`.

- [x] **Step 3: Implement**

`SavedState::default()` writes `version: SAVE_VERSION`; `load_or_default` returns `state.upgraded()` on every path. `shared_to_saved` keeps the current version.

- [x] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test -p acid-app persist`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add rust/crates/acid-app/src/persist.rs
git commit -m "feat(persist): version the save file and upgrade old ones on load"
```

---

### Task 2: one artefact — stop writing the pattern files

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`save`, `PATTERN_DIR`)
- Test: `rust/crates/acid-app/src/persist.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `save(dir, state)` writes exactly one file, `dir/state.json`. `PATTERN_DIR` and the per-pattern loop are deleted.

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn a_save_writes_one_file_and_leaves_no_pattern_directory() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();

    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, vec!["state.json".to_string()]);
}
```

- [x] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app persist`
Expected: FAIL — the listing also holds `patterns`.

- [x] **Step 3: Implement**

Delete the directory creation and the `for (index, pattern) in (0..64)` loop from `save`, and the `PATTERN_DIR` constant with them. `load` is already unchanged (it reads `state.json` only).

- [x] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test -p acid-app persist`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add rust/crates/acid-app/src/persist.rs
git commit -m "refactor(persist): one save file, and no write-only pattern copies"
```

---

### Task 3: the SAVE key and the saved/unsaved indicator

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`PanelState::save_requested`, the switches row, the indicator)
- Modify: `rust/crates/acid-app/src/ui/panel.rs` (`pub fn save_indicator_rect(top: Rect, scale: f32) -> Rect`)
- Modify: `rust/crates/acid-app/src/ui/help.rs` (one line)
- Modify: `rust/crates/acid-app/src/main.rs` (consume `save_requested`)
- Test: `rust/crates/acid-app/src/ui/save.rs` (create: the label helper), `rust/crates/acid-app/tests/save_live.rs` (create)

**Interfaces:**
- Consumes: `transport::transport_button`, `theme::{SILKSCREEN_DIM, LED_ACCENT}`, `persist::save` (via `main`).
- Produces:
  - `PanelState::save_requested: bool` — set by the panel, taken by `main`.
  - `ui::save::state_label(dirty: bool) -> (&'static str, egui::Color32)` — `("UNSAVED", LED_ACCENT)` / `("SAVED", SILKSCREEN_DIM)`.
  - `ui::panel::save_indicator_rect(top, s) -> Rect` — the rect the label is painted in.
  - The switches row gains SAVE: `BANK`, `PATTERN`, `MODE`, `CLEAR`, `SAVE` — five cells here, **six** once the songs amendment below adds `SONGS`.

- [x] **Step 1: Write the failing tests**

```rust
#[test]
fn the_label_says_unsaved_only_while_something_is_dirty() {
    assert_eq!(save::state_label(true).0, "UNSAVED");
    assert_eq!(save::state_label(false).0, "SAVED");
}

#[test]
fn clicking_save_asks_for_a_write_and_touching_a_step_asks_again() {
    // ... harness from tests/clear_live.rs, then:
    click(&ctx, &mut panel, &shared, save_centre(), Modifiers::default());
    assert!(panel.save_requested, "the key must ask for a write");
    // main is what clears it; the panel only ever raises it.
}
```

- [x] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app save`
Expected: FAIL — `no module named save`, `no field save_requested`.

- [x] **Step 3: Implement**

Switches row: `layout::knob_cells(l.switches, 5)`, first three engraved as now, CLEAR stays armed-then-clears, SAVE is a plain `transport_button` that sets `state.save_requested = true`. Paint the indicator at `panel::save_indicator_rect`, using `save::state_label(state.dirty)`. In `main.rs`, at the top of `logic`: `if self.panel.save_requested { self.panel.save_requested = false; self.save(); }` — `save()` already clears `dirty` on success and prints on failure. Add to the help card's NOTES section: `"SAVE: write the session now"`.

- [x] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test --workspace`
Expected: PASS, and the switches row still fits inside the panel at 0.65x (the layout test covers it).

- [x] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): a SAVE key and a saved-or-unsaved indicator"
```

---

## Amendment: named songs (added after the first pass)

**Scope added at the human partner's request:** a song can be **named**, kept, and loaded again — several songs
side by side — and the help card is allowed to be bigger so the panel can say so.

### Task 4: songs on disk

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`SONGS_DIR`, `song_path`, `song_name`, `list_songs`, `save_song`, `load_song`, `apply_saved`, `SavedState::song`)
- Test: `rust/crates/acid-app/tests/persist.rs`

**Interfaces:**
- Consumes: `SavedState` (a song *is* a session file, same format and version).
- Produces:
  - `pub const SONGS_DIR: &str = "songs";`
  - `pub fn song_name(typed: &str) -> Option<String>` — trimmed, at most 48 chars, no path separators or `..`, no leading dot.
  - `pub fn song_path(dir: &Path, name: &str) -> PathBuf`, `pub fn save_song(dir, name, &SavedState) -> Result<()>`,
    `pub fn load_song(dir, name) -> Result<SavedState>` (upgraded like the session file), `pub fn list_songs(dir) -> Vec<String>` (sorted).
  - `pub fn apply_saved(&SavedState, &SharedState, &mut PatternBank)` — pour a saved song into a running app; `saved_to_shared` delegates to it, so a load and a restart cannot disagree about a field.
  - `SavedState::song: Option<String>` — which song the session is working on.
- Tests: `a_song_name_has_to_be_a_name_and_not_a_path`, `songs_round_trip_and_are_listed` (a refused name writes nothing).

### Task 5: the songs card

**Files:**
- Create: `rust/crates/acid-app/src/ui/songs.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`PanelState::{songs, song_request, song_name}`, the SONGS key, the card)
- Modify: `rust/crates/acid-app/src/main.rs` (save/load requests, Escape, session field)
- Test: `rust/crates/acid-app/tests/songs_live.rs`

**Interfaces:**
- Consumes: `persist::{song_name, list_songs}`, `transport::transport_button`.
- Produces: `ui::songs::{Songs { names, name, message }, Request { Save, Load, Close }, paint(ui, panel, scale, &mut Songs) -> Option<Request>}`.
- The switches row becomes six cells: `BANK`, `PATTERN`, `MODE`, `CLEAR`, `SAVE`, `SONGS`.

**Rules the card follows:** it never touches the disk — `main` owns the files. Loading a song first saves the
session, so work that only lived in memory is not thrown away. A refused name leaves `SAVE AS` unlit and raises no
request. Escape closes the card before it reaches the device list.

### Task 6: a bigger help card

**Files:**
- Modify: `rust/crates/acid-app/src/ui/help.rs` (`CARD_INSET`, `card_rect`, tighter line height and padding, three new lines)
- Test: `rust/crates/acid-app/src/ui/help.rs`

The card was already at its 0.65x limit, so `card_rect` insets by 4 design px instead of the panel margin and the
body line height drops from 15 to 13. The three lines it gained: the dim-slide note, `SAVED / UNSAVED sits by the
wordmark`, and `SONGS: name the session, keep it, load one`.

### Task 7: new song and delete (second amendment)

**Files:**
- Modify: `rust/crates/acid-core/src/factory.rs` (`starter()`), `rust/crates/acid-app/src/persist.rs` (`new_song`, `delete_song`), `rust/crates/acid-app/src/ui/songs.rs` (NEW SONG, per-row delete cross), `rust/crates/acid-app/src/main.rs` (New/Delete), `rust/crates/acid-app/src/ui/help.rs` (one line)
- Test: `rust/crates/acid-core/tests/factory.rs`, `rust/crates/acid-app/tests/persist.rs`, `rust/crates/acid-app/tests/songs_live.rs`

**Interfaces:**
- `factory::starter() -> Pattern` — simpler than the launcher: a driving C with an off beat, one octave hop, three
  slides that each land on a sounding step.
- `persist::new_song() -> SavedState` — starter in pattern `A1`, every other pattern empty, knobs, tempo and swing
  at their defaults, no name.
- `persist::delete_song(dir, name) -> Result<()>` — the name is validated first, then the file is removed; a bad or
  missing name is an error.
- `ui::songs::Request::{New, Delete(String)}`; `Songs::delete_armed: Option<String>`.

**Rules:** the delete cross is per row and reads `x`, then `SURE` — a file has no undo, so it takes two clicks, and
loading a row clears a pending delete. `NEW SONG` sits bottom-left, `CLOSE` bottom-right. `main` saves the session
before both New and Load, so the work on screen is never the thing that disappears.

### Task 8: rename (third amendment)

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`rename_song`), `rust/crates/acid-app/src/ui/songs.rs` (`Songs::open`, RENAME), `rust/crates/acid-app/src/ui/mod.rs` (fill `open` when the card opens), `rust/crates/acid-app/src/main.rs` (Rename)
- Test: `rust/crates/acid-app/tests/persist.rs`, `rust/crates/acid-app/tests/songs_live.rs`

**Interfaces:**
- `persist::rename_song(dir, from, to) -> Result<()>` — both names validated (`song_name`), an existing target refused
  so a rename can never overwrite another song, and renaming to the name already shown is a no-op rather than an error.
- `Songs::open: Option<String>` — the song the session is on; `ui::songs::Request::Rename(String)` — the typed name.

**Rules:** RENAME renames the **open** song, so it is dark when nothing is open or when the typed name is already the
current one. A dark key is only a paint, so the click handler is gated on the same condition — a test caught the
version where an unlit RENAME still fired. A successful rename updates `song_name`, the card's list and `open`, and
saves the session so the next restart looks for the new name.

**Not in this plan (still):** a file browser, and renaming a song that is not the open one.
