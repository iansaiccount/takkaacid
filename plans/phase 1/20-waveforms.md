# Sine and Triangle, and a Snapping WAVEFORM Control Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** the oscillator offers four waves — saw, square, sine, triangle — and the WAVEFORM control rests only on
those four, marking each detent and naming the one it is on.

**Architecture:** the wave catalogue lives in `acid_core::oscillator` as the single source of truth (`ALL`, `index`,
`from_index`, `name`). `SharedState` and `persist` stop hand-decoding the enum. The panel keeps its knob geometry and
snaps the knob to a detent so the pointer never sits between waves.

**Tech Stack:** Rust 2024, egui/eframe 0.37, serde, arc-swap.

**Spec:** this plan; the shape follows `plans/13-waveform-selector.md`, which specified a catalogue but was not
implemented (the panel still has a two-position switch).

## Global Constraints

- `Knobs.waveform` stays a `u8` **index**; `0` and `1` keep their meaning (saw, square), so existing saves load
  unchanged and a session saved with `2`/`3` reads back as sine/triangle.
- The audio thread reads an atomic and never allocates: `cargo test -p acid-app --test no_alloc` must stay green.
- No font glyphs as control faces; panel legends stay ASCII (AGENTS.md).
- The seven-knob row keeps its geometry — no `ui::layout` changes.
- Waves stay naive (no band-limiting); Phase 12 owns anti-aliasing, and sine/triangle alias far less than saw/square.

## Review Focus

1. **A save file from a newer build** with `waveform: 9` — it must clamp to the last wave, not index out of bounds.
2. **Switching waves mid-note** — the phase accumulator is shared, so the pitch must not jump.
3. **The ends of the control** — a drag must stop at saw and triangle, never between detents; a click must move exactly
   one detent.
4. **Legibility at 0.65× and 1.6×** — four detent marks plus the wave name must fit the cell.

---

### Task 1: two more waves in the core

**Files:**
- Modify: `rust/crates/acid-core/src/oscillator.rs`
- Test: `rust/crates/acid-core/tests/waveform.rs`

**Interfaces:** `Waveform::{Saw, Square, Sine, Triangle}`; `Waveform::ALL: &'static [Waveform]`,
`fn index(self) -> usize`, `fn from_index(index: usize) -> Self`, `fn name(self) -> &'static str`.

- [ ] **Step 1: The failing tests** — each wave's shape (sine against `sin`, triangle as two linear ramps, saw and
  square unchanged), all four listed, index/from_index a round trip, `from_index` clamping beyond the end.
- [ ] **Step 2: Run them.**
- [ ] **Step 3: Implement** — sine from `f64::sin(TAU·phase)`, triangle from `1 − 4·|phase − 0.5|`, both reading the one
  accumulator so switching waves cannot move the pitch.
- [ ] **Step 4: Verify and mutation-check** that a wrong triangle formula fails its test.

### Task 2: the catalogue is the only decoder

**Files:**
- Modify: `rust/crates/acid-app/src/state.rs` (`set_waveform`/`waveform`), `rust/crates/acid-app/src/persist.rs`
  (`Knobs.waveform` conversions and the clamp)
- Test: both files' unit tests, `rust/crates/acid-app/tests/persist.rs`

- [ ] **Step 1: The failing tests** — index 2 and 3 survive a save/load round trip; a file with `waveform: 9` clamps to
  the last wave instead of panicking.
- [ ] **Step 2: Implement** through `Waveform::{index, from_index}`; delete the two-way `match`es.
- [ ] **Step 3: Verify.**

### Task 3: the control snaps, marks its detents and names the wave

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`waveform_from_knob` → `waveform_from_position`,
  `waveform_position`, the knob pass, the detent marks), `rust/crates/acid-app/src/ui/knob.rs` (a `detents` parameter)
- Test: `rust/crates/acid-app/src/ui/knob.rs`, `rust/crates/acid-app/tests/` live harness

- [ ] **Step 1: The failing tests** — a position maps to the nearest wave with no gaps (0.0 saw, 0.34 saw, 0.5 square,
  0.67 sine, 1.0 triangle); the knob's own value after a drag is exactly a detent.
- [ ] **Step 2: Implement** — the knob takes a detent count, snaps its stored value to `i / (n − 1)`, and draws one tick
  per detent with the selected one lit; the caption is the wave's name.
- [ ] **Step 3: Verify** 52+ suites green, clippy, fmt.
