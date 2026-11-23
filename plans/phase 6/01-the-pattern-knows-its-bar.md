# Plan 01 — the pattern knows its own bar

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** "the bar" stops being the constant 16 and becomes a question every consumer asks the pattern — `Grid` exists as
a catalogue, `Pattern` holds its own grid and its own slots, `timing` divides a bar instead of assuming sixteenths, and the
clock, the sequencer, the playhead, the step row, the drum grid, the selection, the randomiser and the file all ask. **No
pixel changes**: the only grid any face can reach is still `1/16`.

**Spec:** [`2026-12-01-pattern-grids-design.md`](../../docs/superpowers/specs/2026-12-01-pattern-grids-design.md), sections
*The grid*, *One bar, three resolutions*, *The pattern*, *The song the step is on*, *The save file*.

## What is wrong today

- `pattern::STEPS_PER_PATTERN = 16` is imported by eleven source files and pinned in sixteen test files. Nothing can say
  "this pattern is one bar of twelve" without every one of them being wrong.
- `timing::samples_per_step` computes a **sixteenth** (`sample_rate * 60 / (bpm * 4)`), so a step's length is a constant of
  the module and not a property of the pattern being played.
- `Pattern` is `{ steps: [Step; 16] }` with derived serde. A thirty-two-step pattern has nowhere to live, and the file has
  no field that could say which grid a pattern is on.
- Both faces frame the beat with `index % 4 == 0` — `step_key::step_view` and `drum_panel`'s column framing — which is the
  sixteenth assumption written twice.
- `pattern_swap::swap_now` holds a pattern request until `position + 1 >= 16`, and the count it compares against is the
  module constant rather than either bar in play.

## The design

```rust
// acid-core/src/pattern.rs
pub const PATTERN_SLOTS: usize = 32;   // the longest bar any grid needs

pub enum Grid { Sixteenth, ThirtySecond, EighthTriplet }
impl Grid {
    pub const ALL: [Grid; 3];
    pub fn name(self) -> &'static str;                 // "1/16" | "1/32" | "1/8T"
    pub fn from_name(name: &str) -> Option<Grid>;
    pub const fn steps_per_bar(self) -> usize;         // 16 | 32 | 12
    pub const fn is_beat(self, index: usize) -> bool;  // %4 | %8 | %3
}

pub struct Pattern { pub grid: Grid, pub steps: [Step; PATTERN_SLOTS] }
impl Pattern {
    pub fn len(&self) -> usize;                        // grid.steps_per_bar()
    pub fn empty_in(grid: Grid) -> Pattern;
    pub fn from_steps(steps: &[Step]) -> Pattern;                    // a 1/16 pattern
    pub fn from_steps_in(grid: Grid, steps: &[Step]) -> Pattern;     // any grid
    pub fn with_grid(self, grid: Grid) -> Pattern;                    // rests what falls outside
    pub fn iter(&self) -> impl Iterator<Item = &Step>;                // 0..len
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Step>;
}

// acid-core/src/timing.rs
pub fn samples_per_bar(bpm: f32, sample_rate: f32) -> f64;
pub fn samples_per_step(bpm: f32, sample_rate: f32, steps_per_bar: usize) -> f64;
pub fn step_duration_samples(step_index: usize, bpm: f32, sample_rate: f32, swing: f32, steps_per_bar: usize) -> f64;

// acid-core/src/clock.rs
impl Clock {
    pub fn set_steps_per_bar(&mut self, steps_per_bar: usize);  // rescales the step in flight
    pub fn steps_per_bar(&self) -> usize;
}

// acid-core/src/sequencer.rs
pub fn sounding_step(position: usize, reversed: bool, steps: usize) -> usize;

// acid-app/src/ui/pattern_swap.rs
pub fn swap_now(playing: bool, position: usize, last_position: usize, steps: usize) -> bool;

// acid-app/src/ui/layout.rs
pub fn step_cells(area: Rect, count: usize) -> Vec<Rect>;
```

**The invariant:** every slot at or above `len()` is a rest, always. `from_steps_in` rests them, `with_grid` rests the
ones a narrower bar drops, and the deserializer rests everything the file did not name. Every constructor is tested for it.

**The file:** `Pattern`'s serde becomes hand-written through a shadow — `{ "grid": "1/32", "steps": [ …len() entries… ] }`.
`SAVE_VERSION` **13**; the clipboard envelope **2**.

**The bridge:** `STEPS_PER_PATTERN` survives as `pub const STEPS_PER_PATTERN: usize = Grid::Sixteenth.steps_per_bar();` —
derived, not hand-kept — from Task 3 until Task 6 removes its last consumer, so every commit in between compiles and every
gate stays green. It must not gain a new use.

## Tests that make it safe

- `Grid`: the catalogue's order, its captions, its counts, its beats, and an unknown caption reading as `1/16`.
- `timing`: one bar is the same number of samples at every grid, and swing keeps it the same at every grid.
- `Pattern`: `len()` follows the grid; every constructor leaves the slate above `len()` resting; `with_grid` keeps the
  steps the new bar can hold and rests the rest; serde writes the steps that play and reads a version-12 file as `1/16`.
- `Sequencer`: `position` wraps at the pattern's own count; `sounding_step` mirrors within it; a bar line arrives after one
  bar at every grid; a request held through a thirty-two-step bar is not released at its sixteenth.
- `Clock`: a grid change rescales the step in flight and never jumps backwards.
- The panel: the step row has `len()` keys, the selection wraps at `len()`, the drum grid has `len()` columns, and the
  beat framing follows the grid.
- `persist`/`clipboard`: the two version pins and the round trips above.

## Tasks

### Task 1 — `Grid`, a catalogue of three bars

**Files:**
- Modify: `rust/crates/acid-core/src/pattern.rs`
- Test: `rust/crates/acid-core/tests/pattern.rs`

**Interfaces:**
- Produces: `Grid` with `ALL`, `name`, `from_name`, `steps_per_bar`, `is_beat`; `Serialize`/`Deserialize` by caption
  (the crate's `serde` feature is on by default).

- [ ] **Step 1: Write the failing catalogue test** in `acid-core/tests/pattern.rs`

```rust
#[test]
fn the_grid_catalogue_names_one_bar_each() {
    assert_eq!(
        Grid::ALL,
        [Grid::Sixteenth, Grid::ThirtySecond, Grid::EighthTriplet]
    );
    for grid in Grid::ALL {
        assert_eq!(Grid::from_name(grid.name()), Some(grid), "a caption round-trips");
    }
    assert_eq!(Grid::Sixteenth.name(), "1/16");
    assert_eq!(Grid::ThirtySecond.name(), "1/32");
    assert_eq!(Grid::EighthTriplet.name(), "1/8T");
    assert_eq!(Grid::from_name("1/64"), None, "a caption this build has never had");
    assert_eq!(Grid::Sixteenth.steps_per_bar(), 16);
    assert_eq!(Grid::ThirtySecond.steps_per_bar(), 32);
    assert_eq!(Grid::EighthTriplet.steps_per_bar(), 12);
}

#[test]
fn a_beat_is_every_fourth_eighth_or_third_step() {
    for (grid, beats) in [
        (Grid::Sixteenth, [0, 4, 8, 12]),
        (Grid::ThirtySecond, [0, 8, 16, 24]),
        (Grid::EighthTriplet, [0, 3, 6, 9]),
    ] {
        let found: Vec<usize> = (0..grid.steps_per_bar()).filter(|i| grid.is_beat(*i)).collect();
        assert_eq!(found, beats, "{} frames its beats", grid.name());
    }
}
```

- [ ] **Step 2: Run it and watch it fail**

Run: `cd rust && cargo test -p acid-core --test pattern`.
Expected: FAIL — `cannot find type Grid in this scope`.

- [ ] **Step 3: Implement `Grid`** in `pattern.rs`, above `PATTERN_SLOTS`. `steps_per_bar` and `is_beat` are `const fn`;
`name` is a full `match` with no `_` arm; `from_name` is a full `match` on the captions plus `_ => None` (open input, not a
catalogue).

- [ ] **Step 4: Run the tests and make sure they pass**

Run: `cd rust && cargo test -p acid-core --test pattern`.
Expected: PASS.

- [ ] **Step 5: Write the failing serde test**

```rust
#[test]
fn an_unknown_grid_caption_reads_as_sixteenths() {
    assert_eq!(serde_json::from_str::<Grid>("\"1/32\"").unwrap(), Grid::ThirtySecond);
    // A word this build does not know must not cost the player their song: it reads as the grid every
    // version-12 file is on.
    assert_eq!(serde_json::from_str::<Grid>("\"1/64\"").unwrap(), Grid::Sixteenth);
    assert_eq!(serde_json::to_string(&Grid::EighthTriplet).unwrap(), "\"1/8T\"");
}
```

- [ ] **Step 6: Run it, expect FAIL** (`Grid` does not implement `Deserialize`).

- [ ] **Step 7: Implement the two serde impls** by hand, behind `#[cfg(feature = "serde")]`: `Serialize` writes
`self.name()`; `Deserialize` reads a `String` and answers `from_name(&s).unwrap_or(Grid::Sixteenth)`.

- [ ] **Step 8: Gates and commit**

```bash
cd rust && cargo test -p acid-core && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(grid): a catalogue of three bars"
```

### Task 2 — a bar, divided instead of a sixteenth

**Files:**
- Modify: `rust/crates/acid-core/src/timing.rs`, `rust/crates/acid-core/src/clock.rs`
- Test: `rust/crates/acid-core/tests/timing.rs`, `rust/crates/acid-core/tests/clock.rs`
- Consumers to update in the same commit: `rust/crates/acid-core/tests/reverse_and_transpose.rs` (its
  `timing::step_duration_samples` calls)

**Interfaces:**
- Consumes: `Grid::steps_per_bar` (Task 1).
- Produces: `samples_per_bar`, `samples_per_step(bpm, sample_rate, steps_per_bar)`,
`step_duration_samples(step_index, bpm, sample_rate, swing, steps_per_bar)`; `Clock::set_steps_per_bar`,
`Clock::steps_per_bar`.

- [ ] **Step 1: Write the failing timing tests**

```rust
#[test]
fn a_bar_is_the_same_length_at_every_grid() {
    let bar = samples_per_bar(137.0, 44_100.0);
    assert!((bar - 44_100.0 * 240.0 / 137.0).abs() < 1e-6, "four beats at 137");
    for grid in Grid::ALL {
        let steps = grid.steps_per_bar();
        let stepped = samples_per_step(137.0, 44_100.0, steps) * steps as f64;
        assert!((stepped - bar).abs() < 1e-6, "{} does not fill a bar", grid.name());
    }
}

#[test]
fn swing_keeps_a_bar_of_each_grid_the_same_length() {
    for grid in Grid::ALL {
        for swing in [0.0, 0.25, 1.0] {
            let steps = grid.steps_per_bar();
            let total: f64 = (0..steps)
                .map(|i| step_duration_samples(i, 137.0, 44_100.0, swing, steps))
                .sum();
            assert!(
                (total - samples_per_bar(137.0, 44_100.0)).abs() < 1e-6,
                "{} at swing {swing} changed the bar",
                grid.name()
            );
        }
    }
}
```

- [ ] **Step 2: Run it, expect FAIL** — `samples_per_bar` not found.

- [ ] **Step 3: Implement the three timing functions.** `samples_per_bar = sample_rate as f64 * 60.0 /
(clamp_bpm(bpm) as f64 * 4.0)`; `samples_per_step = samples_per_bar / steps_per_bar as f64`; `step_duration_samples` keeps
its even/odd swing factors on top of `samples_per_step`. No `16` anywhere in the module.

- [ ] **Step 4: Run the timing tests, expect PASS.**

- [ ] **Step 5: Write the failing clock tests** in `acid-core/tests/clock.rs`

```rust
#[test]
fn a_grid_change_rescales_the_step_in_flight() {
    let mut clock = Clock::new(48_000.0);
    assert_eq!(clock.steps_per_bar(), 16, "a clock is a sixteenth clock until told");
    let sixteenth = clock.samples_until_next_step();
    clock.advance_by((sixteenth / 2.0) as usize);          // half a step gone
    let left = clock.samples_until_next_step();
    clock.set_steps_per_bar(32);
    let after = clock.samples_until_next_step();
    assert!((after / left - 0.5).abs() < 1e-3, "half a step stays half a step");
    assert!((after - 0.25 * sixteenth).abs() < 2.0, "a 32nd is half a 16th");
}

#[test]
fn setting_the_same_grid_changes_nothing() {
    let mut clock = Clock::new(48_000.0);
    clock.advance_by(100);
    let before = clock.samples_until_next_step();
    clock.set_steps_per_bar(16);
    assert_eq!(clock.samples_until_next_step(), before);
}
```

- [ ] **Step 6: Run them, expect FAIL** (`steps_per_bar` not found).

- [ ] **Step 7: Implement the clock's field and setter.** `steps_per_bar` starts at `Grid::Sixteenth.steps_per_bar()`;
`set_steps_per_bar` returns early when the value is unchanged and otherwise rescales `remaining` exactly as
`rescale` does for tempo: `remaining * new / old`. `restart` and `advance_by` pass `self.steps_per_bar` into
`step_duration_samples`.

- [ ] **Step 8: Run every `acid-core` test; fix the direct `timing::` callers** in `tests/reverse_and_transpose.rs` by
passing `Grid::Sixteenth.steps_per_bar()`.

- [ ] **Step 9: Gates and commit** — as Task 1, with `-m "feat(timing): a bar divided three ways"`.

### Task 3 — the pattern's own slots and its own grid

**Files:**
- Modify: `rust/crates/acid-core/src/pattern.rs`
- Test: `rust/crates/acid-core/tests/pattern.rs`

**Interfaces:**
- Consumes: `Grid` (Task 1).
- Produces: `PATTERN_SLOTS`, `Pattern { grid, steps }`, `len`, `empty_in`, `from_steps(&[Step])`, `from_steps_in`,
`with_grid`, `iter`, `iter_mut`; `Pattern`'s hand-written serde.

- [ ] **Step 1: Write the failing slot tests**

```rust
#[test]
fn a_pattern_is_as_long_as_its_grid() {
    assert_eq!(Pattern::empty().len(), 16, "a fresh pattern is sixteenths");
    assert_eq!(Pattern::empty_in(Grid::EighthTriplet).len(), 12);
    assert_eq!(Pattern::empty_in(Grid::ThirtySecond).len(), 32);
    assert_eq!(Pattern::empty().iter().count(), 16);
}

#[test]
fn a_narrower_grid_rests_the_steps_that_fall_outside() {
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    steps[0] = Step::note(Pitch::A, 1);
    steps[20] = Step::note(Pitch::C, 2);      // beyond a 12-step bar
    let wide = Pattern::from_steps_in(Grid::ThirtySecond, &steps);
    assert_eq!(wide.len(), 32);
    assert!(wide.steps[20].on, "the long bar holds it");

    let narrow = wide.with_grid(Grid::EighthTriplet);
    assert_eq!(narrow.len(), 12);
    assert!(narrow.steps[0].on, "the steps inside the bar are untouched");
    assert!(!narrow.steps[20].on, "the steps outside it are rested");
}

#[test]
fn every_slot_above_len_is_a_rest() {
    let mut steps = [Step::note(Pitch::G, 0); PATTERN_SLOTS];
    for grid in Grid::ALL {
        for pattern in [
            Pattern::empty_in(grid),
            Pattern::from_steps_in(grid, &steps),
            Pattern::from_steps(&steps).with_grid(grid),
        ] {
            for (index, step) in pattern.steps.iter().enumerate().skip(pattern.len()) {
                assert_eq!(*step, Step::rest(), "{} slot {index} holds sound", grid.name());
            }
        }
    }
}
```

- [ ] **Step 2: Run them, expect FAIL.**

- [ ] **Step 3: Implement the type.** `PATTERN_SLOTS = 32`; `grid` and `steps` public; `Default` is `empty()` (a `1/16`
rest bar); `from_steps` is `from_steps_in(Grid::Sixteenth, steps)`; `from_steps_in` copies `min(steps.len(), grid
.steps_per_bar())` and rests the rest; `with_grid` sets the grid and rests every slot at or above the new `len()`;
`iter`/`iter_mut` are `self.steps[..self.len()].iter()`.
Add `#[expect(clippy::len_without_is_empty, reason = "a bar always has steps")]` to `len` — the workspace gate is
`-D warnings` and a public `len` without `is_empty` fails it.

- [ ] **Step 4: Run the slot tests, expect PASS.**

- [ ] **Step 5: Write the failing serde tests**

```rust
#[test]
fn the_file_writes_the_steps_that_play() {
    let sixteenth = Pattern::from_steps(&[Step::note(Pitch::A, 1); 16]);
    let json: serde_json::Value = serde_json::to_value(sixteenth).unwrap();
    assert_eq!(json["grid"], "1/16");
    assert_eq!(json["steps"].as_array().unwrap().len(), 16);

    let wide = Pattern::empty_in(Grid::ThirtySecond);
    let json: serde_json::Value = serde_json::to_value(wide).unwrap();
    assert_eq!(json["grid"], "1/32");
    assert_eq!(json["steps"].as_array().unwrap().len(), 32);
}

#[test]
fn a_version_twelve_pattern_reads_as_sixteenths() {
    // What a version-12 build wrote: sixteen steps and no grid at all.
    let steps: Vec<serde_json::Value> = (0..16)
        .map(|i| if i == 0 { serde_json::json!({"on": true, "pitch": "A", "octave_offset": 1,
            "accent": false, "slide": false, "velocity": 127, "hits": 0, "accents": 0}) }
            else { serde_json::json!({"on": false, "pitch": "C", "octave_offset": 0,
            "accent": false, "slide": false, "velocity": 127, "hits": 0, "accents": 0}) })
        .collect();
    let text = serde_json::json!({ "steps": steps }).to_string();
    let pattern: Pattern = serde_json::from_str(&text).unwrap();
    assert_eq!(pattern.grid, Grid::Sixteenth);
    assert_eq!(pattern.len(), 16);
    assert!(pattern.steps[0].on, "the sixteen steps came through");
}

#[test]
fn a_short_list_pads_and_a_long_one_truncates() {
    let short = serde_json::json!({ "grid": "1/32", "steps": [] }).to_string();
    let pattern: Pattern = serde_json::from_str(&short).unwrap();
    assert_eq!(pattern.len(), 32);
    assert!(pattern.steps.iter().all(|step| *step == Step::rest()), "padded with rests");

    let mut steps = vec![serde_json::json!({"on": true, "pitch": "A", "octave_offset": 1,
        "accent": false, "slide": false, "velocity": 127, "hits": 0, "accents": 0}); 64];
    steps.truncate(64);
    let long = serde_json::json!({ "grid": "1/16", "steps": steps }).to_string();
    let pattern: Pattern = serde_json::from_str(&long).unwrap();
    assert_eq!(pattern.len(), 16, "a 1/16 bar reads sixteen of them");
    for (index, step) in pattern.steps.iter().enumerate() {
        assert_eq!(step.on, index < 16, "the rest are rested, not played");
    }
}
```

- [ ] **Step 6: Run them, expect FAIL.**

- [ ] **Step 7: Implement the serde** as a shadow struct — `{ #[serde(default)] grid: Grid, steps: Vec<Step> }` — with
`Serialize` writing `self.steps[..self.len()]` and `Deserialize` padding to `PATTERN_SLOTS` with rests and truncating to
`len()`. Remove the derive from `Pattern`.

- [ ] **Step 8: Add the bridge constant** `pub const STEPS_PER_PATTERN: usize = Grid::Sixteenth.steps_per_bar();` with a
doc comment saying it is deleted when its last consumer moves (Task 6). Do not add a new use of it.

- [ ] **Step 9: Run the whole workspace** — `cargo test --workspace` should pass with the old array size still in the
`acid-app` call sites that use `STEPS_PER_PATTERN`; `Pattern::from_steps` now takes a slice, so fix those call sites.

- [ ] **Step 10: Gates and commit** — `-m "feat(pattern): a bar that knows its own grid"`.

### Task 4 — `acid-core` asks the pattern

**Files:**
- Modify: `rust/crates/acid-core/src/sequencer.rs`, `rust/crates/acid-core/src/factory.rs`
- Test: `rust/crates/acid-core/tests/sequencer.rs`, `rust/crates/acid-core/tests/factory.rs`,
  `rust/crates/acid-core/tests/reverse_and_transpose.rs`

**Interfaces:**
- Consumes: `Pattern::len`, `Pattern::grid` (Task 3); `Clock::set_steps_per_bar` (Task 2).
- Produces: `sounding_step(position, reversed, steps)`; `Sequencer::position` published as `0..pattern.len()`.

- [ ] **Step 1: Write the failing sequencer tests**

```rust
#[test]
fn a_twelve_step_pattern_wraps_at_twelve() {
    let pattern = Pattern::from_steps_in(Grid::EighthTriplet, &[Step::note(Pitch::A, 1); 12]);
    let mut sequencer = Sequencer::new(48_000.0);
    sequencer.set_pattern(Arc::new(pattern));
    sequencer.start();
    let mut performer = Recorder::default();
    let bar = timing::samples_per_bar(sequencer.clock().bpm(), 48_000.0) as usize;
    sequencer.render(&mut performer, &mut vec![0.0; bar + 2]);
    assert_eq!(performer.steps.len(), 12, "a bar of eighth-triplets is twelve steps");
    assert_eq!(sequencer.position(), 1, "and it has wrapped round to the second");
}

#[test]
fn a_pattern_request_waits_for_the_playing_bars_last_step() {
    // swap_now is the panel's, and is asked with the *playing* bar's count: at 32 steps the sixteenth step is
    // halfway through the bar, so a sixteen-step count would fire the swap there.
    assert!(!acid_app::ui::pattern_swap::swap_now(true, 15, 14, 32));
    assert!(acid_app::ui::pattern_swap::swap_now(true, 31, 30, 32));
}
```

The second test lives in `rust/crates/acid-app/tests/` beside `swap_now`'s existing unit test — put it there, in
`pattern_swap.rs`'s own `mod tests`, and keep the first in `acid-core/tests/sequencer.rs`.

- [ ] **Step 2: Run them, expect FAIL** (positions wrap at 16; `swap_now` takes four arguments).

- [ ] **Step 3: Implement.** `Sequencer::set_pattern` calls `self.clock.set_steps_per_bar(pattern.grid.steps_per_bar())`
and then `self.position %= pattern.len()`; `start_at` wraps by `self.pattern.len()`; `render`'s crossing arm wraps by
`self.pattern.len()`; `deliver` indexes `sounding_step(position, self.reverse, self.pattern.len())`; `sounding_step` takes
the count. `factory`'s rotation is `% pattern.len()`.

- [ ] **Step 4: Run the tests, expect PASS.**

- [ ] **Step 5: Add the bar-line test at two grids** to `acid-core/tests/sequencer.rs`: render exactly one bar of a `1/16`
and of a `1/8T` pattern, and assert each publishes exactly one `bar_line` and at the same sample offset — the claim the
phase rests on.

- [ ] **Step 6: Gates and commit** — `-m "feat(sequencer): the bar comes from the pattern"`.

### Task 5 — the panel asks the pattern

**Files:**
- Modify: `rust/crates/acid-app/src/ui/layout.rs`, `rust/crates/acid-app/src/ui/mod.rs`,
  `rust/crates/acid-app/src/ui/step_key.rs`, `rust/crates/acid-app/src/ui/selection.rs`,
  `rust/crates/acid-app/src/ui/drum_panel.rs`
- Test: `rust/crates/acid-app/tests/step_mode_live.rs`, `rust/crates/acid-app/tests/playhead_live.rs`,
  `rust/crates/acid-app/tests/selection_live.rs`, `rust/crates/acid-app/tests/drum_live.rs`,
  `rust/crates/acid-app/src/ui/layout.rs`'s own module

**Interfaces:**
- Consumes: `Pattern::len`, `Grid::is_beat` (Tasks 1, 3).
- Produces: `layout::step_cells(area, count) -> Vec<Rect>`; `step_view(.., grid: Grid)`; `drum_panel::row_cells(l, drum,
  count) -> Vec<Rect>`, `cell_rect(l, drum, step, count)`, `column_rect(l, step, count)`.

- [ ] **Step 1: Write the failing layout test** — `step_cells(area, 12)`, `(area, 16)` and `(area, 32)` each return that
many rects, all inside `area`, in order, and `(area, 0)` is empty.

- [ ] **Step 2: Change `step_cells`' signature, run the test, expect PASS, and fix its callers.**

- [ ] **Step 3: Write the failing live tests** — in `tests/step_mode_live.rs`, a rack whose pattern is
`Pattern::empty_in(Grid::EightTriplet)` drawn through `draw_panel` puts a key at each of twelve cells and none at a
thirteenth (ask `layout::step_cells(l.steps, 12)` for the rects and assert the panel's selection responds at each); in
`tests/playhead_live.rs`, a device whose published position is 7 lights key 7 of twelve, not key 7 of sixteen; in
`tests/drum_live.rs`, a drum device with a `1/32` pattern has thirty-two columns and the selected column is the one the
keyboard walks to.

- [ ] **Step 4: Implement the panel side.** The acid row draws `state.pattern.len()` keys (no `% STEPS_PER_PATTERN`
left); `step_view` takes the `Grid` and derives `beat: grid.is_beat(index)`; the slide look-ahead is
`(i + 1) % pattern.len()`; `selection.rs`'s call sites pass `pattern.len()`; `drum_panel`'s row, cell and column take the
count from the pattern; the drum face's beat framing is `grid.is_beat(step)`.

- [ ] **Step 5: Run the live tests, then the whole workspace.**

- [ ] **Step 6: Gates and commit** — `-m "feat(panel): the row asks the pattern how long it is"`.

### Task 6 — the rest of the app, and the file

**Files:**
- Modify: `rust/crates/acid-app/src/ui/randomise.rs`, `rust/crates/acid-app/src/ui/pattern_swap.rs`,
  `rust/crates/acid-app/src/main.rs` (the `swap_now` call), `rust/crates/acid-app/src/state.rs` (`set_step`'s bound),
  `rust/crates/acid-app/src/persist.rs`, `rust/crates/acid-app/src/clipboard.rs`, `rust/crates/acid-app/src/demo.rs`,
  `rust/crates/acid-core/src/pattern.rs` (delete the bridge)
- Test: `rust/crates/acid-app/tests/persist.rs`, `rust/crates/acid-app/src/clipboard.rs`,
  `rust/crates/acid-app/tests/drum_banks.rs`, `rust/crates/acid-app/src/ui/randomise.rs`

**Interfaces:**
- Consumes: everything above.
- Produces: `SAVE_VERSION = 13`; `clipboard::VERSION = 2`; `swap_now(playing, position, last_position, steps)`.

- [ ] **Step 1: Write the failing version-pin test change** — rename `the_save_version_is_pinned_at_twelve` to
`the_save_version_is_pinned_at_thirteen`, assert `SAVE_VERSION == 13`, and assert the written document still names it.

- [ ] **Step 2: Write the failing pattern-shape tests** in `tests/persist.rs`: save a world whose `A1` is a `1/32`
pattern, read `state.json`, and assert its `steps` has thirty-two entries and its `grid` is `"1/32"`; then delete the
`grid` key from a saved `1/16` pattern and assert the file still loads with sixteen steps (the shape a version-12 build
reads).

- [ ] **Step 3: Bump both versions, run the tests, expect PASS.**

- [ ] **Step 4: Write the failing clipboard test** — a thirty-two-step pattern round-trips through `encode`/`decode` with
its grid; `VERSION` is 2; a `takka-acid 1` header is refused.

- [ ] **Step 5: Implement the remaining consumers.** `randomise` rolls `0..pattern.len()` and keeps the pattern's grid
(`Pattern::from_steps_in(pattern.grid, &steps)`); `swap_now` takes `steps` and `main` passes the *playing* pattern's
`len()`; `state.rs`'s `set_step` rejects `index >= self.pattern().len()`; `demo.rs`'s `line`/`bar` build a
`[Step; PATTERN_SLOTS]` and answer `Pattern::from_steps(...)` (still `1/16` until round 02); delete
`STEPS_PER_PATTERN` and every remaining use of it.

- [ ] **Step 6: Run the whole workspace, clippy and fmt; then the guide's check** (`./docs/guide/check.sh`) — nothing in
the book described the old constant, so it should stay green untouched.

- [ ] **Step 7: Commit** — `-m "feat(save): a pattern's grid travels with it"`.

## Non-goals

- Any face that can reach a grid other than `1/16`. The `GRID` key, the row at twelve and thirty-two, the demos and the
  book are round [`02`](02-the-three-grids.md).
- A pattern shorter than its bar (the spec's Non-goals).
- A controller for the grid.
- Refusing a save file from a newer build.

## Review focus

The failure modes the spec implies and no task's own test names, each pinned in the task that owns the code:

1. **A step edit at index 20 of a `1/32` pattern.** `SharedState::set_step` must accept it and the edit must land on key
   20 — Task 6's bound, tested there.
2. **`REVERSE` on a twelve-step pattern.** The mirror must use the count, or the playhead points at a step that is not
   sounding — Task 4's `sounding_step` test, and Task 5's playhead live test.
3. **A swap from a thirty-two-step bar into a sixteen-step one.** The request must be released on the *playing* bar's last
   step, and the new bar must begin on its first — Task 4's `swap_now` test and Task 6's `set_pattern` wrap.
4. **A bar line at every grid.** A device joining a running rack waits for the rack's bar; if a grid's bar were not a bar,
   this is the first thing to break — Task 4's bar-line test.
5. **A `1/16` pattern written by this build, read by a version-12 build.** The `grid` field must be the only addition —
   Task 6's strip-the-grid test.

## Progress

- **Task 1 — `Grid`, a catalogue of three bars.** `acid_core::pattern::Grid` with `ALL`, `name`/`from_name`,
  `steps_per_bar` (16 / 32 / 12, `const`), `is_beat`, and serde by caption with a `Sixteenth` fallback. Proved by
  `the_grid_catalogue_names_one_bar_each`, `a_beat_is_every_fourth_eighth_or_third_step` and
  `an_unknown_grid_caption_reads_as_sixteenths` in `crates/acid-core/tests/pattern.rs` — each watched fail first (the
  serde impls were written early by mistake and deleted again so the test could fail for its own reason). Workspace suite
  **941/941**, clippy `-D warnings`, `fmt --check` green.
  **Ruling: `is_beat` is not a `const fn`** — the plan wrote it as one. It counts with `is_multiple_of`, and the
  `% n == 0` spelling that would be `const` is the one clippy's `manual_is_multiple_of` rejects under `-D warnings`. Cost
  if wrong: nothing needs it in a const context; a test that did would have to spell the count itself.

- **Task 2 — a bar, divided instead of a sixteenth.** `timing` gains `samples_per_bar`, and `samples_per_step` /
  `step_duration_samples` take `steps_per_bar`. `Clock` holds the count, publishes `steps_per_bar()`, and `set_steps_per_bar`
  rescales the step in flight through the same `rescale` the tempo and swing use. Proved by
  `a_bar_is_the_same_length_at_every_grid`, `swing_keeps_a_bar_of_each_grid_the_same_length`,
  `a_grid_change_rescales_the_step_in_flight` and `setting_the_same_grid_changes_nothing` — all four watched fail first (the
  clock's two after deleting the methods I had written early). Workspace suite **945/945**, clippy `-D warnings`, `fmt
  --check` green.
  **Ruling: five test files call `timing` directly, not the one the plan named** (`clock`, `pause`, `reverse_and_transpose`,
  `acid-app`'s `renderer`, `timing`). Each now passes `Grid::Sixteenth.steps_per_bar()` rather than a literal, so the count
  still comes from the catalogue. Cost if wrong: none.
  **Ruling: `samples_per_bar` is `sr · 60 / bpm · 4`, not the sixteenth's `sr · 60 / (bpm · 4)`.** The first cut kept the
  sixteenth's parentheses, so a bar came out as a sixteenth and every clock test fell over —
  `eight_steps_fit_in_one_second_at_120_bpm` is what caught it.
