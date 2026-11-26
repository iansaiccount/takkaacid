# Handover — phase 8 in flight: rounds 03 and 06 done, 04 and 05 next

Written 2026-11-26, at the end of the session that finished round 03 (merged) and round 06 (in review). Read this, then
`plans/future/wishlist.md`, then the plan you are picking up. `AGENTS.md` outranks everything below.

## Where the branch stands

`main` is at **`a91a2f2`** — phase 8 round 03, squash-merged as **PR #16** on top of `eee93c3` (PR #15: the rack file and the
card's SONGS list). Two pull requests are **open against `main`** and neither is merged:

- **PR #17 — phase 8 round 06, "the cards explain themselves".** Head `b5b3ddd`, cut from `main` after #16 merged. Eight commits:
  the plan, then one per task, then the Progress, then the review's fixes. **Gates on its head: 1045 tests, clippy
  `-D warnings`, `fmt --check`, `guide: ok`.** Its branch review has been done — a subagent that wrote none of it read the whole
  diff and reported one Important and five Minor; all six are fixed and recorded in the plan's `## Progress`, and the reviewer's
  verdict was *OK with notes*. Nothing needs re-reviewing here: merge it.
- **PR #18 — "the grid swap keeps the step it was on".** Head `67b9916`, branch `chore/pin-the-grid-swap`, one commit. A test
  and a doc comment, **no behaviour change**: it closes the item every phase-8 PR has carried as "still the player's" by pinning
  the current choice and writing down the alternative and its cost. Gates on its base: **1026 tests** (main's 1025 plus the new
  one), clippy, `fmt`, `guide: ok`. It is not reviewed — it is a test and a comment, and the review rule bites hardest on
  behaviour; if you disagree, review it as you would any branch.

Merge them in **either order** — #17 touches `acid-app`'s cards, tests and the guide, #18 touches `acid-core`'s sequencer and one
engine test, and they share no file. After both, `main` carries rounds 03 and 06 plus the pin.

Local branches: `main` (fast-forwarded to `a91a2f2`), `phase-8-round-06` (tracks origin), `chore/pin-the-grid-swap`. Deleted this
session: the stale local `phase-8`, and `phase-8-round-03` (merged). On the remote, `origin/phase-8-round-03` went with the merge
and **`origin/phase-7` is a leftover** whose commits are all in `main` — it can go, and it was not deleted because the player only
authorised `handover-phase-8` (which is gone).

## What shipped since the last handover

**Round 03 — the card as a browser (merged, #16).** `ui::banks::paint` is one searchable, kind-filtered, scrolling list: a search
field (`banks-search`), five chips (`banks-filter-{n}`), one band of rows each carrying its kind, and a strip whose field and keys
follow the chip. The four stacked lists, `section`, `list_limit`, `bank_rows_limit` and `rack_rows_limit` are gone; the save keys
are fixed furniture, which is the answer to round 02's finding that twelve songs took `SAVE RACK` off the card. The card's
arithmetic is a **public `ui::banks::Rects` plus `card_rects`** — the painter draws from it and `tests/common/mod.rs` clicks from
it, and the 31 hand-rolled helpers in the two live test files are deleted. `Filter::door() -> Door` is the one answer to "which
strip"; `Kind::cross` answers a row's id and its delete together.

**Round 06 — the cards explain themselves (in PR #17).** Every key on `BANKS`, the device card, the kit browser and the picker
explains itself on hover, and the keys that write a document say **which**: the strip's line comes from
`persist::save_target` — the function `main` uses — so `SAVE BANK` says `Makes a new file, Easy 2.` or `Writes Easy in place.`
`ui::tips` grew from 49 to **66** entries (appended, the drum face's slice moved to `[40..49]`) and one formatter,
`text_with(detail)`; `text_for`/`hover` stayed as wrappers so no existing surface churned. `Tip` keeps its three `&'static str`
fields and the table stays a table. Fields, chips, rows and engravings stay silent on purpose.

**The grid-swap pin (in PR #18).** `Sequencer::set_pattern` keeps the step **number** across a grid change; the alternative
(keeping the fraction of the bar) is written beside it with its cost, and
`acid-core/tests/engine_transport.rs::a_swap_into_a_longer_bar_keeps_the_step_it_was_on` pins the choice.

## Things that will cost you time if you do not know them

- **`Pattern::from_steps` silently caps at one sixteenth bar.** A test that builds a 32-step pattern with it gets 16 steps and
  never changes the grid — which is exactly what happened here, and only a mutation check caught it. Use
  `Pattern::from_steps_in(grid, steps)`.
- **egui paints one tooltip a frame — the first one asked for.** The panel is painted before a card, so a knob under a card wins
  the tooltip unless the panel goes quiet. `PanelState::help()` is `tooltips && !self.a_card_is_up()`: the panel's sixteen tip
  sites (seven modules) ask that, the cards ask `tooltips`. Put a panel surface on `tooltips` and it steals the card's hint;
  put a card on `help()` and it silences itself.
- **`tests/common::hover(ctx, panel, shared, screen, at)` takes the screen it paints at** — 980 × 560 in the card tests, the
  design size in `device_card_live` and `kits_live`. It starts with a still frame (egui will not start a tooltip's delay on the
  frame a click was released) and continues the clock from `ctx.input(|i| i.time)` (a frame whose time goes backwards panics).
- **A hover leaves its hint on screen for a frame or two.** In a test, assert *absence* before any hover in the same test, or you
  are reading the previous key's tip.
- **Mutation checks earn their keep.** Three of round 06's tests passed without their code, and one never changed the grid at all.
  If a test is cheap to write, break the line it covers and watch it fail.
- **Round 06 changes what round 04's new key must do.** A `DUPLICATE` key on the device card is a card key, so it needs a
  `Control` and a `Tip` — `Control::ALL`/`TIPS` become 67, appended after the cards' seventeen, and the drum face's `[40..49]`
  slice stays as it is. The same goes for any key a later round adds to a card.
- **`cargo test --workspace | head` dies of SIGPIPE** — redirect to a file and sum the `test result` lines.
- **The shell blocks heredocs into `python3`/`perl` and `xargs | sh`**, and needs `lean-ctx allow` for tools like `identify`.
  Write a script to a file, or use the edit tools. `.superpowers/` is git-ignored scratch; this session's ledgers were
  `.superpowers/sdd/06-cards-explain-themselves/` (deleted once the review was recorded in the plan).
- **`git push --force-with-lease` after a rebase** — a rebased branch rejects a plain push, and the lease is what makes it safe.
  **`gh pr edit` fails** with GitHub GraphQL errors; `gh api -X PATCH repos/iansaiccount/takkaacid/pulls/<n> -F body=@file` works.
- **Screenshots**: `DISPLAY=:N xdotool getdisplaygeometry` must say at least `2000 1400` *before* the launch (something pre-starts
  Xvfb at 1280×800), then `getwindowgeometry` must say `1960x1340`. A tooltip picture needs the pointer resting on the key, which
  the book's own recipe forbids — so the guide has **no** tooltip picture by design, and round 06's `## Progress` records the two
  grabs it took instead.

## What is next, in order

1. **Merge PR #17, then PR #18.**
2. **Round 04 — duplicate a device** (`plans/phase 8/04-duplicate-a-device.md`, planned, not started). `Rack::duplicate` built from
   the source's own `SavedDevice`; a `DUPLICATE` key on the device card's second key row; `AddError::NoSuchDevice` beside
   `AtCapacity`; the copy's kit decoded by `main` from the source's kit name. Read round 06's Progress for what a new card key
   owes the tip catalogue.
3. **Round 05 — drag to reorder devices** (named, not yet planned). `Rack::move_device`, the strip's cell gaining `click_and_drag`
   on its existing id, the dragged device remembered by number, the selection following it. The file needs no change: it is
   already strip order.
4. **The ear check** — the one item no session has closed, because it is a listening judgement. `EAR_TESTING.md` has the table;
   the numbers below are what the demos actually render at, and the files are at `rust/earcheck/demos/` (git-ignored):
   ```
   Acid Jazz   13.74 s  peak 0.651  rms 0.2705  brightness 0.050
   Fast Acid   12.67 s  peak 0.504  rms 0.0700  brightness 0.101
   ```
   **Fast Acid sits about 11 dB below Acid Jazz.** A sparser 1/32 line with short notes is quieter by nature, so this may be
   right — but if it feels buried when you listen, levelling the demos is its own small round with a target loudness and a test,
   not a patch to a demo's bytes.
5. **`origin/phase-7`** — a leftover remote branch whose commits are all in `main`. Deleting it is the player's word.

## Still the player's

- **The ear check's judgement**, and with it whether Fast Acid should come up.
- **`origin/phase-7`**, whether it goes.
- **The grid-swap choice is now decided in writing** (PR #18) rather than merely unchanged: the step number won, the alternative
  and its cost are in `Sequencer::set_pattern`'s doc, and one test is the line to change if the other answer is wanted.

## How to work (unchanged)

One task at a time, in plan order: write the failing test, run it, watch it fail, implement, run it, keep the plan's `## Progress`
current — the test that proved it and any ruling with what it costs if wrong — and commit with the gates green
(`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
`./docs/guide/check.sh` when the book moves). The review is of the branch at the pull request, by a subagent that wrote none of
it; it reports, the executor fixes what it found and records it. Do not open a PR on the strength of the gates alone.
