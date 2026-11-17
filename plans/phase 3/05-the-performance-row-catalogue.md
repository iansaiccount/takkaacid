# Plan 05 — the performance row's catalogue

> **For agentic workers:** round 5 of phase 3, on branch `chore/performance-row-catalogue` cut from `main` at `57837ee` (PR #6
> merged). `superpowers:executing-plans`, reviewed by a dispatched subagent per commit.

**Goal:** the performance row stops being a positional list. Its seven controls get a catalogue, and the split into two clusters
is derived from it rather than kept beside it — the last row on the face that finds a control by counting.

## What is wrong today

`ui/row_perform.rs` destructures `l.perform.cells[..]` into seven names:

```rust
if let [octave_down, octave_up, clear_cell, randomise_cell, shuffle_cell, density_cell, scale_cell] = l.perform.cells[..]
```

and the row's two counts are hand-kept numbers in `layout.rs`: `PERFORM_LEFT = 4` and `PERFORM_RIGHT = 3`, with
`PERFORM_COUNT` derived from them. Three test files reach in by index, two sites each:

| file | site | means |
|---|---|---|
| `tests/clear_live.rs:27, :164` | `perform.cells[2]` | `CLEAR` |
| `tests/clear_live.rs:196` | `perform.cells[3]` | `RANDOM` |
| `tests/learn_live.rs:212, :240` | `perform.cells[4]` | `SHUFFLE` |
| `tests/perform_live.rs:41` | `cell(index)` | a helper the whole file drives the row through |

This is the pattern plan 03 removed from the bottom row, one row away. It has the same cost: a control inserted before another
moves what a test clicks, silently, and a comment beside an index fails nothing when it goes stale. The plans recorded it twice
(plan 03's Progress, and plan 01's `cells[..]` note) and did not do it because the row is not one grid: it is two clusters with
open face between them, so a catalogue here has to carry **a split as well as an order**.

## The design

One catalogue, `ui/perform.rs`, beside the row it describes, in the shape `ui/switches.rs` and `ui/transport.rs` already use:

```rust
/// One control in the performance row, in drawing order: the played keys on the left, the dials on the right.
pub enum Perform { OctaveDown, OctaveUp, Clear, Randomise, Shuffle, Density, Scale }

impl Perform {
    pub const LEFT: [Perform; 4] = [OctaveDown, OctaveUp, Clear, Randomise];
    pub const RIGHT: [Perform; 3] = [Shuffle, Density, Scale];
    pub const ALL: [Perform; 7] = [/* LEFT then RIGHT, written out — no `_` arm anywhere */];
    pub fn index(self) -> usize { /* written out */ }
    pub fn in_the_left_cluster(self) -> bool { /* written out */ }
}

/// The rect of one control in the performance row.
pub fn cell(area: Rect, which: Perform) -> Rect {
    layout::perform_cells(area)[which.index()]
}
```

and `layout.rs`'s three counts become derivations:

```rust
pub const PERFORM_LEFT: usize = Perform::LEFT.len();
pub const PERFORM_RIGHT: usize = Perform::RIGHT.len();
pub const PERFORM_COUNT: usize = Perform::ALL.len();
```

**Why the split lives in the catalogue.** `PERFORM_LEFT = 4` is a statement about *which controls are played and which shape
the roll* — a fact about the row, not about the geometry — and today it is the only place that facts exists. Putting it in the
catalogue means a control added to a cluster widens that cluster's cell count instead of silently becoming part of the other
one, and `layout::perform_cells` keeps doing what it is good at (two clusters hanging off two walls) without knowing a control's
name. Precedent: `layout::KNOB_COUNT` is `Knob::ALL.len()`.

**Why not reuse `transport::Cell`.** `Cell` is the bottom row's vocabulary (`Reverse`, `Readout`, `Spare`); a second `Cell` in a
second module is legal, and the call sites would read `perform::cell(area, Cell::Shuffle)`, which says nothing about which row.
`Perform::Shuffle` does.

### Ledger: what the new rules cost

| Ruling | Cost if wrong |
|---|---|
| The catalogue owns the cluster split, not `layout` | `perform_cells` now takes its two counts from a UI module's arrays, so the geometry half of the layout depends on a row's contents. It already depends on `KNOB_COUNT` the same way, and the alternative is a second hand-kept fact beside the first. |
| `LEFT` and `RIGHT` are written out, and `ALL` repeats them | One list repeats another, which the repo has paid for twice. The repetition is confined to one file, and the test below asserts `ALL == LEFT ++ RIGHT`, so a control added to one and not the other fails. The alternative — building `ALL` from the two arrays at compile time — needs a const concat trick that reads worse than the test. |
| `in_the_left_cluster()` is written out rather than inferred from `index() < LEFT.len()` | Two hand-kept facts (order and side) that could disagree. It is written out so a reordered cluster fails to compile rather than silently re-clustering; the geometry test pins the two clusters' rects as a consequence. |

## The tests that make it safe

In `ui/perform.rs` (unit):

1. `the_controls_are_numbered_and_drawn_in_order` — `index()` equals the control's place in `ALL`, walking `ALL`.
2. `every_control_is_inside_the_row_and_clear_of_its_neighbour` — every `cell()` is inside the band, and each cluster abuts
   left-to-right (the left four, then the right three), with the gap between the clusters as the only break.
3. `the_left_cluster_is_the_played_keys_and_the_right_the_dials` — `ALL` is `LEFT` then `RIGHT`, every control agrees with
   `in_the_left_cluster()`, and the two clusters' cells are the ones `layout::PERFORM_LEFT` / `PERFORM_RIGHT` count.

Existing tests, unchanged in what they assert, moved to names: `clear_live`'s three sites, `learn_live`'s two, and
`perform_live`'s `cell(index)` helper becomes `cell(Perform)`.

Mutation checks: (a) swap `Shuffle` and `Density` in `ALL` → test 1 fails; (b) move `Clear` into `RIGHT` (and its count) → test 3
fails; (c) swap the two clusters' order in `ALL` → test 2 fails (the rects no longer abut in that order).

No behaviour changes, nothing on the face moves: same seven cells in the same two clusters, same captions, same `egui::Id`s. The
book does not change and no screenshot is retaken; the round is invisible to a player and this plan says so.

## Tasks

1. **The catalogue.** `Perform`, `LEFT`, `RIGHT`, `ALL`, `index()`, `in_the_left_cluster()`, `cell()`, and `layout`'s three
   counts derived from it; the three unit tests. Nothing else uses it yet. **Green commit.**
2. **The row and its tests ask by name.** `row_perform` looks its seven cells up instead of destructuring; `clear_live`,
   `learn_live` and `perform_live` ask `perform::cell(area, Perform::X)`. **Green commit.**

## Non-goals

- **Any change to the row's controls, captions or geometry.** Seven cells, two clusters, the same gap.
- **A knob catalogue change.** `Knob` already owns its row; the shuffle, density and scale *knobs* are drawn by
  `perform_knob` and their learn targets stay in `midi::cc`.
- **`row_perform`'s `if let [...] = l.perform.cells[..]` guard as a feature.** It matched any length and drew nothing when the
  count was wrong; it goes, as the transport row's did.
- **The lib-side `Row.cells` field.** `PanelLayout::perform` keeps its `cells`; the catalogue is the way tests find one.

## Progress

- [x] **Baseline** — branch `chore/performance-row-catalogue` from `main` at `57837ee` (PR #6 merged). Gates green before any
      change: 698 tests, clippy `-D warnings`, `fmt --check`, `./docs/guide/check.sh`.

- [x] **Task 1 — the catalogue.** `ui::perform` holds `Perform` (`LEFT`, `RIGHT`, `ALL`, `index()`, `in_the_left_cluster()`) and
      `cell(area, which)`; `layout`'s `PERFORM_COUNT`, `PERFORM_LEFT` and `PERFORM_RIGHT` are now `Perform::ALL/LEFT/RIGHT.len()`
      instead of `4`, `3` and a sum, so a control added to a cluster widens that cluster and the geometry hears about it. Three
      unit tests (slot order, the row's shape with the break between the clusters, and `ALL` = `LEFT` then `RIGHT` with the
      layout's counts). Nothing else uses it yet — task 2 is the row and the tests. 701 tests.
      **Mutation run:** swapping the two clusters inside `ALL` (leaving `index()` alone) fails all three — the slot test, the
      shape test (the rects no longer abut in that order) and the cluster test; reverted.
      **Review: OK with notes.** It confirmed the shape against `ui/switches.rs` and `ui/transport.rs`, that no `_` arm exists,
      and that the repetition in the ledger *is* closed — and it found the real safety gain, which the entry above understates:
      `layout.rs`'s `cells.len() == PERFORM_COUNT` used to be a tautology (`PERFORM_LEFT + PERFORM_RIGHT` against itself) and is
      now a cross-check between the two arrays and `ALL`. Taken from it: test 3's three `layout::PERFORM_*` assertions were
      unfalsifiable — each constant *is* the array's `.len()`, so they restated the derivation — replaced with one that can fail
      (`layout::perform_cells(area()).len() == Perform::ALL.len()`); and the module doc said `cells[4]` where the three test
      files clicked 2, 3 and 4. Recorded, not fixed: the ledger's line about `in_the_left_cluster()` says a reorder "fails to
      compile", which is only true of *adding* a variant (a reorder is caught by test 3 instead) — corrected below.
