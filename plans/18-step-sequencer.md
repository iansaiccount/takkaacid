# Step Sequencer and Editing Keys Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A note typed on the computer or MIDI keyboard writes the selected step; a STEP mode key does that and walks
to the next step; and the arrows are swapped so the plain arrows walk steps and shift reaches the pattern.

**Architecture:** Placement is one pure function, `selection::place_note`, which writes the step and advances the
selection when step mode is on. The computer keyboard calls it from `play_computer_keys` with the shift flag carried by
the key *event*; the MIDI thread only publishes "a note arrived" (number plus a counter) and the panel places it, so no
UI state has to live in the audio/MIDI path.

**Tech Stack:** Rust 2024, egui/eframe 0.37, atomics for the MIDI side.

**Spec:** this plan — the scope decisions below are the spec, taken from the operator's brief.

## Scope decisions

- **Shift + note is an edit, not an audition**: it writes the selected step and does not open the gate, so typing a
  phrase does not play it. A MIDI note still sounds, because the MIDI gate is opened before the panel sees it.
- **STEP mode** does the same and then selects the next step, wrapping from 16 to 1. It is remembered while the app
  runs but is not saved: a session should not reopen mid-edit.
- **The arrows are swapped** (operator's ruling): plain left/right walk the steps, shift+left/right choose the pattern.
  Shift+up/down keep the note edit, and plain up/down keep the tempo nudges.
- **Only the notes the computer keyboard already maps** are reachable in step mode (the home row, C..C). The MIDI path
  accepts any note and folds it into the panel's `-12..=24` range.
- **Not in this plan:** rests typed from the keyboard, per-step velocity, a scrolling long pattern, or recording while
  the transport runs.

## Global Constraints

- `place_note` is the only writer: no second path may set a step's pitch from input.
- Nothing new on the audio thread; the MIDI side writes atomics only (`tests/no_alloc.rs` stays green).
- Panel legends ASCII; the help card must still fit at 0.65x.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` clean.

## Review Focus

1. **Typing a scale in step mode past step 16.** The selection wraps to step 1 rather than clamping, so a phrase longer
   than a bar keeps going.
2. **Shift held with the mouse resting on a step.** The key edit goes to the *selected* step; nothing about the pointer
   changes it, and the step under the mouse is untouched.
3. **A note arriving from MIDI while the transport runs.** The current step is rewritten in the live pattern and the
   sequencer picks it up on the next pass; nothing panics or blocks.
4. **The same MIDI note seen twice (counter unchanged).** Nothing is written twice — one note, one placement.
5. **Arrows after the swap.** Plain arrows never change the pattern; shift+arrows never move the selection.

---

### Task 1: the arrow swap

**Files:**
- Modify: `rust/crates/acid-app/src/ui/selection.rs` (`edit_key`), `rust/crates/acid-app/src/ui/shortcuts.rs` (`shortcut_for`)
- Test: both files' unit tests, `rust/crates/acid-app/src/ui/dispatch.rs`, `rust/crates/acid-app/tests/selection_live.rs`

- [x] **Step 1: Flip the tests** — plain arrows move the selection and never touch the pattern; shift+arrows step the
  pattern and never move the selection; shift+up/down still edit the note; plain up/down still change the tempo.
- [x] **Step 2: Run them to see the old mapping fail** (`shift_turns_the_arrows_into_editing_keys`, the dispatch glue
  tests and the live selection tests all fail).
- [x] **Step 3: Implement** — `edit_key` reads the plain arrows and `shortcut_for` reads the shifted ones, with the
  tempo nudges kept unshifted.
- [x] **Step 4: Verify and commit.**

### Task 2: a note writes the selected step

**Files:**
- Modify: `rust/crates/acid-app/src/ui/selection.rs` (`place_note`), `rust/crates/acid-app/src/ui/keys.rs` (`midi_note`, `fresh_presses` now carries shift), `rust/crates/acid-app/src/ui/mod.rs` (`play_computer_keys`), `rust/crates/acid-app/src/ui/edit.rs` (`from_midi_note`)
- Test: `rust/crates/acid-app/src/ui/selection.rs`, `rust/crates/acid-app/src/ui/keys.rs`, `rust/crates/acid-app/tests/step_mode_live.rs`

**Interfaces:**
- `selection::place_note(&mut PanelState, &SharedState, Pitch, i8) -> Step`
- `keys::midi_note(Pitch, i32) -> u8`, `keys::fresh_presses(&[Event]) -> Vec<(Key, bool)>`
- `edit::from_midi_note(u8) -> (Pitch, i8)`

- [x] **Step 1: The failing tests** — shift+note writes the selected pitch and does not sound; step mode writes and
  walks; the walk wraps; a plain note only sounds.
- [x] **Step 2: Run them.**
- [x] **Step 3: Implement** — `place_note` writes the step, remembers the pitch and advances when `step_mode` is set;
  the keyboard path converts its key back to a MIDI number so the panel and a MIDI keyboard agree on pitch.
- [x] **Step 4: Verify and commit.**
- Finding: the shift flag must come from the key *event*, not from `InputState::modifiers` — egui only refreshes that
  from `ModifiersChanged`, so a single-frame press read shift as false. `fresh_presses` now returns `(Key, bool)`.

### Task 3: MIDI notes and the STEP key

**Files:**
- Modify: `rust/crates/acid-app/src/state.rs` (`step_note`, `step_note_seq`), `rust/crates/acid-app/src/midi/message.rs` (publish the note), `rust/crates/acid-app/src/ui/mod.rs` (`PanelState::{step_mode, step_note_seen}`, the STEP key, the poll), `rust/crates/acid-app/src/ui/help.rs`
- Test: `rust/crates/acid-app/tests/step_mode_live.rs`

- [x] **Step 1: The failing test** — a MIDI note published while editing lands on the selected step, walks on, and is
  placed once (the counter guards against a second placement).
- [x] **Step 2: Run it.**
- [x] **Step 3: Implement** — `message::handle` stores the note number and bumps the counter; the panel places it when
  the counter moves and the panel is editing; STEP is the eighth switches cell.
- [x] **Step 4: Verify and commit.**

---

### Task 4: the switches row owns its own count

**Files:**
- Create: `rust/crates/acid-app/src/ui/switches.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (the row), and the live harnesses `clear_live`/`save_live`/`songs_live`/`learn_live`/`step_mode_live`

**Interfaces:**
- `switches::Switch` (`ALL`, `index`, `label`, `is_live`) and `switches::cell(Rect, Switch) -> Rect`

The count `8` had been copied into five live test harnesses, and each new key broke them in turn (four times). They
ask `switches::cell` now, so the row's length lives exactly once, and `Switch::is_live` says which keys are engraving.

- [x] **Step 1: The failing test** — every key is numbered and labelled in order, ASCII only; a cell sits inside the row
  and clear of its neighbour; only the first three are engraving.
- [x] **Step 2: Run it.**
- [x] **Step 3: Implement** and move the row and all five harnesses onto it.
- [x] **Step 4: Verify.** 50 suites green.

### Task 5: velocity becomes the step's accent

**Files:**
- Modify: `rust/crates/acid-app/src/ui/selection.rs` (`place_note` takes `accent: Option<bool>`), `rust/crates/acid-app/src/ui/mod.rs` (both call sites)
- Test: `rust/crates/acid-app/src/ui/selection.rs`, `rust/crates/acid-app/tests/step_mode_live.rs`

- [x] **Step 1: The failing tests** — a hard MIDI note accents the step it lands on and a soft one does not; a hard note
  clears an accent that was there, and `None` (a computer key) keeps whatever the step had.
- [x] **Step 2: Run them.**
- [x] **Step 3: Implement** — the MIDI thread already reduces velocity with `message::is_accent` and stores `midi_accent`
  before it bumps the note counter, so the panel reads that value when it places the note; the computer keyboard passes
  `None`, because a typed pitch must not silently unaccent an accented step.
- [x] **Step 4: Verify, and mutation-check** that passing `None` fails the hard-note test.

### Task 6: a rest from the keyboard

**Files:**
- Modify: `rust/crates/acid-app/src/ui/selection.rs` (`rest_step`, one shared `place`), `rust/crates/acid-app/src/ui/mod.rs` (Backspace), `rust/crates/acid-app/src/ui/help.rs`
- Test: both of the above

- [x] **Step 1: The failing tests** — Backspace rests the selected step and walks in step mode; shift+Backspace rests it
  and stays; plain Backspace writes nothing and marks nothing for saving; a rest keeps the pitch and the flags.
- [x] **Step 2: Run them.**
- [x] **Step 3: Implement** — `place_note` and `rest_step` share one private `place(note: Option<(Pitch, i8)>, accent)`, so
  there is still exactly one writer of a step; a rest sets `on: false` and keeps `pitch`/`slide`/`accent` so the next
  note is one key away.
- [x] **Step 4: Verify.** Help card line now reads `STEP/shift+note: write, backspace: rest`.
