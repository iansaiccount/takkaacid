# WAVEFORM Selector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the continuous WAVEFORM knob with a stepped selector that can only rest on a wave, names the selected wave on the panel, and absorbs future wave types by adding one enum variant.

**Architecture:** The catalogue and the index mapping move into `acid-core::oscillator` as the single source of truth. `SharedState` and `persist` stop hand-decoding the enum and use it. A new `stepped_knob` sits beside the continuous knob in `ui::knob`, and `draw_panel` drives it from a `usize` index.

**Tech Stack:** Rust 2024, egui/eframe 0.37, `serde`, `arc-swap`; tests run through `cargo test` with headless egui frames.

**Spec:** `docs/superpowers/specs/2026-11-06-waveform-selector-design.md`

## Global Constraints

- The seven-knob row keeps its geometry: no changes to `ui::layout` band fractions or cell counts.
- `Knobs.waveform` in the save file stays a `u8` **index** with today's meaning (`0` saw, `1` square); existing saves must load unchanged.
- The audio thread reads an atomic and never allocates; `cargo test -p acid-app --test no_alloc` must stay green.
- Panel legends are ASCII only (AGENTS.md).
- `cargo clippy --workspace --all-targets -- -D warnings` stays clean; `cargo fmt` is expected to have run.
- The master chain (drive → limiter → volume) is untouched; nothing in this plan changes a sample value.

## Review Focus

- **A save file from a newer build (index ≥ catalogue length).** A reasonable person expects their file to load, not to crash or be rejected; out-of-range indices must clamp to the last wave.
- **A catalogue of one wave.** A future build with a single variant must not divide by zero or draw at a nonsense angle; the control sits at 12 o'clock and clicks are harmless.
- **The ends of the control.** Repeated clicks must wrap; a drag must step and stop at the ends; the index must never leave `0..count`.
- **Clicking while the transport plays.** Switching waves mid-render must not produce NaN, a panic, or a dead stream; a timbral jump is expected.
- **Caption legibility at 0.65× and 1.6× scale.** The name and legend must stay inside their cell and clear of the knob body at both ends of the panel's scale range.

---

### Task 1: The wave catalogue and its index

**Files:**
- Modify: `rust/crates/acid-core/src/oscillator.rs` (the `Waveform` enum, currently lines 9–16)
- Test: `rust/crates/acid-core/tests/waveform.rs` (create)

**Interfaces:**
- Consumes: nothing.
- Produces (all in `impl Waveform`): `pub const ALL: &'static [Waveform]`, `pub fn index(self) -> usize`,
  `pub fn from_index(index: usize) -> Waveform` (clamps), `pub fn name(self) -> &'static str`.

- [ ] **Step 1: Write the failing test**

```rust
use acid_core::oscillator::Waveform;

#[test]
fn every_wave_round_trips_through_its_index() {
    for wave in Waveform::ALL {
        assert_eq!(Waveform::from_index(wave.index()), *wave);
    }
}

#[test]
fn the_catalogue_is_ordered_and_complete() {
    assert_eq!(Waveform::ALL, &[Waveform::Saw, Waveform::Square]);
    for (i, wave) in Waveform::ALL.iter().enumerate() {
        assert_eq!(wave.index(), i);
    }
}

#[test]
fn an_index_from_the_future_clamps_to_the_last_wave() {
    let last = *Waveform::ALL.last().unwrap();
    assert_eq!(Waveform::from_index(Waveform::ALL.len()), last);
    assert_eq!(Waveform::from_index(usize::MAX), last);
}

#[test]
fn names_are_distinct_ascii_and_short() {
    let names: Vec<&str> = Waveform::ALL.iter().map(|w| w.name()).collect();
    assert_eq!(names, vec!["SAW", "SQR"]);
    for name in names {
        assert!(name.is_ascii(), "{name} is not ASCII");
        assert!((1..=4).contains(&name.len()), "{name} does not fit the caption");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p acid-core --test waveform`
Expected: FAIL — no associated const `ALL` on `Waveform`.

- [ ] **Step 3: Implement in `rust/crates/acid-core/src/oscillator.rs`**

`ALL` is the authority and `index` searches it (not `self as usize`, so adding a variant in the middle
cannot silently renumber the others). `from_index` clamps to `ALL.len() - 1`, so a file from a build with
more waves lands on the last wave instead of panicking. `name` is a `match` returning `"SAW"` / `"SQR"`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p acid-core --test waveform`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): waveform catalogue with clamp-safe index mapping"
```

---

### Task 2: Shared state and the save file use the catalogue

**Files:**
- Modify: `rust/crates/acid-app/src/state.rs` (`set_waveform`/`waveform`, currently lines 142–155),
  `rust/crates/acid-app/src/persist.rs` (`saved_to_shared`, `sanitize`)
- Test: `rust/crates/acid-app/tests/waveform_state.rs` (create)

**Interfaces:**
- Consumes: `Waveform::{ALL, index, from_index}` from Task 1.
- Produces: no new names — `SharedState::set_waveform`/`waveform` and `persist::*` keep their signatures;
  `Knobs.waveform` stays a `u8` index.

- [ ] **Step 1: Write the failing test**

```rust
use acid_app::persist::{self, SavedState};
use acid_app::state::SharedState;
use acid_core::oscillator::Waveform;

#[test]
fn the_shared_state_round_trips_every_wave() {
    let s = SharedState::new();
    for wave in Waveform::ALL {
        s.set_waveform(*wave);
        assert_eq!(s.waveform(), *wave);
    }
}

#[test]
fn a_save_from_a_newer_build_loads_the_last_wave() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = SavedState::default();
    saved.knobs.waveform = 9; // a wave this build has never heard of
    persist::save(dir.path(), &saved).unwrap();
    let loaded = persist::load_or_default(dir.path());
    let shared = persist::saved_to_shared(&loaded);
    assert_eq!(shared.waveform(), *Waveform::ALL.last().unwrap());
}

#[test]
fn an_existing_save_with_index_one_is_still_square() {
    let dir = tempfile::tempdir().unwrap();
    let mut saved = SavedState::default();
    saved.knobs.waveform = 1;
    persist::save(dir.path(), &saved).unwrap();
    let shared = persist::saved_to_shared(&persist::load_or_default(dir.path()));
    assert_eq!(shared.waveform(), Waveform::Square);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p acid-app --test waveform_state`
Expected: FAIL — the last-wave case decodes to `Saw` (the current decode is `if raw == 1 { Square } else { Saw }`).

- [ ] **Step 3: Implement**

`SharedState::set_waveform` stores `waveform.index()`; `waveform()` returns
`Waveform::from_index(self.waveform.load(Ordering::Relaxed))`. `saved_to_shared` reads
`Waveform::from_index(usize::from(state.knobs.waveform))`, `shared_to_saved` writes
`Waveform::index(snapshot.waveform) as u8`, and `sanitize` clamps
`state.knobs.waveform` to `(Waveform::ALL.len() - 1) as u8` instead of the hard-coded `1`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p acid-app --test waveform_state`
Expected: PASS — 3 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(app): waveform index decoded through the catalogue"
```

---

### Task 3: The stepped knob widget

**Files:**
- Modify: `rust/crates/acid-app/src/ui/knob.rs` (beside the continuous `knob`)
- Test: `rust/crates/acid-app/src/ui/knob.rs` (`#[cfg(test)] mod tests`), `rust/crates/acid-app/tests/stepped_knob.rs` (create)

**Interfaces:**
- Consumes: `theme`, `position_angle`'s siblings in the same module.
- Produces:
  - `pub fn position_angle(index: usize, count: usize) -> f32` — index clamped into `0..count`;
    `count <= 1` returns `0.0`, otherwise the positions are spread evenly over `-135°..=+135°`.
  - `pub fn caption_band(cell: Rect, diameter: f32) -> Rect` — the strip under the knob body where the
    wave name and the legend are painted, inside `cell`.
  - `pub fn stepped_knob(ui: &mut egui::Ui, id: egui::Id, label: &str, index: &mut usize, count: usize,
    diameter: f32) -> Response` — allocates `diameter × (diameter + caption)` at the Ui's cursor.

- [ ] **Step 1: Write the failing tests**

Pure tests, in `knob.rs`:

```rust
#[test]
fn position_angle_spreads_the_catalogue_over_the_sweep() {
    let lo = -135f32.to_radians();
    let hi = 135f32.to_radians();
    assert!(position_angle(0, 1).abs() < 1e-6, "one wave sits at the top");
    assert!((position_angle(0, 2) - lo).abs() < 1e-5);
    assert!((position_angle(1, 2) - hi).abs() < 1e-5);
    assert!((position_angle(0, 3) - lo).abs() < 1e-5);
    assert!(position_angle(1, 3).abs() < 1e-5);
    assert!((position_angle(2, 3) - hi).abs() < 1e-5);
    // out-of-range indices clamp instead of drawing off the dial
    assert!((position_angle(99, 2) - hi).abs() < 1e-5);
}

#[test]
fn the_caption_stays_inside_the_cell_at_both_scale_extremes() {
    for (cell, scale) in [(700.0, 0.65), (1600.0, 1.6)] {
        let cells = super::super::layout::knob_cells(
            super::super::layout::layout(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(cell, cell * 560.0 / 980.0),
            ))
            .knobs
            .area,
            7,
        );
        let cell_rect = cells[0];
        let diameter = cell_rect.width().min(cell_rect.height() * 0.78);
        let band = caption_band(cell_rect, diameter);
        assert!(cell_rect.contains_rect(band), "caption escapes its cell at {scale}x");
        assert!(band.height() > 0.0);
        assert!(band.top() >= cell_rect.top() + diameter, "caption overlaps the knob body");
    }
}
```

Headless tests, in `tests/stepped_knob.rs`. The widget allocates at the Ui cursor, so the root Ui places
it at `(0,0)`–`(130,158)`: point every event at `(65, 65)`.

```rust
use acid_app::ui::knob::stepped_knob;
use egui::{Pos2, Rect, vec2};

fn window() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

/// Drive the widget through the given frames; returns the index it ended on.
fn drive(index: &mut usize, count: usize, frames: Vec<Vec<egui::Event>>) -> Response { .. }

#[test]
fn a_click_advances_and_wraps() {
    // count 2: 0 -> 1 on the first click, 1 -> 0 on the second
}

#[test]
fn a_drag_steps_once_per_thirty_pixels() {
    // press, then move up 30 px: +1; a further 30 px: +2 in total
}

#[test]
fn a_drag_stops_at_the_ends() {
    // drag down 300 px from index 0 leaves it at 0
}

#[test]
fn a_single_wave_catalogue_ignores_input() {
    // count 1: click and drag both leave the index at 0
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p acid-app stepped`
Expected: FAIL — no `position_angle` / `stepped_knob`.

- [ ] **Step 3: Implement in `rust/crates/acid-app/src/ui/knob.rs`**

Reuse the continuous knob's body painting (well, body, cap ring, track arc) so the two read as one family.
Add a detent tick at every `position_angle(i, count)`; draw the indicator at `position_angle(*index, count)`;
paint the wave name above the legend inside `caption_band`. Input: click → `(*index + 1) % count`; drag →
`(press_index as i32 - (accumulated_dy / 30.0).round() as i32).clamp(0, count-1)`; scroll → one step per notch,
clamped. Every path ends inside `0..count`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd rust && cargo test -p acid-app stepped`
Expected: PASS — 6 passed (2 in the module, 4 in the integration test).

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): stepped knob with detents and a state caption"
```

---

### Task 4: The panel drives the selector

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`PanelState`, `draw_panel`; delete `waveform_from_knob`)
- Test: `rust/crates/acid-app/tests/knob_live.rs` (replace the three waveform tests)

**Interfaces:**
- Consumes: `stepped_knob`, `caption_band`, `Waveform::{ALL, index, from_index, name}`.
- Produces: `PanelState::waveform: usize` — the selector's position, read from `SharedState` every frame.
  `PanelState::knobs` stays `[f32; 7]` so it keeps lining up with `theme::KNOB_LABELS`; index 0 is no
  longer read, because the selector owns position 0. `PanelState::knobs_synced` and
  `waveform_from_knob` are removed.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn clicking_the_waveform_control_selects_the_next_wave() {
    // one click on the WAVEFORM cell of a fresh panel: shared.waveform() == Waveform::Square
}

#[test]
fn the_panel_mirrors_the_selected_wave() {
    // shared.set_waveform(Square), then one frame: panel.waveform == Waveform::index(Square)
}

#[test]
fn switching_waves_while_rendering_stays_finite() {
    // AudioRenderer on the same state, playing; click the selector; keep rendering;
    // every sample finite, and the wave actually changed
}

#[test]
fn the_caption_fits_at_both_scale_extremes() {
    // layout at 700x400 and 1600x900: the WAVEFORM cell's caption_band is inside the cell
}
```

Also delete `the_waveform_knob_is_a_switch_at_the_midpoint` and the two
`dragging_the_waveform_knob_*` tests: they exercise the continuous knob and `waveform_from_knob`, both of
which this task retires.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rust && cargo test -p acid-app knob_live`
Expected: FAIL — `PanelState` has no `waveform` field.

- [ ] **Step 3: Implement in `rust/crates/acid-app/src/ui/mod.rs`**

`PanelState` gains `pub waveform: usize` (default `0`) and loses `knobs_synced` (`knobs[0]` is simply no
longer written). In `draw_panel`, before the knob loop: `state.waveform = Waveform::index(shared.waveform());`
For cell 0 call `stepped_knob(ui…, "WAVEFORM", &mut state.waveform, Waveform::ALL.len(), diameter)` and, when
it returns, `shared.set_waveform(Waveform::from_index(state.waveform))` plus `state.dirty = true` if the
index moved. Cells 1..6 keep using the continuous `knob`. Delete `waveform_from_knob`.

- [ ] **Step 4: Run the tests, the whole suite, clippy, and look at it**

Run: `cd rust && cargo test -p acid-app knob_live`
Expected: PASS — 4 passed.

Run: `cd rust && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all green, no warnings.

Then run the app, capture the panel, and confirm the WAVEFORM cell reads `SAW` above `WAVEFORM`, that
clicking it changes the caption to `SQR`, and that no other cell moved.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): waveform selector on the panel"
```

## Self-Review

- **Spec coverage:** catalogue and mapping → Task 1; shared state and save file, including the
  newer-build clamp → Task 2; the widget with detents, caption and discrete movement → Task 3; the panel,
  the caption and the old knob's removal → Task 4. No section of the spec is without a task.
- **Step scan:** every step names a file, a signature or an exact assertion; nothing says "handle edge
  cases". The widget's drag arithmetic is the one algorithm the signature does not determine, so Task 3
  Step 3 states it in a line rather than a body.
- **Type consistency:** `ALL`/`index`/`from_index`/`name` are used identically in Tasks 2–4;
  `PanelState::waveform` is a `usize` everywhere; `caption_band(cell, diameter)` is the same signature in
  Task 3's tests and Task 4's caption check.
- **Review Focus:** newer-build index → Task 2 `a_save_from_a_newer_build_loads_the_last_wave`;
  single-wave catalogue → Task 3 `a_single_wave_catalogue_ignores_input` and `position_angle(_, 1)`;
  ends of the control → Task 3 `a_click_advances_and_wraps` / `a_drag_stops_at_the_ends`; clicking while
  playing → Task 4 `switching_waves_while_rendering_stays_finite`; caption at scale extremes → Task 3's
  pure test and Task 4's layout test.
- **Proportion:** four tasks, one commit each, no task carrying more than one test cycle.
