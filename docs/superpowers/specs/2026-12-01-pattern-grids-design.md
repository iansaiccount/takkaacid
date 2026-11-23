# Pattern grids — phase 6

**Status:** approved in conversation on 2026-12-01. The plans argue from this document; where a plan disagrees with
it, the spec wins and the plan is wrong.

## Intent

A pattern stops being sixteen steps and becomes **one 4/4 bar at whatever grid it names**. Three grids ship:
`1/16` (sixteen steps, everything that exists today), `1/32` (thirty-two steps, finer than a sixteenth) and `1/8T`
(twelve steps, eighth-note triplets, which sit *off* the sixteenth grid rather than inside it). The grid belongs to the
pattern, so it is saved with it, copied with it and banked with it.

This is also the round the wishlist has been carrying as *Pattern length per device*. Its own words are that the two
"cannot both be designed separately: they are the same change". What phase 6 builds is the seam that makes them one:
**"the bar" stops being a constant and becomes a question every consumer asks the pattern.** What it deliberately does
not build is a pattern *shorter* than its bar (see Non-goals) — that is the other half, and it is the half that would
break "the bar is the same bar for every device".

## What exists today

- `pattern::STEPS_PER_PATTERN = 16` is imported by eleven source files and pinned in sixteen test files. `Pattern` is
  `{ steps: [Step; 16] }` — `Copy`, and `acid-core`'s only serde type.
- `timing::samples_per_step` computes a **sixteenth**: `sample_rate * 60 / (bpm * 4)`. `step_duration_samples` applies
  swing to that, as a pair-preserving trade, so the bar never changes length at any swing.
- `Clock` counts steps forever and maps them onto the pattern; `Sequencer` owns the `Arc<Pattern>`, wraps `position` at
  sixteen, and publishes a `bar_line` when step 0 comes round. The rack's bar — when a device may change direction, and
  when a device waiting to join is let in — is that line.
- `Sequencer::sounding_step(position, reversed)` mirrors the playhead for `REVERSE` using the same constant.
- `pattern_swap::swap_now` holds a controller's pattern request until `position + 1 >= STEPS_PER_PATTERN`.
- The step row is `layout::step_cells(area) -> [Rect; 16]`, and both editors frame the beat with `index % 4 == 0`
  (`step_key::step_view`, `drum_panel`'s column framing). At `1/8T` a beat is three steps and at `1/32` it is eight.
- The transport row already has two cells labelled `TBC` that refuse with `TBC IS NOT DECIDED YET`. They exist because
  the row is seven to match the switch row above it.
- `SAVE_VERSION` is **12**; the clipboard is its own envelope at `VERSION` 1.

## Goals

1. `acid_core::pattern::Grid` — a catalogue of three grids, one place that answers "how many steps to the bar", "is this
   step a beat", and what the grid is called.
2. `Pattern` holds its grid and a slot table long enough for the longest bar, and answers `len()`.
3. One step of any grid is `1 / steps_per_bar` of a 4/4 bar: **twelve `1/8T` steps, sixteen `1/16` steps and thirty-two
   `1/32` steps are all exactly one bar**, so every device's bar line still lands with every other's.
4. Every consumer of the bar asks the pattern: the clock, the sequencer, the playhead, the pattern swap, the step row,
   the drum grid, the selection, the randomiser, the factory rotation and the demos.
5. The grid is chosen on the face, and the value is on the face with it.
6. Two new demo songs put all three grids in front of a player: a jazz line on `1/8T` and a fast acid line on `1/32`.
7. The guide says what the face now does, with pictures.

## Non-goals

- **A pattern shorter than its bar.** An eight-step pattern that loops twice a bar is *Pattern length per device*, and
  it breaks what phase 2 relies on: the rack's bar line, a device joining mid-run, and `REVERSE`'s bar. The seam phase 6
  builds is the one that change needs; the change itself is later.
- **A grid per device.** The grid is the pattern's (decided during planning). Two grids on one device means two
  patterns.
- **A controller for the grid.** No `midi::cc::CcTarget` this round; `LEARN` has nothing to teach the key.
- **Refusing a save file from a newer build.** Still the wishlist's *A file from a newer build is refused, not
  relabelled*. Phase 6 confines the break to files that genuinely use a new grid (see Risks).
- **The rack document, the arrangement, stems.** All still later.

## Anti-goals

- A second hand-kept table of step counts. The count is `Grid::steps_per_bar()`; nothing else may spell 12, 16 or 32.
- A `_` arm over `Grid`. Adding a fourth grid must fail to compile where a decision is owed.
- A pattern whose slots above `len()` hold sound. That invariant is what makes the file, the row and the memory agree.
- A grid change that silently destroys notes.

## Approaches considered

**How many steps a `1/32` pattern holds.** *Thirty-two, one bar* (chosen) or *sixteen, half a bar*. The second reading
turns `1/8T` into `1⅓` bars, and a bar that is not a bar is the one thing the rack cannot survive: the pattern swap,
`REVERSE`'s bar and a joining device all assume every bar is the same bar. Thirty-two it is, and the row grows to
thirty-two keys.

**Storage.** `[Step; 32]` plus a `len()` from the grid (chosen) or `Vec<Step>`. `Pattern` is `Copy` on purpose — the
audio thread holds an `Arc` snapshot, `ArcSwap::rcu` copies the whole pattern on every step edit, and the bank replaces
slots by value. An array keeps all three allocation-free; the longest possible pattern is 32 slots.

**What the file writes.** Only the steps that play (chosen) or all thirty-two slots always. Writing all thirty-two
makes *every* document this build writes unreadable by a version-12 build, even one that never leaves `1/16`. Writing
the steps that play leaves a `1/16` pattern's step list exactly sixteen entries, so an older build reads it — serde
ignores the `grid` field it does not know — and confines the break to files that use a new grid.

**Where the grid is chosen.** One of the two `TBC` cells becomes `GRID` and the other becomes its readout, in the
transpose pair's shape: a key and the value it is on. Adding a key to a full row, or hiding the value in a tooltip,
were the alternatives.

## The design

### The grid

```rust
// acid-core/src/pattern.rs
pub enum Grid { Sixteenth, ThirtySecond, EighthTriplet }

impl Grid {
    pub const ALL: [Grid; 3] = [Grid::Sixteenth, Grid::ThirtySecond, Grid::EighthTriplet];
    /// What the face and the file call it: `1/16`, `1/32`, `1/8T`.
    pub fn name(self) -> &'static str;
    pub fn from_name(name: &str) -> Option<Grid>;
    /// Steps in one 4/4 bar of this grid: 16, 32, 12.
    pub const fn steps_per_bar(self) -> usize;
    /// Whether `index` falls on a beat: every fourth step, every eighth, every third.
    pub const fn is_beat(self, index: usize) -> bool;
}
```

It is a catalogue in the repo's sense: `ALL` is the order the key cycles in, `name`/`from_name` are the mapping the file
uses, and every `match` over it is written out in full. `Deserialize` reads the caption and falls back to `Sixteenth`
for a word this build does not know — the rule the save file already has for `DeviceKind` and `Scheme` captions — so a
hand-edited grid cannot cost the player their song.

### One bar, three resolutions

`timing` stops naming a sixteenth:

```rust
/// One 4/4 bar, in samples.
pub fn samples_per_bar(bpm: f32, sample_rate: f32) -> f64;
/// Samples in one step of a bar divided into `steps_per_bar`.
pub fn samples_per_step(bpm: f32, sample_rate: f32, steps_per_bar: usize) -> f64;
pub fn step_duration_samples(step_index: usize, bpm: f32, sample_rate: f32, swing: f32, steps_per_bar: usize) -> f64;
```

Swing keeps its rule unchanged: even steps `1 + 0.5·swing`, odd steps `1 - 0.5·swing`, so a pair always lasts exactly
two steps and the bar is the same length at every swing setting. Every grid's count is even, so the rule holds at
twelve and at thirty-two.

**The claim the whole phase rests on:** a bar of sixteen `1/16` steps, of thirty-two `1/32` steps and of twelve `1/8T`
steps is the same number of samples, so `Clock`'s bar line, `Sequencer::bar_line` and the rack's join all still mean
one thing.

### The pattern

```rust
pub const PATTERN_SLOTS: usize = 32; // the longest bar any grid needs

pub struct Pattern {
    pub grid: Grid,
    /// The bar's slots. Every slot at or above `len()` is a rest, always.
    pub steps: [Step; PATTERN_SLOTS],
}

    pub fn len(&self) -> usize;                  // grid.steps_per_bar()
    pub fn from_steps(steps: &[Step]) -> Pattern;                    // a 1/16 pattern
    pub fn from_steps_in(grid: Grid, steps: &[Step]) -> Pattern;     // any grid
    /// An empty bar **of this grid**: what `CLEAR` leaves behind.
    pub fn empty_in(grid: Grid) -> Pattern;
    /// The same music on a different grid. Slots the narrower bar cannot hold are rested.
    pub fn with_grid(self, grid: Grid) -> Pattern;
    pub fn iter(&self) -> impl Iterator<Item = &Step>;               // 0..len
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Step>;
}
```

`STEPS_PER_PATTERN` is deleted. A count is `pattern.len()` in code that has a pattern, and
`Grid::Sixteenth.steps_per_bar()` in a test that needs sixteen slots to build one.

**The invariant: every slot at or above `len()` is a rest.** `from_steps_in` rests them, `with_grid` rests the ones the
narrower bar drops, and the deserializer rests everything the file did not name. It is what makes "the row is the
pattern" and "the file is the row" the same sentence, and what makes a grid change's one destructive act *visible*.

`with_grid` is the only place the **face** loses sound: narrowing from `1/32` to `1/8T` rests the twenty steps the bar
can no longer hold. The panel says so on the status line (see The face). Keeping them hidden in memory would make the
same key press restore them before a save and lose them after one. The loader's truncation (below) is the second way a
pattern loses sound, and it is reachable only from a hand-edited file.

### The song the step is on

`Clock` gains `steps_per_bar` (`16` at construction) and `set_steps_per_bar(n)`, which **rescales the step in flight**
exactly as `set_bpm` does — half a step elapsed stays half a step elapsed — so a grid change is heard from the step it
was made in and the clock never jumps backwards. `Sequencer::set_pattern` hands the pattern's grid to the clock and
wraps its own `position` into the new `len()`. `sounding_step(position, reversed, steps)` and
`pattern_swap::swap_now(playing, position, last_position, steps)` take the count instead of knowing it. The count
`swap_now` takes is the **playing** pattern's — the bar that is actually running — because its two step values are that
bar's published position, and a request held through a thirty-two-step bar must not be released on its sixteenth,
which is where a *sixteen*-step count would fire it. `Sequencer::set_pattern` then wraps `position` into the arriving
pattern's `len()`, so a swap from a long bar into a short one begins the new bar at its first step.

Nothing else about the transport moves. A bar is a bar (above), so `bar_line`, `start_later`, `settle_direction`, the
bar-line guards on `REVERSE` and the rack's join are untouched in behaviour, and the engine's tests for them should
pass unchanged.

### The save file — `SAVE_VERSION` 13

`Pattern`'s serde becomes hand-written, through a shadow:

```jsonc
{ "grid": "1/32", "steps": [ /* exactly len() steps */ ] }
```

- `grid` serialises as its caption and deserialises through `from_name`, defaulting to `1/16` when absent (every
  version-12 file) or unfamiliar — the rule the save file already has for `DeviceKind` and `Scheme` captions.
- `steps` writes exactly `len()` entries. A version-12 file has sixteen and reads as a `1/16` pattern; a `1/32` pattern
  writes thirty-two and a version-12 build fails to parse it rather than dropping half a bar.
- On load, a list shorter than `len()` is padded with rests and a longer one is truncated at `len()`. The truncation is
  the one place an unknown caption *can* cost sound — a file naming `1/64` with sixty-four steps reads as a
  sixteen-step `1/16` pattern with forty-eight steps gone — and it is accepted for the same reason
  `PatternBank::normalise` truncates a bank with more than sixty-four patterns: a hand-edited file is repaired to
  something playable rather than refused, and the alternative loses the whole song. A file this build wrote never
  hits it. The test pins the loss rather than leaving it to be discovered.

`SAVE_VERSION` goes to **13** and `tests/persist.rs`'s pin moves with it. The clipboard's envelope goes to `VERSION` 2
for the same reason it has a version at all: a version-1 build must refuse a payload that may hold a thirty-two-step
pattern rather than misread it.

### The face

**The key.** `transport::Cell::Tbc` becomes `Cell::Grid` and `Cell::Spare` becomes `Cell::GridWell` — a cell nothing
presses, drawn as a well in the shape the transpose readout already has. The key prints `GRID`, the well prints the
grid's caption (`1/16`, `1/32`, `1/8T`), and a press cycles `Grid::ALL`. The write path is a step edit's, not a
transport request's: the grid belongs to the pattern, so the key calls `SharedState::set_grid`, re-reads
`shared.pattern()` into the panel's copy, and marks the session dirty.

**The notice.** After a press the status line says what happened, as the bar before against the bar after:
`16 TO 32 STEPS` when nothing was lost, `32 TO 12 STEPS, 3 CLEARED` when three steps that had sound fell outside the
shorter bar. `CLEARED` counts steps **that had sound** — the number a player cares about — while the two lengths are the
shape of the change. The face already shows the new length, so a count of bare slots would say nothing the row is not
already saying.

**The row.** `layout::step_cells(area, count) -> Vec<Rect>` draws `pattern.len()` keys. At `1/16` the cells are 53 px
wide, at `1/32` about 24 px, at `1/8T` about 72 px, all 104 tall at the design size. Both of a key's labels stay at
every grid: the note at 12 pt and the corner step number at 8 pt. **Whether the number still reads in a 24 px key is a
manual check on the real screen**, taken with the round's pictures and recorded in the plan's `## Progress`; the note has a
measurement test, because a name that overflows its key is a bug rather than a judgement.

**The beat.** `Grid::is_beat` replaces `index % 4 == 0` on both faces: every fourth key at `1/16`, every eighth at
`1/32`, every third at `1/8T`.

**Elsewhere.** The selection wraps at `len()`; `left`/`right` walk it; the drum face's grid is `Drum::ALL` rows against
`len()` columns; the randomiser rolls over `len()` steps; the factory's rotation is `% len()`. `CLEAR` empties the bar
without leaving it — `Pattern::empty_in(pattern.grid())`, never `Pattern::empty()` — because a key that silently turned
thirty-two keys into sixteen would be the grid reset the whole phase exists to stop. A song load, a paste or a pattern
change clamps `selected` into the arriving pattern's bar.

**The card.** Two more demo rows. The card's four sections already share one budget and stop at the limit, so the cost
is two rows of the player's own lists when there are many — not a fifth face.

### The demos

Two, appended after `funk()`, so the card's existing order and `tests/drum_banks.rs`'s `nth(4)` stay valid:

| demo | grid | steps | BPM | what it is |
|---|---|---|---|---|
| **Acid Jazz** | `1/8T` | 12 | 140 | a walking line over a ride on every quarter (steps 1, 4, 7, 10), snare comping and a brush-ish shaker; the triplet grid *is* the swing, so the shuffle knob stays at zero |
| **Fast Acid** | `1/32` | 32 | 152 | sixteenth slides broken by 32nd rolls, kick on every eighth (steps 1, 9, 17, 25), hats on the offbeat 32nds |

Each arrives with its `drums: [Pattern; 8]` like every other demo, and the drum patterns are on the same grid as the
line: twelve columns for the jazz groove, thirty-two for the fast one. That is the drum machine's own showcase of the
new grids, and it costs no new mechanism.

The names are the player's: they go on the card and in the book.

### The book

A new page **`23-pattern-grids.md`** in Part III, after *Copying and pasting a pattern*: what a grid is, the three, the
`GRID` key and its well, what a grid change does to the steps that fall outside it, and which pages the count appears
on. Every page that says "sixteen steps" as a fact about the instrument is edited in place — `01`, `04`, `08`, `09`,
`10`, `11`, `12` (the demo list), `15`, `16`, `18`, `20`, `21`, `22`. That list is a starting point and not the rule:
the claim is found by grepping the book for what it says about a pattern's length, and a page that states it in its own
words — page `16`'s "Sixteen steps, one voice" — is still the claim. `check.sh` gains the pins that fail when the book
stops saying them: the three grid captions, `GRID`, and that the demos number seven.

Pictures, through `docs/guide/src/img/README.md`: the transport row retaken (the `GRID` key and its well), the step row
at `1/32` and at `1/8T`, the drum grid at `1/8T`, and `banks-card.png` retaken for seven demo rows.

## Tests

- **`acid-core`** — `Grid`: `ALL` is the cycle order; `name`/`from_name` round-trip and an unknown caption reads as
  `Sixteenth`; `steps_per_bar` is 16, 32, 12; `is_beat` is every fourth, eighth, third step; every grid's count divides
  a bar exactly.
- **`acid-core::timing`** — `samples_per_bar` is `sr * 60 / bpm * 4`; a bar of sixteen sixteenths, thirty-two
  thirty-seconds and twelve eighth-triplets are the same number of samples; swing preserves the bar at every grid.
- **`acid-core::pattern`** — a narrower `with_grid` rests the slots above `len()` and keeps the ones inside; `iter`
  yields `len()` steps; `from_steps_in` rests unnamed slots; `empty_in(grid)` leaves an empty bar of that grid; the
  invariant holds after every constructor.
- **`acid-core::sequencer`** — a twelve-step pattern wraps at twelve and a thirty-two-step at thirty-two; `sounding_step`
  mirrors within the count; a pattern swapped for one of another grid keeps the clock's bar honest (the engine's bar
  line test, run at two grids); and a pattern request held through a thirty-two-step bar is not released at its
  sixteenth — `swap_now` is asked with the playing bar's count, not the arriving one's.
- **`acid-app` persistence** — the version pin reads **13**; a `1/16` pattern writes sixteen steps and a grid key; a
  `1/32` pattern writes thirty-two; a version-12 document loads as `1/16` with sixteen steps; a file whose `steps` is
  longer than its grid truncates, and shorter pads with rests; an unknown grid caption loads as `1/16` and a file naming
  `1/64` with sixty-four steps reads as sixteen, the loss pinned rather than discovered; a `1/16` pattern written by this
  build still parses when the `grid` field is removed (the version-12 shape).
- **`acid-app` clipboard** — the envelope's version pin reads **2**; a thirty-two-step pattern round-trips with its grid.
- **live** (`tests/grid_live.rs`) — pressing `GRID` cycles the pattern's grid and the well prints it; the row has 32
  keys at `1/32` and 12 at `1/8T`; narrowing the bar rests the steps outside and the status line says how many; the
  position of a thirty-two-step pattern drives the playhead within thirty-two; the drum face's columns follow the count;
  the step band's keys stay inside the band at every grid; `CLEAR` on a thirty-two-step pattern leaves a thirty-two-step
  bar; the notice's `CLEARED` count is the steps that had sound, not the slots that fell outside.
- **`tests/drum_banks.rs`** — seven demos, each bank distinct and striking; the jazz bank is `1/8T` with twelve steps,
  the fast one `1/32` with thirty-two; and a demo index past the last still answers nothing (`load_drum_bank(rack, 7)`
  is `None`), which is where seven demos and a test that said `nth(5)` meet.

## Risks

- **A version-12 build cannot read a document holding a non-`1/16` pattern.** Narrowed deliberately: the writer emits
  only the steps that play, so `1/16` songs keep their old shape and are read by an older build. The rest is the
  wishlist's *A file from a newer build is refused, not relabelled* — which this round makes more pressing, not less,
  and which is the next round's to fix.
- **Thirty-two keys at 24 px.** The note name at 12 pt is measured by a test and must fit. The corner number at 8 pt is a
  *reading* judgement, not an arithmetic one, so it is checked by eye on the round's picture — and the number stays, ruled
  by the player, rather than being removed before it has been seen failing.
- **A grid change mid-bar rescales the step in flight.** The bar line then lands at a different wall time than it would
  have, between two devices for one bar. It is a pattern edit, not a performance, and the alternative — holding the
  change until the bar line like `REVERSE` — makes an edit wait for music that may not be playing.
- **`with_grid` destroys notes.** Visible, counted on the status line, and the alternative is a file and a face that
  disagree.

## What is left to the builder

- The musical content of the two demos: which steps carry slides, which pitches the jazz line walks, which voices the
  grooves strike. The acceptance test is by ear (`EAR_TESTING.md`), and the shape is the table above.
- The exact wording of the status line's two notices, within the shape above.
- Where `set_grid` lives and what it is called — `SharedState::set_grid` is the suggestion, beside `set_step`, which is
  the same edit through the same seam.
