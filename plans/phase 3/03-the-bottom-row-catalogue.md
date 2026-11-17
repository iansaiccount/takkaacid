# Plan 03 — the bottom row's catalogue

> **For agentic workers:** this is a stacked follow-up to plan 01, taken on branch `phase-3a` on top of the pushed `phase-3`
> (PR #4), and it is finding **F30** of that PR's final review. `superpowers:executing-plans`, reviewed by a dispatched
> subagent per commit.

**Goal:** give the bottom row a `Cell` catalogue, so no code and no test finds a key by counting.

## What is wrong today

The row the tape freed (`REVERSE  TRANS −  [+n]  TRANS +  TBC  TBC  LEARN`) is the last positional list on the face:

- `ui/row_transport.rs` destructures `layout::knob_cells(l.transport, transport::TRANSPORT_CELLS)` into seven names in one
  `if let [reverse_cell, transpose_down, readout, transpose_up, tbc_cell, spare_cell, learn_cell] = cells[..]`.
- `TRANSPORT_CELLS` is a hand-kept `7`, and it *is* the row's layout: the count decides every cell's width, so a cell added
  without the constant changes the row's geometry as well as its meaning.
- Three test files reach into the row by index, each with a comment naming the key it means:
  `transport_live.rs` (`click_transport(..., 4) // TBC is the cell after the transpose pair`, `..., 3) // TRANS +`, and two
  helpers that slice `knob_cells(...)` and then read `cells[0]`, `cells[2]`), `learn_live.rs`'s `learn_centre`
  (`[TRANSPORT_CELLS - 1]`, "LEARN is the row's last cell"), and `learn_highlight_live.rs`'s (`cells[TRANSPORT_CELLS - 1]`).

`ui/switches.rs` exists because this pattern already broke tests once: adding a key silently moved what a test clicked, and
the tests kept passing against the wrong key. The comment beside an index is not a name — nothing fails when it goes stale,
which is what `TRANSPORT_CELLS - 1` was: a count pretending to know where LEARN is.

## The design

One catalogue, in `ui/transport.rs`, beside the row it describes and next to `ui/switches.rs`'s:

```rust
/// One cell of the bottom row, left to right.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Cell { Reverse, TransposeDown, Readout, TransposeUp, Tbc, Spare, Learn }

impl Cell {
    pub const ALL: [Cell; 7] = [ /* drawing order */ ];
    pub fn index(self) -> usize { /* written out, no `_` arm */ }
}

/// The rect of one cell of the bottom row.
pub fn cell(area: Rect, which: Cell) -> Rect {
    layout::knob_cells(area, Cell::ALL.len())[which.index()]
}

/// Cells in the bottom row.
pub const TRANSPORT_CELLS: usize = Cell::ALL.len();
```

Then:

- `row_transport` asks for the seven rects by name and the destructuring is gone; the geometry still comes from one
  `knob_cells` call per frame, because `cell` derives the row from the catalogue every time and seven calls would allocate
  seven vectors for one row.
- Every test asks `transport::cell(area, transport::Cell::Learn)`; `click_transport` takes a `Cell` rather than a `usize`;
  `TRANSPORT_CELLS` stops being a test's way of naming a key.

**Why `Transport` is not the type's home.** `ui::transport` is already the row's module — `Request`, `nudge_tempo`,
`transport_button` all live there — and it is where `ui::switches::Switch` sits relative to the switches row. `layout` stays
geometry only.

### Ledger: what the new rules cost

| Ruling | Cost if wrong |
|---|---|
| The catalogue lives in `ui/transport.rs`, not `ui/layout.rs` | The row's width constant stays outside the module that lays the bands out, unlike `layout::KNOB_COUNT`. The alternative moves the catalogue one module away from the row it names, and `layout` would then know about a row's contents rather than its bands. |
| `Cell::Readout` is a cell with no key | The catalogue names a thing that cannot be pressed. It is a real rect in the row (the well the transpose reads out in), and a test clicks it to prove it answers nothing, so it belongs to the row's identity. |
| `TRANSPORT_CELLS` is kept, derived from `Cell::ALL.len()` | A second name for the row's width. It is the same derivation `layout::KNOB_COUNT` makes, and the number now cannot drift from the catalogue because it is computed from it. |
| No behaviour change at all | If the row's geometry was accidentally coupled to the destructuring, this round would move it. Both are one `knob_cells` call with the same count, and the live tests click the row's cells by name, so a cell that moved lands on a neighbour and fails. |

## The tests that make it safe

In `ui/transport.rs` (unit):

1. `the_cells_are_numbered_and_drawn_in_order` — `Cell::index()` equals the variant's place in `Cell::ALL`, walking `ALL`.
2. `every_cell_is_inside_the_row_and_clear_of_its_neighbour` — every `cell()` is inside the band and abuts the previous
   one's right edge, walking `ALL`.
3. `the_bottom_row_is_as_wide_as_the_switch_row_above_it` — `Cell::ALL.len() == Switch::ALL.len()`, the player's choice from
   plan 01 (the two rows share one column grid), pinned where a cell count would be changed.

Existing tests, unchanged in what they assert, moved to names: `transport_live` (three sites plus `click_transport`),
`learn_live::learn_centre`, `learn_highlight_live::learn_centre`.

Test 2 pins the row's shape but not its *count*: `cell()` takes the count from `ALL` and `knob_cells` anchors both walls, so
any count of two or more passes the wall and abut checks. The count is test 3's job — an eighth cell fails it at `8 != 7` —
which is what the review verified it does.

Mutation checks: (a) give `Spare` 4 and `Tbc` 5 in `index()` and leave `ALL` alone → test 1 fails; (b) swap `Cell::Tbc` and
`Cell::Spare` inside `ALL` → tests 1 and 2 fail (run, below); (c) add an eighth cell with its `index()` arm and its `ALL`
entry → test 3 fails; (d) swap `TransposeDown` and `TransposeUp` in `index()` only → after task 2, `transport_live`'s transpose
test asks for the wrong key and fails. Each reverted.

No behaviour is documented and no screenshot changes: this round renames nothing a player reads, so the book is untouched and
this plan's Progress says why.

## Tasks

1. **The catalogue.** `Cell`, `ALL`, `index()`, `cell()`, `TRANSPORT_CELLS` derived from it, and the three unit tests; nothing
   else uses it yet. **Green commit.**
2. **The row and its tests ask by name.** `row_transport` looks its cells up; `transport_live`'s `click_transport` takes a
   `Cell` and its three call sites name one; both `learn_centre` helpers ask for `Cell::Learn`; `TRANSPORT_CELLS` is no
   longer a test's way of finding a key. **Green commit.**

## Non-goals

- **Any change to the row's cells, labels or geometry.** Seven cells, one grid with the switch row, unchanged.
- **A `layout` change.** `PanelLayout::transport` stays a bare `Rect`; giving the row a `cells` field beside `knobs` and
  `perform` is a different round.
- **The tape deck's catalogue.** `tape::TapeKey` already has one; nothing here touches it.
- **Screenshots and the guide.** Nothing a player sees changes.

## Progress

- [x] **Baseline** — `phase-3a` cut from `phase-3` at `b64d605`; gates green before any change: 683 tests,
      `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `./docs/guide/check.sh` → `guide: ok`.
- [x] **Task 1 — the catalogue.** `Cell`, `ALL`, `index()` (written out, no `_` arm, the way `Switch`'s is), `cell()`, and
      `TRANSPORT_CELLS` derived from `Cell::ALL.len()`; three unit tests (slot order, the row's geometry, the row is as wide as
      the switch row). Nothing else uses it yet — task 2 is the row and its tests. 686 tests.
      **Mutation run:** `Cell::Tbc` and `Cell::Spare` swapped inside `ALL` with `index()` left alone failed
      `the_cells_are_numbered_and_drawn_in_order` and `every_cell_is_inside_the_row_and_clear_of_its_neighbour` (`5 overlaps`),
      then reverted. The review worked out what the other two need: an eighth cell fails test 3 at `8 != 7`, and test 2 cannot
      catch a wrong count at all, because `cell()` takes the count from `ALL` and `knob_cells` anchors both walls — so the count
      is test 3's job and the plan says so.
      **Review: BLOCK on the record, not the code.** Two Important, both fixed before this commit: the phase-overview line was
      pasted into this plan's Non-goals instead of `00-phase-3.md` (the anchors of a file I had just written are not the anchors
      of the next one), and this Progress entry did not exist. Three Minors, all taken: the module doc's "a lit RUN is amber"
      was a drive-by that *also* overclaimed — the light schemes put dark ink on a lit key — so it now says what the code does,
      "a lit key's legend is ink for its raised face"; `Cell`'s doc claimed the drawing and the tests already ask the catalogue,
      which only becomes true at task 2, so it says "ask by name"; and the ledger promised a "golden-geometry test" that no
      test list contained, replaced with what actually guards the geometry — the live tests click the row's cells by name, so a
      cell that moved lands on its neighbour. Also taken: `02-the-top-band` was missing from the phase overview's round list.
