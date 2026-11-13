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

- [ ] Task 1 — the help card
- [ ] Task 2 — the README
- [ ] Task 3 — judging two machines by ear
- [ ] Task 4 — `AGENTS.md` names the bus
- [ ] Task 5 — the gate, the checks and the review

Rulings made while writing this plan, and their cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| The guide is not rebuilt in this phase | Every screenshot is 30 px of layout out of date, and the strip is missing from all of them. Accepted: the guide round is 18 pages and would double the phase. |
| The save limit is stated twice (footnote and README) and nowhere else | A player who reads neither is surprised once, on restart. A third telling would be noise. |
| `EAR_TESTING.md` gets a section rather than a new file | The document exists to judge the sound; two machines is a way of judging the sound. |
