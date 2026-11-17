# Plan 04 — a device in time, and what the whole-branch read found

> **For agentic workers:** post-merge round on `phase-3`, taken on chore branch `chore/post-merge-fixes` cut from `main` after
> PR #4 and #5 were squashed in. `superpowers:executing-plans`, reviewed by a dispatched subagent per commit.

**Goal:** a device added while the rack is playing waits for the bar line and starts on it, in step with every other device;
and the findings of PR #4's second whole-branch read are worked down.

## What is wrong today

**The device that will not wait.** `Rack::add` fans the rack's transport out, and `Mixer::reconcile` hands a new slot the
reference device's published step (`AudioRenderer::seek`), so a device added mid-bar starts *at once*, on whatever step the
others are on. Its clock restarted, so it is a step out of phase with the rack until the bar wraps, and its first bar does not
line up with the bar the others are in. The book says so out loud
(`docs/guide/src/18-a-rack-of-devices.md`: "a device added while the line is running comes in on the step the others are on
rather than at the top of the bar") — that sentence is the bug written down. The player asked for the opposite: **wait for the
current cycle to finish, then start in sync.**

**The read's findings.** PR #4's second whole-branch read returned BLOCK: no P0, six P1, eleven P2. Three of the P1s are
changes the plans record as shipped and that are not in the tree —

| | What is wrong | Where |
|---|---|---|
| F1 | the first page a player reads says a pause "begins from the top of the bar" — that is `STOP`'s rule, on a stranded `pause` / `stop:` line | `docs/guide/src/03-first-sound.md` |
| F2 | a direction asked for mid-bar is lost by a stop, although `01`'s Progress says `stop` takes the pending direction | `acid-core/src/sequencer.rs` |
| F3 | a tempo moved from the arrow keys (or a learned fader) never marks the session unsaved; only the deck's stepper does | `ui/mod.rs`, `main.rs` |
| F4 | the help card still says "saved on exit and 2 s after a change"; the 2-second write is `AUTOSAVE`-only and off | `ui/help.rs` |
| F5 | `img/README.md` still has `SONG New song` **on its lit plate**, `RUN` lit, "click `SONGS`", and a bottom row that never existed — and `check.sh`'s guard for "on a lit plate" misses "on **its** lit plate" | `img/README.md`, `check.sh` |
| F6 | the page that names the face names the song twice and never names the tape deck or the tempo stepper | `docs/guide/src/04-the-face.md` |

and the eleven P2s: a `_` arm back on `TapeKey::lit`; the transpose well not scaling while every key beside it does; two
comments describing the tempo pair as flanking a number it is stacked beside; an empty heading left in `13-settings.md`;
`UNSAVED` in backticks on page 12; one `TBC` named where the face engraves two; `CLEAR` missing from the performance row's own
page; three botched comment edits in `draw_panel`; a hand-kept positional list in `tape_live`; the capture recipe starting
`Xvfb :90` and running the app on `:82`; and `banks.rs`'s ids still `song-*` while the handover records them renamed.

## The design

### A device joins on the bar line

The bar is `STEPS_PER_PATTERN` steps, and every playing device is in step with every other, so **the reference device's bar
line is the rack's bar line**. The joining device has to stay silent until it, then begin step 0 on the same sample.

- **The bar is announced.** `Sequencer` records, per `render`, the sample offset at which step 0 began: `bar_line: Option<usize>`,
  set when the position wraps to 0 and when an arm lands on step 0. `Engine` passes it through.
- **A start can be delayed inside a block.** `Sequencer::start_later(step, silence)` renders `silence` samples first and arms
  at that offset, so a call at the top of a block can begin a step mid-block. It overrides any start already issued this block
  (`running = false`, the pending trigger cleared), the way `start_at` overrides `start` today.
- **The mixer does the timing.** `Slot` gains `waiting`. `reconcile` marks a new slot at index > 0 `waiting` when the reference
  is **Playing** (still `seek`ing to the held step when the reference is **Paused** — a paused rack has no bar to wait for, and
  the join must keep the step the resume will use). A waiting slot is not rendered at all, so its engine never starts. In
  `render_mono` the reference slot renders first; if it reports a bar line at offset `at`, every waiting slot is released with
  `start_later(0, at)` in that same block. Sample-exact: both devices' step 0 lands on the same sample.

Why not seek the new device to the current step (what it does now): that is what puts its bar out of phase, which is the bug.
Why not wait a whole block: a block is up to `MAX_BLOCK` samples and typically 10–20 ms of the device's buffer, so block
granularity would flam the downbeat against the rest of the rack.

Nothing new is saved and no atomic is added: the waiting is the mixer's own state, and a restart forgets it, like the transport.

### The findings

Fix them where they are, in four groups, with the record corrected to match the tree:

- **The two rules with no gate** (F2, F3): `Sequencer::stop` takes the pending direction; the tempo's `dirty` moves to where
  the tempo actually changes (`main`'s `NudgeTempo` arm), so the arrow keys and a learned fader dirty the session too.
- **The book** (F1, F4, F5, F6, F10, F11, F12, F13, F16): the sentences, the five `img/README.md` rows, the orphaned apology,
  the empty heading, and the two pages that undercount the row. `check.sh` gains the facts and the **negative** guards that
  would have caught them — `on (its )?lit plate` is the one that was already there and did not.
- **The small code ones** (F7, F8, F9, F14, F15): the `TapeKey::lit` arm written out; the transpose well painting at the
  row's own scale; the two comments that put the tempo's `+`/`−` either side of a number they are stacked beside; the three
  botched comment edits in `draw_panel`; `tape_live`'s positional list replaced by the catalogue.
- **F17** is a record, not a rename: `banks.rs`'s ids stay `song-*` on purpose (an `egui::Id` is behaviour and renaming them
  buys nothing a player sees), and the Progress says so instead of claiming `bank-*`.

## The tests that make it safe

Through `Mixer::render_mono`, a real rack, real blocks (`tests/mixer.rs`'s pattern):

1. `a_device_added_mid_bar_waits_for_the_bar_and_lands_on_it` — play, render until the reference is a few steps in, add a
   device, then render block by block: the new device's published step stays at 0 for every block before the wrap (**the old
   behaviour jumped it to the reference's step — that assertion is the regression**), and in the block the reference wraps, the
   two published steps are equal and advance together afterwards.
2. `a_device_added_to_a_paused_rack_still_joins_the_held_step` — the existing pause rule, unchanged: the joiner takes the
   paused step and stays silent.
3. `the_joining_device_makes_no_sound_before_the_bar` — the mixed output of a block before the wrap equals the mix of the same
   rack without the second device (a silent slot adds nothing), so "waiting" is silence and not a muted-sounding note.
4. In `acid-core`'s sequencer tests: `a_delayed_start_begins_on_the_sample_it_was_told_to` (`start_later(0, 100)` renders 100
   samples of silence inside the block, then step 0) and `a_bar_line_is_reported_at_the_sample_it_happened` (the offset the
   mixer's release depends on).
5. F2's regression: `a_direction_asked_for_mid_bar_survives_a_stop` — play, set reverse mid-bar, `stop`, `start`, and the first
   bar plays reversed. F3's: pressing `ArrowUp` through `dispatch` dirties the panel.

## Tasks

1. **The device in time.** `Sequencer::start_later` and `bar_line`, `Engine` passthroughs, `AudioRenderer`'s delayed start
   (replacing `pending_seek` with one `pending_start`), `Mixer`'s `waiting` slot and the reference-first render; the mixer and
   sequencer tests above; page 18 rewritten to say a device waits for the bar; `check.sh` fact updated. **Green commit.**
2. **The two rules with no gate** (F2, F3) with their tests. **Green commit.**
3. **The book and the ledger of pictures** (F1, F4, F5, F6, F10–F13, F16), with the `check.sh` facts and negative guards that
   would have caught them. **Green commit.**
4. **The small code findings** (F7, F8, F9, F14, F15) and F17's correction of the record. **Green commit.**

## Non-goals

- **Anything on the audio path that allocates.** The waiting device is skipped, not rendered into a buffer nobody hears.
- **A per-device "join" flag in `SharedState` or the save file.** The waiting belongs to the mixer; a restart does not need it.
- **A count-in.** The new device waits for the bar; it does not announce one.
- **The performance row's catalogue.** Recorded in plan 03, not this round.
- **Renaming `banks.rs`'s `egui` ids.** F17 is fixed as a record, deliberately.

## Progress

- [x] **Baseline** — branch `chore/post-merge-fixes` from `main` at `2381118` (PRs #4 and #5 squashed in). Gates green before
      any change: 688 tests, clippy `-D warnings`, `fmt --check`, `./docs/guide/check.sh`.

- [x] **Task 1 — the device in time.** `Sequencer::start_later(step, silence)` starts a step part-way through a block and
      `Sequencer::bar_line()` says where step 0 began in the last one (including a start, so a restart releases a waiter);
      `AudioRenderer`'s `pending_seek` became one `pending_start: (step, silence)`, and `Mixer` holds a `waiting` slot that is
      never rendered and is released on the reference's bar line with `start_later(0, at)`. A paused rack keeps the
      seek-to-the-held-step rule; nothing new is saved and no atomic was added. 694 tests.
      **Mutation run:** the old rule (`Transport::Playing => renderer.seek(reference.step)`) fails
      `a_device_added_mid_bar_waits_for_the_bar_and_lands_on_it` on the first block (`joined` 4 while the reference was
      mid-bar, where the wait wants 0); reverted. Removing the trigger-branch `bar_line` would leave a waiter silent across a
      restart, and `a_waiting_device_joins_the_restart_on_the_downbeat` is the test that says so.
      **Review: OK with notes.** It traced the path and confirmed the release is sample-exact — the reference's wrap offset
      inside a block is the offset the joiner's first step begins on, and the speculative `start()` from `settle_transport` is
      overridden rather than added to. Taken from it: four doc comments that still described the deleted behaviour despite the
      change rewriting the path under them; `Sequencer::stop` now drops a delayed start (it could otherwise fire after a
      stop), since `start_later` is public; the reconcile comment on clearing `waiting` at slot 0 now says what is true (slot 0
      is rendered whatever the flag says); and three tests were made able to fail on what they are named for — the delayed
      start's block now asserts the step *sounds* from the sample it was given (before, it passed if the start never
      happened), the bar-line offset is asserted exactly (`at == 1`: a sixteenth at 130 BPM rounds to 5 538 samples, so
      sixteen of them run a sample past sixteen blocks), and the mixer's mid-bar test uses 500-sample blocks against a
      2 400-sample step so the release lands *inside* a block; the lockstep premise of the silence test is now asserted rather
      than assumed. Also taken: the guide's strip paragraph gained the lamp clause the review noticed (a waiting device's cell
      is lit while its sequencer has not run).
      **Ruling:** the `check.sh` guard for page 18 lands with task 3's guards rather than here — the file also carries that
      task's four new guards and staging a line at a time would be a second way to write it. Cost if wrong: one commit where
      the sentence is pinned by no gate, with the push carrying both.
      **Recorded, not fixed:** a start can still be dropped if the transport flips to `Stopped` between the reference's render
      and the waiter's in the same block (`renderer.rs` takes `pending_start` and then tests the transport). Microsecond-wide,
      and the honest fix is the renderer reporting whether it started, or the mixer re-marking the slot — a design change, not
      a line. It costs one bar of the old out-of-phase bug in that window.
