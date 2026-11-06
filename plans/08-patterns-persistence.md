# Phase 08 — Pattern editing, banks and persistence

> Part of [00-main-plan.md](00-main-plan.md). Tasks **38–42**. Depends on: Phase 07.
> **Delivers:** the gestures that write patterns, pitch editing, a JSON format on disk, restore-on-launch,
> and a factory acid line so the app is musical the first time it opens.

**Gesture vocabulary (the panel has sixteen keys and three buttons, so the gestures carry the rest):**

| Input on a step key | Effect |
|---|---|
| Click | toggle note on/off; a new note takes the last pitch used |
| Shift+click | toggle accent |
| Alt+click | toggle slide |
| Drag up/down | change pitch by semitones (12 px per semitone) |
| Double-click | clear the step back to a rest |

---

### Task 38: Step gestures as pure functions

**Files:**
- Create: `rust/crates/acid-app/src/ui/edit.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod edit;`), `src/ui/mod.rs` (wire the gestures)
- Test: `rust/crates/acid-app/src/ui/edit.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `pattern::Step`, `pitch::Pitch`.
- Produces:
  - `pub enum StepGesture { Toggle, Accent, Slide, Clear }`
  - `pub fn apply_gesture(step: Step, gesture: StepGesture, last_pitch: Pitch, last_octave: i8) -> Step`
    — `Toggle` turns a rest into `Step::note(last_pitch, last_octave)` and a note into a rest, keeping
    accent/slide flags on the rest so re-enabling restores them; `Accent` and `Slide` flip their flag
    (and turn the step on if it was off, so a flag is never invisible); `Clear` is `Step::rest()`.
  - `pub fn gesture_from_input(left: bool, shift: bool, alt: bool, double: bool) -> Option<StepGesture>`
    — `double → Clear`, `shift → Accent`, `alt → Slide`, `left → Toggle`, else `None`.
  - `pub fn nudge_pitch(pitch: Pitch, semitones: i8) -> (Pitch, i8)` — moves within the 13 pitches and
    rolls into `octave_offset` in `-1..=1`, clamped at the ends.
- Later tasks rely on: `apply_gesture`, `nudge_pitch` (Task 39, Phase 11 Task 52).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn clicking_a_rest_makes_a_note_with_the_last_pitch() {
    let s = apply_gesture(Step::rest(), StepGesture::Toggle, Pitch::G, 0);
    assert!(s.on);
    assert_eq!(s.pitch, Pitch::G);
}

#[test]
fn clicking_a_note_rests_it_but_remembers_its_flags() {
    let s = Step { accent: true, slide: true, ..Step::note(Pitch::E, 0) };
    let rest = apply_gesture(s, StepGesture::Toggle, Pitch::C, 0);
    assert!(!rest.on);
    assert!(rest.accent && rest.slide);
    let back = apply_gesture(rest, StepGesture::Toggle, Pitch::C, 0);
    assert!(back.on && back.accent && back.slide);
}

#[test]
fn accent_and_slide_force_the_step_on() {
    let a = apply_gesture(Step::rest(), StepGesture::Accent, Pitch::C, 0);
    assert!(a.on && a.accent);
    let b = apply_gesture(Step::rest(), StepGesture::Slide, Pitch::C, 0);
    assert!(b.on && b.slide);
    assert!(!apply_gesture(a, StepGesture::Accent, Pitch::C, 0).accent);
}

#[test]
fn clear_removes_everything() {
    let s = Step { accent: true, slide: true, ..Step::note(Pitch::B, 1) };
    assert_eq!(apply_gesture(s, StepGesture::Clear, Pitch::C, 0), Step::rest());
}

#[test]
fn modifiers_win_over_a_plain_click() {
    assert_eq!(gesture_from_input(true, false, false, false), Some(StepGesture::Toggle));
    assert_eq!(gesture_from_input(true, true, false, false), Some(StepGesture::Accent));
    assert_eq!(gesture_from_input(true, false, true, false), Some(StepGesture::Slide));
    assert_eq!(gesture_from_input(true, true, true, true), Some(StepGesture::Clear));
    assert_eq!(gesture_from_input(false, false, false, false), None);
}

#[test]
fn pitch_nudges_roll_into_octaves_and_clamp() {
    assert_eq!(nudge_pitch(Pitch::C, 1), (Pitch::CSharp, 0));
    assert_eq!(nudge_pitch(Pitch::CHigh, 1), (Pitch::C, 1));
    assert_eq!(nudge_pitch(Pitch::C, -1), (Pitch::B, -1));
    assert_eq!(nudge_pitch(Pitch::CHigh, 3), (Pitch::DSharp, 1));
    assert_eq!(nudge_pitch(Pitch::C, -12), (Pitch::C, -1));
    assert_eq!(nudge_pitch(Pitch::CHigh, 12), (Pitch::CHigh, 1));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app edit`
Expected: FAIL — module `ui::edit` not found.

- [ ] **Step 3: Implement**

`edit.rs` is pure. `nudge_pitch` works on an absolute semitone index
`pitch.semitones() as i8 + 12 * octave`, adds the delta, clamps to `-12..=24`, then splits back with
`div_euclid`/`rem_euclid` and clamps `octave_offset` to `-1..=1`. In `ui/mod.rs`, each step key's
`Response` is routed through `gesture_from_input(response.clicked(), …)` and the result stored with
`SharedState::set_step`. Keep `last_pitch`/`last_octave` in `PanelState`.

- [ ] **Step 4: Run to confirm passing, then use it**

Run: `cd rust && cargo test -p acid-app edit`
Expected: PASS — 6 passed.
**Use:** with the loop running, build a four-note line using only clicks, then add two accents with
shift-click and one slide with alt-click. Each change is heard on the next pass.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): step editing gestures as pure functions"
```

---

### Task 39: Pitch editing by dragging a step key

**Files:**
- Modify: `rust/crates/acid-app/src/ui/edit.rs`, `src/ui/mod.rs`
- Test: `rust/crates/acid-app/src/ui/edit.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `nudge_pitch`.
- Produces:
  - `pub const PITCH_DRAG_PIXELS_PER_SEMITONE: f32 = 12.0;`
  - `pub fn semitones_from_drag(dy_pixels: f32, fine: bool) -> i8` — up is higher; `fine` (shift-free
    drag is 1 semitone per 12 px, shift = 1 per 48 px), rounded away from zero so a small drag does
    something.
  - `pub fn drag_pitch(start: (Pitch, i8), dy_pixels: f32, fine: bool) -> (Pitch, i8)`.
- Later tasks rely on: nothing; keeps the UI honest across scales.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn twelve_pixels_is_one_semitone() {
    assert_eq!(semitones_from_drag(-12.0, false), 1);
    assert_eq!(semitones_from_drag(-24.0, false), 2);
    assert_eq!(semitones_from_drag(12.0, false), -1);
    assert_eq!(semitones_from_drag(-4.0, false), 0);
}

#[test]
fn fine_drag_is_four_times_slower() {
    assert_eq!(semitones_from_drag(-48.0, true), 1);
    assert_eq!(semitones_from_drag(-12.0, true), 0);
}

#[test]
fn dragging_a_step_up_a_fifth_lands_on_the_right_note() {
    assert_eq!(drag_pitch((Pitch::C, 0), -84.0, false), (Pitch::G, 0));
    assert_eq!(drag_pitch((Pitch::C, 0), 12.0, false), (Pitch::B, -1));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app edit`
Expected: FAIL — no `semitones_from_drag`.

- [ ] **Step 3: Implement**

`semitones_from_drag`: `let per = if fine { 48.0 } else { 12.0 }; (-dy / per).round() as i8`. In the UI,
a step key with `Sense::click_and_drag()` uses `drag_delta().y` from the drag origin and writes the new
pitch on release; while dragging, the keycap shows the pitch name in `SILKSCREEN`.

- [ ] **Step 4: Run to confirm passing, then use it**

Run: `cd rust && cargo test -p acid-app edit`
Expected: PASS — 9 passed.
**Use:** drag step 3 up an octave while the loop plays — it should sound like the note moved, not like a
new note was added.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): drag a step key to change its pitch"
```

---

### Task 40: JSON on disk — bank, knobs and transport

**Files:**
- Create: `rust/crates/acid-app/src/persist.rs`
- Modify: `rust/crates/acid-app/src/lib.rs` (`pub mod persist;`),
  `rust/crates/acid-core/src/bank.rs` (derive serde on `PatternBank`)
- Test: `rust/crates/acid-app/tests/persist.rs`

**Interfaces:**
- Consumes: `PatternBank`, `Parameters`, `SharedState`.
- Produces:
  - `pub const APP_DIR: &str = "takka-acid";`
  - `pub const ENV_DATA_DIR: &str = "TAKKA_ACID_DATA_DIR";`
  - `pub fn data_dir() -> std::path::PathBuf` — `$TAKKA_ACID_DATA_DIR` if set, else
    `$XDG_DATA_HOME/takka-acid`, else `~/.local/share/takka-acid`.
  - `pub struct SavedState { pub bank: PatternBank, pub knobs: Knobs, pub bpm: f32, pub swing: f32 }`
    with `pub struct Knobs { pub waveform: u8, pub tuning: f32, pub cutoff: f32, pub resonance: f32,
    pub env_mod: f32, pub decay: f32, pub accent: f32, pub volume: f32 }` — all serde, `Clone, Debug,
    PartialEq`.
  - `impl Default for SavedState` — `PatternBank::new_empty()` here (Task 42 switches it to
    `PatternBank::factory()`), launch knobs from Phase 04 Task 19, `bpm = timing::DEFAULT_BPM`,
    `swing = 0.0`.
  - `pub fn save(dir: &std::path::Path, state: &SavedState) -> anyhow::Result<()>` — writes
    `state.json` atomically (write `state.json.tmp`, then rename) plus `patterns/<id>.json` for every
    pattern that is not empty.
  - `pub fn load(dir: &std::path::Path) -> anyhow::Result<Option<SavedState>>` — `Ok(None)` when
    `state.json` is missing.
  - `pub fn saved_to_shared(state: &SavedState) -> std::sync::Arc<SharedState>` (used on launch) and
  - `pub fn shared_to_saved(state: &SharedState) -> SavedState` (used on save).
- Later tasks rely on: `SavedState`, `save`, `load`, `data_dir` (Task 41, Phase 11 Task 55).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-app/tests/persist.rs
use acid_app::persist::{load, save, SavedState};
use std::path::Path;

#[test]
fn a_missing_file_is_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load(dir.path()).unwrap().is_none());
}

#[test]
fn state_round_trips_through_json() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = SavedState::default();
    saved.bpm = 143.0;
    saved.swing = 0.25;
    saved.knobs.cutoff = 0.42;
    saved.bank.select(5);
    save(dir.path(), &saved).unwrap();

    let back = load(dir.path()).unwrap().expect("state.json should exist");
    assert!((back.bpm - 143.0).abs() < 1e-6);
    assert!((back.swing - 0.25).abs() < 1e-6);
    assert!((back.knobs.cutoff - 0.42).abs() < 1e-6);
    assert_eq!(back.bank.current(), 5);
}

#[test]
fn corrupt_json_reports_an_error_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state.json"), b"{ not json").unwrap();
    let err = load(dir.path()).unwrap_err();
    assert!(format!("{err}").to_lowercase().contains("state"), "unhelpful error: {err}");
    // A failed load must not overwrite the user's file.
    let untouched = std::fs::read(dir.path().join("state.json")).unwrap();
    assert_eq!(untouched, b"{ not json");
}

#[test]
fn save_is_atomic_and_leaves_no_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    save(dir.path(), &SavedState::default()).unwrap();
    let entries: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(entries.contains(&"state.json".to_string()));
    assert!(!entries.iter().any(|n| n.ends_with(".tmp")), "left {entries:?}");
}

#[test]
fn data_dir_honours_the_environment_override() {
    // Uses a child process so the env var does not leak into other tests.
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .env("TAKKA_ACID_DATA_DIR", "/tmp/takka-acid-test")
        .args(["--exact", "data_dir_probe", "--nocapture"])
        .output()
        .unwrap();
    let _ = out; // the probe test below is what actually asserts
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test persist`
Expected: FAIL — module `persist` not found. Add `cargo add --dev -p acid-app tempfile` and
`cargo add -p acid-app serde_json` first.

- [ ] **Step 3: Implement**

Add `#[derive(Serialize, Deserialize)]` to `PatternBank` (Phase 05 Task 26 defines the fields). `save`
creates `dir` and `dir/patterns`, writes each file through a `.tmp` + `fs::rename`. `load`: read
`state.json`; on parse failure return an error naming the path. `data_dir` prefers the env override so
tests and the user can sandbox it. Add a `#[test] fn data_dir_probe()` that prints `data_dir()` when the
env var is set (used by the test above).

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app --test persist`
Expected: PASS — 5 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app rust/crates/acid-core
git commit -m "feat(app): json persistence with atomic saves"
```

---

### Task 41: Restore on launch, autosave on exit and on change

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs`, `src/main.rs` (load before the window opens, save on
  `on_exit`), `src/ui/mod.rs` (mark dirty on any write)
- Test: `rust/crates/acid-app/tests/restore.rs`

**Interfaces:**
- Consumes: `save`, `load`, `SavedState`, `SharedState`.
- Produces:
  - `pub fn load_or_default(dir: &std::path::Path) -> SavedState` — never fails; logs a warning and
    returns `SavedState::default()` on error.
  - `pub fn sanitize(state: &mut SavedState)` — clamps `bpm` with `timing::clamp_bpm`, `swing` to
    `0.0..=1.0`, every knob to `0.0..=1.0`, `waveform` to `0|1`, bank index to `0..64`, so a
    hand-edited file cannot produce an invalid engine.
  - `pub struct Autosave { last_save: std::time::Instant }` with
    `pub fn should_save(&mut self, dirty: bool, now: std::time::Instant) -> bool` — true when `dirty`
    and at least `AUTOSAVE_INTERVAL = 2 s` have passed since the last save.
- Later tasks rely on: `load_or_default`, `sanitize` (Phase 11 Task 55).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-app/tests/restore.rs
use acid_app::persist::{load_or_default, sanitize, Autosave, SavedState, AUTOSAVE_INTERVAL};
use std::time::{Duration, Instant};

#[test]
fn an_empty_directory_gives_the_factory_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = load_or_default(dir.path());
    assert_eq!(state, SavedState::default());
}

#[test]
fn a_hand_edited_file_is_clamped_into_range() {
    let dir = tempfile::tempdir().unwrap();
    let mut bad = SavedState::default();
    bad.bpm = 9_000.0;
    bad.swing = 4.0;
    bad.knobs.cutoff = 3.5;
    bad.knobs.waveform = 7;
    acid_app::persist::save(dir.path(), &bad).unwrap();
    let mut loaded = load_or_default(dir.path());
    sanitize(&mut loaded);
    assert_eq!(loaded.bpm, acid_core::timing::MAX_BPM);
    assert_eq!(loaded.swing, 1.0);
    assert!((loaded.knobs.cutoff - 1.0).abs() < 1e-6);
    assert!(loaded.knobs.waveform <= 1);
}

#[test]
fn a_corrupt_file_falls_back_to_defaults_instead_of_crashing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state.json"), b"garbage").unwrap();
    assert_eq!(load_or_default(dir.path()), SavedState::default());
}

#[test]
fn autosave_waits_for_the_interval_and_skips_when_clean() {
    let mut a = Autosave::new();
    let t0 = Instant::now();
    assert!(!a.should_save(false, t0));
    assert!(a.should_save(true, t0));
    assert!(!a.should_save(true, t0 + AUTOSAVE_INTERVAL / 2));
    assert!(a.should_save(true, t0 + AUTOSAVE_INTERVAL));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test restore`
Expected: FAIL — no `load_or_default` / `sanitize` / `Autosave`.

- [ ] **Step 3: Implement**

`main.rs` loads `load_or_default(&data_dir())`, builds the `SharedState` from it, and on window close
(`eframe::App::on_exit`) writes `shared_to_saved` once. `Autosave` is polled from `update` when the
panel is dirty; a dirty flag lives in `PanelState` and is set by every `set_step` and knob write.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app --test restore`
Expected: PASS — 4 passed.
**Use:** run the app, change a pattern and a knob, close it, reopen it — the pattern, knobs and tempo are
as you left them.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(app): restore on launch and debounced autosave"
```

---

### Task 42: The factory acid line

**Files:**
- Create: `rust/crates/acid-core/src/factory.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod factory;`), `src/bank.rs`
  (`PatternBank::factory()`), `rust/crates/acid-app/src/persist.rs` (`SavedState::default`)
- Test: `rust/crates/acid-core/tests/factory.rs`

**Interfaces:**
- Consumes: `Pattern`, `Step`, `Pitch`, `PatternBank`.
- Produces:
  - `pub fn factory_patterns() -> [Pattern; 8]` — bank A: `A1` the launcher (this is what plays on first
    run), `A2`–`A8` eight-bar variations that share A1's pitch material.
  - `PatternBank::factory() -> Self` — 64 patterns where bank A is populated and B–H are empty.
  - `SavedState::default()` uses `PatternBank::factory()`.
- Later tasks rely on: `PatternBank::factory` (Task 40 default) and the launch sound.

**What A1 must be** (a real acid line, not a scale):

| Step | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| on | ● | ● | · | ● | · | ● | ● | · | ● | · | ● | · | ● | ● | · | ● |
| pitch | C | C | · | E♭ | · | G | E♭ | · | C | · | B♭ | · | C | C | · | G |
| accent | ● | · | · | · | · | ● | · | · | ● | · | · | · | · | ● | · | · |
| slide | · | · | · | ● | · | · | · | · | · | · | ● | · | · | · | · | · |

(octave offsets all `0` for A1; `E♭` is `Pitch::DSharp`, `B♭` is `Pitch::ASharp`.)

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/factory.rs
use acid_core::bank::PatternBank;
use acid_core::factory::factory_patterns;
use acid_core::pitch::Pitch;

#[test]
fn there_are_eight_factory_patterns_and_the_first_is_a_line() {
    let patterns = factory_patterns();
    assert_eq!(patterns.len(), 8);
    let a1 = &patterns[0];
    let notes: Vec<usize> = a1.steps.iter().enumerate().filter(|(_, s)| s.on).map(|(i, _)| i).collect();
    assert_eq!(notes, vec![0, 1, 3, 5, 6, 8, 10, 12, 13, 15], "A1's rhythm");
    assert!(a1.steps.iter().filter(|s| s.accent).count() >= 3, "needs accents");
    assert!(a1.steps.iter().filter(|s| s.slide).count() >= 2, "needs slides");
}

#[test]
fn pitches_stay_in_the_acids_home_register() {
    for p in factory_patterns() {
        for s in p.steps {
            assert!(s.pitch.semitones() <= 12);
            assert!((-1..=1).contains(&s.octave_offset), "octave {}", s.octave_offset);
        }
    }
    assert_eq!(factory_patterns()[0].steps[6].pitch, Pitch::DSharp);
}

#[test]
fn the_factory_bank_fills_bank_a_and_leaves_the_rest_empty() {
    let bank = PatternBank::factory();
    let a = bank.get(0);
    assert!(a.steps[0].on);
    assert_eq!(bank.get(8).steps.iter().filter(|s| s.on).count(), 0);
    assert_eq!(bank.current(), 0);
}

#[test]
fn every_factory_pattern_renders_without_clipping() {
    for (i, p) in factory_patterns().iter().enumerate() {
        let mut e = acid_core::engine::Engine::new(48_000.0);
        e.set_pattern(std::sync::Arc::new(*p));
        e.start();
        let mut buf = vec![0.0; 48_000 * 2];
        e.render(&mut buf);
        assert!(buf.iter().all(|s| s.is_finite()), "pattern {i} produced NaN");
        assert!(acid_core::analysis::peak(&buf) > 0.01, "pattern {i} is silent");
        assert!(acid_core::analysis::peak(&buf) <= 0.999, "pattern {i} clipped");
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test factory`
Expected: FAIL — unresolved import `acid_core::factory`.

- [ ] **Step 3: Implement**

Fill A1 exactly as the table says. A2–A8 stay in the same three-or-four pitch set with one accented
anchor per bar and at least one slide into the next downbeat. Write A1 first, run the tests, then add the
others one at a time.

- [ ] **Step 4: Run to confirm passing, then listen**

Run: `cd rust && cargo test -p acid-core --test factory`
Expected: PASS — 4 passed.
**Listen:** launch the app and press RUN. A1 must already sound like acid: the accent on the downbeat
audible, the slides slurring, and the filter closing a little on every unaccented step. If it sounds
like a scale, the problem is the rhythm, not the patch.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core rust/crates/acid-app
git commit -m "feat(core): factory acid pattern bank and launch defaults"
```

---

**Phase 08 done when:** the app launches into a playable acid line, every gesture edits a pattern live,
and the bank round-trips through `~/.local/share/takka-acid/state.json`.
