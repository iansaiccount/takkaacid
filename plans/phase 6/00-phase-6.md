# Phase 6 — pattern grids

Phase 6 is the phase where **a pattern stops being sixteen steps**. A pattern becomes one 4/4 bar at whatever grid it
names: `1/16` (sixteen steps, everything that exists), `1/32` (thirty-two, finer than a sixteenth) and `1/8T` (twelve,
eighth-note triplets, which sit off the sixteenth grid rather than inside it). The grid belongs to the pattern, so it is
saved, copied and banked with it.

It is also the phase the wishlist has been carrying as *Pattern length per device*. What lands here is the seam that
entry needs — **"the bar" stops being a constant and becomes a question every consumer asks the pattern** — and what
is deliberately left is a pattern *shorter* than its bar, the half that would break "the bar is the same bar for every
device" (see [`../future/wishlist.md`](../future/wishlist.md), *Pattern length per device*, and the spec's Non-goals).

Rounds live here as `NN-topic.md`, one per round, in the shape the earlier phases use.

- [`01-the-pattern-knows-its-bar`](01-the-pattern-knows-its-bar.md) — `acid_core::pattern::Grid`, `Pattern` with its own
  slots and its own grid, `timing` dividing a bar instead of assuming sixteenths, the clock, the sequencer, the playhead,
  the panel and the file all asking the pattern. **Nothing a player sees changes**: the only grid any face can reach is
  still `1/16`, so the tests are what prove the other two play, save and land on the bar line. `SAVE_VERSION` **13**.
- [`02-the-three-grids`](02-the-three-grids.md) — the `GRID` key and its well in the transport row, the row at twelve and
  thirty-two keys, the drum face's columns with the grid, and two demos that put a `1/8T` jazz line and a `1/32` acid
  line on the card.

**Spec:** [`docs/superpowers/specs/2026-12-01-pattern-grids-design.md`](../../docs/superpowers/specs/2026-12-01-pattern-grids-design.md)
— approved in conversation on 2026-12-01. The plans argue from it; where they disagree, the spec wins and the plan is
wrong.

**Gate:** unchanged from phases 1–5 — `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, and `./docs/guide/check.sh` whenever the book moves. Commits are made with the gates green and the
Progress lines current; the review is of the branch at the pull request, by a subagent that wrote none of it
(`AGENTS.md`).

**State: round 01 complete and reviewed; round 02 not started.** Round 01 changed no pixels, so the book did not move
in it; the plan's `## Progress` says so rather than leaving the absence unexplained, and records the branch review
and what was done about each finding. Round 02 is where the face, the demos and the book change.

**What phase 6 does not do**, and where each is recorded: a pattern shorter than its bar (*Pattern length per device*
in the wishlist — the same seam, the next change); a grid per device (the grid is the pattern's); a controller for the
grid (`LEARN` has nothing to teach it, and no `CcTarget` is added); refusing a save file from a newer build (still the
wishlist's *A file from a newer build is refused, not relabelled* — and this phase makes it more pressing, not less);
the rack document; the arrangement; stems.

**Rulings this phase rests on, and what they cost if wrong:**

| Ruling | Cost if wrong |
|---|---|
| A pattern is always exactly one 4/4 bar, and the grid decides how many steps that is (32 / 16 / 12) | A player who wanted a half-bar loop of thirty-seconds cannot have one, and the row is 24 px keys rather than 16 comfortable ones. The alternative breaks every device's bar landing on the same bar. |
| The grid lives in the `Pattern`, not in the device's clock | Two grids on one device means two patterns. A bank whose patterns are all `1/8T` records it sixty-four times. The alternative cannot mix grids in a bank, which is the point of the key. |
| `Pattern` keeps a fixed 32-slot array and derives `len()` from its grid | A pattern costs 512 bytes instead of 256, copied on every step edit on the UI thread. The alternative (`Vec`) loses `Copy`, which the audio-thread snapshot and the bank's by-value replace both lean on. |
| The file writes only the steps that play | A `1/32` pattern is unreadable by a version-12 build — but a `1/16` pattern stays byte-compatible, so only files that use a new grid break. Writing all thirty-two slots would break every file this build writes. |
| Shrinking the bar **rests** the steps that fall outside it, and the status line says how many had sound | A grid press is destructive, unlike any other key on the row. Keeping them hidden in memory makes the same press restore them before a save and lose them after one — two behaviours for one gesture. |
| `swap_now` is asked with the **playing** bar's count | A request held through a thirty-two-step bar lands on its sixteenth instead of its last step, which is the mid-bar swap it exists to prevent. |
| `CLEAR` keeps the grid (`Pattern::empty_in(grid)`) | A key a player meets in the first minute silently turns thirty-two keys into sixteen. |
