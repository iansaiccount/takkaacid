# Plan 02 — the card in the order a player reads it

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** the `BANKS` card's `SONGS` list sits directly under the name field and the two keys that save it, so the one document
whose field is at the top has its list at the top too. Everything else keeps its place, and `NOW:` stays in the header.

**Where the design came from:** the conversation that approved it (2026-11-25), and this phase's spec —
[`2026-11-25-rack-documents-design.md`](../../docs/superpowers/specs/2026-11-25-rack-documents-design.md), *The card*. **No new
spec**: this moves controls that already ship, and the design is the order itself, below.

## What is wrong today

- **Three documents, two idioms.** `PATTERN BANKS` and `RACKS` each draw their rows and then the name field and key that save
  them (`banks.rs:216-330`, `:390-470`). The song's field and its `SAVE` / `SAVE A COPY` are the first thing under the title
  (`banks.rs:130-190`), while its rows are the **fourth** list down — under `DEMO BANKS` and `PATTERN BANKS` (`banks.rs:340-380`).
- So a player types a song name at the top and then looks past two sections to find the songs it names. The player reported
  exactly this: "the songs are listed after the banks but the song name box is at the top".
- The order was two defensible decisions that read as one accident: phase 5 put `PATTERN BANKS` above `SONGS` on purpose ("a
  bank is what you reach for while making a tune"), and the song's field stayed at the top because the song is the card's
  subject — the other half of the `NOW:` line.

## The design

The card, top to bottom, after this round:

```
BANKS                                   NOW: New song - UNSAVED
[ song name ..................... ]  [ SAVE ]  [ SAVE A COPY ]
SONGS           rows, each with an x/SURE cross
DEMO BANKS      the seven demos
PATTERN BANKS   rows, [ bank name ]  [ SAVE BANK ]
RACKS           rows, [ rack name ]  [ SAVE RACK ]
NEW   IMPORT   EXPORT                                            CLOSE
```

- **Only the `SONGS` block moves** — its heading and its rows, from after `PATTERN BANKS` to directly under the song field.
  `DEMO BANKS`, `PATTERN BANKS`, `RACKS`, the footer and the one-card rule are untouched.
- **`NOW:` stays in the header.** It is the session's state, not the list's, and the field beside it already prints the name;
  moving it down would say the name twice and leave the header holding the word `BANKS` alone.
- **The one cost, accepted by the player:** the card has no scrolling and draws what fits, so a long song list can now push the
  demo rows off the bottom, where today the demos are safe because they come first. The rule the card already follows is "the
  card lists what fits rather than promising rows it cannot show" (`docs/guide/src/12-songs-and-demos.md`), and it now bites
  the demos first. Capping the song rows to protect them would be a rule nobody asked for.
- **Nothing else changes:** the arm keys (`song:{name}`, `bank:{name}`, `rack:{name}`), the ids (`song-{index}`, `bank-{index}`,
  `rack-load-{index}`), the two-click guards, the field ids and the footer are all as they were.

## Tests that make it safe

- **`tests/banks_live.rs`** — the card's geometry helpers mirror `paint`'s arithmetic, so they are the test of the order: put
  the section chain in the new order and the existing clicks prove it. If the code still drew `DEMO BANKS` first, a click at the
  song row's centre would land on a demo row and ask `Request::LoadDemo(0)`.
- **One new test** — `the_song_list_sits_under_the_name_that_saves_it`: with one song and one demo, the song's first row is one
  heading under the field row, and a click there asks `Request::Load`; the demo row is below it.
- **The book and the picture** — page 12's list order, page 20's key list, and `banks-card.png` retaken.

## Tasks

### Task 1 — the songs list moves up

**Files:** modify `rust/crates/acid-app/src/ui/banks.rs`; test `rust/crates/acid-app/tests/banks_live.rs`.

**Interfaces:** consumes `load_row`, `guarded_load`, `section`, `song_row_centre`; produces no new names — the section's
position is the change.

- [ ] **Step 1** Put the test's section chain in the new order: `songs_top()` is the first section under `field_y()`,
      `demos_top()` follows the song rows, `banks_top()` follows the demos, `racks_top()` follows the bank rows and the bank's
      save row. Add `the_song_list_sits_under_the_name_that_saves_it` as described above.
- [ ] **Step 2** Run `cargo test -p acid-app --test banks_live` and watch it FAIL: the song click asks `LoadDemo(0)`.
- [ ] **Step 3** Move the `SONGS` heading and its rows block in `paint` to directly after the song field's row, before
      `DEMO BANKS`. Change no other line of the painter.
- [ ] **Step 4** Run the test file, then the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(panel): the songs list sits under the name that saves it"`.

### Task 2 — the book and the picture

**Files:** modify `docs/guide/src/12-songs-and-demos.md`, `docs/guide/src/20-reference.md`,
`docs/guide/src/img/README.md`; retake `docs/guide/src/img/banks-card.png`.

- [ ] **Step 1** Reorder page 12's four lists: `SONGS` first (under the field that saves it), then `DEMO BANKS`,
      `PATTERN BANKS`, `RACKS`. Keep every sentence; only the order and the one sentence that says the song's field is "at the
      top" if any move.
- [ ] **Step 2** Page 20's browser line names the sections in the card's order too.
- [ ] **Step 3** Take `banks-card.png` again: `DISPLAY=:N xdotool getdisplaygeometry` must say at least `2000 1400` **before**
      the launch, `getwindowgeometry` must say `1960x1340`, a fresh `TAKKA_ACID_DATA_DIR`, the pointer parked away from the
      panel, and the card showing `SONGS` above `DEMO BANKS`. Record the row in `img/README.md` with the state and the command.
- [ ] **Step 4** `./docs/guide/check.sh` → `guide: ok`; gates; commit — `-m "docs: the card's lists in the order they are drawn"`.

## Non-goals

- Moving `NOW:`, or merging the song's field into its section.
- Capping the song rows, adding a scroll, or any other change to what the card can hold.
- Reordering the rest of the card (the player chose the smallest fix over ordering by scope).
- The arm keys, ids, guards or footer.

## Review focus

1. **Only the songs block moved.** A diff that also reorders the demos, the banks or the racks has done more than the round.
2. **The geometry helpers moved with the code, and the click tests are what prove it.** A helper chain that trails the painter
   would pass its own clicks against any order.
3. **`SONGS` is the first list and `DEMO BANKS` the second** — including on a first run, where `SONGS` draws a heading and no
   rows.
4. **The book's order is the card's order**, and the picture shows it.

## Progress

*(One line per task, written as it is done: what landed, the test that proved it, and any ruling with what it costs if
wrong. The branch review's findings and what was done about each go here too, before the pull request.)*
- **Task 1 — the songs list moves up.** `paint` draws `SONGS` immediately under the song field and its `SAVE` / `SAVE A COPY`,
  before `DEMO BANKS`; the demos', banks' and racks' blocks are untouched, `NOW:` stays in the header, and the two comments
  that said "the demos first" and "then the player's own songs" now say what the card does. Proved by
  `the_song_list_sits_under_the_name_that_saves_it` — watched fail first: with the test's section chain in the new order, the
  song row's click asked `LoadDemo(0)`, because the painter still drew the demos first. Suite **1000/1000**, clippy
  `-D warnings`, `fmt --check`.
  **Ruling: the card's geometry is now in two test files, and that is recorded rather than fixed here.** `tests/songs_live.rs`
  keeps its own hand-rolled helpers for the song and demo rows, and this task had to move them too
  (`songs_top`, and `demo_row_centre` gained a `count` parameter so the demo offset follows the song rows). Two copies of one
  card's arithmetic is the hand-kept list `AGENTS.md` warns about; the fix is a `tests/common/` module both files include, and
  it is a refactor of two test files that this round's plan did not carry. Cost if wrong: the next card change has to be made
  in two places, and the second one will be found by a failing test — as this one was.
- **Task 2 — the book and the picture.** Page 12's four lists are in the card's order (`SONGS` first, with the sentence "`SONGS`
  is the first list on the card, directly under the name field that saves the song it names"), the cross paragraph now names all
  three lists that have one, and page 20's browser line is in the same order. `check.sh` pins the new sentence. `banks-card.png`
  retaken on a fresh `Xvfb :88` (`getdisplaygeometry` → `2000 1400`, window `1960x1340`, a fresh `TAKKA_ACID_DATA_DIR`, `BANKS`
  clicked at `762 561` logical) and its `img/README.md` row records the state and the command. `./docs/guide/check.sh` →
  `guide: ok`, suite **1000/1000**, clippy `-D warnings`, `fmt --check`.
  **Ruling: the picture shows `SONGS` as a heading with no rows.** A row would be a song file the photographer made, and a guide
  picture is taken on a fresh data directory; the adjacency the round is about is visible as the heading's position, directly
  under the field. Cost if wrong: the book has no picture of a filled song list.
  **Ruling: the pinned sentence sits on one line.** `check.sh`'s positive pins are raw `grep -F`, not the flattened read the
  negative guards use, so a phrase the page wraps is not found — the first version of the sentence wrapped and the pin failed.
  Cost if wrong: none; the page's wrap is chosen by the pin, which the comment above the pin does not say.
