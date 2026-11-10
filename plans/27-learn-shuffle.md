# Plan 27 — teach the shuffle knob

SHUFFLE was the last control with a *fixed* controller (CC 73) that the learn flow refused. It is a dial like the others,
a performance may want it under a fader the player knows, and there is no reason left for it to be special: the only
argument was "a performance never loses the shuffle", and that argument died with the pickup rule — a learned fader has
to reach the knob before it may turn it, so a stray fader cannot knock the groove sideways.

## What changes

- `CcTarget::Swing` moves from `FIXED_CC` into `LEARNABLE`, appended at slot **10** so every existing slot keeps its
  controller. `DEFAULT_CC` gains `73` at the end: a rig that already drives the shuffle keeps driving it.
- **`FIXED_CC` disappears.** With it goes the "learned table first, fixed pair after" branch in `cc_target`: one table,
  one answer. A second table with one entry was a distinction without a difference.
- `SharedState::cc_map` grows to `[AtomicI32; 11]`; the persisted `cc_map` is already a `Vec`, so `upgraded()` pads it to
  `LEARNABLE.len()` with the factory controller — an eight- or ten-entry map gains `73` at the end and nothing moves.
  `SAVE_VERSION` 8.
- **Pickup**: swing is a dial, so a learned fader has to reach it. `knob_value` gains its position; the rule already
  excludes only the two steppers (waveform, pattern).
- **The pick**: while a learn is *choosing*, clicking the SHUFFLE knob picks `Swing` instead of turning it — the same
  contract the tone knobs have, and the reason a dark-but-live control is a bug this codebase has met twice.
- The tip says so: `SHUFFLE` mentions that LEARN can teach it a controller.

## Tests (failing first)

| Test | What it pins |
|---|---|
| `cc::the_shuffle_is_a_learnable_slot_now` | `slot(Swing) == Some(10)`, factory `73`, `set_cc` moves it and frees `73` |
| `cc::a_shuffle_fader_has_to_reach_the_knob` | a fader at 0 with swing at 0.6 leaves it; sweeping past takes over |
| `persist::a_map_from_before_the_shuffle_knob_keeps_every_fader_it_had` | ten entries → eleven, `73` appended |
| `learn_live::clicking_the_shuffle_knob_while_choosing_asks_for_a_shuffle_fader` | the pick, and that the knob does not turn |
| `perform_live` (existing) | the knob still turns when no learn is running |

## Global constraints

- The MIDI thread writes atomics only; `tests/no_alloc.rs` stays green.
- ASCII legends only; no control face may be a font glyph.
- Every step ends green: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`.

## Review focus

- One controller drives one control: `set_cc(Swing, 73)` must not leave a second slot pointing at 73.
- The learn pick must not fire outside `Choosing`, and must not turn the knob while it does — both checked.
- `cc::mapped()` (the help overlay's list) must now include the shuffle without anyone editing it, since it is derived
  from the catalogue.
