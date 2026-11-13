# Phase 03 — Oscillator, waveform, tuning and slide

> Part of [00-main-plan.md](00-main-plan.md). Tasks **10–13**. Depends on: Phase 02.
> **Delivers:** the real oscillator behind `Voice` — sawtooth and square from one phase accumulator,
> a tuning knob, and the slide (portamento) the sequencer will drive in Phase 05.

**Why this phase before the filter:** pitch errors are cheap to find on a bare oscillator and
expensive to find through a filter. The oscillator is naive by design: band-limiting it is Phase 12,
and the panel's WAVEFORM switch must work from the moment it exists.

---

### Task 10: Sawtooth and square oscillator from one phase accumulator

**Files:**
- Create: `rust/crates/acid-core/src/oscillator.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod oscillator;`)
- Test: `rust/crates/acid-core/tests/oscillator.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub enum Waveform { Saw, Square }` — `Copy, Clone, PartialEq, Eq, Debug, Default` (default `Saw`),
    serde-capable.
  - `pub struct Oscillator` with `pub fn new(sample_rate: f32) -> Self`,
    `pub fn set_frequency(&mut self, hz: f32)`, `pub fn set_waveform(&mut self, w: Waveform)`,
    `pub fn next_sample(&mut self) -> f32`, `pub fn reset_phase(&mut self)`.
  - `pub const MIN_HZ: f32 = 16.0;` `pub const MAX_HZ: f32 = 4_000.0;` — frequencies are clamped to
    this range inside `set_frequency`.
- Later tasks rely on these exact names: Phase 03 Task 11 (Voice wiring), Phase 05 Task 24 (slide
  target), Phase 06 Task 27 (WAVEFORM switch).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/oscillator.rs
use acid_core::analysis::{dominant_hz, rms};
use acid_core::oscillator::{Oscillator, Waveform};

fn render(wave: Waveform, hz: f32, sr: f32, n: usize) -> Vec<f32> {
    let mut o = Oscillator::new(sr);
    o.set_frequency(hz);
    o.set_waveform(wave);
    (0..n).map(|_| o.next_sample()).collect()
}

#[test]
fn saw_holds_the_requested_pitch_at_44k1_and_48k() {
    for sr in [44_100.0, 48_000.0] {
        let buf = render(Waveform::Saw, 110.0, sr, 65_536);
        let hz = dominant_hz(&buf, sr);
        assert!((hz - 110.0).abs() < 1.1, "sr {sr} got {hz}");
    }
}

#[test]
fn square_holds_the_same_pitch_as_saw_without_switching() {
    let sr = 48_000.0;
    let saw = render(Waveform::Saw, 220.0, sr, 65_536);
    let sq = render(Waveform::Square, 220.0, sr, 65_536);
    assert!((dominant_hz(&saw, sr) - dominant_hz(&sq, sr)).abs() < 2.0);
}

#[test]
fn both_waveforms_are_bipolar_and_dc_free() {
    let sr = 48_000.0;
    for w in [Waveform::Saw, Waveform::Square] {
        let buf = render(w, 55.0, sr, 48_000);
        let mean: f32 = buf.iter().sum::<f32>() / buf.len() as f32;
        assert!(mean.abs() < 0.02, "{w:?} mean {mean}");
        assert!(rms(&buf) > 0.4, "{w:?} too quiet");
        assert!(buf.iter().all(|s| s.abs() <= 1.001));
    }
}

#[test]
fn frequency_is_clamped_and_never_produces_nan() {
    let mut o = Oscillator::new(48_000.0);
    o.set_frequency(0.0);
    let buf: Vec<f32> = (0..1_000).map(|_| o.next_sample()).collect();
    assert!(buf.iter().all(|s| s.is_finite()));
    o.set_frequency(1.0e9);
    let buf: Vec<f32> = (0..1_000).map(|_| o.next_sample()).collect();
    assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 1.001));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test oscillator`
Expected: FAIL — unresolved import `acid_core::oscillator`.

- [ ] **Step 3: Implement `oscillator.rs`**

`phase: f64` in `0.0..1.0`, `inc: f64 = hz as f64 / sample_rate as f64`. `next_sample`: read the value
for the current phase (`2.0 * phase - 1.0` for saw; `if phase < 0.5 { 1.0 } else { -1.0 }` for square),
then `phase = (phase + inc).fract()`. Both waveforms read the same accumulator so switching cannot
change pitch. No `sin`, no table, no anti-aliasing (Phase 12).

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test oscillator`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): saw and square oscillator from a shared phase accumulator"
```

---

### Task 11: Voice uses the oscillator, exposes waveform and tuning

**Files:**
- Modify: `rust/crates/acid-core/src/voice.rs` (replace the sine placeholder), `src/engine.rs`
- Test: `rust/crates/acid-core/tests/voice_tuning.rs`

**Interfaces:**
- Consumes: `Oscillator`, `Waveform`, `param::tuning_semitones`.
- Produces:
  - `Voice`: `pub fn set_waveform(&mut self, w: Waveform)`,
    `pub fn set_tuning_semitones(&mut self, semitones: f32)`.
  - `Engine`: `pub fn set_waveform(&mut self, w: Waveform)`,
    `pub fn set_tuning(&mut self, knob: f32)` (knob domain, converted with `tuning_semitones`).
- Later tasks rely on `Engine::set_waveform` and `Engine::set_tuning` (Phase 07 Task 34 wires knobs,
  Phase 08 Task 40 saves them).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/voice_tuning.rs
use acid_core::analysis::dominant_hz;
use acid_core::engine::Engine;
use acid_core::oscillator::Waveform;

fn pitch_of(e: &mut Engine, hz: f32) -> f32 {
    e.note_on(hz, false);
    let mut buf = vec![0.0; 65_536];
    e.render(&mut buf);
    dominant_hz(&buf[65_536 - 32_768..], e.sample_rate())
}

#[test]
fn tuning_knob_bends_the_pitch_by_an_octave_each_way() {
    let mut e = Engine::new(48_000.0);
    e.set_tuning(0.5);
    let base = pitch_of(&mut e, 110.0);
    e.note_off();
    e.set_tuning(1.0);
    let up = pitch_of(&mut e, 110.0);
    e.note_off();
    e.set_tuning(0.0);
    let down = pitch_of(&mut e, 110.0);
    assert!((base - 110.0).abs() < 2.0, "base {base}");
    assert!((up - 220.0).abs() < 5.0, "up {up}");
    assert!((down - 55.0).abs() < 2.0, "down {down}");
}

#[test]
fn waveform_switch_changes_timbre_not_pitch() {
    let mut e = Engine::new(48_000.0);
    e.set_tuning(0.5);
    e.set_waveform(Waveform::Saw);
    let saw = pitch_of(&mut e, 82.4);
    e.note_off();
    e.set_waveform(Waveform::Square);
    let square = pitch_of(&mut e, 82.4);
    assert!((saw - square).abs() < 2.0, "saw {saw} square {square}");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test voice_tuning`
Expected: FAIL — no `set_waveform` / `set_tuning`.

- [ ] **Step 3: Implement**

`Voice` keeps `base_hz: f32`, `tuning_semitones: f32`, `waveform: Waveform`, and recomputes the
oscillator frequency as `base_hz * 2f32.powf(tuning_semitones / 12.0)` whenever either changes.
Delete the sine path; keep the 20 ms gate fade from Task 7.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test voice_tuning`
Expected: PASS — 2 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): voice uses the oscillator and honours tuning and waveform"
```

---

### Task 12: Slide — gliding to a new pitch without retriggering

**Files:**
- Modify: `rust/crates/acid-core/src/voice.rs`
- Test: `rust/crates/acid-core/tests/voice_glide.rs`

**Interfaces:**
- Consumes: `Oscillator::set_frequency`.
- Produces:
  - `Voice`: `pub fn glide_to(&mut self, hz: f32)` — exponential approach with time constant
    `GLIDE_TC_MS = 60.0`, gate stays open, envelope is *not* retriggered.
  - `Voice`: `pub fn set_pitch(&mut self, hz: f32)` — immediate jump (unchanged behaviour).
  - `Voice::glide_target_hz(&self) -> f32` for tests and UI readback.
- Later tasks rely on `glide_to`: Phase 05 Task 24 (slide steps), Phase 08 Task 38 (slide editing).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn glide_reaches_the_target_and_never_overshoots() {
    let mut v = Voice::new(48_000.0);
    v.note_on(55.0, false);
    v.render(&mut vec![0.0; 4_800]);
    v.glide_to(110.0);
    let mut buf = vec![0.0; 24_000]; // 500 ms
    v.render(&mut buf);
    let early = dominant_hz(&buf[0..2_400], 48_000.0);
    let late = dominant_hz(&buf[21_600..], 48_000.0);
    assert!(early > 55.0 && early < 110.0, "should be mid-glide, got {early}");
    assert!((late - 110.0).abs() < 1.5, "should have arrived, got {late}");
}

#[test]
fn glide_does_not_retrigger_the_gate() {
    let mut v = Voice::new(48_000.0);
    v.note_on(55.0, false);
    let mut buf = vec![0.0; 4_800];
    v.render(&mut buf);
    let before = rms(&buf[buf.len() - 2_400..]);
    v.glide_to(110.0);
    let mut buf = vec![0.0; 2_400];
    v.render(&mut buf);
    let after = rms(&buf);
    assert!((after / before - 1.0).abs() < 0.3, "level jumped: {before} -> {after}");
    assert!(v.is_active());
}

#[test]
fn set_pitch_is_immediate() {
    let mut v = Voice::new(48_000.0);
    v.note_on(55.0, false);
    v.set_pitch(220.0);
    let mut buf = vec![0.0; 4_096];
    v.render(&mut buf);
    assert!((dominant_hz(&buf, 48_000.0) - 220.0).abs() < 3.0);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test voice_glide`
Expected: FAIL — no `glide_to`.

- [ ] **Step 3: Implement the glide**

Keep `current_hz: f32` and `target_hz: f32`. In `render`, per sample move `current_hz` toward
`target_hz` with the one-pole coefficient for `GLIDE_TC_MS` at the voice's sample rate (compute the
coefficient once in `new`, not per sample), then `oscillator.set_frequency(current_hz)`. Guard:
`if (target_hz - current_hz).abs() < 0.01 { current_hz = target_hz }` so the oscillator stops
recomputing forever.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test voice_glide`
Expected: PASS — 3 passed.
**Listen:** temporarily call `glide_to` in `Engine::start` and run the app — a 55 Hz saw should scoop
up to 110 Hz over ~200 ms with no click and no level jump. Revert the temporary call.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): portamento glide with gate continuity"
```

---

### Task 13: WAVEFORM switch and TUNING knob reachable from `Engine`

**Files:**
- Modify: `rust/crates/acid-core/src/engine.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (a temporary WAVEFORM toggle and TUNING slider)
- Test: `rust/crates/acid-core/tests/engine_tuning_sweep.rs`

**Interfaces:**
- Consumes: `Engine::set_waveform`, `Engine::set_tuning`, `Voice::glide_to`.
- Produces: `Engine` applies `set_waveform` and `set_tuning` to a *live* note without retriggering
  (verified by the test below). This is the pattern every later knob follows.
- Later tasks rely on it: Phase 07 Task 34 (all seven knobs), Phase 08 Task 40 (state persistence).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn knobs_can_be_moved_while_a_note_is_sounding() {
    let mut e = Engine::new(48_000.0);
    e.note_on(110.0, false);
    e.render(&mut vec![0.0; 4_800]);

    e.set_tuning(0.75); // +6 semitones
    let mut buf = vec![0.0; 9_600];
    e.render(&mut buf);
    let hz = dominant_hz(&buf[4_800..], 48_000.0);
    assert!((hz - 155.5).abs() < 3.0, "expected ~155.6 Hz, got {hz}");

    e.set_waveform(Waveform::Square);
    let mut buf = vec![0.0; 4_800];
    e.render(&mut buf);
    assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 1.001));
    assert!(e.is_playing_something(), "a live note must survive a knob move");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test engine_tuning_sweep`
Expected: FAIL — no `is_playing_something`.

- [ ] **Step 3: Implement**

`Engine::is_playing_something(&self) -> bool` forwards `voice.is_active()`. `set_waveform` and
`set_tuning` forward to `Voice` (pitch recomputed from `base_hz`, glide target untouched). Add the two
temporary controls to the app's UI so the switch is playable by hand.

- [ ] **Step 4: Run to confirm passing and listen**

Run: `cd rust && cargo test -p acid-core --test engine_tuning_sweep`
Expected: PASS — 1 passed.
**Listen:** `cargo run -p acid-app` — the saw/square toggle is audibly brighter/duller at the same
pitch, and the tuning slider sweeps ±1 octave smoothly with no zipper noise.

- [ ] **Step 5: Commit**

```bash
git add rust/crates
git commit -m "feat(core): live waveform and tuning changes from the engine"
```

---

**Phase 03 done when:** `cargo test --workspace` is green, and by hand in the app a held note can be
switched between saw and square and bent an octave each way while it sounds.
