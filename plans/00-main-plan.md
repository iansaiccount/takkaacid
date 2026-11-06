# takka-acid — TB-303 Clone in Rust — Main Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> This file is the **index and design record**. Each phase lives in its own file in this folder,
> numbered `01`–`12`. Tasks are numbered **globally (1–56)** and are executed in phase order.

**Goal:** A jam-ready desktop acid bassline synthesizer in Rust — one TB-303-style voice, a
16-step sequencer, and a black-and-yellow 303 panel — playable within seconds of launch.

**Architecture:** A Cargo workspace with two crates. `acid-core` is pure DSP and sequencing with no
audio, UI, or platform dependencies, so every sound claim is provable by rendering buffers offline in
tests. `acid-app` owns the platform: a `cpal` output stream that pulls `Engine::render` on the audio
thread, an `eframe`/`egui` window that paints a bespoke 303 panel, and a lock-free shared-state layer
(atomic parameter scalars plus an atomically swapped `Arc<Pattern>`) between them.

**Tech Stack:** Rust 1.99 / edition 2024, `cpal` (ALSA → PipeWire), `eframe` + `egui`, `serde` +
`serde_json`, `midir`, `arc-swap`, `anyhow`. Verification: `cargo test`, `cargo clippy`.

**Spec:** This file (§Design below). The design was agreed with the project owner on 2026-11-06:
jam-first, must sound great, envelope nonlinearity and anti-aliasing deliberately deferred
(Phase 12). Location of code and plans is the owner's choice: code in `rust/`, plans in `plans/`.

## Global Constraints

- Rust, edition 2024. `rustfmt` default style. `cargo clippy -- -D warnings` must pass at every phase
  boundary (documented waivers allowed only with an `#[allow(...)]` and a comment saying why).
- **No allocation, no locking, no syscalls on the audio thread.** The audio callback may only call
  `Engine::render` and touch preallocated state. Tests in Phase 02 pin this.
- `acid-core` must **not** depend on `cpal`, `eframe`, `egui`, or any I/O crate. It may depend on
  `serde` (behind the default-on `serde` feature) and nothing else.
- Audio is **mono internally, duplicated to every output channel**. Internal sample type `f32`
  in `-1.0..=1.0`, phase/time accumulated in `f64`.
- Output never leaves `-0.999..=0.999`. Never emits `NaN` or `inf`. (Tested, see Phase 04 Task 18.)
- Sample rate comes from the device at runtime; the engine must be correct at **44 100** and
  **48 000 Hz** and at block sizes from **1 to 4096** frames.
- All user-visible tuning parameters are **knob-domain `0.0..=1.0`** in the UI layer and are mapped
  to physical units by functions in `acid-core::param` — one mapping, one place, unit-tested.
- Palette and layout constants live only in `acid-app/src/ui/theme.rs`. No hex literal appears
  anywhere else in the tree.
- Every phase ends **green**: `cargo test --workspace` passes and the app still runs.
- Commit at the end of every task, message form: `feat(scope): summary` / `test(scope): summary`.

## Design

### Why

The owner jams with it: launch, hit play, and get a bassline moving. It must sound genuinely good —
punchy, squelchy, never harsh — while remaining simple enough that a stranger can read the panel in
five seconds. Faithful 303 emulation is a later, separate effort (Phase 12), so nothing in this plan
depends on it.

### The voice (signal chain)

```
sequencer step → pitch (with slide glide) → oscillator (saw | square)
  → 4-pole zero-delay-feedback lowpass (cutoff = base + env_mod·env, resonance)
  → VCA (envelope × accent gain)
  → master (drive → soft clip → limiter → volume)   [Phase 09]
  → output
```

- **Oscillator:** naive saw or square from a phase accumulator. Deliberately naive — anti-aliasing
  is Phase 12. Square is derived from the same phase so switching waveform cannot jump pitch.
- **Filter:** two cascaded TPT one-pole stages ×2 (4-pole ladder topology), cutoff clamped to
  `20 Hz .. 0.45·sr`, feedback with a resonance taper that reaches self-oscillation only near 1.0.
- **Envelope:** linear-in-dB-ish ADSR fixed attack (2 ms), `decay` knob → 30 ms .. 2 s, fixed
  sustain 0.35, release 20 ms. Accent adds a second, faster envelope contribution to both filter and
  VCA.
- **Sequencer:** 16 steps of 16th notes; per step `on`, `pitch` (C..C+1 octave, 13 values),
  `octave_offset` (−1..=1), `accent`, `slide`. Swing delays odd 16ths by 0–0.5 of a step. Step
  boundaries are scheduled **sample-accurately**: `render` splits a block at each boundary.

### Panel (black and yellow)

Recessed near-black panel, amber silkscreen legends, yellow LEDs, no stock egui widgets on the panel
surface — knobs, buttons, LEDs and the display are painted shapes.

| Region | Contents |
|---|---|
| Top strip | `takka-acid` wordmark, pattern display (`A1`..`H8`), tempo display (`120`) |
| Knob row | WAVEFORM, TUNING, CUT OFF, RESONANCE, ENV MOD, DECAY, ACCENT |
| Step row | 16 step buttons with LEDs, accent dot and slide line per step |
| Switch row | BANK (A–H), PATTERN (1–8), MODE (WRITE/PLAY) |
| Transport row | RUN, STOP, TEMPO −, TEMPO +, master VOLUME |

Palette (single source of truth, `acid-app/src/ui/theme.rs`):

| Constant | Hex | Use |
|---|---|---|
| `PANEL` | `#0B0B0B` | panel face |
| `PANEL_RAISED` | `#191919` | knob body, button face |
| `WELL` | `#040404` | display well, knob recess |
| `SILKSCREEN` | `#FFC300` | legends, borders, engraving |
| `SILKSCREEN_DIM` | `#7A5C00` | secondary legends, unlit step numbers |
| `LED_ON` | `#FFE066` | lit LED core |
| `LED_ACCENT` | `#FF9500` | accent indicator |
| `LED_OFF` | `#2A2208` | unlit LED |
| `TEXT` | `#F5F0E0` | display digits |
| `WHITE_KEY` | `#E8E2D0` | step button keycap |

### Jam behaviour that must survive review

- Knob moves are heard immediately, at any point in playback, with no clicks.
- Editing a step while running changes the very next loop, not the one after.
- Nothing ever produces a stuck note: stopping, losing the audio device, or closing the window leaves
  the engine silent.
- The launch state is a factory pattern that already sounds like acid, not an empty grid.

## Review Focus

The five inputs/conditions most likely to bite a player. Each has a test in the task named.

1. **Device sample rate is 44 100 or 96 000, not 48 000** — pitch, step timing, filter cutoff and
   glide must all track the real rate. Test: Phase 02 Task 5, Phase 03 Task 10, Phase 05 Task 21.
2. **Tempo or pattern changed mid-note while RUN is on** — no double trigger, no dropped step, no
   stuck note. Tests: Phase 05 Task 26, Phase 07 Task 35.
3. **Resonance and drive both pushed to maximum** — output stays under 0.999, no NaN, no runaway
   self-oscillation buzz. Test: Phase 04 Task 18, Phase 09 Task 44.
4. **A step edited during playback** — takes effect at the next pass of that step, never corrupts the
   pattern being read by the audio thread. Test: Phase 07 Task 35, Phase 08 Task 38.
5. **Audio device disappears or refuses to open (other app exclusive, unplugged USB, no ALSA
   device)** — the UI still opens, shows the failure, and never hangs or leaves a phantom voice.
   Tests: Phase 02 Task 8/9, Phase 11 Task 55.

## Phase index

| File | Phase | Tasks | Delivers |
|---|---|---|---|
| [01-scaffold.md](01-scaffold.md) | Workspace scaffold | 1–4 | Two-crate workspace, pitch model, knob mappings, empty window |
| [02-audio-engine.md](02-audio-engine.md) | Audio output & engine skeleton | 5–9 | `Engine::render`, cpal stream, device selection, recovery |
| [03-oscillator.md](03-oscillator.md) | Oscillator & glide | 10–13 | Saw/square, tuning, slide |
| [04-filter-vca.md](04-filter-vca.md) | Filter, envelope, accent | 14–19 | Full voice that sounds like a 303 |
| [05-sequencer.md](05-sequencer.md) | Sequencer & patterns | 20–26 | 16-step playback, swing, accent/slide, sample-accurate |
| [06-ui-panel.md](06-ui-panel.md) | UI panel foundation | 27–32 | Black/yellow panel, knobs, step keys, display |
| [07-ui-wiring.md](07-ui-wiring.md) | UI ↔ audio wiring | 33–37 | Live knobs, transport, position LEDs |
| [08-patterns-persistence.md](08-patterns-persistence.md) | Pattern editing & saving | 38–42 | Gestures, banks, JSON persistence, factory preset |
| [09-master-section.md](09-master-section.md) | Drive, limiter, level | 43–46 | Gain staging, safety limiter, volume |
| [10-midi.md](10-midi.md) | MIDI input & sync | 47–51 | Note in, clock sync, CC map, no-device path |
| [11-polish.md](11-polish.md) | Polish & ergonomics | 52–56 | Keyboard play, shortcuts, help, device picker |
| [12-deferred-accuracy.md](12-deferred-accuracy.md) | Deferred accuracy backlog | — | Anti-aliasing, nonlinear models, calibration |

## Execution status

Updated 2026-11-06. The ledger with per-task evidence and every ruling lives in
`.superpowers/sdd/00-main-plan/progress.md` (git-ignored); `git log` is the authoritative record.

| Phase | Tasks | Status |
|---|---|---|
| 01 scaffold | 1–4 | **done** |
| 02 audio & engine | 5–9 | **done** |
| 03 oscillator | 10–13 | **done** |
| 04 filter, envelope, VCA | 14–19 | in progress: 14–18 done; 19 integration next |
| 05 sequencer | 20–26 | not started |
| 06 panel | 27–32 | not started |
| 07 wiring | 33–37 | not started |
| 08 patterns & persistence | 38–42 | not started |
| 09 master section | 43–46 | not started |
| 10 MIDI | 47–51 | not started |
| 11 polish | 52–56 | not started |
| 12 deferred accuracy | — | backlog |

Current tree: 59 tests green in 19 suites, `cargo clippy --workspace --all-targets -- -D warnings` clean,
app opens a 48 kHz stream and plays a held note on the temporary RUN button. Resume at **Task 19**.

## Execution notes

- Run phases in order. Within a phase, tasks are ordered and each ends green.
- Each task's **Interfaces** block is the contract neighbouring tasks rely on; if an implementer must
  deviate, they update the later task files in the same commit.
- The engine is always testable without a sound card: `acid-core` renders offline. Use that for every
  DSP assertion. Manual listen checks are labelled **Listen:** and are not substitutes for tests.
- Start the app with `cargo run -p acid-app`; it prints the chosen device, sample rate and buffer size
  on startup from Phase 02 onward.
- Do not add features not in this plan while executing it. Ideas go in `plans/12-deferred-accuracy.md`
  under **Backlog**.
