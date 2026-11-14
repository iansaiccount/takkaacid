# Plan 43 — Documents, and the phase's gate

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). This plan is short by design: it is the
> paperwork that keeps phase 2's promises discoverable, plus the two checks the design asked for that no single plan owns.
> Steps use checkbox (`- [ ]`) tracking; each task ends in a green commit.
>
> **Review:** every task ends with the review gate in `00-phase-2.md` — the diff is reviewed against this task's **Files** and
> **Interfaces** blocks, and every finding is actioned, before the commit. No subagents elsewhere: this plan runs inline, in one
> session, by one pair of hands.

**Goal:** the help card, the README, `EAR_TESTING.md` and `AGENTS.md` describe the app that phase 2 built — the strip, the
device card, the keyboard, and what is not saved — and the phase ends with the gates green, the design's mutation checks
recorded, and one whole-branch review.

**Architecture:** no code. Every fact added here is read off the code, not off another document: the labels are quoted exactly
as the panel draws them, and anything the app cannot do yet is written as a limit rather than left out.

**Tech Stack:** Markdown, `mdBook` (the guide is not rebuilt — see task 5, step 3).

**Spec:** `docs/superpowers/specs/2026-11-13-device-list-design.md` — the Goals and Non-goals sections are the list of claims
this plan has to make honest.

## Global Constraints

- **No code changes.** If a document cannot be written truthfully without one, that is a bug in an earlier plan: record it in
  `## Progress` and fix it there, with its test.
- **Quote the panel, not the plan.** The strip says `MUTE`, the card says `REMOVE` then `CONFIRM?`, the picker's footnote says
  `NAMES AND EXTRA DEVICES ARE NOT SAVED YET`. Copy them from `ui/strip.rs` and `ui/device_card.rs`, not from this document.
- **A limit stated once, in the place a player meets it.** The footnote on the picker is the only place the save limit is
  explained in the UI; the README says it too, for a reader who never opens the card. Do not add a third.

## Review Focus

1. **Help that promises a shortcut that does not work.** `F3`, `+` and `Ctrl+1..4` must be listed only after plan 42's gate is
   green, and the wording must match what `dispatch` does.
2. **A README that still describes one instrument.** It is the first thing a stranger reads; "MIDI input" is now a set.
3. **A guide that quietly lies.** Eighteen pages of screenshots predate the strip. Say so, once, where the guide starts.

---

### Task 1: The help card

**Files:**
- Modify: `rust/crates/acid-app/src/ui/help.rs`
- Test: `rust/crates/acid-app/tests/help_live.rs` (the existing card-content test, extended)

**Interfaces:** no API. `help.rs` holds the key list and the NOTES block; both are plain strings the card paints.

- [ ] **Step 1: Write the failing test** in `tests/help_live.rs`:

```rust
#[test]
fn the_help_card_names_the_devices_and_what_is_not_saved() {
    let state = SharedState::new();
    let mut panel = PanelState { help_visible: true, ..Default::default() };
    let ctx = Context::default();
    let said = strings(&run_on(&ctx, &mut panel, &state, vec![])).join(" ");

    for wanted in ["F3", "Ctrl+1", "MUTE", "four devices", "not saved"] {
        assert!(said.contains(wanted), "the help card should mention {wanted}: {said}");
    }
}
```

- [ ] **Step 2: Run it to watch it fail**

Run: `cd rust && cargo test -p acid-app --test help_live`
Expected: FAIL — the card says nothing about devices.

- [ ] **Step 3: Add the lines**

Three additions, in the card's own register (short lines, ASCII, no glyphs):

- the key list gains `F3  device card` and `Ctrl+1..4  pick a device`, beside the existing `F1`/`F2` rows;
- a NOTES line for the strip: `the strip adds, picks and mutes devices; + adds one` — say `+` in words if the bundled font's
  `+` is not certain, e.g. `the key after the last cell adds a device`;
- a NOTES line for the limit: `four devices; names and extra devices are not saved yet`.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test help_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/ui/help.rs rust/crates/acid-app/tests/help_live.rs
git commit -m "docs(help): the card knows about devices"
```

---

### Task 2: The README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add the section**

After the paragraph that describes the panel, add a short section with these facts, each read off the code:

- the app holds up to four devices, one of each kind, and the strip above the panel adds (`+`), selects (click or `Ctrl+1..4`)
  and mutes them;
- `F3` opens a device's card: its name, whether it plays notes or only answers controllers, which MIDI ports and which channel
  it hears, its mute, and its removal;
- one transport over all of them, one shuffle each;
- MIDI learn is per device, so the same controller number can drive two devices' knobs;
- **`NAMES AND EXTRA DEVICES ARE NOT SAVED YET`** — the session file holds the first device only, and the wishlist's
  "save the whole rack" is where that changes.

- [ ] **Step 2: Read it back against the app**

Run the app and check each sentence against what you see. A README claim is a test that only the next reader runs.

- [ ] **Step 3: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add README.md
git commit -m "docs(readme): one window, up to four machines"
```

---

### Task 3: Judging two machines by ear

**Files:**
- Modify: `EAR_TESTING.md`

- [ ] **Step 1: Add the two-device section**

Three things a single device cannot reveal, each with what to listen for:

- **Phase.** Two devices on the same pattern must land together: add one while the transport runs and listen for a doubled
  attack rather than a flam. A flam means the joining device's seek is a step or more out.
- **Shuffle per device.** Give one device swing and leave the other straight. You should hear one machine lurching and the
  other not, *without* either drifting out of the bar — if the pair drifts, the tempo is being read twice.
- **Mute as a gain.** Mute one mid-phrase: it should fade over about a tenth of a second, not cut, and unmuting should return
  it in time — no repeat of the step it was muted on.

- [ ] **Step 2: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add EAR_TESTING.md
git commit -m "docs(ear): what two machines can show you that one cannot"
```

---

### Task 4: `AGENTS.md` names the bus

**Files:**
- Modify: `AGENTS.md` (the "Two threads, two rules" paragraph)

- [ ] **Step 1: Add the third reader**

The paragraph says the panel writes atomics and the audio callback reads them. Phase 2 added a second reader: the MIDI
handlers, on midir's thread, reading `device::BusState` through its `ArcSwap`. One sentence, in the existing paragraph's voice:

> `device::BusState` is the one thing both readers share: `Rack::publish` writes it, the mixer loads it once per block and each
> port's `Router` loads it once per message, and neither may lock or allocate around it.

Also, in the code-conventions list, add the device catalogue to the "one catalogue per kind of control" examples, because
`DeviceKind::ALL` is what the picker's rows derive from.

- [ ] **Step 2: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add AGENTS.md
git commit -m "docs(agents): the bus is a third reader, and a catalogue"
```

---

### Task 5: The phase's gate, and the checks nobody else owns

**Files:** none. This task is a verification, and it is the one that can send you back to an earlier plan.

- [ ] **Step 1: The three gates, on the whole workspace**

```bash
cd rust && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: PASS. Record the test count in `## Progress` — it is the number the next phase starts from.

- [ ] **Step 2: Re-run the two mutation checks the design asked for**

Both are in plan 39; run them again here, because a refactor since then could have made them vacuous:

1. Delete the `Arc::ptr_eq` half of `Mixer::reconcile`'s fast path. `a_rack_that_did_not_change_is_not_rebuilt` must FAIL.
   Restore, confirm green.
2. Make a muted slot's ramp target `1.0` instead of `0.0`.
2. Make a muted slot's ramp target `1.0` instead of `0.0`. Both `unmuting_a_running_device_is_audible_and_it_kept_time_meanwhile`
   and `a_device_born_muted_does_not_click_on_its_first_block` must FAIL. Restore, confirm green.

If either mutation survives, the test that was supposed to catch it is not testing what it says: fix the test in the plan that
owns it, and say so in `## Progress`.

- [ ] **Step 3: The things phase 2 promised not to change**

```bash
cd rust && cargo test -p acid-app --test persist
cargo run -p acid-app --bin takka-acid -- --midi-map | head -5
```

Expected: the persist pin passes with no edit to the file, and `--midi-map` prints the same table it printed before the phase —
the note mapping is device-independent and stays so.

The guide's screenshots are now stale: the panel grew 30 px and the strip did not exist when they were taken. Do **not** rebuild
the guide in this phase; note it in `## Progress` as the guide round's first task.

- [ ] **Step 4: The whole-branch review**

Use `superpowers:requesting-code-review` on the branch `phase-2` against `main`. Give the reviewer the design document and this
phase's six plans as its brief, and ask it for the things a per-task review cannot see: the seam between 38 and 39, the seam
between 40 and 41, and whether any two plans agreed on a name that they each spelled differently.

Land the review's fixes as their own commits, one per item, each with the test that would have caught it.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "chore(phase 2): the phase's gate, its mutation checks and its review"
```

---

## Progress

**The phase is done.** Three gates on the whole workspace: `cargo test --workspace` → **67 suites, 623 tests, 0 failed** (the
number the next phase starts from), `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --check` clean.
`docs/guide/check.sh` → `guide: ok`.

- [x] **Task 1 — the help card.** `tests/help_live.rs` is new: the card's words are read back off the paint of a real frame,
  so a line that stops being drawn fails the test even though `help_sections()` still contains it. The card gained
  `Ctrl+1..4: pick the device the panel shows`, `the strip: click picks a device; M is its MUTE; + adds one` and
  `four devices; names and extra devices are not saved yet`, and lost a claim phase 2 made false (`one voice` became `each
  device is one voice`). Commits `b19846c`, `6fc669f` (the by-eye check in task 2 caught the card saying `MUTE` where the
  strip draws `M`). Review (inline): the plan's `tests/help_live.rs` did not exist — the card's content test was and stays
  `help.rs`'s own `mod tests`; and `strings()` is now copied a third time across test binaries, kept as a copy because a
  shared `tests/` module would reach outside this task's Files block.
- [x] **Task 2 — the README.** A `## Up to four machines` section between "What it is" and "Features", the "one voice"
  opener qualified, the MIDI feature bullet rewritten (`Which ports are open is the settings card's; which of them a device
  hears, and on what channel, is on that device's card`), and the save limit stated as a cost with its wishlist entry named.
  Commit `27ac3af`, which also carries the one-sentence disclosure at the top of the guide (`docs/guide/src/01-what-it-is.md`)
  that Review Focus 3 asks for — no task owned it. Review (inline): the app was walked under `Xvfb :77` (`LIBGL_ALWAYS_SOFTWARE=1`,
  `xdotool`, `import -window`) and every sentence checked against what it drew — the strip, the picker and its footnote, the
  card's rows, `CTRL ONLY`, `Ctrl+2` moving the selection, the new help lines. That walk found two things: the strip's mute
  key is captioned `M` (not `MUTE`), and `MUTE_MS` is **10 ms**, not the tenth of a second task 3's text assumed.
- [x] **Task 3 — judging two machines by ear.** `## Two machines, and what one cannot show you`: phase, shuffle per device,
  and mute as a gain. Commit `30d0e9c`. Review (inline): the plan's "a flam means the seek is a step or more out" reads as
  the pair being in phase, and it is not — a scratch `Mixer` run (120 BPM, a join 720 samples into a 6000-sample step, four
  steps of per-sample positions) showed the second device **720 samples behind and staying there**, because
  `Sequencer::start_at` restarts the clock rather than taking the reference's phase inside the step. The bullet now states
  that as the limit it is, with the measurement in it. `58ced28` also corrected "the device on the panel" to the rack's step,
  which is the first published device's.
- [x] **Task 4 — `AGENTS.md` names the bus.** The `device::BusState` sentence landed in the "two threads" paragraph, which
  therefore had to become `Three threads, two rules` — a paragraph titled for two threads that then describes three is the
  kind of stale count this file exists to prevent. `device::DeviceKind` joined the catalogue list. Commit `f04e57f`.
- [x] **Task 5 — the gate, the checks and the review.** Both of plan 39's mutation checks originally **survived**; both were
  re-pinned in `a35c461` after watching each fail (the mix path in `tests/no_alloc.rs` reports 1000 allocations in 1000
  blocks with the fast path gone; the rewritten mute test reports a mix that moved by 8.8e-5). `tests/persist.rs` passes with
  no edit and `--midi-map` prints its unchanged 89-line table. The whole-branch review found **one Important code defect** —
  a removed device handed its editor to the survivor — fixed in `7e7e933` with the test that was RED on the old code.

**Review mode.** Tasks 1-4 were reviewed inline, as this plan's header says: one pair of hands, the three questions asked of
the staged diff. The whole-branch review was meant to be fresh. `pi-subagents` is loaded, but its only model
(`qwen/qwen3.8-27b`) refused the connection, and the `claude-code` agent needs its own interactive session. So:
`Final review: self-review (no subagent tool)`. A self-review by the author is weaker than a fresh reader, and this round is
the evidence: it took a second pass over the *seam* to find the editor-slot bug, which no per-task diff showed.

### What the whole-branch review found

The three questions it was given, answered in order: the 38-39 seam (bus, transport, mute) held — one writer, readers that
only load, a fast path now pinned by an allocation count; the 40-41 seam (routing, selection, request queues) **did not** —
the editor slots were keyed by position; and the plans agreed on every name except three drifts, two of them in this plan.

- **Critical:** none.
- **Important — a removed device handed its editor to the survivor.** `PanelState::edit_slots` was a `Vec<PanelEdit>` indexed
  by cell position, and `focused` was a position, so removing the first device (the normal two-click `REMOVE` on the device
  on the panel) gave the survivor the removed device's selected step, step mode and randomise dials — none of which live in
  the device. Fixed by keying both by `device.number`, which never moves (`StripCell::number`): a removal now leaves the
  survivor on its own editor, and a device that is gone keeps no slot. The test is
  `removing_the_first_device_does_not_hand_its_editor_to_the_survivor` (RED before the fix: `left: 4, right: 0`), and the
  same removal was walked in the app — after removing `ACID 1`, `ACID 2` came up on step 1.
- **Important — the two mutations plan 39 asked for did not pin what they claimed.** Detail in the ruling below and in
  `a35c461`.
- **Minor — `EAR_TESTING.md` said the join follows "the device on the panel".** It follows the first *published* device's
  step. `58ced28`.
- **Minor (deferred) — `state.rs` still calls `SharedState` "the seam between the two threads"**, while the MIDI handlers
  are a third writer. Left as it is: that sentence is about the panel and the audio callback, and `AGENTS.md` now says
  three threads.
- **Minor (deferred) — the save limit is told three times**, not twice: the picker's footnote, the `+` key's hover tip, and
  the README. The tooltip stays — it is what a hand meets before opening the picker.
- **Minor (deferred) — plan 39's test name drift**: the plan says `a_rack_that_did_not_change_is_not_rebuilt`, the code says
  `a_steady_rack_rebuilds_no_engine`. A plan is the record of the round that wrote it; it is not reworded.
- **Declined to judge:** the sound claims — whether the mute fade clicks, whether a joining device flams — are
  `EAR_TESTING.md`'s to settle with ears, and this session has none. What can be said without them is in that file, with the
  measurement behind the one claim that could be measured.

Rulings made while writing this plan, and their cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| The guide is not rebuilt in this phase | Every screenshot is 30 px of layout out of date, and the strip is missing from all of them. Accepted: the guide round is 18 pages and would double the phase. One sentence at the top of the guide says so, added under Review Focus 3 because no task owned it. |
| The save limit is stated twice (footnote and README) and nowhere else | **Wrong as written**: the `+` key's hover tip states it too (`ui/tips.rs`), so it is three. The tooltip is kept — a hand hovering `+` needs it before the picker opens. A player who reads none of them is surprised once, on restart. |
| `EAR_TESTING.md` gets a section rather than a new file | The document exists to judge the sound; two machines is a way of judging the sound. |
| Task 3's ear text contradicts its own measurement | It did: the plan says a flam means the seek is a step out, and the seek is *one step* of alignment, not phase. The section states the limit instead. Cost if wrong: a listener hears the flam the code guarantees and reports it as a bug — which, for the next round, it is. |
| `AGENTS.md`'s "Two threads, two rules" became "Three threads, two rules" | `00-phase-2.md`'s invariants still say "Two threads". The two documents differ in wording; the rule is the same. |
| Plan 39's mutations were re-pinned in plan 43's files, not plan 39's | The mix path's allocation belongs in `tests/no_alloc.rs` (the only binary with a counting allocator) and the mute test in `tests/mixer.rs`. Plan 39's own file now under-describes its tests; this Progress section is where that is said. |
| The whole-branch review was a self-review | The author's blind spot is the seam this round found last. A fresh reader would have found the editor-slot bug in one pass; it took a targeted second pass here. Cost if wrong: a defect of the same class ships, and the next phase's first review finds it. |
| A test's "audible" assertion compared two time windows | It passed whatever the mute did. Rewritten to twin two identical racks. Cost if wrong: a mute that stops muting has to break the *step* comparison as well as the window one. |
