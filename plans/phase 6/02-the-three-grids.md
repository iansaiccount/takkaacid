# Plan 02 — the three grids on the face

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** a player can choose a pattern's grid on the face, see which one it is on, watch the row change shape with it, and
load two demos that put the triplet and the thirty-second grids on the card — and the book says what the face now does.

**Spec:** [`2026-12-01-pattern-grids-design.md`](../../docs/superpowers/specs/2026-12-01-pattern-grids-design.md), sections
*The face*, *The demos*, *The book*. Depends on round [`01`](01-the-pattern-knows-its-bar.md) for `Grid`, `Pattern::len`,
`Pattern::with_grid`, the clock and `layout::step_cells(area, count)`.

## What is wrong today

- No key on the face names a grid, and no cell says which one a pattern is on. The two cells at the end of the transport row
  are labelled `TBC` and refuse with `TBC IS NOT DECIDED YET`.
- The row draws a fixed count of keys, so a `1/32` pattern's back sixteen steps have no key to land on even though round
  01 taught the model to hold them.
- `CLEAR` builds `Pattern::empty()`, which is a sixteenth pattern: pressing it on a triplet bar would shrink the row.
- The demos are five, all on `1/16`: nothing on the card shows what a grid is for.
- The book says "sixteen steps" as a fact about the instrument on a dozen pages.

## The design

**The key and the well.** `transport::Cell::Tbc` becomes `Cell::Grid` and `Cell::Spare` becomes `Cell::GridWell`. The key
prints `GRID` and cycles `Grid::ALL`; the well prints the pattern's caption in the transpose readout's shape, dim at `1/16`
and in the accent colour otherwise — "this pattern is not on the plain grid" at a glance. The press is a pattern edit, not
a transport request:

```rust
// acid-core/src/pattern.rs
impl Grid { pub fn next(self) -> Grid; }                       // cycles ALL, wrapping
impl Pattern { pub fn sounding_steps_beyond(&self, grid: Grid) -> usize; }  // what a press would cost

// acid-app/src/state.rs
impl SharedState { pub fn set_grid(&self, grid: Grid); }       // rcu, like set_step

// acid-app/src/ui/transport.rs — the row's words, beside the Request they answer
pub fn grid_notice(before: usize, after: usize, cleared: usize) -> String;
```

`grid_notice(16, 32, 0)` is `"16 TO 32 STEPS"`; `grid_notice(32, 12, 3)` is `"32 TO 12 STEPS, 3 CLEARED"`.

**The keycap.** Both labels stay at every grid — the note at 12 pt and the corner step number at 8 pt — because the player
ruled that the number is not removed before it has been seen failing. At `1/32` that is 8 pt in a 24 px key: a reading
judgement, not an arithmetic one, so it is a **manual check** on the round's picture, recorded in `## Progress`. The note
gets a measurement test, because an overflowing name is a bug.

**The demos.** `Acid Jazz` — `1/8T`, twelve steps, 140 BPM — and `Fast Acid` — `1/32`, thirty-two steps, 152 BPM —
appended after `funk()`, each with its eight drum patterns on the same grid.

## Tests that make it safe

- `Grid::next` cycles the catalogue and wraps; `sounding_steps_beyond` counts sounding steps past a grid's bar and nothing
  else.
- `grid_notice`'s three shapes, exactly.
- Live: the key cycles the pattern's grid, the well prints it, the session goes dirty, `CLEAR` keeps the grid, the row's
  key count follows, the notice says what was lost.
- Live: the note fits a 24 px key; the keys stay inside the step band at every grid and at the smallest window.
- Manual: the corner step number reads at 8 pt in a 24 px key — checked on the round's picture, not by a test.
- Live: the drum face's columns follow the grid, and the selected column is reachable.
- `tests/drum_banks.rs`: seven demos, distinct, striking, and the two new ones on their grids.
- `./docs/guide/check.sh`: the three captions, `GRID`, seven demos.

## Tasks

### Task 1 — the `GRID` key and the 303's row

**Files:**
- Modify: `rust/crates/acid-core/src/pattern.rs` (`Grid::next`, `sounding_steps_beyond`),
  `rust/crates/acid-app/src/state.rs`, `rust/crates/acid-app/src/ui/transport.rs`,
  `rust/crates/acid-app/src/ui/row_transport.rs`, `rust/crates/acid-app/src/ui/tips.rs`,
  `rust/crates/acid-app/src/ui/step_key.rs` (nothing — see step 10; the keycap keeps both labels)
- Test: `rust/crates/acid-core/tests/pattern.rs`, `rust/crates/acid-app/src/ui/transport.rs`'s module,
  `rust/crates/acid-app/tests/grid_live.rs` (new)

**Interfaces:**
- Consumes: `Grid`, `Pattern::with_grid`, `Pattern::iter` (round 01).
- Produces: `Grid::next`, `Pattern::sounding_steps_beyond`, `SharedState::set_grid`,
`transport::Cell::{Grid, GridWell}`, `tips::Control::Grid`, `transport::grid_notice`.

- [ ] **Step 1: Write the failing model tests**

```rust
#[test]
fn the_grid_key_cycles_the_catalogue() {
    assert_eq!(Grid::Sixteenth.next(), Grid::ThirtySecond);
    assert_eq!(Grid::ThirtySecond.next(), Grid::EighthTriplet);
    assert_eq!(Grid::EighthTriplet.next(), Grid::Sixteenth);
}

#[test]
fn a_press_counts_the_sound_it_would_drop() {
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    steps[0] = Step::note(Pitch::A, 1);
    steps[20] = Step::note(Pitch::C, 2);
    steps[21] = Step::note(Pitch::D, 2);
    steps[22] = Step::rest();                 // a rest is not sound
    let wide = Pattern::from_steps_in(Grid::ThirtySecond, &steps);
    assert_eq!(wide.sounding_steps_beyond(Grid::EighthTriplet), 2);
    assert_eq!(wide.sounding_steps_beyond(Grid::ThirtySecond), 0);
    assert_eq!(wide.with_grid(Grid::Sixteenth).sounding_steps_beyond(Grid::Sixteenth), 0,
        "nothing sounds past a bar it already fits");
}
```

- [ ] **Step 2: Run them, expect FAIL.**

- [ ] **Step 3: Implement** `Grid::next` (a full `match` on `ALL`'s order) and
`Pattern::sounding_steps_beyond(self, grid)` as `self.steps[grid.steps_per_bar()..].iter().filter(|s| s.on).count()`.

- [ ] **Step 4: Write the failing `grid_notice` tests** in `ui/transport.rs`'s module

```rust
#[test]
fn the_notice_reads_as_a_bar_before_and_after() {
    assert_eq!(transport::grid_notice(16, 32, 0), "16 TO 32 STEPS");
    assert_eq!(transport::grid_notice(32, 12, 3), "32 TO 12 STEPS, 3 CLEARED");
    assert_eq!(transport::grid_notice(16, 12, 0), "16 TO 12 STEPS");
}
```

- [ ] **Step 5: Run it, implement `grid_notice`, run it again.**

- [ ] **Step 6: Rename the two cells and add the control's help**

`transport::Cell::Tbc` → `Cell::Grid`, `Cell::Spare` → `Cell::GridWell` (and `ALL`, `index`, and the `cell()` lookups);
`tips::Control::Tbc` → `Control::Grid` with its tip — "Sets this pattern's grid: sixteenths, thirty-seconds or eighth-note
triplets." — and no `TBC` string left anywhere (`grep -rn "TBC" rust/crates`).

- [ ] **Step 7: Write the failing live test** in a new `rust/crates/acid-app/tests/grid_live.rs`: drive `draw_panel`, click
the `GRID` cell with the rack **running and mid-bar** (position 5 of 16), and assert `PanelState.pattern.grid() ==
Grid::ThirtySecond`, `panel.dirty`, that a second click is `EighthTriplet`, that the status line reads `16 TO 32 STEPS`,
and that the pattern the engine holds (`shared.pattern().grid()`) agrees — and that a further rendered block still advances
the position one step at a time, so the mid-bar rescale neither stalls the clock nor double-triggers a step. A `1/32`
pattern with two sounding steps past twelve, then clicked to `1/8T`, says `32 TO 12 STEPS, 2 CLEARED` and leaves those two
slots resting.

- [ ] **Step 8: Draw the key and the well** in `row_transport.rs`: the key with id `"grid"` labelled `GRID`, the well
painted as the transpose readout is (`rect_filled` + `rect_stroke`, `theme::legend(14.0 * scale)`), its text
`pattern.grid().name()`, dim at `Sixteenth` and `p.led_accent` otherwise. The press calls `shared.set_grid(next)`, re-reads
`state.pattern = shared.pattern()`, sets `dirty`, and notifies with `grid_notice(before, after, cleared)` where `cleared`
is `before_pattern.sounding_steps_beyond(next)`.

- [ ] **Step 9: `CLEAR` keeps the grid.** In `ui/row_perform.rs`, build `Pattern::empty_in(shared.pattern().grid())` rather
than `Pattern::empty()`. Add to `tests/grid_live.rs`: on a thirty-two-step pattern, `CLEAR` twice leaves `len() == 32`.

- [ ] **Step 10: The keycap's labels.** Change nothing in `step_key.rs`: both the note and the corner number stay at every
grid, by the player's ruling. Add the measurement below to `tests/grid_live.rs`, run at **both** the design size and the
smallest window (`layout::scale` clamps at 0.65, so the panel is 637 × 436) — the text scale and the cell width are both
linear in the panel, and the ratio this asserts is what must hold at every size.
```rust
#[test]
fn the_note_fits_a_thirty_second_key() {
    // ... a rack on a 1/32 pattern, drawn at the design size
    let l = layout::layout(egui::Rect::from_min_size(egui::Pos2::ZERO, theme::DESIGN_SIZE));
    let cells = layout::step_cells(l.steps, 32);
    let widest = cells[0];
    let ctx = /* the live test's egui context */;
    let text = ctx.fonts(|f| {
        f.layout_no_wrap("C#3".to_owned(), theme::legend(12.0), theme::BLACK.keycap_ink).size().x
    });
    assert!(
        text < widest.width() - 4.0,
        "the note ({text}) does not fit a {} px key",
        widest.width()
    );
}
```

- [ ] **Step 11: Gates and commit** — `-m "feat(grid): the key, the well, and the words it says"`.

### Task 2 — the drum face at twelve and thirty-two

**Files:**
- Modify: `rust/crates/acid-app/src/ui/drum_panel.rs`
- Test: `rust/crates/acid-app/tests/drum_live.rs`, `rust/crates/acid-app/tests/playhead_live.rs`

**Interfaces:**
- Consumes: `Pattern::len`, `Grid::is_beat`, `drum_panel::{row_cells, cell_rect, column_rect}` with their count (round 01).
- Produces: nothing new — this task makes the existing columns *look* right at the new counts.

- [ ] **Step 1: Write the failing live tests** — on a drum device whose pattern is `1/8T`: twelve cells per row, the
selected column's border on step 11 after eleven `right` presses, and the twelfth press wrapping to 0; on a `1/32`
pattern: thirty-two cells per row, all inside the grid area, and the playing column lit on the step the position names.

- [ ] **Step 2: Run them, expect FAIL** (or a visible mismatch: the column count is right from round 01, the framing and
clamping are not).

- [ ] **Step 3: Implement** — the beat framing is `pattern.grid().is_beat(step)` (not `step % 4 == 0`), `column_rect` and
`cell_rect` clamp to `pattern.len()`, and the row's `Vec` is built once per row rather than once per cell (the module's own
comment already promises this; check it while the count is in hand).

- [ ] **Step 4: Run the tests, then the whole workspace.**

- [ ] **Step 5: Gates and commit** — `-m "feat(drums): the grid's columns follow the bar"`.

### Task 3 — the two demos

**Files:**
- Modify: `rust/crates/acid-app/src/demo.rs`, `EAR_TESTING.md`
- Test: `rust/crates/acid-app/tests/drum_banks.rs`

**Interfaces:**
- Consumes: `Pattern::from_steps_in`, `Grid` (round 01).
- Produces: `demo::acid_jazz()`, `demo::fast_acid()`, both `Demo`; `demo::line(grid, hits)`,
`demo::bar(grid, strikes)`.

- [ ] **Step 1: Write the failing bank tests** — `NAMES` grows to seven with `"Acid Jazz"` and `"Fast Acid"` last;
`every_demo_variation_has_a_drum_bank_of_eight_patterns` still holds for both; a new test asserts the jazz line is
`1/8T` with twelve steps and every one of its eight drums, and the fast one `1/32` with thirty-two; and
`load_drum_bank(&mut rack, 7)` is `None` (the test that said `nth(5)` is out of date twice over).

- [ ] **Step 2: Run them, expect FAIL.**

- [ ] **Step 3: Make the constructors grid-aware.** `line(grid, hits)` and `bar(grid, strikes)` build a
`[Step; PATTERN_SLOTS]` and answer `Pattern::from_steps_in(grid, &steps)`; `sparser`, `without`, `also` walk
`pattern.iter_mut()` so they touch only the bar; every existing demo passes `Grid::Sixteenth` and its bytes do not change.

- [ ] **Step 4: Write the two demos.** `acid_jazz`: a walking line of twelve eighth-triplet steps, a ride on steps 1, 4, 7
and 10, snare comping on 3 and 9, the kick on 1 and 8, shuffle `0.0`. `fast_acid`: thirty-two steps, sixteenth slides
broken by 32nd rolls, kick on 1, 9, 17 and 25, closed hats on the odd steps, 152 BPM. Both appended after `funk()`.

- [ ] **Step 5: Run the bank tests, then `cargo test --workspace`.**

- [ ] **Step 6: Listen to them** (`EAR_TESTING.md`'s procedure) and record the two lines it asks for: tempo, grid, and how
the groove should sit. A demo nobody has heard is not a demo.

- [ ] **Step 7: Gates and commit** — `-m "feat(demos): a triplet jazz line and a thirty-second acid line"`.

### Task 4 — the book, the pins and the pictures

**Files:**
- Create: `docs/guide/src/23-pattern-grids.md`
- Modify: `docs/guide/src/SUMMARY.md`, `docs/guide/src/08-writing-steps.md`, `11-the-bank.md`, `22-the-drum-machine.md`,
  `12-songs-and-demos.md`, `04-the-face.md`, `01-what-it-is.md`, `09-playing.md`, `10-performance-row.md`,
  `15-keyboard.md`, `16-sound-like-acid.md`, `18-a-rack-of-devices.md`, `20-reference.md`, `21-copy-and-paste.md`,
  `docs/guide/check.sh`, `docs/guide/src/img/README.md`
- Test: `./docs/guide/check.sh`

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Find every claim the round makes stale.** `grep -n "sixteen\|Sixteen\|sixteenth" docs/guide/src/*.md`
and read each hit rather than rewriting it blind: a "sixteen drum voices" is still true, a "sixteen steps" is what moved.

- [ ] **Step 2: Write `23-pattern-grids.md`** — what a grid is; the three and what each is for; the `GRID` key and the well
beside it; what a grid change does to steps that fall outside the bar, and that the status line counts them; that a bank and
a song keep each pattern's grid; and that a step is `1/16`, `1/32` or `1/8T` of a bar, so every machine's bar is still the
same bar. Cross-reference it from `08-writing-steps.md` and `22-the-drum-machine.md`, and name `GRID` in backticks.

- [ ] **Step 3: Add it to `SUMMARY.md`**, in Part III under *Copying and pasting a pattern*.

- [ ] **Step 4: Edit the pages the claim lives on** — `01` (one voice, some steps), `04` (the face's bands), `08` (a pattern
is a bar of steps), `09`/`10` (the swing leans sixteenths unless the grid says otherwise), `11` (a bank of sixty-four
patterns of any grid), `12` (seven demos), `15` (the keys walk the bar), `16` ("Sixteen steps, one voice"), `18` (a device
owns its steps and its patterns), `20` (the reference's row of keys — add `GRID`), `21` (the clipboard carries the grid),
`22` (the drum grid's columns).

- [ ] **Step 5: Add the pins** to `check.sh`'s fact list: `1/16`, `1/32`, `1/8T` and `GRID` are all said, the demo list
names seven, and `TBC` is gone from the book (`gone 'TBC'`). Each pin's comment says which page it belongs to.

- [ ] **Step 6: Run `./docs/guide/check.sh`.** Expected: exit 0.

- [ ] **Step 7: Take the pictures** — check `DISPLAY=:N xdotool getdisplaygeometry` says at least `2000 1400` *before*
launching, `getwindowgeometry` says `1960x1340`, a fresh `TAKKA_ACID_DATA_DIR`, pointer parked outside the window:
`transport.png` retaken (`GRID` and its well, no `TBC`), `banks-card.png` retaken (seven demo rows), and two new crops —
`35-grid.png` (the row on a `1/32` pattern, `-crop` from the layout's own rect) and `35-drum-grid.png` (a `1/8T` drum
pattern's twelve columns). Then **the manual check**: open `35-grid.png` at 100 % and look at the corner step number in a
24 px key — `2` at 8 pt, `17` at 8 pt — and record in `## Progress` what it showed. A number that cannot be read is a
finding this round records, not one it silently designs around. Record each row in `img/README.md` with the state and the
command, as every other picture has.

- [ ] **Step 8: Reference the two new pictures** from `23-pattern-grids.md`, run `check.sh` again, and commit —
`-m "docs: the grid, and the demos that show it off"`.

## Non-goals

- A pattern shorter than its bar; a grid per device; a controller for the grid; refusing a newer save file. All four are the
  spec's own Non-goals and stay out.
- A grid key on the drum face of its own. The key is in the transport row, which both faces draw.
- Retaking `step-row.png`: a first run is a `1/16` pattern and the picture is still true.

## Review focus

1. **`GRID` pressed mid-bar while the rack runs.** The clock rescales the step in flight; nothing may double-trigger or
   jump backwards — Task 1's live test presses it mid-bar, and round 01's clock test is the arithmetic.
2. **A `1/8T` pattern's slide off its last step.** The slide look-ahead is `(i + 1) % len`, so step 11 must glide into step
   0 — pinned in Task 1's live test.
3. **`CLEAR` on a thirty-two-step pattern.** Thirty-two keys must remain — Task 1, step 9.
4. **The thirty-two-key row at the smallest window.** The keys must stay inside the band and the note inside the key at
   `scale` 0.65 and at the design size — Task 1's measurement test. The corner number is the other half of this and has no
   test on purpose: 8 pt in a 24 px key is judged by eye on Task 4's picture, and the Progress records the verdict.
5. **`Acid Jazz` on a drum device.** Twelve columns, and the tempo and shuffle it carries — Task 3's bank test and Task 2's
   columns.

## Progress

- **Task 1 — the `GRID` key and the 303's row.** `Grid::next` cycles the catalogue with a full `match`;
  `Pattern::sounding_steps_beyond` counts the steps **that sound** outside a grid's bar; `SharedState::set_grid` is a
  copy-on-write on the same `rcu` seam as `set_step`; `transport::Cell::{Tbc, Spare}` are now
  `{Grid, GridWell}` and `tips::Control::Grid` carries the words (`TBC` no longer appears anywhere in `rust/`);
  `transport::grid_notice` says the bar before and after, and the count of sound it cost. The row draws the key with id
  `"grid"` and the well beside it in the transpose readout's shape — dim at `1/16`, `p.led_accent` otherwise — and a
  press is a pattern edit: `shared.set_grid(next)`, the panel re-reads the pattern, the session goes dirty, the status
  line says what happened. `CLEAR` now builds `Pattern::empty_in(shared.pattern().grid)`.
  Proved by `the_grid_key_cycles_the_catalogue` and `a_press_counts_the_sound_it_would_drop` (watched fail first, as
  `no method next`), `the_notice_reads_as_a_bar_before_and_after`, `the_grid_key_cycles_and_its_well_is_not_a_key`,
  and the new `tests/grid_live.rs`: `the_key_cycles_the_grid_mid_bar_and_the_engine_hears_it` (the engine run to
  position 5 of 16 by rendering, then the press, then a half-thirty-second block that advances exactly one step —
  no stall, no double trigger — with the well's caption read back out of the frame at each of `1/16`, `1/32`, `1/8T`),
  `narrowing_the_bar_counts_the_sound_it_rests` (`32 TO 12 STEPS, 2 CLEARED`, the two sounding steps rested rather
  than hidden), `clear_keeps_the_grid` (two clicks, thirty-two keys still), and `the_note_fits_a_thirty_second_key`.
  Workspace suite **972/972**, clippy `-D warnings`, `fmt --check`, `guide: ok`.
  **Ruling: the keycap's note now fits its key, which the plan said not to change.** The plan's measurement test fails
  as written: a sharp note at 12 pt measures 21.9 px in a 23.8 px thirty-second key at the design size (0.9 px a
  side, not the 4 px the plan asked for), and 14.25 px in a 13.4 px key at the smallest window. The premise "the text
  scale and the cell width are both linear in the panel" is false: `theme::GAP` is a design-space constant, so a
  thirty-two-key row loses proportionally more width than the font does. `step_key` now draws the note at **12 pt
  wherever 12 pt fits**, and scales it down by one measurement of the label against the keycap's room only where it
  would spill out. The test reads every label back out of the frame's text shapes and asserts it is narrower than the
  key it was drawn in, at both sizes — the note and the corner number alike. Cost if wrong: at `1/32` in the smallest
  window the note falls to about 6.6 pt, which is small; it is legible at the design size (~11.9 pt) and the picture
  in Task 4 is where the player judges it. The corner step number still never shrinks, by the player's ruling.
  **Ruling: the test reads `panel.pattern.grid`, the public field, not `pattern.grid()`.** The plan's live-test
  sketch wrote `grid()`; `Pattern` exposes the field and no accessor, and adding one would be a second spelling of
  the same thing. Cost if wrong: none.
  **Round 01's open question is now reachable.** `Sequencer::set_pattern`'s short→long wrap (a request released at
  step 15 of a sixteen-step bar leaves `15 % 32 = 15`, so the arriving thirty-two-step bar starts on its second half
  and its bar line lands half a bar late — for good) could not be reached before this key existed. It still waits for
  the player: the fix changes which step a swap into a longer bar is heard from.
