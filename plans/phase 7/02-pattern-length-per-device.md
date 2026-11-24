# Plan 02 — a pattern shorter than its bar

> **Status: not ready to execute.** The wishlist entry (*Pattern length per device*) holds the shape; the questions under
> *Open, for the player* are the ones that decide it, and this round starts when they are answered. Everything below is
> the design as far as it is settled, and what would have to move.

**Goal:** a pattern may hold fewer steps than its bar's grid — eight steps where the grid says sixteen — and it **loops
inside the bar**, so the rack's bar line, a device joining mid-run, `REVERSE`'s bar and the pattern swap all still mean
one thing.

**Wishlist:** `plans/future/wishlist.md`, *Pattern length per device* — "a device with an eight-step pattern would break
*the bar is the same bar for every device*, which phase 2 relies on for the pattern swap and `last_position`. It is a
transport change, not a sequencer change."

## What exists after phase 6, and why it is the seam this needs

- `Pattern::len()` is `grid.steps_per_bar()`; the invariant is that every slot at or above `len()` is a rest, and every
  consumer asks the pattern rather than a constant (`STEPS_PER_PATTERN` is gone).
- The clock holds `steps_per_bar` and rescales the step in flight; `Sequencer::set_pattern` publishes the grid and wraps
  `position` into it; `sounding_step(position, reversed, steps)` mirrors within the count; `swap_now(.., steps)` is asked
  with the **playing** bar's count and `main` passes exactly that.
- The row draws `pattern.len()` keys, the drum face `len()` columns, the selection clamps into the bar, `CLEAR` keeps the
  grid, the randomiser rolls only the bar, and the file writes exactly `len()` steps.

What none of that answers is a pattern whose length is **not** its grid's bar: every place above assumes `len()` is the
bar, because phase 6 made them agree by construction.

## The design, as far as it is settled

Two lengths, and they stop being the same number:

```rust
pub struct Pattern {
    pub grid: Grid,          // the resolution: 16 / 32 / 12 steps to a *bar*
    pub steps: [Step; PATTERN_SLOTS],
    pub length: usize,       // how many of those slots this pattern plays before it loops (1..=len())
}

impl Pattern {
    /// The grid's bar in steps — what the rack's bar is made of.
    pub fn bar(&self) -> usize;      // grid.steps_per_bar()
    /// How many steps the pattern plays before it repeats. Divides `bar()`.
    pub fn len(&self) -> usize;      // length
}
```

The clock still counts **bar steps** (`grid.steps_per_bar()`), so every device's bar lands together; the sequencer maps
its bar position onto the pattern with `position % pattern.len()`, and the pattern's step repeats
`bar() / len()` times inside one bar. `REVERSE` mirrors within `len()`, so a short bar reads backwards as a short bar.

**The file** grows one field, `"length"`, and `SAVE_VERSION` goes to **14**; a document without it reads as the grid's
bar (every file phase 6 wrote), so no migration is needed. A `length` that does not divide the bar is repaired on load to
the nearest divisor below it — the same rule `PatternBank::normalise` keeps for a truncated bank.

## Open, for the player

1. **Where does a player set the length?** A `LENGTH` key beside `GRID` with its own readout (the transpose pair's
   shape, a third time), a drag on the row's right edge, or a knob? The row is already seven wide.
2. **What does the row draw for a pattern shorter than its bar?** `len()` keys with the bar's remaining steps shown as
   a dim gap, or `len()` keys spread across the band, or the bar's keys with the repeat marked?
3. **Does a short pattern repeat, or hold its last step?** Repeating is what the plan assumes (a loop inside the bar);
   holding is what a tape echo does and is a different instrument.
4. **What does `GRID` do to a pattern whose `length` does not divide the new grid's bar?** Round it down (the loader's
   rule), refuse the press, or rest the overflow?
5. **Is the length the pattern's or the device's?** The wishlist's title says per device; phase 6's ruling put the grid
   in the pattern. Putting the length in the pattern keeps one rule and one file field; putting it in the device makes
   two patterns with the same music differ by a switch that is not in either of them.
6. **What does the swap do?** `swap_now` takes the *playing* bar's count — which is now the grid's bar, not the pattern's
   length. A pattern of eight steps inside a sixteen-step bar is still swapped at the bar, not the loop. (This one has a
   right answer — the bar — but it changes the words in `swap_now`'s doc comment and `00-phase-6.md`'s ruling table row.)

## Non-goals

- A bar that is not 4/4, a pattern that spans several bars (the arrangement is later).
- Per-pattern tempo or swing (still the device's / the rack's).
- A length that does not divide the bar; the loader repairs it, the face does not offer it.

## Tests it will need, whenever it starts

- **`Pattern`**: `len()` follows `length`, `bar()` follows the grid, a `length` above the bar is repaired down, the
  invariant (every slot at or above `len()` is a rest) holds after every constructor and after `with_length`.
- **`Sequencer`**: an eight-step pattern in a sixteen-step bar publishes its steps twice per bar; the bar line arrives
  once; `REVERSE` mirrors within eight; a swap is released at the bar, not the loop.
- **the file**: version 14, `"length"` written, a version-13 document reads as the full bar, a `length` of 3 on a
  sixteen-step bar is repaired to 2.
- **the panel**: the row draws what the player ruled in question 2, the selection clamps into `len()`, `CLEAR` keeps the
  length as well as the grid.
- **the book**: a page section under `23-pattern-grids` (or a page of its own) and the pictures retaken with a short bar
  on the face.

## Progress

*(Empty: this round has not started, and this plan is not executable until the six questions above are answered.)*
