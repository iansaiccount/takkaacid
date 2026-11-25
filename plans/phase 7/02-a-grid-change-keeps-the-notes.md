# Plan 02 — a grid change keeps the notes it stops playing

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** changing a pattern's grid to a narrower one **stops playing** the steps outside the new bar without destroying
them; changing back plays them again — including across a save. A "short pattern" is not a mode: the bar keeps its length,
the music stops where the player stopped writing, and the empty keys are where the next notes go.

**This reverses a phase-6 ruling.** `01-the-pattern-knows-its-bar.md`'s Progress and `00-phase-6.md`'s rulings table say a
narrower grid **rests** the steps that fall outside it, and that hiding them was rejected because "the same press would
restore them before a save and lose them after one". The player has ruled the other way: nothing is destroyed, and the
file carries what is not playing. The cost the phase-6 ruling named is paid by this round's file change — the held steps
are written, so they do restore after a save.

## What is wrong today

- `Pattern::with_grid` (`acid-core/src/pattern.rs:309-317`) walks `steps[len..]` and writes `Step::rest()` — the sound is
  gone the moment the grid narrows.
- The invariant the whole model rests on is "**every slot at or above `len()` is a rest**". It becomes "**…is not
  played**": a slot above the bar may hold a note, and the row, the clock and `iter()` never show it.
- `Pattern::sounding_steps_beyond(grid)` counts notes a press would drop, and `transport::grid_notice` says
  `32 TO 12 STEPS, 2 CLEARED`. Nothing is cleared now; the count is of notes that stop playing.
- `Pattern`'s serde writes exactly `len()` steps and nothing else, so a held note would not survive a save.

## The design

**The model.** One length, the grid's. A pattern is a 32-slot table; slots below `len()` play, slots at or above it are
held (they may hold a note) and are silent. `with_grid` changes the grid and nothing else:

```rust
/// The same music on another grid. Nothing is destroyed: the steps the new bar cannot play are **held** — silent, kept,
/// and played again when a later grid can hold them.
pub fn with_grid(self, grid: Grid) -> Self {
    Self { grid, ..self }
}
```

**The file** grows one field, and only fills it when it has something to say:

```jsonc
{ "grid": "1/8T", "steps": [ /* the 12 that play */ ], "held": [ /* the 20 that do not */ ] }
```

- `held` is `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, so **a pattern with nothing held writes exactly
  today's bytes** — a `1/16` pattern's file is unchanged, and a version-13 build still reads one (it ignores `held`).
- A version-13 document has no `held`, so it loads with every slot above `len()` resting — which is what phase 6's
  destructive `with_grid` left behind anyway.
- `SAVE_VERSION` goes to **14**; the pin test is renamed and asserts it.

**The words.** `grid_notice(before, after, held)` says what stops playing:
`16 TO 12 STEPS` when nothing is held, `32 TO 12 STEPS, 2 HELD` when two notes fall outside. **The word `HELD` is the
player's and is ruled** — `32 TO 12 STEPS, 2 HELD` is what the face says; everything else here is the code's.

**What does not change.** `len()` is still the grid's bar and still what the row draws, the selection clamps to, the
randomiser rolls, `set_step` bounds, and `iter()` yields. `Pattern::empty_in` and `from_steps_in` still rest slots they
are not given (a constructor builds a slate; only a *grid change* keeps what was already there). The drum masks are held
exactly as the notes are.

## Tests that make it safe

- **`Pattern`**: narrowing to `1/8T` leaves the steps at 12..16 holding their notes and `iter()` not yielding them;
  widening back to `1/16` yields them again, unchanged; narrowing twice and widening twice is the identity; a step held
  on a `1/32` pattern and narrowed to `1/16` and back is byte-identical.
- **the file**: a `1/32` pattern with a note at step 20, narrowed to `1/8T`, saves with `held` non-empty and its `steps`
  twelve long; loading it and widening to `1/32` gives the note back; a pattern with nothing held writes **no `held`
  key** and its bytes match today's; a version-13 document (no `held`) loads and holds nothing.
- **`grid_notice`**: three shapes — `16 TO 32 STEPS`, `16 TO 12 STEPS`, `32 TO 12 STEPS, 2 HELD`.
- **the panel** (`grid_live`): a `1/32` pattern with notes at 20 and 21, pressed to `1/8T`, keeps them (they are not
  rested, and the notice counts them), and pressing back to `1/32` plays them; the row draws twelve keys in between.
- **the book**: `23-pattern-grids` says a narrower grid stops playing the outer steps and keeps them, and no page says a
  press destroys notes.

## Tasks

### Task 1 — `with_grid` keeps, and the invariant becomes "not played"

**Files:** modify `rust/crates/acid-core/src/pattern.rs`; test `rust/crates/acid-core/tests/pattern.rs`.

- [ ] **Step 1** Rewrite the failing tests: `a_narrower_grid_holds_the_steps_that_fall_outside` (the current one asserts
      they are rests) and `every_slot_above_len_is_not_played` (the current `every_slot_above_len_is_a_rest`), plus the
      round trip narrow→wide.
- [ ] **Step 2** Run them and watch them fail (the slots come back as rests).
- [ ] **Step 3** Implement `with_grid` as `Self { grid, ..self }`; update its doc comment and `iter`/`len`'s
      "not played" wording.
- [ ] **Step 4** Run the workspace: every test that asserted a rest after `with_grid` moves to the new claim.
- [ ] **Step 5** Gates and commit — `-m "feat(pattern): a grid change keeps the notes it stops playing"`.

### Task 2 — the count and the words

**Files:** modify `acid-core/src/pattern.rs` (`sounding_steps_beyond` → `held_steps_beyond`), `ui/transport.rs`
(`grid_notice`); tests in both.

- [ ] **Step 1** Write the failing tests: `a_press_counts_the_notes_it_would_stop` (the renamed model test) and the three
      `grid_notice` shapes above.
- [ ] **Step 2** Run them, expect FAIL.
- [ ] **Step 3** Rename the counter, keep its arithmetic (a note past the new bar), and change the notice's word.
- [ ] **Step 4** Run them; fix the callers (`ui/row_transport.rs`).
- [ ] **Step 5** Gates and commit — `-m "feat(panel): the notice says what it holds, not what it cleared"`.

### Task 3 — the file carries what is held

**Files:** modify `persist`'s pin test and `pattern.rs`'s serde; tests `acid-core/tests/pattern.rs`,
`acid-app/tests/persist.rs`.

- [ ] **Step 1** Write the failing tests: `SAVE_VERSION` is 14 (`the_save_version_is_pinned_at_fourteen`); a narrowed
      pattern writes twelve `steps` and a non-empty `held`; a pattern with nothing held writes no `held` key and its
      bytes equal today's; a version-13 document loads and holds nothing.
- [ ] **Step 2** Run them, expect FAIL.
- [ ] **Step 3** Implement: `held` in the serde shadow (`default`, `skip_serializing_if` empty), written from
      `steps[len..]`; `SAVE_VERSION = 14`.
- [ ] **Step 4** Run them, then the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(save): a pattern's held steps travel with it"`.

### Task 4 — the panel, end to end

**Files:** test `rust/crates/acid-app/tests/grid_live.rs`.

- [ ] **Step 1** Rewrite `narrowing_the_bar_counts_the_sound_it_rests`: the same `1/32` bar with notes at 20 and 21,
      pressed to `1/8T`, now asserts the notes are **still there** (`panel.pattern.steps[20].on`), that the row draws
      twelve keys, that the notice says `32 TO 12 STEPS, 2 HELD`, and that a second press back to `1/32` plays them.
- [ ] **Step 2** Run it, expect FAIL.
- [ ] **Step 3** Fix whatever the panel still rests (the press arm, the drum face's `strike_step` clamp if it writes a
      held slot).
- [ ] **Step 4** Run the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(panel): a grid press keeps the music"`.

### Task 5 — the book

**Files:** `docs/guide/src/23-pattern-grids.md` (two paragraphs), `docs/guide/check.sh`.

- [ ] **Step 1** Rewrite the "What a press does to the steps outside the new bar" section: the outer steps stop playing
      and are kept; the status line says how many are held; nothing is destroyed; a narrower grid is reversible.
- [ ] **Step 2** Pin it (`gone 'CLEARED'` in the book's notice example, and the sentence that must be said), run
      `./docs/guide/check.sh` → `guide: ok`.
- [ ] **Step 3** Gates and commit — `-m "docs: a grid change keeps what it stops playing"`.

## Non-goals

- A `length` field, a per-device length, a pattern that repeats inside its bar, or a bar that is not 4/4. The player's
  model has none of these: the bar keeps its length, and a "short" pattern is a bar whose later steps hold nothing to
  play. The wishlist's *Pattern length per device* entry is superseded by this plan and should be amended to say so.
- A held step being editable while it is held: `set_step` still refuses past `len()`, and the row does not draw it. It
  comes back when a grid can hold it.

## Review focus

1. **The file's shape is a contract.** A pattern with nothing held must write the same bytes it wrote at version 13 —
   the `held` key must be absent, not empty. Task 3's byte test is the guard.
2. **Nothing is destroyed on any path.** `with_grid`, the press, a paste of a wider pattern into a narrower one, a demo
   load, `CLEAR` (which *does* empty everything, held included — it empties the pattern).
3. **The invariant reads "not played"**, and every place that used to assume a rest (`iter`, the row, the drum columns,
   `sounding_step`) still asks `len()`.
4. **The old word is gone**: no page, notice or doc comment says a press "clears" a step it holds.

## Progress

*(One line per task, written as it is done: what landed, the test that proved it, and any ruling with what it costs if
wrong. The branch review's findings and what was done about each go here too, before the pull request.)*
- **Tasks 1, 2 and 4 landed together, as the bug the player reported.** "A full pattern on 32, switch to `1/8T`, back to
  32" lost the notes: phase 6's `with_grid` rested everything outside the narrower bar. It is now `Self { grid, ..self }`
  — nothing is destroyed — the notice says `32 TO 12 STEPS, 2 HELD`, and the invariant test reads "not played" (a
  constructor still rests what it was not given; a grid *change* is the one path that may hold a note above the bar).
  Proved by `a_narrower_grid_holds_the_steps_it_stops_playing` (narrow → hold → widen → **equals the original pattern**),
  `every_slot_above_len_is_not_played`, and `narrowing_the_bar_counts_the_sound_it_rests` in `grid_live`, which now
  presses back to thirty-two and asserts the notes play again. Suite **983/983**, clippy `-D warnings`, `fmt --check`.
  **Ruling: the file does not carry the held steps yet** — that is Task 3, and until it lands a held note survives a
  grid change but not a save. Cost if wrong: the restoration is session-only for one commit; the next task closes it.
- **Task 3 — the file carries what it holds.** `PatternFile` grows `held`, written as the steps above `len()` up to the
  last one that is **not a rest** and skipped entirely when empty — so a pattern with nothing held writes exactly the
  bytes it wrote at version 13 (the first cut wrote the trailing rest slate and the version-nine mask test caught it,
  4096 where 2048 was owed). The deserialiser restores `held` above `len()`, where it does not play. `SAVE_VERSION`
  **14**, pin renamed and asserting it. Proved by `a_held_note_travels_through_the_file_and_plays_again` (narrow → save →
  load → widen equals the original, and an untouched pattern has no `held` key) and the version-nine/ten/eleven upgrade
  tests, which pin the old files' shape. Suite **984/984**, clippy `-D warnings`, `fmt --check`.
  **Ruling: `held` is truncated at the last non-rest step.** A held *rest* is nothing to carry, and writing the slate
  would make every untouched pattern's file longer for no music — the cost of the naive version, caught by
  `a_version_nine_file_upgrades_with_empty_drum_masks`. Cost if wrong: a hand-edited file that lists a held rest gets it
  dropped, which is no loss.
- **Task 5 — the book, the pin.** `23-pattern-grids`'s "What a press does to the steps outside the new bar" now says the
  outer steps **stop playing and are kept**, that `HELD` is not `lost`, and that a wider grid plays them again — with the
  notice quoted as `32 TO 12 STEPS, 2 HELD`. `check.sh` pins `HELD` and pins `CLEARED` gone. `./docs/guide/check.sh` →
  `guide: ok`. Suite **984/984**, clippy `-D warnings`, `fmt --check`.

**Branch review (the PR), round 02's share.** Two real findings, both fixed here.
- **P1: `RANDOM` destroyed held steps.** `randomised` copied the slot table, rolled only the bar, then rebuilt through
  `from_steps_in`, which rests everything above `len()` — so one press on a narrowed pattern lost the held notes and the
  next save wrote them gone. It now keeps the pattern and writes only the bar; proved by
  `a_reroll_keeps_the_steps_a_narrow_grid_is_holding` (narrow → randomise → the note is still held → widening plays it).
- **P1: four doc comments still said a grid change "rests"** what it holds, including `Pattern`'s own note that *every*
  slot above `len()` "is a rest, always" — the invariant this round reversed. All rewritten around "not played"/"held".
- **P2: `sounding_steps_beyond` renamed `held_steps_beyond`**, which the plan asked for and the notice already assumed.
