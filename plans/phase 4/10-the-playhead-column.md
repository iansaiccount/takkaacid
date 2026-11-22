# Plan 10 — the playhead is on the sound

**Goal:** the marker shows the step that **sounds**. The drum grid gains a playhead — the 303 has an LED over the step key, the
drum face's columns *are* the steps, so it lights the whole column — and both faces learn to follow the direction, because
`REVERSE` plays the pattern from its other end while the clock keeps walking forward, and today both markers are on the clock.

**Spec:** none. The player's words, twice:

> *"We need a visual indicator of where the pattern is playing on the drum machine. We have one on the 303 above the note markers.
> Can we highlight the current point in the pattern by setting the matching column on the grid to an appropriate theme colour?
> We can include this in the same current branch."*
>
> *"Reverse should also reverse the visual indicator. This should happen on the 303 too. It currently has a bug."*

## What is wrong today

- **The drum face has no playhead at all.** `draw_grid` (`rust/crates/acid-app/src/ui/drum_panel.rs`) paints each cell from its
  mask bit, frames every fourth column and draws the step **selection** as a border, and nothing moves while the transport runs. A
  comment there ends *"the playhead belongs to the 303's step row, which a drum face does not draw"* — this round deletes it.
- **The 303's playhead is on the clock, not on the sound.** `ui/mod.rs:735` passes `state.position` to `step_key::step_view`,
  which lights the LED where `index == position`. But `Sequencer::deliver` (`acid-core/src/sequencer.rs:251`) plays
  `pattern.steps[15 - position]` when the line runs backwards, so on `REVERSE` the LED walks forward across keys whose notes are
  **not** the ones sounding. The player's bug.
- **The book already promises it does not do that.** `docs/guide/src/09-playing.md` has a section headed *"Why the step LED
  follows the sound"*: *"Every frame it asks the audio engine where it is in the pattern and lights the LED over that step, so the
  light is where the sound is rather than where a timer in the panel thinks it should be."* That is true except on `REVERSE`,
  where the book is wrong about the app. Fixing the app makes the book true; the paragraph gains the direction it left out.
- **The panel cannot see the direction that is playing.** `SharedState::reversed` is the **request**: a running sequencer defers
  it to the bar line (`Sequencer::set_reverse`, applied at `:306`). So a naive `15 - position` in the panel would jump the marker
  a whole bar early — a second bug in place of the first.
- **The arithmetic is buried in the audio path.** `15 - position` exists in exactly one place, `deliver`, private to the
  sequencer. The panel drawing the same thing must not write it a second time.

## The design

- **One mapping, in core.** `Sequencer::deliver`'s mirroring becomes
  `pub fn acid_core::sequencer::sounding_step(position: usize, reversed: bool) -> usize` — `15 - position` when the line runs
  backwards, `position` when it does not, `position` wrapped into `0..16` first, as `start_at` already does. `deliver` calls it,
  and so do both faces, so the playhead cannot disagree with the step it is drawn on: there is one definition and the audio is
  on it.
- **The effective direction is published, not assumed.** The renderer publishes `SharedState::reverse_active` beside the
  position it already publishes each block (`render_mono`, after the machine renders): the direction the sequencer is
  **actually** playing, which is `Sequencer::is_reverse()` and changes only at the bar line. `Machine` gains `is_reverse()`
  (two arms, written out, no `_`), `Drums` gains one that delegates to its sequencer, and `Engine::is_reverse()` already exists.
  `SharedState::reversed` keeps its meaning — what the player asked for, which is what the `REVERSE` key shows — and the two are
  now honestly different things.
- **Both faces draw the sounding step.** `ui/mod.rs`'s step row and `drum_panel::draw_grid` each read
  `sounding_step(state.position, shared.is_reverse_active())` and draw there. `step_view`'s third parameter is renamed
  `sounding` and documented; no view computes the arithmetic itself. So under `REVERSE` the 303's LED and the drum column walk
  **right to left** over the keys, landing on the note that is sounding.
- **The drum grid's mark is a wash over the column.** `draw_grid` fills `column_rect(l, sounding)` with
  `faint(p.led_on, PLAYHEAD_ALPHA)` — the lamp the 303's playhead already uses, at a fraction of its opacity, the same idea as
  the selected row's band (`faint(p.led_accent, 36)`). Painted **after** the sixteen rows and **before** the selection border.
- **Why a wash and not a per-cell recolour.** The three ways were laid out and the player picked this one: recolouring only the
  *rest* cells would vanish on a column struck on all sixteen rows, exactly when the player most wants to know where the line is.
  A wash lights rests, plain strikes, accents and muted strikes together, so the playhead is never hidden, and it is one shape a
  frame rather than a branch per cell.
- **While the line runs, and only then.** `state.playing`, the same gate the 303's LED uses; a stopped or paused machine shows
  no playhead. `column_rect` already clamps its step, so a stale position cannot land outside the grid.
- **The selection does not mirror.** It is the edit cursor on the pattern as stored, and a player edits what they see. Under
  `REVERSE` the playhead and the selection therefore sit on different columns at the same instant — which is right, and another
  reason the two marks must read apart.
- **Nothing else moves.** `position` stays the clock's: `mixer` uses it to place a joining device, `pattern_swap` uses it for the
  bar line, and both need the clock, not the sound. No `SavedState` field, no `SAVE_VERSION` bump (still 11), no allocation and
  no lock on the audio path — one atomic store per block, which the renderer already does for the position.

## Tests that make it safe

- `acid-core/tests/reverse_and_transpose.rs::sounding_step_is_the_mirror_of_the_clock` — the pure rule: `(0, false) == 0`,
  `(0, true) == 15`, `(3, true) == 12`, `(15, true) == 0`, and a position past the bar wraps rather than panicking.
- `acid-app/tests/renderer.rs::the_published_direction_waits_for_the_bar_line` — the reason `reverse_active` exists: playing, set
  `reversed` true, render one block and the request reads true while `is_reverse_active()` is still false and the audio is still
  the forward step; render past the bar line and it is true. A panel reading the request would fail this.
- `acid-app/tests/playhead_live.rs` (new — the round's point is that the two faces agree, so their playhead tests sit together),
  driven through `draw_panel` and read back from the painted shapes, which is how these faces are tested:
  - `the_303_lights_the_step_that_sounds_when_the_bar_runs_backwards` — an empty pattern (so only the playhead lights an LED),
    playing, `position` 3, `reversed` **false** and `reverse_active` **true**: the only `led_on` fill in the step band is over
    cell 12. With `reverse_active` false it is over cell 3. The `reversed`/`reverse_active` split is deliberate, so a panel that
    reads the request fails here too.
  - `the_wash_is_on_the_column_that_sounds` — the drum face in the same state: a fill exactly on `column_rect(&l, 12)` equal to
    `drum_panel::playhead_fill(palette(scheme))`, none on `column_rect(&l, 3)`, none on any column when stopped, and the same
    frame under two schemes painting two different inks, each its own palette.
  - `the_wash_reads_over_a_column_of_strikes` — the reason the wash was chosen: strike all sixteen voices on the sounding step
    and assert (a) the wash is still painted on the column, (b) each struck cell still reports its own strike colour, and (c) the
    wash's index in the paint order comes **after** every cell fill in that column. (c) is the one that matters: a wash drawn
    before the cells is present in the shape list and invisible on screen, so a test that only asks "is there a fill at that
    rect" would pass a face with no playhead on it.

Mutations to run and record, each of which must fail a named test: `sounding_step` returning `position` in both directions; the
panel reading `is_reversed()` instead of `is_reverse_active()`; the wash at `state.selected`; the `state.playing` gate dropped;
the wash drawn before the cells; a single `cell_rect` instead of the column; and the 303's caller passing `state.position`
unmirrored.

## Tasks

### Task 1 — one mapping, and the direction the panel can see

**Files:** `rust/crates/acid-core/src/sequencer.rs` (the `pub fn`, `deliver` calling it), `rust/crates/acid-core/src/drums.rs`
(`is_reverse`), `rust/crates/acid-app/src/state.rs` (`reverse_active` and its getter, beside `reversed`'s doc),
`rust/crates/acid-app/src/audio/renderer.rs` (`Machine::is_reverse`, the publish in `render_mono`),
`rust/crates/acid-core/tests/reverse_and_transpose.rs`, `rust/crates/acid-app/tests/renderer.rs`.

**Interfaces:** produces `pub fn acid_core::sequencer::sounding_step(position: usize, reversed: bool) -> usize`,
`pub fn acid_core::drums::Drums::is_reverse(&self) -> bool`, `Machine::is_reverse`, and
`SharedState::reverse_active` with `pub fn is_reverse_active(&self) -> bool`. Nothing existing changes shape; `position` keeps
its meaning.

1. Write the two failing tests. Run — expected FAIL (neither compiles).
2. Add the function, point `deliver` at it, add the direction publishing. Run — expected PASS, and
   `acid-core/tests/reverse_and_transpose.rs`'s existing reverse tests still pass, which is `deliver`'s proof.
3. Mutation check, recorded in Progress.
4. Gates, then commit.

### Task 2 — the 303's playhead follows the sound

**Files:** `rust/crates/acid-app/src/ui/mod.rs` (the sounding step, once, before the loop), `rust/crates/acid-app/src/ui/step_key.rs`
(the parameter's name and doc), `rust/crates/acid-app/tests/playhead_live.rs` (the 303 test).

1. Write the failing live test. Run — expected FAIL (the LED is on cell 3).
2. Mirror the step the row is handed. Run — expected PASS.
3. Mutation check, recorded in Progress.
4. Gates, then commit.

### Task 3 — the drum grid's column

**Files:** `rust/crates/acid-app/src/ui/drum_panel.rs` (`PLAYHEAD_ALPHA`, `playhead_fill`, the wash, the sounding step, the
rewritten comment), `rust/crates/acid-app/tests/playhead_live.rs` (the two drum tests).

**Interfaces:** produces `pub const PLAYHEAD_ALPHA: u8` and `pub fn playhead_fill(p: &theme::Palette) -> egui::Color32` in
`drum_panel`, beside the geometry its tests already reach for.

1. Write the two failing live tests. Run — expected FAIL (nothing is painted on a column).
2. Add the const, the helper and the wash; rewrite the selection comment for the two marks that are now there. Run — expected
   PASS.
3. Mutation check, recorded in Progress.
4. Gates, then commit.

### Task 4 — the book says it

**Files:** `docs/guide/src/09-playing.md` (the LED section), `docs/guide/src/22-the-drum-machine.md` (`## The grid`),
`docs/guide/check.sh`, `plans/phase 4/00-phase-4.md`.

1. Page 9: the LED section learns the direction — the panel reads the engine's position **and its direction**, so a reversed line
   walks the light back along the keys, still on the sound; a `REVERSE` pressed mid-bar turns the light at the bar line with the
   sound, not before it.
2. Page 22: the playing column is washed in the lamp while the line runs, the wash reads over the cells so a column struck on
   every row still shows the line, the selected step's border stays on top, a stopped or paused machine has no playhead, and
   `REVERSE` runs the column right to left with the pattern.
3. `check.sh` pins for both claims; the round's line in `00-phase-4.md`. Run `./docs/guide/check.sh`.
4. Commit.

### Task 5 — the picture

**Files:** `docs/guide/src/img/drum-playing.png`, `docs/guide/src/22-the-drum-machine.md`, `docs/guide/src/img/README.md`.

1. Grab the drum face with the transport running: `space` from the picker's drum state, then a grab once the playhead is away
   from step 1 so it and the selection border read as two marks. Follow `AGENTS.md`: `getdisplaygeometry` before launching,
   `getwindowgeometry` at `1960x1340`, a fresh `TAKKA_ACID_DATA_DIR`, the pointer parked outside the window.
2. A row in `img/README.md` recording the state and the command; the picture named from `## The grid`, beside the sentence it
   shows. Run `./docs/guide/check.sh`.
3. Commit.

## Non-goals

A playhead on the strip, the device card or the song list; a chase light, a trail, a beat blink, or a colour per voice; mirroring
the **selection**; moving the direction's bar-line rule itself (`Sequencer` already does that and this round only reads it);
changing what `position` means for the mixer's join or the pattern swap; anything in `SavedState`, the MIDI map, or a setting to
turn the playhead off.

## Rulings, and what they cost if wrong

| Ruling | Cost if wrong |
|---|---|
| The playhead is on the **sound**, so it mirrors under `REVERSE` — and the panel reads a newly published `reverse_active`, not the `reversed` request | The panel and the audio share one more atomic, and `Machine` grows a method for it. Reading the request instead would be one line and would put the marker a bar ahead of the sound: the same class of bug being fixed, and `renderer.rs`'s test exists to catch it. |
| The mirroring lives in `acid_core::sequencer::sounding_step`, called by `deliver` and by both faces | `acid-core` grows a function that only a panel uses. The alternative is the face writing `15 - position` a second time — the hand-kept second copy of a mapping this repo has paid for twice. |
| The wash is one translucent rect over the cells, not a per-cell recolour | Accents inside the playing column are seen through a tint, so their hue shifts for the one step the column is on. The alternative disappears behind a fully struck column, which is why it was rejected. |
| The column lights while `is_playing()`, so a pause shows nothing | A paused machine keeps its position but loses its mark, matching the 303's LEDs. If a paused playhead is wanted, the gate is one word. |
| `playhead_fill` is `pub`, for the test | One more public item in a module that already exposes `cell_rect`, `row_band` and `column_rect` to its tests. The alternative is the test keeping its own `from_rgba_unmultiplied(…, N)`, as `the_selected_row_is_plated` does with its `36`. |
| The round rides `chore/readme-drum-machine`, at the player's instruction, so that branch's name stops describing its contents | The branch's review reads a feature round against a README chore. This plan is on the branch, so the work is claimed rather than smuggled. |

## Progress

- [x] **Task 1 — one mapping, and the direction the panel can see.** `acid_core::sequencer::sounding_step(position,
      reversed)` is the one place the mirroring lives: `Sequencer::deliver` plays through it, so the audio and a panel cannot
      answer differently, and a position past the bar wraps rather than panicking (the panel reads it from a cell a block
      behind). `SharedState::reverse_active` — one `AtomicBool`, never saved — carries the direction the machine is **playing**:
      `render_mono` publishes `Machine::is_reverse()` in the same place it publishes the position, `Machine` gained that method
      (two arms, written out), `Drums` gained one that delegates to its sequencer, and `Engine::is_reverse()` already existed.
      `SharedState::reversed` keeps its meaning — the request, which is what the `REVERSE` key shows — and `is_reverse_active()`
      is the answer, with the reason it is not the request written on both. `position` still means the clock's, because the
      mixer's join and the pattern swap need that one.
      Tests: `reverse_and_transpose::sounding_step_is_the_mirror_of_the_clock` (the two ends, a mid-bar pair, the wrap, and both
      directions) and `renderer::the_published_direction_waits_for_the_bar_line` — **both machines**, a loop over `Acid` and
      `Drums`: the request reads true on the block it is set while the published direction is still forward, and the bar line
      turns it. The existing reverse-delivery tests in `reverse_and_transpose` and `drums` still pass unchanged, which is
      `deliver`'s proof.
      **Mutations: 6 of 6 caught** (`.superpowers/mutate_playhead_task1.py`): the direction ignored, the mirror reversed,
      `deliver` reading the raw position, the renderer publishing `params.reversed`, and each machine's arm answering for the
      wrong direction. The last two were rewritten after the first draft: one would not have compiled and one targeted a machine
      the test never built, and a compile failure or an unreached arm both read as "caught" while proving nothing — widening the
      renderer test to both kinds is what made the drums' arm catchable. Gates: `cargo test --workspace` **907**, clippy
      `-D warnings`, `fmt --check`.
- [ ] **Task 2 — the 303's playhead follows the sound.**
- [ ] **Task 3 — the drum grid's column.**
- [ ] **Task 4 — the book says it.**
- [ ] **Task 5 — the picture.**
