# Phase 11 — Polish, ergonomics and release

> Part of [00-main-plan.md](00-main-plan.md). Tasks **52–56**. Depends on: Phase 10.
> **Delivers:** a laptop-keyboard instrument, shortcuts, an in-app help card, a device picker that explains
> itself, and the release checklist that ends the plan.

**Rule for this phase:** every addition must be reachable without reading documentation. If a control needs
explaining, the panel should say it.

---

### Task 52: Play it from the computer keyboard

**Files:**
- Create: `rust/crates/acid-app/src/ui/keys.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod keys;`), `src/state.rs`
  (`octave: AtomicI32` for the computer-keyboard octave, default `0`)
- Test: `rust/crates/acid-app/src/ui/keys.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `pitch::Pitch`.
- Produces:
  - `pub const KEY_MAP: [(egui::Key, Pitch); 13]` — `A W S E D F T G Y H U J K` in `Pitch::ALL` order, so a
    laptop keyboard plays a chromatic scale plus the octave above (the layout most DAWs use).
  - `pub fn pitch_for(key: egui::Key) -> Option<Pitch>` — `None` for keys outside the map.
  - `pub fn octave_key(key: egui::Key) -> Option<i8>` — `Some(-1)` for `Z`, `Some(1)` for `X`.
  - `pub fn key_hz(pitch: Pitch, octave: i32) -> f32` — the pitch at the computer keyboard's octave offset,
    clamped to `-2..=2`.
- Later tasks rely on: `pitch_for`, `key_hz` (Task 53 shortcuts, Task 54 help text).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_home_row_is_a_chromatic_scale() {
    assert_eq!(pitch_for(egui::Key::A), Some(Pitch::C));
    assert_eq!(pitch_for(egui::Key::W), Some(Pitch::CSharp));
    assert_eq!(pitch_for(egui::Key::D), Some(Pitch::D));
    assert_eq!(pitch_for(egui::Key::K), Some(Pitch::CHigh));
    assert_eq!(pitch_for(egui::Key::Q), None);
    assert_eq!(KEY_MAP.len(), 13);
}

#[test]
fn z_and_x_shift_the_octave_and_clamp() {
    assert_eq!(octave_key(egui::Key::Z), Some(-1));
    assert_eq!(octave_key(egui::Key::X), Some(1));
    assert_eq!(octave_key(egui::Key::A), None);
    assert!((key_hz(Pitch::C, 0) - 55.0).abs() < 0.1);
    assert!((key_hz(Pitch::C, 1) - 110.0).abs() < 0.2);
    assert!((key_hz(Pitch::C, 9) - key_hz(Pitch::C, 2)).abs() < 1e-3);
}

#[test]
fn every_mapped_key_is_distinct() {
    let mut keys: Vec<egui::Key> = KEY_MAP.iter().map(|(k, _)| *k).collect();
    keys.sort_by_key(|k| format!("{k:?}"));
    keys.dedup();
    assert_eq!(keys.len(), 13);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app keys`
Expected: FAIL — module `ui::keys` not found.

- [ ] **Step 3: Implement**

`KEY_MAP` pairs the keys with `Pitch::ALL` order (`C, C#, D, D#, E, F, F#, G, G#, A, A#, B, CHigh`) using
`A W S E D F T G Y H U J K` as the keyboard layout (the row pattern most DAWs use). Note the test above
expects `W` → `CSharp` and `K` → `CHigh`; implement exactly that. In `ui/mod.rs`, key presses call
`SharedState::set_step`-free path: write `midi_hz`/`midi_gate`/`midi_accent` directly, so the computer
keyboard and a MIDI keyboard share one route.

- [ ] **Step 4: Run to confirm passing, then play**

Run: `cd rust && cargo test -p acid-app keys`
Expected: PASS — 3 passed.
**Play:** with the loop running, hold `A` and `D` in turn — notes sound while held and release cleanly. `Z`
and `X` move the octave.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): play the voice from the computer keyboard"
```

---

### Task 53: Shortcuts for transport, tempo and patterns

**Files:**
- Create: `rust/crates/acid-app/src/ui/shortcuts.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod shortcuts;`)
- Test: `rust/crates/acid-app/src/ui/shortcuts.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `SharedState`, `timing::{clamp_bpm, MIN_BPM, MAX_BPM}`, `PatternBank`.
- Produces:
  - `pub enum Shortcut { PlayStop, TempoUp, TempoDown, NextPattern, PrevPattern, ToggleHelp }`
  - `pub fn shortcut_for(key: egui::Key, shift: bool) -> Option<Shortcut>` — `Space → PlayStop`,
    `ArrowUp → TempoUp`, `ArrowDown → TempoDown`, `ArrowRight → NextPattern`, `ArrowLeft → PrevPattern`,
    `F1` or `?` (shift+slash) → `ToggleHelp`.
  - `pub fn apply(shortcut: Shortcut, state: &SharedState, bank: &mut acid_core::bank::PatternBank)
    -> bool` — changes something and returns `true`; `NextPattern`/`PrevPattern` wrap within 0–63.
- Later tasks rely on: `shortcut_for`, `apply` (Task 54).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn space_toggles_the_transport() {
    let s = SharedState::new();
    let mut bank = acid_core::bank::PatternBank::factory();
    assert_eq!(shortcut_for(egui::Key::Space, false), Some(Shortcut::PlayStop));
    apply(Shortcut::PlayStop, &s, &mut bank);
    assert!(s.playing.load(std::sync::atomic::Ordering::Relaxed));
    apply(Shortcut::PlayStop, &s, &mut bank);
    assert!(!s.playing.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn arrows_move_tempo_in_whole_bpm_and_clamp() {
    let s = SharedState::new();
    let mut bank = acid_core::bank::PatternBank::factory();
    s.tempo_bpm.set(130.0);
    apply(Shortcut::TempoUp, &s, &mut bank);
    assert_eq!(s.tempo_bpm.get(), 131.0);
    s.tempo_bpm.set(acid_core::timing::MAX_BPM);
    apply(Shortcut::TempoUp, &s, &mut bank);
    assert_eq!(s.tempo_bpm.get(), acid_core::timing::MAX_BPM);
}

#[test]
fn pattern_selection_wraps_at_both_ends() {
    let s = SharedState::new();
    let mut bank = acid_core::bank::PatternBank::factory();
    bank.select(0);
    apply(Shortcut::PrevPattern, &s, &mut bank);
    assert_eq!(bank.current(), 63);
    apply(Shortcut::NextPattern, &s, &mut bank);
    assert_eq!(bank.current(), 0);
}

#[test]
fn help_is_reachable_by_f1_and_by_question_mark() {
    assert_eq!(shortcut_for(egui::Key::F1, false), Some(Shortcut::ToggleHelp));
    assert_eq!(shortcut_for(egui::Key::Questionmark, true), Some(Shortcut::ToggleHelp));
    assert_eq!(shortcut_for(egui::Key::B, false), None);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app shortcuts`
Expected: FAIL — module `ui::shortcuts` not found.

- [ ] **Step 3: Implement**

`apply` matches the variant; pattern changes must also push the new pattern into
`SharedState::set_pattern`, mirroring what the panel's bank/pattern switches do.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app shortcuts`
Expected: PASS — 4 passed.
**Use:** `Space`, arrows and `F1` all work with the mouse untouched.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): keyboard shortcuts for transport, tempo and patterns"
```

---

### Task 54: The help card

**Files:**
- Create: `rust/crates/acid-app/src/ui/help.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod help;`), `src/ui/panel.rs`
  (a small `?` dot in the top strip is the click target)
- Test: `rust/crates/acid-app/src/ui/help.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `midi::cc::CC_MAP`, `ui::keys::KEY_MAP`, `ui::shortcuts`.
- Produces:
  - `pub fn help_sections() -> [(&'static str, &'static [&'static str]); 4]` — sections `Steps`,
    `Keyboard`, `MIDI`, `Notes`, each with 2–5 one-line entries.
  - `pub fn help_line_count() -> usize`
  - `pub fn paint_help(painter: &egui::Painter, panel: egui::Rect, scale: f32)` — a `PANEL_RAISED` card
    with a `SILKSCREEN` frame, `SILKSCREEN` headings, `TEXT`-coloured body at `PANEL` opacity 1.0, sized to
    fit inside `panel`.
- Later tasks rely on: `help_sections` (Task 56 documentation check).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_help_card_names_every_gesture_and_shortcut() {
    let text: String = help_sections()
        .iter()
        .flat_map(|(_, lines)| lines.iter())
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "click", "shift", "alt", "drag", "double",
        "SPACE", "RUN", "F1", "Z", "X",
        "MIDI", "clock", "CC",
    ] {
        assert!(text.contains(needle), "help is missing {needle:?}");
    }
    assert!(help_line_count() >= 12);
}

#[test]
fn help_text_is_plain_ascii_so_the_panel_font_can_draw_it() {
    for (_, lines) in help_sections() {
        for line in lines {
            assert!(line.is_ascii(), "non-ascii help line: {line}");
            assert!(line.len() <= 64, "help line too long: {line}");
        }
    }
}

#[test]
fn every_cc_mapping_is_documented() {
    let midi: String = help_sections()[2].1.join("\n");
    for (cc, _) in acid_app::midi::cc::CC_MAP {
        assert!(midi.contains(&cc.to_string()), "CC {cc} is undocumented");
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app help`
Expected: FAIL — module `ui::help` not found.

- [ ] **Step 3: Implement**

Write the four sections as literal string arrays; keep lines ≤ 64 ASCII characters. `paint_help` draws a
card inset `MARGIN` inside the panel, with 12 px left padding and 18 px line spacing scaled by `scale`.
Toggling is a `bool` in `PanelState`, flipped by `Shortcut::ToggleHelp` or a click on the `?` dot.

- [ ] **Step 4: Run to confirm passing, then read it**

Run: `cd rust && cargo test -p acid-app help`
Expected: PASS — 3 passed.
Press `F1` in the app: the card must fit inside the panel at 0.65× and 1.6× scale with no text clipped.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): help card covering gestures, keys and midi"
```

---

### Task 55: Audio device picker and honest status

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (device picker in the top strip; a click opens a painted
  list), `src/audio/mod.rs` (`pub fn list_output_devices() -> Vec<String>`)
- Test: `rust/crates/acid-app/src/audio/status.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `AudioStatus`, `midi::supervisor::status_line`.
- Produces:
  - `pub fn status_text(status: &AudioStatus) -> String` — `"48 kHz · PipeWire"` when running,
    `"starting"` before the first stream, `"failed: <reason>"` when failed; always ASCII, never empty.
  - `pub fn device_label(name: &str, max: usize) -> String` — middle-ellipsised so a long ALSA name cannot
    overflow the strip.
  - `pub fn list_output_devices() -> Vec<String>` — names only, no opening of streams; empty when the host
    reports none.
- Later tasks rely on: `status_text` (Task 56 release check).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn status_text_names_every_state() {
    assert_eq!(
        status_text(&AudioStatus::Running { device: "PipeWire".into(), sample_rate: 48_000 }),
        "48 kHz · PipeWire"
    );
    assert_eq!(status_text(&AudioStatus::Starting), "starting");
    let failed = status_text(&AudioStatus::Failed { reason: "device busy".into() });
    assert!(failed.starts_with("failed:") && failed.contains("device busy"));
}

#[test]
fn long_device_names_are_ellipsised_in_the_middle() {
    let label = device_label("Built-in Audio Analog Stereo Digital Surround", 24);
    assert!(label.len() <= 24, "{label}");
    assert!(label.contains('…') || label.contains("..."), "no ellipsis: {label}");
    assert_eq!(device_label("PipeWire", 24), "PipeWire");
}

#[test]
fn listing_devices_never_panics_or_hangs() {
    let devices = list_output_devices();
    assert!(devices.iter().all(|d| !d.trim().is_empty()));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app status`
Expected: FAIL — no `audio::status` module / no `status_text`.

- [ ] **Step 3: Implement**

`status_text` formats the sample rate as `sr / 1000` kHz with no decimals (`48`, `44`). `device_label`
keeps the first and last thirds with an ellipsis between. The picker lists devices from `cpal`'s
`output_devices()`; selecting one stores the name in `SharedState.pending_device` and the supervisor
reopens on the UI thread's next tick (never on the audio thread).

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app status`
Expected: PASS — 3 passed.
**Use:** open the picker, choose another output — audio moves there within a second; the strip shows the
new rate and name. Stop PipeWire: the strip says failed with a reason and the app stays usable.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): output device picker and audio status line"
```

---

### Task 56: Tooltips with real units, README, and the release check

**Files:**
- Create: `rust/README.md`
- Modify: `rust/crates/acid-app/src/ui/knob.rs` (tooltip), `src/ui/mod.rs`
- Test: `rust/crates/acid-app/src/ui/knob.rs` (`#[cfg(test)] mod tests`), plus a documented manual check

**Interfaces:**
- Consumes: `param::{cutoff_hz, decay_seconds, tuning_semitones, master_volume}`, `timing::samples_per_step`.
- Produces:
  - `pub fn knob_tooltip(label: &str, value: f32, sample_rate: f32) -> String` — e.g.
    `"CUT OFF: 812 Hz"`, `"DECAY: 0.31 s"`, `"TUNING: +3.0 st"`, `"VOLUME: -6.0 dB"`,
    `"RESONANCE: 62 %"`; ASCII only, never more than 32 characters.
  - `rust/README.md` — how to build (`cargo build --release`), run (`cargo run -p acid-app`), where state
    lives (`$XDG_DATA_HOME/takka-acid`), the gesture table, the shortcut table, the CC map, and a
    `## Not yet` section pointing at [plans/12-deferred-accuracy.md](../plans/12-deferred-accuracy.md).
- Later tasks rely on: nothing; this closes the plan.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn tooltips_show_physical_units_not_raw_numbers() {
    assert_eq!(knob_tooltip("CUT OFF", 0.5, 48_000.0), "CUT OFF: 800 Hz");
    let decay = knob_tooltip("DECAY", 0.35, 48_000.0);
    assert!(decay.contains(" s"), "{decay}");
    assert_eq!(knob_tooltip("TUNING", 0.75, 48_000.0), "TUNING: +6.0 st");
    assert_eq!(knob_tooltip("VOLUME", 0.5, 48_000.0), "VOLUME: -12.0 dB");
    assert_eq!(knob_tooltip("RESONANCE", 0.62, 48_000.0), "RESONANCE: 62 %");
}

#[test]
fn tooltips_are_short_and_ascii() {
    for label in acid_app::ui::theme::KNOB_LABELS {
        for v in [0.0, 0.33, 0.5, 1.0] {
            let tip = knob_tooltip(label, v, 44_100.0);
            assert!(tip.is_ascii(), "{tip}");
            assert!(tip.len() <= 32, "{tip}");
        }
    }
}

#[test]
fn cutoff_tooltips_follow_the_log_mapping() {
    assert_eq!(knob_tooltip("CUT OFF", 0.0, 48_000.0), "CUT OFF: 80 Hz");
    assert_eq!(knob_tooltip("CUT OFF", 1.0, 48_000.0), "CUT OFF: 8000 Hz");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app knob_tooltip`
Expected: FAIL — no `knob_tooltip`.

- [ ] **Step 3: Implement**

Round hertz to whole numbers, seconds to two decimals, semitones and dB to one decimal with an explicit
sign. Attach via `response.on_hover_text(knob_tooltip(...))` in `knob()`. Write `rust/README.md` with the
tables copied from Phases 08 and 11.

- [ ] **Step 4: Run the release check and record it**

Run: `cd rust && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo build --release`
Expected: all green, no warnings.
Manual, one line each in `plans/notes/phase11-release.md`:

1. Launch with no audio server: window opens, strip says failed, no hang.
2. Launch with no MIDI: strip says `MIDI: none (retrying)`, everything else works.
3. Press RUN: A1 plays and sounds like acid. Press STOP: silence within a frame, no click.
4. Sweep every knob to both ends while running: no clicks, no clipping, no stuck notes.
5. Build a pattern with clicks, shift-click and alt-click; close and reopen: it is still there.
6. Play five minutes with the loop running: `top` shows the app under 5 % CPU and RSS stable.
7. Set tempo to 40 and to 300 while running: timing stays musical, LEDs keep up.
8. Play from the computer keyboard and from MIDI at the same time: no stuck notes.
9. Drag the window from 700×400 to 1920×1080: nothing clips, everything stays legible.
10. Maximum drive, resonance and volume together: output never clips.

- [ ] **Step 5: Commit**

```bash
git add rust/README.md rust/crates/acid-app plans/notes
git commit -m "docs: readme, unit tooltips and release checklist results"
```

---

**Phase 11 done when:** all ten release checks are recorded and passing, `cargo clippy -D warnings` is
clean, and the app can be handed to someone who has never seen it, who can start a loop and edit it without
being told how.
