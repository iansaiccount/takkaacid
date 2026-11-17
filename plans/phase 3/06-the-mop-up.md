# Plan 06 — the mop-up: every recorded-not-fixed bit

> **For agentic workers:** round 6 of phase 3, on branch `chore/performance-row-catalogue` (the same PR as plan 05 — the player
> asked for a mop-up, not a new branch). `superpowers:executing-plans`, reviewed by a dispatched subagent per commit.

**Goal:** pay down every debt the phase's plans recorded and did not fix, so the PR leaves no "known and recorded" list behind
except the two things a face or a round of its own owns.

## What is wrong today

The plans have been honest about what they left, and the list has grown:

| # | Where | What is still true |
|---|---|---|
| 1 | `tests/learn_live.rs` | `the_shuffle_knob_stays_on_the_panel_while_a_learn_is_choosing` asserts only `CursorIcon::ResizeVertical` — the density and scale dials would satisfy that too, so a wrong-but-still-a-knob name would pass |
| 2 | `ui/tips.rs:107` | the control catalogue's variant is still `Control::Songs` while the key it names is engraved `BANKS` |
| 3 | `ui/tape.rs:89`, `ui/dispatch.rs:412` | both have a `"NOTHING TO DO"` arm that no click can reach: dead words kept only to make a `match` exhaustive |
| 4 | `Rack::set_transport` | a `PAUSE` from `Stopped` is refused and says nothing — a silent dead key, which this repo calls broken |
| 5 | `ui/transport.rs:97` | `TRANSPORT_CELLS` has no production reader since the row asked the catalogue; it is a second name for `Cell::ALL.len()` |
| 6 | `ui/mod.rs:729` | the `COPY` key encodes the whole pattern to a `String` **every frame** to decide whether it is holding it |
| 7 | plan 04 | a tempo moved by a **learned fader** does not dirty the session: the CC handler writes the tempo cell from midir's thread and no `Request` exists to carry it |
| 8 | plan 04 | a start can be dropped if the transport flips to `Stopped` between the reference's render and the waiter's in one block: the slot stops waiting, the start is thrown away, and the device plays from step 0 under a rack that resumed mid-bar |
| 9 | `tests/knob_live.rs` | two helpers find a knob by bare index |
| 10 | `AGENTS.md:10,119` | the standing instruction still says `plans/phase 2/` is the round in flight |

## The design, item by item

1. **The assertion, not the cursor.** After clicking `SHUFFLE` while a learn is choosing, the panel must be in
   `Step::Listening(CcTarget::Swing)` — the same assertion `learn_live` already makes for the tempo fader, and the one the
   cursor check was standing in for. The cursor check stays as the "the knob is there" half.
2. **`Control::Banks`.** Rename the variant and its tip; the caption the key prints is already `BANKS`.
3. **One honest answer per key.** `TapeKey::notice() -> Option<&'static str>` (the three unwired keys) and
   `dispatch::not_wired(shortcut) -> Option<&'static str>` replace both `"NOTHING TO DO"` arms. `None` means "this key is
   wired; its notice never comes from here", which is true, where `"NOTHING TO DO"` was a string pretending to be one.
4. **A pause with nothing to pause says so.** `transport::pause(rack) -> bool` in the module that owns `Request`: it pauses a
   playing or paused rack and answers `false` for a stopped one, and `main` turns the `false` into
   `transport::NOTHING_TO_PAUSE`. In the lib so it can be tested, the way `apply_nudge` is.
5. **`TRANSPORT_CELLS` deleted.** `Cell::ALL.len()` is the width, and its own test no longer restates it.
6. **`COPY`'s text cached.** The panel keeps the current pattern's text beside the `Arc` it was encoded from, so a frame costs
   one pointer compare and the encoding happens when the pattern changes. Panel state only — nothing is saved.
7. **A learned tempo dirties the session.** `SharedState` gains `tempo_written: AtomicBool`, set by the CC handler's tempo arm
   (the MIDI thread may not touch `PanelState`), and `draw_panel` takes it once a frame and marks the panel dirty. The panel is
   the only reader, so a tempo a fader moved is written by `AUTOSAVE` like one the keys moved.
8. **A start that could not happen keeps waiting.** `AudioRenderer` puts a pending start back when its own transport says
   `Stopped` instead of dropping it, and `Mixer` re-marks a slot `waiting` when its release did not take. The device then joins
   on the next bar line rather than playing from step 0 under a rack that resumed mid-bar.
9. **Knobs by name.** `knob_live`'s two helpers take a `Knob` and use `Knob::index()`.
10. **The standing instruction.** `AGENTS.md` names `phase 3/` as the round in flight and gives the plan path as
    `plans/phase 3/NN-topic.md`.

**The one thing that stays recorded, and why:** the legend test cannot see a `Tbc`/`Spare` swap, because the face engraves both
cells `TBC` and the player asked for exactly that. The only fixes are a face change (a different caption for the spare) or a test
that reads `egui`'s per-widget ids, which the panel does not expose. It stays in the Progress as inherent, with that reason.

## The tests that make it safe

1. `learn_live`: the shuffle click lands on `CcTarget::Swing` (item 1) — the assertion that would fail for a wrong name.
2. `transport::pause` unit test: a playing rack pauses; a stopped one is left alone and answers `false` (item 4).
3. `Rack`/`transport` test: the notice's words exist and are ASCII (item 4's string is the face's, and a glyph there is a bug).
4. A live test: a CC tempo write, then one frame of `draw_panel`, and the panel is dirty (item 7) — and a second frame with no
   write leaves it alone, so the flag is taken and not re-read.
5. `mixer` test: a slot whose own transport says `Stopped` when its bar line arrives stays `waiting` and silent, and joins on the
   next bar line when the transport is playing again (item 8).
6. `tape_live`/`dispatch` tests keep their current words for the three unwired keys; a wired key's notice is `None` (item 3).

Mutation checks: revert item 4's refusal to a silent `set_transport` → the pause test fails; remove the `draw_panel` fold → the
fader test fails; drop the renderer's put-back → the mixer race test fails; give `pattern_text` the old per-frame call → nothing
fails (it is a cost, not a behaviour) and that is said in Progress.

## Tasks

1. **The panel's small debts.** Items 1–6 and 9, with tests 1–3. **Green commit.**
2. **The two audio rules.** Items 7–8, with tests 4–5. **Green commit.**
3. **The record.** Item 10, this plan's Progress, and the inherent `TBC` note. **Green commit.**

## Non-goals

- **A face change.** Nothing here moves a cell, a caption the player reads, or a screenshot — said explicitly because item 2
  renames a *catalogue variant*, not the engraving, and item 4 adds a notice, which is the status line's business.
- **Renaming `banks.rs`'s `egui` ids or `PanelState::songs`.** Decided in plan 04: an id is behaviour and the field is the
  document.
- **A save-file change.** Nothing here touches `SavedState`; the tempo flag and the copy cache are session state.

## Progress

- [x] **Baseline** — the branch plan 05 was written on, gates green at `82ca0eb`: 701 tests, clippy `-D warnings`, `fmt --check`,
      `./docs/guide/check.sh`.

- [x] **Task 1 — the panel's small debts.** (1) `the_shuffle_knob_stays_on_the_panel_while_a_learn_is_choosing` now clicks
      the knob and asserts `Step::Listening(CcTarget::Swing)` — the cursor check said only that *a* knob was under the pointer,
      which the density and scale dials would satisfy too. (2) `tips::Control::Songs` is `Control::Banks`, as the key is
      engraved. (3) `TapeKey::notice()` and `dispatch::not_wired` answer `Option<&str>`, so `"NOTHING TO DO"` is gone from both:
      `None` means "this key is wired; its notice never comes from here", which is what was true all along. (4) a `PAUSE` with
      nothing to pause says `NOTHING TO PAUSE`: `transport::pause(rack) -> bool` is the refusal, in the library so it has a test,
      and `main` turns the `false` into the notice. (5) `TRANSPORT_CELLS` deleted — no production reader, and its own test's
      assertion was a second name for `Cell::ALL.len()`. (6) the `COPY` key's pattern text is cached beside the `Arc` it was
      encoded from, so a frame costs one pointer compare instead of a sixteen-step `String`. (9) `knob_live`'s two drag helpers
      take a `Knob`. 702 tests.
      **Mutation run:** deleting `transport::pause`'s refusal (letting a stopped rack take a pause) fails
      `a_pause_with_nothing_to_pause_answers_so`; the copy cache has no behavioural test because it changes a cost and not an
      outcome — the clipboard tests stay green, which is the honest coverage for it.

- [x] **Task 1's review, and what it changed.** The review returned OK with notes and two Important findings, all actioned:
      **(a) item 4 put the pause rule in two places.** `transport::pause` re-implemented `Rack::set_transport`'s refusal, so the
      rack's own rule was never exercised by the pause path and the two could drift silently in both directions — a permissive
      rack would be refused by the copy, a stricter one would be silent again. `set_transport` now **answers** (`-> bool`) and
      `pause` asks the rack; one rule, one home, and the words stay with the `Request`. **(b) `TapeKey::notice`, `not_wired` and
      the `knob_live` conversion had no test for their own function** — the plan's own test 6 was half unbuilt. Two unit tests in
      `ui/tape.rs` now pin the three unwired keys' words and that a *wired* key's notice is `None`. Taken with them: the three
      sentences lived in two modules (`tape` and `dispatch`), so they are now `tape`'s consts and `dispatch` reads them — one
      copy, pinned by the existing `tape_symbols_live` test; the `COPY` cache is keyed on `(Arc, PatternId)`, because
      `main` advances `pattern_id` to a *pending* pattern while the `Arc` stands still, so the collar could stay lit for a bar
      over a readout naming another pattern; and `knob_live`'s remaining `knobs[0]` literals became `Knob::Waveform.index()`.
      **Recorded:** the `learn_live` assertion overlaps `clicking_the_shuffle_knob_while_choosing_asks_for_a_shuffle_fader`,
      which makes the same claim — the cursor half is the part of that test that carries weight, and the new assertion is the
      one the plan asked for rather than new coverage.

- [x] **Task 2 — the two audio rules.** (7) `SharedState` gained `tempo_written: AtomicBool`, set by the CC handler's tempo arm
      and taken once a frame by `draw_panel` (`take_tempo_written` swaps it false), so a tempo a **learned fader** moves marks
      the session unsaved exactly as the keys do — it writes from midir's thread and can never touch panel state, which is why
      the flag exists. (8) a release that arrives while the device's own transport says `Stopped` is no longer dropped: the
      renderer puts the pending start back, `has_pending_start()` reports it, and the mixer re-marks the slot `waiting`, so the
      device joins on the next bar line instead of playing from step 0 under a rack that resumed mid-bar. 706 tests.
      **Mutations run:** deleting the `take_tempo_written` fold fails `a_tempo_a_controller_moves_marks_the_session_unsaved`
      (`the tempo is in the save file`); reverting the renderer to drop the start fails
      `a_release_that_arrives_while_the_device_is_stopped_keeps_it_waiting` (`block 4: it started before the bar line`). Both
      reverted.
      **One commit rather than two:** `ui/mod.rs` carries both tasks (the copy cache and the tempo fold), and staging one file
      twice to split them would be a second way to write the same tree. The panel's debts and the audio rules land together.

- [x] **Task 3 — the record.** `AGENTS.md` names `phase 3/` as the round in flight and gives the plan path as
      `plans/<the phase in flight>/NN-topic.md`, so the next session is not told to write to a finished phase.
      **Stays recorded, and why:** the legend test still cannot see a `Tbc`/`Spare` swap. Both cells are engraved `TBC` at the
      player's request, and the only routes to a test are a face change (a different caption for the spare) or reading `egui`'s
      per-widget ids, which the panel does not expose. Nothing else from the phase's plans is left open.

- [x] **The gate.** `cargo test --workspace` 706 passed, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo fmt --check`, `./docs/guide/check.sh` → `guide: ok`. Nothing a player sees moved: no cell, no caption they read,
      no screenshot; the one new sentence on the face is a status-line notice.

- [x] **The final review's findings, and what they changed.** It returned **BLOCK** on two claims the diff made false, and both
      were the round's own subject — a record that had stopped being true:
      **Important — `apply_nudge`'s doc still said a learned fader could not dirty the session**, which is exactly what item 7
      makes false; it now points at the road that does (`SharedState::tempo_written`, taken by the panel).
      **Important — `tempo_written` was per device but the tempo is rack-wide.** A device hearing its own channel could move the
      tempo while the panel drew *another* device, and the flag would sit on a state nobody drained. `transport::take_tempo_moved`
      now drains every device's flag and `main` folds the answer once a frame; `draw_panel`'s fold stays as the drawn device's
      path (both are takes, so whichever runs first wins and the other costs one atomic). Taken with it, all cheap: `rack.rs`
      now asserts the `bool` the rack answers, so the rule is pinned where it lives rather than only through `pause`;
      `dispatch` gained the `not_wired` test for its `None` half (unreachable from a keystroke, which is why nothing else says
      it); `tape`'s `notice` doc no longer points at `ui::tips` for words that live four lines above it; the `COPY` press uses
      the frame's own encoding and the `holding` compare no longer has a dead `unwrap_or_default`; and page 09 now says a
      `PAUSE` with nothing to pause says so, with `NOTHING TO PAUSE` added to the guide's facts. **Recorded:** a removal can
      promote a slot that is waiting to slot 0, whose kept start would be consumed at the block's head rather than on a bar
      line — reachable only in that window, and noted in `mixer.rs` rather than managed.
- [x] **The gate, after the review's fixes.** `cargo test --workspace` 707 passed, clippy `-D warnings`, `fmt --check`,
      `./docs/guide/check.sh` → `guide: ok`.
