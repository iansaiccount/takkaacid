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

---

### Task 7: velocity is a level, not just an accent

**Files:**
- Modify: `rust/crates/acid-core/src/pattern.rs` (`Step::velocity`, `DEFAULT_VELOCITY`, manual `Default`), `rust/crates/acid-core/src/voice.rs` (`note_at`, `velocity_to_gain`), `rust/crates/acid-core/src/sequencer.rs`, `rust/crates/acid-app/src/ui/selection.rs` (`place_note` takes `Option<u8>`), `rust/crates/acid-app/src/persist.rs` (`SAVE_VERSION` 2)
- Test: `rust/crates/acid-core/tests/velocity.rs`, `rust/crates/acid-app/src/ui/selection.rs`, `rust/crates/acid-app/tests/step_mode_live.rs`

**Model:** a step carries `velocity: u8` (`0..=127`) — the level it was struck at. `accent` stays a separate boost, so a
soft accented note is still louder than a soft plain one, and an untouched step plays at `DEFAULT_VELOCITY = 127`, which
is unity, so every existing pattern keeps its level exactly.

- [x] **Step 1: The failing tests** — half the velocity is half the peak (the voice is linear in level); zero is silent;
  a step nothing struck is at full scale; a step saved before the field loads at full scale; a round trip keeps it.
- [x] **Step 2: Run them.**
- [x] **Step 3: Implement** — `Step::velocity` with `#[serde(default)]` at `DEFAULT_VELOCITY` and a manual `Default` (a
  derived one would make every rest silent the moment it was switched back on); `Voice::note_at(hz, accent, velocity)`
  with `note_on` delegating at full scale, so nothing outside the sequencer changed; `velocity_to_gain` linear.
- [x] **Step 4: Mutation-check** that dropping the gain factor from `render` fails the level test.
- [x] **Step 5: Persistence** — `SAVE_VERSION` 2. The new field is `#[serde(default)]`, so a version-1 file loads with
  full-scale steps and is stamped on the next save; old song files are per-pattern and take the same default.

### Task 8: two notes in one frame are two notes

**Files:**
- Create: `rust/crates/acid-app/src/notes.rs` (`NoteQueue`, `WaitingNote`)
- Modify: `rust/crates/acid-app/src/state.rs` (replace `step_note`/`step_note_seq`), `rust/crates/acid-app/src/midi/message.rs`, `rust/crates/acid-app/src/ui/mod.rs` (drain the queue)
- Test: `rust/crates/acid-app/src/notes.rs`, `rust/crates/acid-app/tests/step_mode_live.rs`

**Why:** one slot plus a counter could only ever place one note. A second note arriving in the same frame overwrote the
first before the panel looked, so both steps took the later note's pitch and velocity — a rolled chord landed as one
step twice.

- [x] **Step 1: The failing tests** — two notes inside one frame land on consecutive steps with their own pitch, velocity
  and accent; notes that arrive while the panel is not editing are drained, not left to leak into a later edit.
- [x] **Step 2: Run them.**
- [x] **Step 3: Implement** — a 32-slot lock-free SPSC ring, one `AtomicU32` per note (velocity high byte, number low),
  free-running head/tail masked into the ring (the capacity divides 2^32, so a wrap needs no handling). A full queue
  drops the newest note rather than blocking the MIDI thread.
- [x] **Step 4: Mutation-check** that a one-slot limit fails the two-note test.
- [x] **Step 5: Verify** — 51 suites green, clippy `-D warnings`, fmt clean.

---

### Task 9: one press, one step — and the pitch class always survives

**Files:**
- Modify: `rust/crates/acid-app/src/ui/keys.rs` (`KeyAction`, `key_actions`, `first_press`, `release`), `rust/crates/acid-app/src/ui/edit.rs` (`from_keyboard`), `rust/crates/acid-app/src/ui/selection.rs` (`place_note`/`rest_step` stop walking; `walk` is new), `rust/crates/acid-app/src/ui/mod.rs` (`play_computer_keys`, `PanelState::placed`), `rust/crates/acid-app/src/ui/help.rs`
- Test: all four modules plus `rust/crates/acid-app/tests/step_mode_live.rs`

**The report:** typing in step mode filled two steps per press, and every key wrote the same `C1`.

**Cause 1 — the walk was on the press.** Placement advanced the border immediately, so anything that delivered the press
twice (a second frame, a repeat egui did not label) wrote two steps. Advancing is now on the key **release**, so a held
key cannot fill the pattern at the repeat rate; a press for a key already down is a repeat and writes nothing.

**Cause 2 — a round trip that clamped the wrong end.** The computer keyboard placed `from_midi_note(midi_note(pitch, octave))`.
The keyboard's octave reaches `-2..=2` but the panel carries `-1..=1`, and `from_absolute` clamped the *absolute* note
before the pitch class was read back — so at `+2` everything became `(C, 1)` and at `-2` everything became `(C, -1)`.
`edit::from_keyboard` clamps the octave first and keeps the pitch class; the test sweeps every pitch at every offset.

**A finding about the tests:** egui drops a `pressed` event for a key it already believes is down, so a live panel can
never show the duplicated press. The rule therefore lives in `keys::first_press`/`keys::release` as plain functions with
their own unit tests; the live test covers the other half (one release walks one step, a second walks nothing, nothing is
written ahead). A live test here would have passed without ever exercising the case.

- [x] **Step 1: The failing tests** — press writes and the release walks (wraps after 16); a release with no press walks
  nothing; every pitch keeps its class at octaves `-2..=2`; a second press of a held key is not a second note; a release
  of a key that never went down is not ours.
- [x] **Step 2: Run them** (the octave test and the walk tests failed; the duplicate-press test failed to compile, then
  proved to be untestable through a live panel — see the finding above).
- [x] **Step 3: Implement.**
- [x] **Step 4: Mutation-check** — reverting to the MIDI round trip fails the octave test; making `first_press` always
  succeed fails its unit test; calling the walk on the press fails the release tests.
- [x] **Step 5: Help card** — `STEP: key down writes, key up walks on` and `backspace: rest, shift+note: write where you
  are`.
- [x] **Step 6: Verify** — 51 suites green, clippy `-D warnings`, fmt clean.

**Not changed:** a MIDI note still walks immediately, because the queue carries note-ons and no release; one note in, one
step on.

---

### Task 10: the MIDI clamp, and a trace for what the device really sends

**Files:**
- Modify: `rust/crates/acid-app/src/ui/edit.rs` (`from_midi_note`), `rust/crates/acid-app/src/trace.rs` (new), `rust/crates/acid-app/src/midi/message.rs`, `rust/crates/acid-app/src/ui/mod.rs`
- Test: `rust/crates/acid-app/src/ui/edit.rs`, `rust/crates/acid-app/src/trace.rs`

**The report:** the correct note *sounds*, but the step that gets stored is C1 — and it happens on a normal note edit too.

**Cause:** the sound and the stored step took different routes. A MIDI keyboard auditions through `note_hz(note)`, which
computes the frequency from the note number directly, so it is always right; the stored step went through
`from_midi_note` → `from_absolute`, which clamped the **absolute** note to `-12..=24` *before* the pitch class was read
back. An E-MU Xboard25 spans MIDI 48..=72, and everything from 57 up clamped to 24 — `(C, 1)` — so the whole upper half
of the keyboard stored C1 while sounding correctly. `from_midi_note` now keeps the class and clamps only the octave, like
`from_keyboard` already did.

- [x] **Step 1: The failing tests** — every MIDI note `0..=127` keeps its class and clamps only its octave; neighbouring
  keys from 57 up never write the same pitch.
- [x] **Step 2: Run them** (both fail against the old clamp).
- [x] **Step 3: Implement.**
- [x] **Step 4: Mutation-check** — reinstating the clamp fails both tests.

**The second symptom is not yet explained.** "One press writes two steps" cannot be reproduced headless: a test drives
exactly one event per frame, and egui drops a repeated press for a key it already believes is down. Rather than guess a
third time, `trace.rs` now prints, when `TAKKA_TRACE_INPUT=1` is set, every MIDI note that arrives and every step the
panel places (with the frame time) — so the next report can carry the actual event stream:

```
[TAKKA_TRACE_INPUT] midi in: note 60 velocity 100
[TAKKA_TRACE_INPUT] place midi: note 60 velocity 100 -> C 1 on step 7
```

A run of the harness with the flag shows one line per press, which is the behaviour to compare against.

---

### Task 11: one note per press — the doubled connection

**Files:**
- Modify: `rust/crates/acid-app/src/midi/supervisor.rs` (`on_success(now)`, `probe_now`), `rust/crates/acid-app/src/midi/gate.rs` (new), `rust/crates/acid-app/src/midi/ports.rs` (`is_listed`), `rust/crates/acid-app/src/main.rs` (the probe and the handler)
- Test: `rust/crates/acid-app/src/midi/{supervisor,gate,ports}.rs`

**The trace said it all:**

```
[TAKKA_TRACE_INPUT] midi in: note 64 velocity 88
[TAKKA_TRACE_INPUT] midi in: note 64 velocity 88
[TAKKA_TRACE_INPUT] place midi: note 64 velocity 88 -> G 1 on step 1
[TAKKA_TRACE_INPUT] place midi: note 64 velocity 88 -> G 1 on step 2
```

Two arrivals for one press, so the app was hearing every note twice. `MidiSupervisor::on_success` left `next_probe: None`,
and `should_probe` reads `None` as "never probed" — so a *connected* app asked to probe again on the very next frame,
opened a **second** connection, and kept the first until the new one arrived in `poll`. Both delivered every message, and
both writes walked the border: two steps per press.

- [x] **Step 1: The failing test** — a port that just opened must not be probed again at once, but must be checked again
  after the interval; asking for a port explicitly probes at once.
- [x] **Step 2: Run it** (fails: `should_probe` is true immediately after success).
- [x] **Step 3: Implement** — `on_success(now)` records the interval, and the new `probe_now()` is what choosing a port
  means. While a port is open the app now asks `ports::is_listed(name)` — enumerating ports costs nothing and delivers no
  messages — and only reconnects when the name has actually gone.
- [x] **Step 4: Defence in depth** — `midi::gate::NoteGate` refuses a note-on for a note that is already down: that is the
  same note delivered twice, never a second press, because a keyboard cannot repeat without a note-off. Clock, transport
  and controllers pass straight through (the tempo follower needs every clock), and a refusal is traced.
- [x] **Step 5: Mutation-check** — `if false` in `NoteGate::note_on` fails the duplicate test; the old `next_probe = None`
  fails the probe test.
- [x] **Step 6: Verify** — 51 suites green, clippy `-D warnings`, fmt clean.

---

### Task 12: arrows do not walk, and a diagnostic that separates keyboard from app

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (only a key that writes a step is tracked), `rust/crates/acid-app/src/main.rs` (`--midi-map`, raw byte trace), `rust/crates/acid-app/src/midi/message.rs` (`concert_name`)
- Test: `rust/crates/acid-app/tests/step_mode_live.rs`, `rust/crates/acid-app/src/midi/message.rs`

**The arrow bug:** "left walks back a step, then it immediately jumps forward again". `first_press` was called for *every*
press, so `ArrowLeft` was remembered as a held key; its press moved the border back (dispatch) and its release called
`walk`, moving it forward. Only a key that writes a step (a mapped note, or Backspace) is tracked now, so an arrow's
release does nothing.

- [x] **Step 1: The failing test** — press+release `ArrowLeft`; the border must not move and nothing may be tracked.
- [x] **Step 2: Run it** (fails: the release walks).
- [x] **Step 3: Implement** — `let writes = pitch_for(key).is_some() || key == Backspace; let fresh = writes && first_press(..)`.
- [x] **Step 4: Mutation-check** — dropping the `writes &&` guard fails the test.

**The pitch question:** "I press a C and the app registers a different note — is it my keyboard or the app?" The app
cannot answer that on its own, because its *own* register is not concert pitch: `step_hz` roots `C0` at MIDI 33, which is
really A1 (55 Hz). So for a MIDI note `n` the app stores class `(n - 33) mod 12`, which runs **three semitones sharp** of
the keyboard's name: a C4 (60) is stored as `D#1` and sounds 130.8 Hz, which *is* C3. Two offsets stack:

| layer | offset |
|---|---|
| keyboard's own note | unknown until the raw bytes are read |
| app's register (C0 = MIDI 33 = A1) | names run 3 semitones sharp; the sound keeps the class, an octave down |

- [x] `--midi-map` prints, without a window or audio device, every note number with the keyboard's name, the pair the app
  writes, the label it shows, and the frequency — so the app's half of the question is on paper.
- [x] `TAKKA_TRACE_INPUT=1` now prints the **raw bytes** (`midi raw: 90 40 58`) before parsing, so the keyboard's half
  is on paper too: if the bytes say `40` when the C key is pressed, the keyboard is transposed; if they say `3C`, the app
  is the one relabelling it.

**Open decision (operator's call):** make the labels truthful (no sound change: `D#1` would display as `C3`), or re-root
the register so the panel's `C0` is a real C (labels and stored data right, every existing pattern 3 semitones higher).
