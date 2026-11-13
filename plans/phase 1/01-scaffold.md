# Phase 01 — Workspace scaffold

> Part of [00-main-plan.md](00-main-plan.md). Tasks **1–4**. Depends on: nothing.
> **Delivers:** a two-crate Cargo workspace, the pitch model, the knob→unit mappings every later
> phase uses, and an app that opens an empty near-black window.

**Why this phase:** every later task needs one place to convert a knob position into hertz or seconds
(`acid-core::param`) and one place to turn a note name into a frequency (`acid-core::pitch`). Getting
those two files right first means no later task invents its own conversion.

**Directory layout created here (and extended later):**

```
rust/
  Cargo.toml                     # [workspace] members = crates/*
  .gitignore                     # /target
  crates/acid-core/
    Cargo.toml                   # no audio/UI deps; serde behind default feature
    src/lib.rs                   # pub mod pitch; pub mod param;
    src/pitch.rs
    src/param.rs
  crates/acid-app/
    Cargo.toml                   # acid-core, cpal, eframe, egui, anyhow, arc-swap, serde_json, midir
    src/main.rs
    src/ui/mod.rs
    src/ui/theme.rs
```

Dependency resolution: add crates with `cargo add`, which records the current versions in the
manifests and `Cargo.lock`. Where an API has moved between versions the implementer adapts the call
site only — never the design, never with a downgrade below the version `cargo add` chose.

---

### Task 1: Create the workspace and both crate skeletons

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/.gitignore`
- Create: `rust/crates/acid-core/Cargo.toml`, `rust/crates/acid-core/src/lib.rs`
- Create: `rust/crates/acid-app/Cargo.toml`, `rust/crates/acid-app/src/main.rs`
- Test: `rust/crates/acid-core/tests/smoke.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: workspace members `acid-core` and `acid-app`; `acid-core` is a library, `acid-app` is a
  binary named `takka-acid` (`[[bin]] name = "takka-acid"`).

- [ ] **Step 1: Write the failing test**

```rust
// rust/crates/acid-core/tests/smoke.rs
#[test]
fn core_crate_is_linkable() {
    assert_eq!(acid_core::crate_name(), "acid-core");
}
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cd rust && cargo test -p acid-core`
Expected: FAIL — `error: could not find 'Cargo.toml'` / package `acid-core` does not exist.

- [ ] **Step 3: Create the workspace files**
- `rust/Cargo.toml`: `[workspace] resolver = "3"`, `members = ["crates/acid-core", "crates/acid-app"]`,
  and `[workspace.package]` with `version = "0.1.0"`, `edition = "2024"`, `license = "MIT"`.
- `rust/.gitignore`: `/target`.
- `acid-core/Cargo.toml`: `name = "acid-core"`, workspace edition; `[features] default = ["serde"]`,
  `serde = ["dep:serde"]`, `serde = { version = "1", features = ["derive"], optional = true }`.
- `acid-core/src/lib.rs`: `pub fn crate_name() -> &'static str { "acid-core" }`.
- `acid-app/Cargo.toml`: `[[bin]] name = "takka-acid"`, `path = "src/main.rs"`; dependency
  `acid-core = { path = "../acid-core" }`; `main.rs`: `fn main() { println!("takka-acid"); }`.

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cd rust && cargo test -p acid-core`
Expected: PASS — 1 passed.

- [ ] **Step 5: Commit**

```bash
git add rust plans
git commit -m "feat(scaffold): two-crate cargo workspace for takka-acid"
```

---

### Task 2: Pitch model and frequency conversion

**Files:**
- Create: `rust/crates/acid-core/src/pitch.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (add `pub mod pitch;`)
- Test: inline `#[cfg(test)] mod tests` in `pitch.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub enum Pitch { C, CSharp, D, DSharp, E, F, FSharp, G, GSharp, A, ASharp, B, CHigh }`
    — `Copy, Clone, PartialEq, Eq, Debug, Default` (default `C`), serde-capable.
  - `impl Pitch { pub fn semitones(self) -> u8 }` — `0..=12`.
  - `impl Pitch { pub const ALL: [Pitch; 13] }` — the order played by the 13 note keys.
  - `pub const ROOT_MIDI: u8 = 33;` (A1, 55 Hz — the 303's home register).
  - `pub fn midi_to_hz(midi: f32) -> f32` — 12-TET, A4 = 69 = 440 Hz.
  - `pub fn step_hz(pitch: Pitch, octave_offset: i8) -> f32` — `ROOT_MIDI + semitones + 12·offset`.
- Later tasks rely on these exact names: Phase 03 Task 11 (tuning), Phase 05 Task 20 (pattern),
  Phase 08 Task 39 (pitch editing).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn semitones_count_up_one_octave() {
    assert_eq!(Pitch::C.semitones(), 0);
    assert_eq!(Pitch::E.semitones(), 4);
    assert_eq!(Pitch::B.semitones(), 11);
    assert_eq!(Pitch::CHigh.semitones(), 12);
    assert_eq!(Pitch::ALL.len(), 13);
}

#[test]
fn concert_a_is_440() {
    assert!((midi_to_hz(69.0) - 440.0).abs() < 0.01);
    assert!((midi_to_hz(81.0) - 880.0).abs() < 0.01);
    assert!((midi_to_hz(57.0) - 220.0).abs() < 0.01);
}

#[test]
fn root_is_the_a_below_middle_c() {
    assert!((step_hz(Pitch::C, 0) - 55.0).abs() < 0.05);
    assert!((step_hz(Pitch::CHigh, 0) - 110.0).abs() < 0.1);
    assert!((step_hz(Pitch::C, 1) - 110.0).abs() < 0.1);
    assert!((step_hz(Pitch::C, -1) - 27.5).abs() < 0.05);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core pitch`
Expected: FAIL — unresolved import / `Pitch` not found.

- [ ] **Step 3: Implement `pitch.rs`**

`midi_to_hz(m) = 440.0 * 2f32.powf((m - 69.0) / 12.0)`. `Pitch::ALL` lists the variants in chromatic
order; `semitones` returns the index. `step_hz` casts offsets so the result stays `f32`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core pitch`
Expected: PASS — 3 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): pitch model and midi-to-hz conversion"
```

---

### Task 3: Knob→unit mappings and parameter smoothing

**Files:**
- Create: `rust/crates/acid-core/src/param.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (add `pub mod param;`)
- Test: inline `#[cfg(test)] mod tests` in `param.rs`

**Interfaces:**
- Consumes: `Pitch` (Task 2) is not needed here.
- Produces (all take a knob position `k: f32`, clamped to `0.0..=1.0` internally):
  - `pub fn tuning_semitones(k: f32) -> f32` — `-12.0..=12.0`, linear.
  - `pub fn cutoff_hz(k: f32) -> f32` — logarithmic `80.0..=8000.0`.
  - `pub fn decay_seconds(k: f32) -> f32` — logarithmic `0.03..=2.0`.
  - `pub fn env_mod(k: f32) -> f32` — `0.0..=1.0`.
  - `pub fn resonance(k: f32) -> f32` — `0.0..=1.0`, `k.powf(1.5)` taper (keeps the first third tame).
  - `pub fn accent_amount(k: f32) -> f32` — `0.0..=1.0`.
  - `pub fn master_volume(k: f32) -> f32` — `0.0..=1.0`, `k.powf(2.0)` (perceptual taper).
  - `pub struct Smooth { value: f32, coeff: f32 }` with `Smooth::new(value: f32, ms: f32, sample_rate: f32)`,
    `fn process(&mut self, target: f32) -> f32` (one-pole), and `fn snap(&mut self, target: f32)`.
- Later tasks rely on these exact names: Phase 04 (filter/envelope), Phase 07 Task 34 (UI mapping),
  Phase 09 Task 45 (volume).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn endpoints_are_exact() {
    assert_eq!(tuning_semitones(0.0), -12.0);
    assert_eq!(tuning_semitones(0.5), 0.0);
    assert_eq!(tuning_semitones(1.0), 12.0);
    assert!((cutoff_hz(0.0) - 80.0).abs() < 0.01);
    assert!((cutoff_hz(1.0) - 8000.0).abs() < 0.01);
    assert!((decay_seconds(0.0) - 0.03).abs() < 1e-6);
    assert!((decay_seconds(1.0) - 2.0).abs() < 1e-6);
}

#[test]
fn cutoff_midpoint_is_geometric() {
    // sqrt(80 * 8000) = 800 Hz
    assert!((cutoff_hz(0.5) - 800.0).abs() < 0.5);
}

#[test]
fn knob_positions_are_clamped() {
    assert_eq!(tuning_semitones(-2.0), -12.0);
    assert!((cutoff_hz(9.0) - 8000.0).abs() < 0.01);
    assert!((master_volume(2.0) - 1.0).abs() < 1e-6);
}

#[test]
fn smoothing_approaches_its_target_without_overshoot() {
    let mut s = Smooth::new(0.0, 20.0, 48_000.0);
    let mut last = 0.0;
    for _ in 0..(48_000 / 2) {
        last = s.process(1.0);
        assert!(last <= 1.0 + 1e-6, "overshoot: {last}");
    }
    assert!(last > 0.99);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core param`
Expected: FAIL — unresolved names / module `param` not found.

- [ ] **Step 3: Implement `param.rs`**

Log mappings as `min * (max / min).powf(k)`. `Smooth` coefficient
`coeff = 1.0 - (-1.0 / (ms * 0.001 * sample_rate)).exp()`, `process` returns
`self.value += (target - self.value) * self.coeff` (`f32`, no `exp` per sample — coefficient is
computed once in `new`).

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core param`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): knob-to-unit mappings and one-pole smoothing"
```

---

### Task 4: App window with the black-and-yellow panel palette

**Files:**
- Create: `rust/crates/acid-app/src/ui/mod.rs`, `rust/crates/acid-app/src/ui/theme.rs`
- Modify: `rust/crates/acid-app/Cargo.toml` (add `eframe`, `egui`), `rust/crates/acid-app/src/main.rs`
- Test: inline `#[cfg(test)] mod tests` in `theme.rs`

**Interfaces:**
- Consumes: nothing from `acid-core` yet.
- Produces:
  - `pub mod theme` exposing the ten `egui::Color32` constants named in the main plan's palette table,
    plus `pub const WINDOW_SIZE: [f32; 2] = [980.0, 560.0]`, `pub fn legend(size: f32) -> egui::FontId`,
    `pub fn luminance(c: egui::Color32) -> f32` (Rec. 709 weighted, `0.0..=1.0`).
  - `acid-app::ui::TakkaApp` implementing `eframe::App` — Phase 06 replaces its body.
- Later tasks rely on: every palette constant, `WINDOW_SIZE`, `luminance`.

- [ ] **Step 1: Write the failing tests**

The test module lives inside `theme.rs`, so refer to the constants bare (`PANEL`, not `theme::PANEL`).

```rust
#[test]
fn panel_is_the_darkest_thing_on_screen() {
    assert!(luminance(theme::PANEL) < 0.02);
    assert!(luminance(theme::PANEL_RAISED) < luminance(theme::SILKSCREEN));
    assert!(luminance(theme::WELL) < luminance(theme::PANEL));
}

#[test]
fn leds_read_as_on_and_off() {
    assert!(luminance(theme::LED_ON) > 0.5);
    assert!(luminance(theme::LED_OFF) < 0.1);
    assert!(luminance(theme::LED_ACCENT) > 0.25);
    assert!(luminance(theme::TEXT) > 0.8);
}

#[test]
fn silkscreen_is_yellow_not_white() {
    let c = theme::SILKSCREEN;
    assert!(c.r() > 200 && c.g() > 150 && c.b() < 80, "expected amber, got {c:?}");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app`
Expected: FAIL — cannot find `ui` / `theme`.

- [ ] **Step 3: Add dependencies and implement the window**
- `cargo add -p acid-app eframe egui anyhow` (record versions in the manifest).
- `main.rs`: `eframe::run_native("takka-acid", NativeOptions { viewport: ViewportBuilder::default()
  .with_inner_size(WINDOW_SIZE).with_resizable(true), ..Default::default() },
  Box::new(|_cc| Ok(Box::new(TakkaApp))))?`. Verified against the eframe in this workspace (0.37): the
  creator returns `Result<Box<dyn App>, …>` and `run_native` returns `eframe::Result`.
- `impl eframe::App for TakkaApp`: the required method is **`fn ui(&mut self, ui: &mut egui::Ui, frame:
  &mut eframe::Frame)`** — there is no `update(ctx, frame)` in this version. Paint the panel background with
  `ui.painter().rect_filled(ui.max_rect(), 0.0, theme::PANEL)` and the `takka-acid` wordmark in
  `SILKSCREEN`, so the window is visibly not default. Add `fn logic(&mut self, ctx, frame)` later when
  per-frame work appears.

- [ ] **Step 4: Run tests, then open the window**

Run: `cd rust && cargo test -p acid-app`
Expected: PASS — 3 passed.
Run: `cd rust && cargo run -p acid-app`
Expected: a 980×560 near-black window with a yellow `takka-acid` wordmark; closing it exits with code 0.
**Listen:** nothing — there is no audio yet.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): eframe window and black-and-yellow panel palette"
```

---

**Phase 01 done when:** `cd rust && cargo test --workspace && cargo clippy --workspace -- -D warnings`
is green and `cargo run -p acid-app` opens the dark window.
