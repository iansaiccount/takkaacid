# ACCENT like a TB-303 Implementation Plan

**Goal:** an accented step sounds like a 303's accent — louder, with a fast filter "wow" at the attack — and the ACCENT
knob decides how much of the whole character there is.

**Why:** the knob only scaled half of it. Level moved with the knob; the filter effect was a fixed 1.5x multiplier on the
envelope depth, so an accent was *uniformly* brighter for the whole note, and at knob 0 it still bit.

## What a 303 does

Accent is a per-step pulse. It lifts the VCA (strong, several dB) and dumps a charge into the VCF's cut-off through a
capacitor, so the note opens bright and closes within tens of milliseconds: a "wow" at the attack, not a brighter note.
The ACCENT pot sets how much of that character is applied.

## Design

- **Level**: `amp = AMPLITUDE × (1 + accent_amount × (ACCENT_GAIN − 1)) × velocity`, `ACCENT_GAIN = 1.6` (+4 dB at full
  knob). At 0 the accent is neither louder nor brighter.
- **Filter kick**: a one-pole pulse, `1.0` at note-on and decaying with a 70 ms time constant, added to the cut-off in
  octaves: `cutoff = base × 2^(env_mod·filter_env + ACCENT_SWEEP_OCTAVES × amount × pulse)`, `ACCENT_SWEEP_OCTAVES = 3.0`.
  A note either carries the pulse or does not — the accent retriggers with the note.
- **Staging**: `AMPLITUDE` came down 0.92 → 0.88 so the loudest accented step still peaks under 0.9 (gain staging).
- **Not modelled**: the 303's accent also nudges the resonance and shortens the envelope slightly; both are small, and
  changing them without a reference 303 in the room would be guessing.

## Tasks

- [x] Failing tests first (`crates/acid-core/tests/accent.rs`): knob at 0 makes an accent inaudible; knob at full lifts
  the level by more than 1.4x; an accent's attack is >1.5x brighter than the plain note's and decays back to it (<1.15x);
  the kick is proportional to the knob and monotonic.
- [x] Implement the pulse in `Voice` (`accent_pulse`, `accent_pulse_coeff`), replacing the fixed
  `ACCENT_FILTER_MULTIPLIER`.
- [x] Fix the coefficient: it was built as `1 − e^(−1/τsr)` and used as a multiplier, which killed the pulse in one
  sample. It is `e^(−1/τsr)` now.
- [x] Restage `AMPLITUDE` and re-tune the gain-staging test's expectations.
- [x] The loop-wrap test measured the global steepest step, which reports a deliberately bright accent attack as a click.
  It now measures at the wrap and compares against the steepest step elsewhere in the phrase.
- [x] Verify: 52 suites green, clippy `-D warnings`, fmt clean.
