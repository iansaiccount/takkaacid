# Phase 12 — Deferred: envelope accuracy and anti-aliasing

> Part of [00-main-plan.md](00-main-plan.md). **No tasks — this is the decision record and the backlog.**
> Nothing in Phases 01–11 depends on anything here. Read this before starting any DSP "improvement" so the
> project does not quietly drift from a jam instrument into an emulation project.

## The decision

On 2026-11-06 the owner chose **jam-first, must sound great** over faithful emulation. Two consequences
were agreed explicitly:

- **The oscillator stays naive for now.** A sawtooth built from a phase ramp aliases above roughly
  `sr/2`, and a square does so worse. At bass frequencies with a 4-pole lowpass after it, the fold-back is
  largely masked, which is why Phase 03 shipped without band-limiting.
- **The envelope stays a linear segmented ADSR.** A real TB-303's decay is exponential against a
  capacitor, and its VCA/VCF pair is a single envelope with coupled behaviour. Ours has two linear
  envelopes (amp and filter) with fixed attack/release.

Both are documented in the plan as deliberate, not as oversights. If either becomes a problem, it becomes
a task in a *new* phase, not a quiet edit inside Phase 03 or 04.

## When to revisit

Revisit this file when any of these becomes true:

1. Someone A/Bs the app against a real 303, a recording, or a well-regarded clone and the difference is
   distracting — especially on the *tail* of a note (the envelope) or on high notes above ~C3 (the alias).
2. The user starts making tracks with it and wants a mix that survives mastering.
3. A preset is wanted that only works with resonant self-oscillation or filter nonlinearity.
4. A CPU-budget problem appears (aliasing fixes cost CPU; check before optimising anything else).

## Backlog — oscillator and anti-aliasing

- **PolyBLEP saw/square.** Add a one-sample polynomial residual at each discontinuity, scaled by
  `phase increment`. Expected: alias suppression of 20–30 dB, no measurable CPU concern, no change to the
  `Oscillator` API — a change entirely inside `next_sample`. Tests to write first: measure energy at
  non-harmonic frequencies for a 1.5 kHz saw using a Goertzel probe (`analysis::band_energy` would be a
  new helper), before and after.
- **2× oversampling with a half-band decimator.** Bigger win, 2× the voice cost, needs a small FIR. Only
  worth it after polyBLEP proves insufficient.
- **Drift and per-note variation.** A real VCO is not perfectly periodic. A tiny random offset per note
  (±3 cents) buys a lot of "alive" for very little code, and is cheap to test (statistical spread over
  100 notes).

## Backlog — envelope and voice nonlinearity

- **Exponential decay curves.** Replace `decay_inc` with a per-sample multiplier
  (`level *= decay_coeff`) so the tail is exponential like a capacitor. Test: the decay slope in dB/s is
  constant rather than the level itself. This is the single change that most alters the feel of a note.
- **One coupled envelope.** Feed the filter with a scaled copy of the VCA envelope (with its own offset
  and a higher sustain) instead of a second ADSR, matching the 303's actual topology.
- **Envelope time calibration.** Measure a real 303's DECAY knob at several positions and fit the mapping
  in `param::decay_seconds` — currently a log guess (`30 ms..2 s`).
- **Diode-ladder nonlinearity.** Replace the linear TPT feedback path with a `tanh`-shaped stage, giving
  the characteristic compression at high resonance and a softer self-oscillation. Must come with a
  stability test at `resonance = 1.0` (Phase 04 Task 14's test is the template).
- **Accent circuit model.** The real accent boosts both VCA and filter envelope *and* shortens the decay;
  ours only scales amplitude and filter depth.
- **Slide behaviour.** Our glide is a 60 ms one-pole; a 303's slide time depends on the interval. Small
  change, audible on large jumps.

## Backlog — features deliberately left out of Phases 01–11

- Song mode / pattern chaining (the hardware has one; the app has a 64-pattern bank with manual selection).
- Per-step velocity (the 303 has accent, not velocity).
- Audio export to WAV (offline render of `Engine` would make this easy: `render` in 512-frame blocks).
- Distortion *after* the filter as a separate stage (Phase 09's drive is post-VCA and global).
- Preset sharing/import, and a preset browser beyond the 64 slots.
- Any form of timeline automation.

## How to start a deferred phase

1. Copy the shape of an existing phase file (`plans/04-filter-vca.md` is the closest in style).
2. Number tasks from `57` upward; update the phase index in `00-main-plan.md`.
3. Every task follows the same TDD shape: test that measures the *audible* property (band energy, decay
   slope, self-oscillation stability), then the implementation, then a **Listen:** line.
4. Record before/after measurements in `plans/notes/` so the next change has a baseline.
