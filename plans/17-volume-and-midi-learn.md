# Volume Corner and MIDI Learn Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move VOLUME to the top strip's left corner before the wordmark, and let any knob be taught to follow a
controller: press LEARN, click the knob, move a fader.

**Architecture:** The learnable knobs get a runtime controller table in `SharedState` (`cc_map`, seven atomics), so the
MIDI thread reads the same numbers the panel writes and no lock crosses threads. `midi::cc` owns the table's meaning;
`ui::learn` owns the flow; the panel records the first controller it hears while `learn_active` is set.

**Tech Stack:** Rust 2024, egui/eframe 0.37, `midir`, atomics only.

**Spec:** this plan — the scope decisions below are the spec.

## Scope decisions

- **The volume knob moves and nothing else does.** Its cell in the transport row becomes the LEARN key, so the row
  keeps five keys and the panel keeps its shape.
- **Learnable: the six tone knobs plus volume.** The waveform selector has no controller (it is a two-state switch),
  and tempo/swing stay on their fixed controllers so a performance never loses the clock.
- **One controller drives one control.** Teaching a CC that another knob owns takes it away from that knob.
- **Nothing is applied while listening.** The controller's number is recorded instead, so a knob cannot jump under the
  hand that is teaching it.
- **Bindings are saved with the session** (`SavedState::cc_map`), so a rig survives a restart.
- **Not in this plan:** learning the computer keyboard, MIDI-note learn, envelopes or per-song controller overrides
  (the map travels with the song because it is part of `SavedState`).

## Global Constraints

- Nothing new on the audio thread: `tests/no_alloc.rs` stays green, and only atomics are written from MIDI.
- A file without `cc_map` loads with the factory pairing (`DEFAULT_CC`), never with silence.
- Escape gives up on a learn in progress and clears the shared flag with it.
- Panel legends ASCII; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` clean.

## Review Focus

1. **Pressing LEARN and then nothing.** The panel waits in `Choosing`; no controller is captured and no knob moves;
   Escape or LEARN again returns to idle.
2. **A controller arriving twice while listening.** The first is remembered; the bind happens once and listening stops.
3. **Teaching a CC that already drives another knob.** The other knob loses it, so one fader never moves two controls.
4. **A learned map that points at a controller the interface no longer sends.** The control keeps its last value; the
   knob still works with the mouse.
5. **Loading a song or a session saved with a different map.** The learned rig comes back with it.

---

### Task 1: the volume corner

**Files:**
- Modify: `rust/crates/acid-app/src/ui/panel.rs` (`volume_knob_rect`, `brand_rect`), `rust/crates/acid-app/src/ui/mod.rs` (draw it, drop it from the transport row)
- Test: `rust/crates/acid-app/src/ui/panel.rs`

**Interfaces:**
- Produces: `panel::volume_knob_rect(top: Rect) -> Rect` (proportional, no scale argument), and `brand_rect` starting
  right of it.

- [x] **Step 1: The failing test** — `top_strip_regions_are_ordered_left_to_right` also asserts
  `volume.right() <= brand.left()` and `volume.left() >= top.left()`.
- [x] **Step 2: Run it** — `cargo test -p acid-app --lib panel`, expect `cannot find function volume_knob_rect`.
- [x] **Step 3: Implement** — the knob as `top.height() * 0.92` square inset by `theme::GAP`; `brand_rect` starts at
  its right edge plus a gap; `draw_panel` draws it there with `knob::knob(..., ("knob", 7), ...)` and publishes
  `shared.volume` exactly as before.
- [x] **Step 4: Verify and commit** — `cargo test --workspace`.

### Task 2: the controller table

**Files:**
- Modify: `rust/crates/acid-app/src/state.rs` (`cc_map`, `learn_active`, `learn_cc`), `rust/crates/acid-app/src/midi/cc.rs` (`LEARNABLE`, `DEFAULT_CC`, `FIXED_CC`, `slot`, `cc_for`, `set_cc`, `cc_target(state, cc)`, learn capture in `apply_cc`)
- Test: `rust/crates/acid-app/src/midi/cc.rs`

**Interfaces:**
- Produces: `cc::LEARNABLE: [CcTarget; 7]`, `cc::DEFAULT_CC: [u8; 7]`, `cc::FIXED_CC: [(u8, CcTarget); 2]`,
  `cc::slot(CcTarget) -> Option<usize>`, `cc::cc_for(&SharedState, CcTarget) -> Option<u8>`,
  `cc::set_cc(&SharedState, CcTarget, u8)`, `cc::cc_target(&SharedState, u8) -> Option<CcTarget>`.
- `apply_cc` records `learn_cc` and applies nothing while `learn_active` is set.

- [x] **Step 1: The failing tests** — a learned controller replaces the factory one and never doubles; listening
  records instead of moving; tempo and swing cannot be taught.
- [x] **Step 2: Run them** — `cargo test -p acid-app --lib midi::cc`.
- [x] **Step 3: Implement** — table lookup first, `FIXED_CC` after; `set_cc` clears the controller from every other slot.
- [x] **Step 4: Verify and commit.**

### Task 3: the learn flow

**Files:**
- Create: `rust/crates/acid-app/src/ui/learn.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`PanelState::learn`, LEARN key, knob selection, polling, hint),
  `rust/crates/acid-app/src/main.rs` (Escape cancels)
- Test: `rust/crates/acid-app/src/ui/learn.rs`, `rust/crates/acid-app/tests/learn_live.rs`

**Interfaces:**
- Produces: `learn::Step { Idle, Choosing, Listening(CcTarget) }`, `learn::label(Step)`, `learn::hint(Step)`,
  `learn::control_for_knob(usize)`, `learn::choose(CcTarget, &SharedState) -> Step`, `learn::heard(&SharedState) -> Option<u8>`,
  `learn::cancel(&SharedState)`.

- [x] **Step 1: The failing tests** — the key says what it waits for; choosing starts listening; the live test binds a
  controller end to end (`LEARN` → `CUT OFF` → CC 22 → `cc_for(Cutoff) == Some(22)`, factory 74 released, dirty).
- [x] **Step 2: Run them.**
- [x] **Step 3: Implement** — while `Choosing`, knobs turn a scratch value and a click picks them; the status line
  shows the hint; a frame with `heard()` set binds and stops listening.
- [x] **Step 4: Verify and commit.**

### Task 4: the map is saved

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`SavedState::cc_map`, `default_cc_map`, `apply_saved`, `shared_to_saved`), `rust/crates/acid-app/src/ui/help.rs` (one line)
- Test: `rust/crates/acid-app/tests/persist.rs`

- [x] **Step 1: The failing test** — a learned map round-trips; a file without the field loads `DEFAULT_CC`.
- [x] **Step 2: Run it.**
- [x] **Step 3: Implement** — `#[serde(default = "default_cc_map")]`; `apply_saved` writes the slots; the help card
  gains `LEARN: pick a knob, then move a fader`.
- [x] **Step 4: Verify and commit.**
