# Phase 05 — Sequencer and patterns

> Part of [00-main-plan.md](00-main-plan.md). Tasks **20–26**. Depends on: Phase 04.
> **Delivers:** the 16-step pattern model, tempo and swing maths, step-to-voice playback with accent and
> slide, sample-accurate scheduling, and a 64-pattern bank that can be swapped while it runs.

**Why the pattern model comes first:** everything else in the app — the step keys, the file format, the
factory preset — is a view of this one small type. It is also `acid-core`'s only serde type, so the
model's shape is locked before any UI exists.

---

### Task 20: `Step`, `Pattern` and the pattern identity

**Files:**
- Create: `rust/crates/acid-core/src/pattern.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod pattern;`)
- Test: `rust/crates/acid-core/tests/pattern.rs`

**Interfaces:**
- Consumes: `pitch::{Pitch, step_hz}`.
- Produces:
  - `pub const STEPS_PER_PATTERN: usize = 16;`
  - `pub struct Step { pub on: bool, pub pitch: Pitch, pub octave_offset: i8, pub accent: bool,
    pub slide: bool }` — `Copy, Clone, PartialEq, Debug, Default` (default = rest, pitch `C`, offset 0),
    serde-capable.
  - `impl Step { pub fn note(pitch: Pitch, octave_offset: i8) -> Self; pub fn rest() -> Self;
    pub fn hz(&self) -> f32 }`
  - `pub struct Pattern { pub steps: [Step; STEPS_PER_PATTERN] }` — serde-capable,
    `impl Pattern { pub fn empty() -> Self; pub fn from_steps(steps: [Step; 16]) -> Self }`
  - `pub struct PatternId { pub bank: u8, pub number: u8 }` — `bank` 0–7 (A–H), `number` 0–7 (displayed
    1–8); `impl std::fmt::Display for PatternId` renders `A1`..`H8`.
- Later tasks rely on every field name: Phase 06 Task 30 (step keys), Phase 08 Task 38 (editing
  gestures), Task 40 (JSON), Task 42 (factory preset).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/pattern.rs
use acid_core::pattern::{Pattern, PatternId, Step, STEPS_PER_PATTERN};
use acid_core::pitch::Pitch;

#[test]
fn a_new_pattern_is_sixteen_rests() {
    let p = Pattern::empty();
    assert_eq!(p.steps.len(), STEPS_PER_PATTERN);
    assert!(p.steps.iter().all(|s| !s.on && !s.accent && !s.slide));
}

#[test]
fn a_note_step_reports_its_frequency() {
    let s = Step::note(Pitch::A, 0);
    assert!(s.on);
    assert!((s.hz() - 110.0).abs() < 0.1);
    let up = Step::note(Pitch::A, 1);
    assert!((up.hz() - 220.0).abs() < 0.2);
}

#[test]
fn patterns_round_trip_through_json() {
    let mut p = Pattern::empty();
    p.steps[0] = Step { accent: true, ..Step::note(Pitch::E, 0) };
    p.steps[3] = Step { slide: true, ..Step::note(Pitch::DSharp, 0) };
    let json = serde_json::to_string(&p).unwrap();
    let back: Pattern = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn pattern_ids_read_like_the_hardware() {
    assert_eq!(PatternId { bank: 0, number: 0 }.to_string(), "A1");
    assert_eq!(PatternId { bank: 2, number: 4 }.to_string(), "C5");
    assert_eq!(PatternId { bank: 7, number: 7 }.to_string(), "H8");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test pattern`
Expected: FAIL — unresolved import `acid_core::pattern`; `serde_json` not a dev-dependency yet
(`cargo add --dev -p acid-core serde_json`).

- [ ] **Step 3: Implement the model**

Derive `Serialize`/`Deserialize` behind the existing `serde` feature. `Step::hz` is
`step_hz(self.pitch, self.octave_offset)`. `PatternId::Display` uses `(b'A' + bank)` and
`number + 1`. `Pitch` has no `Sharp` variant — the accidentals are `CSharp`, `DSharp`, `FSharp`, `GSharp`,
`ASharp`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test pattern`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): 16-step pattern model with json round-trip"
```

---

### Task 21: Tempo → step duration, and swing

**Files:**
- Create: `rust/crates/acid-core/src/timing.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod timing;`)
- Test: `rust/crates/acid-core/tests/timing.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub const MIN_BPM: f32 = 40.0;` `pub const MAX_BPM: f32 = 300.0;`
  - `pub const DEFAULT_BPM: f32 = 130.0;`
  - `pub fn clamp_bpm(bpm: f32) -> f32`
  - `pub fn samples_per_step(bpm: f32, sample_rate: f32) -> f64` — a 16th note:
    `sample_rate * 60 / (bpm * 4)`.
  - `pub fn step_duration_samples(step_index: usize, bpm: f32, sample_rate: f32, swing: f32) -> f64` —
    even steps get `1 + 0.5·swing` of a step, odd steps `1 - 0.5·swing`; `swing` clamped to `0.0..=1.0`.
    Each *pair* of steps always lasts exactly two steps, so the loop length never drifts.
- Later tasks rely on these names: Task 23 (clock), Task 25 (scheduling), Phase 06 Task 31 (tempo
  display), Phase 11 Task 53 (tempo shortcuts).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/timing.rs
use acid_core::timing::*;

#[test]
fn a_16th_at_120_bpm_is_12000_samples_at_48k() {
    assert!((samples_per_step(120.0, 48_000.0) - 12_000.0).abs() < 1e-6);
}

#[test]
fn a_16th_at_120_bpm_is_11025_samples_at_44k1() {
    assert!((samples_per_step(120.0, 44_100.0) - 11_025.0).abs() < 1e-6);
}

#[test]
fn tempo_is_clamped_to_the_panel_range() {
    assert_eq!(clamp_bpm(10.0), MIN_BPM);
    assert_eq!(clamp_bpm(999.0), MAX_BPM);
    assert_eq!(clamp_bpm(140.0), 140.0);
}

#[test]
fn swing_lengthens_even_steps_and_shortens_odd_ones() {
    let step = samples_per_step(120.0, 48_000.0);
    let even = step_duration_samples(0, 120.0, 48_000.0, 1.0);
    let odd = step_duration_samples(1, 120.0, 48_000.0, 1.0);
    assert!((even - step * 1.5).abs() < 1e-6, "even {even}");
    assert!((odd - step * 0.5).abs() < 1e-6, "odd {odd}");
}

#[test]
fn a_pair_of_steps_always_lasts_two_steps() {
    for swing in [0.0, 0.25, 0.5, 1.0] {
        let pair = step_duration_samples(0, 137.0, 44_100.0, swing)
            + step_duration_samples(1, 137.0, 44_100.0, swing);
        let expected = 2.0 * samples_per_step(137.0, 44_100.0);
        assert!((pair - expected).abs() < 1e-6, "swing {swing}: {pair} vs {expected}");
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test timing`
Expected: FAIL — unresolved import `acid_core::timing`.

- [ ] **Step 3: Implement `timing.rs`**

Clamp `bpm` with `clamp_bpm`, `swing` with `swing.clamp(0.0, 1.0)`. Return `f64` so long runs never
lose precision.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test timing`
Expected: PASS — 5 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): tempo and swing step timing"
```

---

### Task 22: Clock that counts down to the next step boundary

**Files:**
- Create: `rust/crates/acid-core/src/clock.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod clock;`)
- Test: `rust/crates/acid-core/tests/clock.rs`

**Interfaces:**
- Consumes: `timing::{samples_per_step, step_duration_samples, clamp_bpm, DEFAULT_BPM}`.
- Produces:
  - `pub struct Clock` with `pub fn new(sample_rate: f32) -> Self`,
    `pub fn set_bpm(&mut self, bpm: f32)`, `pub fn set_swing(&mut self, swing: f32)`,
    `pub fn bpm(&self) -> f32`, `pub fn swing(&self) -> f32`,
    `pub fn step_index(&self) -> usize`,
    `pub fn samples_until_next_step(&self) -> f64`,
    `pub fn advance_by(&mut self, samples: usize) -> usize` — returns how many step boundaries were
    crossed, updating `step_index` for each; carries the remainder forward without drift.
  - Starting state: `step_index() == 0` and `samples_until_next_step() == samples_per_step(...)` for
    step 0 (the first step starts immediately when the transport starts).
- Later tasks rely on this API: Task 25 (`Engine::render` splits blocks at boundaries), Phase 07
  Task 36 (UI reads `step_index`).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/clock.rs
use acid_core::clock::Clock;

#[test]
fn four_steps_take_exactly_four_step_durations() {
    let mut c = Clock::new(48_000.0);
    c.set_bpm(120.0);
    let crossed = c.advance_by(48_000); // one second = 4 sixteenths
    assert_eq!(crossed, 4);
    assert_eq!(c.step_index(), 4);
    assert!(c.samples_until_next_step() > 11_990.0 && c.samples_until_next_step() <= 12_000.0);
}

#[test]
fn odd_block_sizes_do_not_accumulate_drift() {
    let mut c = Clock::new(44_100.0);
    c.set_bpm(137.0);
    let total = 44_100 * 10;
    let mut consumed = 0;
    while consumed < total {
        let n = (total - consumed).min(337);
        c.advance_by(n);
        consumed += n;
    }
    let expected_steps = (total as f64 / acid_core::timing::samples_per_step(137.0, 44_100.0)) as usize;
    assert!((c.step_index() as i64 - expected_steps as i64).abs() <= 1);
}

#[test]
fn the_clock_counts_steps_without_wrapping() {
    // Wrapping to 0–15 is the sequencer's job (Task 23); the clock counts forever.
    let mut c = Clock::new(48_000.0);
    c.set_bpm(300.0);
    let crossed = c.advance_by(48_000 * 4);
    assert!(crossed > 16, "four seconds at 300 bpm is more than 16 steps, got {crossed}");
    assert_eq!(c.step_index(), crossed);
    assert_eq!(Clock::new(48_000.0).step_index(), 0);
}

#[test]
fn tempo_changes_take_effect_on_the_next_boundary() {
    let mut c = Clock::new(48_000.0);
    c.set_bpm(120.0);
    c.advance_by(6_000);
    c.set_bpm(240.0); // step duration halves
    let crossed = c.advance_by(6_000);
    assert_eq!(crossed, 2, "at 240 bpm, 6000 samples is two further steps");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test clock`
Expected: FAIL — unresolved import `acid_core::clock`.

- [ ] **Step 3: Implement the clock**

State: `remaining: f64` (samples left in the current step), `step_index: usize` (unbounded, wraparound
done by callers via `% 16`), `bpm`, `swing`. `advance_by(n)`: repeatedly subtract `remaining`; when it
reaches zero, increment `step_index`, set `remaining = step_duration_samples(step_index, bpm, sr,
swing)`, and count the crossing. `set_bpm` only affects *future* step durations (do not rescale
`remaining`), so a tempo change never jumps backwards.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test clock`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): drift-free step clock with swing"
```

---

### Task 23: Sequencer turns steps into voice events

**Files:**
- Create: `rust/crates/acid-core/src/sequencer.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod sequencer;`)
- Test: `rust/crates/acid-core/tests/sequencer.rs`

**Interfaces:**
- Consumes: `Clock`, `Pattern`, `Step`, `Voice::{note_on, glide_to, note_off, set_pitch}`.
- Produces:
  - `pub struct Sequencer` with `pub fn new(sample_rate: f32) -> Self`,
    `pub fn set_pattern(&mut self, pattern: std::sync::Arc<Pattern>)`,
    `pub fn pattern(&self) -> &Arc<Pattern>`,
    `pub fn set_bpm(&mut self, bpm: f32)`, `pub fn set_swing(&mut self, swing: f32)`,
    `pub fn start(&mut self)`, `pub fn stop(&mut self)`, `pub fn is_running(&self) -> bool`,
    `pub fn position(&self) -> usize` (0–15), `pub fn clock(&self) -> &Clock`,
    `pub fn render(&mut self, voice: &mut Voice, out: &mut [f32])`.
  - Trigger rule, evaluated at each boundary for step `i`: no `on` → `voice.note_off()`; `on` and the
    *previous* step carried `slide` → `voice.glide_to(step.hz())` (no envelope retrigger); `on`
    otherwise → `voice.note_on(step.hz(), step.accent)`. `start` triggers step 0 immediately;
    `stop` calls `voice.note_off()`.
- Later tasks rely on: `render`, `position`, `set_pattern`, `set_bpm`, `set_swing` (Task 25, Phase 07
  Task 35/36).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/sequencer.rs
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;
use acid_core::sequencer::Sequencer;
use acid_core::voice::Voice;
use std::sync::Arc;

fn pattern_with(indices: &[(usize, Step)]) -> Arc<Pattern> {
    let mut p = Pattern::empty();
    for (i, s) in indices {
        p.steps[*i] = *s;
    }
    Arc::new(p)
}

#[test]
fn a_single_note_reaches_the_voice_on_step_zero() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(pattern_with(&[(0, Step::note(Pitch::A, -1))]));
    seq.set_bpm(120.0);
    let mut v = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; 4_800];
    seq.render(&mut v, &mut buf);
    assert!(acid_core::analysis::rms(&buf[1_000..]) > 0.01);
}

#[test]
fn rests_close_the_gate_for_exactly_their_step() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(pattern_with(&[(0, Step::note(Pitch::A, 0))])); // steps 1..15 are rests
    seq.set_bpm(120.0);
    let mut v = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; 24_000]; // two steps
    seq.render(&mut v, &mut buf);
    let first = acid_core::analysis::rms(&buf[2_000..10_000]);
    let second = acid_core::analysis::rms(&buf[13_000..23_000]);
    assert!(first > 0.01, "first step silent");
    assert!(second < first * 0.1, "second step should be a rest");
}

#[test]
fn position_wraps_at_sixteen_and_stop_closes_the_gate() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    seq.set_pattern(Arc::new(Pattern::empty()));
    seq.set_bpm(300.0);
    seq.start();
    let mut v = Voice::new(sr);
    seq.render(&mut v, &mut vec![0.0; 4_800]);
    assert!(seq.position() < 16);
    seq.stop();
    assert!(!seq.is_running());
    let mut buf = vec![0.0; 4_800];
    seq.render(&mut v, &mut buf);
    assert!(acid_core::analysis::rms(&buf) < 0.001);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test sequencer`
Expected: FAIL — unresolved import `acid_core::sequencer`.

- [ ] **Step 3: Implement the sequencer**

`render` loop: while frames remain, `take = min(frames_left, clock.samples_until_next_step().floor() as
usize)`; render `take` frames from the voice; `advance_by(take)`; if a boundary was crossed, apply the
trigger rule for `position()`. Note the order: the trigger for step `i` happens **after** the samples of
step `i-1` were rendered, so step 0 sounds immediately after `start()`. Keep `prev_slide: bool`
alongside `position`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test sequencer`
Expected: PASS — 3 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): sequencer drives the voice from pattern steps"
```

---

### Task 24: Accent and slide in playback

**Files:**
- Modify: `rust/crates/acid-core/src/sequencer.rs`
- Test: `rust/crates/acid-core/tests/sequencer_feel.rs`

**Interfaces:**
- Consumes: `Step::{accent, slide}`, `Voice::glide_to`.
- Produces: the audible rules — an accented step is louder and brighter; a step following a `slide`
  step glides into pitch instead of retriggering, so a slide chain sounds continuous.
- Later tasks rely on: nothing new (this is behaviour, not API).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn an_accented_step_peaks_higher_than_a_plain_one() {
    let sr = 48_000.0;
    let peak_of = |accent: bool| {
        let mut seq = Sequencer::new(sr);
        let step = Step { accent, ..Step::note(Pitch::A, 0) };
        seq.set_pattern(Arc::new(Pattern::from_steps([step; 16])));
        seq.set_bpm(120.0);
        let mut v = Voice::new(sr);
        seq.start();
        let mut buf = vec![0.0; 12_000];
        seq.render(&mut v, &mut buf);
        peak(&buf[1_000..])
    };
    assert!(peak_of(true) > peak_of(false) * 1.2);
}

#[test]
fn a_slide_step_does_not_restart_the_envelope() {
    let sr = 48_000.0;
    let mut seq = Sequencer::new(sr);
    let mut first = Step::note(Pitch::A, -1);
    first.slide = true;
    let second = Step::note(Pitch::A, 0);
    let mut steps = [Step::rest(); 16];
    steps[0] = first;
    steps[1] = second;
    seq.set_pattern(Arc::new(Pattern::from_steps(steps)));
    seq.set_bpm(120.0);
    let mut v = Voice::new(sr);
    seq.start();
    let mut buf = vec![0.0; 24_000];
    seq.render(&mut v, &mut buf);
    // Step 1 is entered by glide: the level must not dip to silence at the boundary,
    // and the pitch must have risen by the end of it.
    let at_boundary = rms(&buf[11_800..12_200]);
    let before_boundary = rms(&buf[8_000..11_000]);
    assert!(at_boundary > before_boundary * 0.5, "level collapsed: {before_boundary} -> {at_boundary}");
    assert!(dominant_hz(&buf[18_000..], sr) > 90.0, "should have glided up an octave");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test sequencer_feel`
Expected: FAIL — the accent step is not louder (accent not wired), and the slide boundary drops to
silence because every step calls `note_on`.

- [ ] **Step 3: Implement**

Ensure the trigger rule from Task 23 uses `prev_step.slide` (not the current step's flag) and passes
`step.accent` into `note_on`. Accent gain and filter multiplier already exist from Phase 04.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test sequencer_feel`
Expected: PASS — 2 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): accent and slide playback rules"
```

---

### Task 25: `Engine` runs the sequencer, splitting blocks at step boundaries

**Files:**
- Modify: `rust/crates/acid-core/src/engine.rs`
- Test: `rust/crates/acid-core/tests/engine_transport.rs`

**Interfaces:**
- Consumes: `Sequencer`, `Pattern`, `timing::DEFAULT_BPM`.
- Produces (`Engine` methods, final names):
  - `pub fn set_pattern(&mut self, pattern: std::sync::Arc<Pattern>)`
  - `pub fn set_bpm(&mut self, bpm: f32)`, `pub fn bpm(&self) -> f32`
  - `pub fn set_swing(&mut self, swing: f32)`
  - `pub fn position(&self) -> usize`, `pub fn is_playing(&self) -> bool` (already exists)
  - `pub fn start(&mut self)` now starts the sequencer; `pub fn stop(&mut self)` stops it and closes the
    gate.
- Later tasks rely on every one of these: Phase 07 Task 35/36, Phase 08 Task 40, Phase 10 Task 49.

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/engine_transport.rs
#[test]
fn a_step_boundary_lands_on_the_exact_expected_sample() {
    let sr = 48_000.0;
    let mut e = Engine::new(sr);
    let mut steps = [Step::rest(); 16];
    steps[0] = Step::note(Pitch::C, 0); // 55 Hz
    steps[2] = Step::note(Pitch::C, 1); // one octave up, at sample 24000
    e.set_pattern(Arc::new(Pattern::from_steps(steps)));
    e.set_bpm(120.0);
    e.start();
    let mut buf = vec![0.0; 36_000];
    e.render(&mut buf);
    let first_sound = buf.iter().position(|s| s.abs() > 1e-4).unwrap();
    let second_sound = (24_000..).find(|&i| buf[i].abs() > 1e-4).unwrap();
    assert!(first_sound <= 2, "step 0 started at {first_sound}");
    assert!(second_sound >= 24_000 && second_sound <= 24_002, "step 2 at {second_sound}");
}

#[test]
fn block_size_never_changes_the_output() {
    let make = || {
        let mut e = Engine::new(44_100.0);
        let mut steps = [Step::rest(); 16];
        steps[0] = Step::note(Pitch::A, -1);
        steps[4] = Step { accent: true, ..Step::note(Pitch::E, 0) };
        e.set_pattern(Arc::new(Pattern::from_steps(steps)));
        e.set_bpm(137.0);
        e.set_swing(0.4);
        e.start();
        e
    };
    let mut one = make();
    let mut a = vec![0.0; 44_100];
    one.render(&mut a);
    let mut many = make();
    let mut b = Vec::new();
    for n in [1usize, 3, 511, 997, 4096, 38_492] {
        let mut chunk = vec![0.0; n];
        many.render(&mut chunk);
        b.extend_from_slice(&chunk);
    }
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert!((x - y).abs() < 1e-6, "sample {i}: {x} vs {y}");
    }
}

#[test]
fn position_advances_and_stop_silences() {
    let mut e = Engine::new(48_000.0);
    e.set_pattern(Arc::new(Pattern::from_steps([Step::note(Pitch::A, -1); 16])));
    e.set_bpm(240.0);
    e.start();
    e.render(&mut vec![0.0; 24_000]);
    assert!(e.position() > 0);
    e.stop();
    let mut buf = vec![0.0; 4_800];
    e.render(&mut buf);
    assert!(acid_core::analysis::rms(&buf[2_400..]) < 0.001);
    assert!(!e.is_playing());
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test engine_transport`
Expected: FAIL — `Engine` has no `set_pattern`/`position`.

- [ ] **Step 3: Implement**

`Engine::render` delegates to `sequencer.render(&mut voice, out)` when `playing`, otherwise
`out.fill(0.0)`. `Engine::start` calls `sequencer.start()` (which triggers step 0); `stop` calls
`sequencer.stop()`. Delete the Task 9 placeholder `note_on` in `start`.

- [ ] **Step 4: Run to confirm passing, then listen**

Run: `cd rust && cargo test -p acid-core --test engine_transport`
Expected: PASS — 3 passed.
**Listen:** the app still has no step editor, so use a temporary test pattern in `Engine::new`. A
16-note pattern at 130 bpm should loop seamlessly with no click at the wrap; stop must be instant and
silent.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): engine runs the sequencer with sample-accurate boundaries"
```

---

### Task 26: Pattern bank of 64 and live pattern switching

**Files:**
- Create: `rust/crates/acid-core/src/bank.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod bank;`), `src/engine.rs`
- Test: `rust/crates/acid-core/tests/bank.rs`

**Interfaces:**
- Consumes: `pattern::{Pattern, PatternId, STEPS_PER_PATTERN}`, `Engine::set_pattern`.
- Produces:
  - `pub const BANKS: usize = 8;` `pub const PATTERNS_PER_BANK: usize = 8;`
  - `pub struct PatternBank { patterns: Vec<Pattern>, current: usize }` with
    `pub fn new_empty() -> Self`, `pub fn id_of(index: usize) -> PatternId`,
    `pub fn index_of(id: PatternId) -> usize`, `pub fn current(&self) -> usize`,
    `pub fn select(&mut self, index: usize)`, `pub fn get(&self, index: usize) -> &Pattern`,
    `pub fn current_pattern(&self) -> &Pattern`, `pub fn replace(&mut self, index: usize, p: Pattern)`,
    serde-capable (a flat `Vec<Pattern>` plus `current`).
  - `Engine::set_pattern` must keep playing: switching at step 5 continues at step 5, and the *next*
    boundary uses the new pattern.
- Later tasks rely on every name: Phase 06 Task 31 (bank/pattern switches), Phase 08 Task 40
  (persistence), Task 42 (factory bank).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/bank.rs
#[test]
fn sixty_four_patterns_are_addressed_as_a1_to_h8() {
    let bank = PatternBank::new_empty();
    assert_eq!(bank.id_of(0).to_string(), "A1");
    assert_eq!(bank.id_of(63).to_string(), "H8");
    assert_eq!(PatternBank::index_of(PatternId { bank: 7, number: 7 }), 63);
}

#[test]
fn selecting_keeps_the_engine_at_its_current_step() {
    let sr = 48_000.0;
    let mut e = Engine::new(sr);
    let a = Pattern::from_steps([Step::note(Pitch::A, -1); 16]);
    let b = Pattern::from_steps([Step::note(Pitch::C, 1); 16]);
    e.set_pattern(Arc::new(a));
    e.set_bpm(120.0);
    e.start();
    e.render(&mut vec![0.0; 24_000]); // 2 steps in
    let position = e.position();
    e.set_pattern(Arc::new(b));
    assert_eq!(e.position(), position, "switching must not restart the loop");
    let mut buf = vec![0.0; 12_000];
    e.render(&mut buf);
    assert!(acid_core::analysis::rms(&buf[6_000..]) > 0.01, "new pattern must keep sounding");
}

#[test]
fn the_bank_round_trips_through_json() {
    let mut bank = PatternBank::new_empty();
    bank.replace(9, Pattern::from_steps([Step::note(Pitch::D, 0); 16]));
    bank.select(9);
    let json = serde_json::to_string(&bank).unwrap();
    let back: PatternBank = serde_json::from_str(&json).unwrap();
    assert_eq!(back.current(), 9);
    assert!(back.get(9).steps[0].on);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test bank`
Expected: FAIL — unresolved import `acid_core::bank`.

- [ ] **Step 3: Implement**

`PatternBank::new_empty` makes 64 `Pattern::empty()`. `select` clamps to `0..64` and does nothing if
equal. `Engine::set_pattern` forwards to `Sequencer::set_pattern`, which swaps the `Arc` and leaves
`position`, `prev_slide` and the clock untouched.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test bank`
Expected: PASS — 3 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): 64-pattern bank with seamless live switching"
```

---

**Phase 05 done when:** `cargo test --workspace` is green, and a temporary hard-coded pattern loops
cleanly in the app with audible accents and slides, unaffected by block size or tempo changes.

## Revisions from execution (2026-11-06)

The step-duration arithmetic in Tasks 21–25 was written 2x too long: a 16th note at 120 BPM is 125 ms,
which is **6000** samples at 48 kHz (8 steps/second), not 12000. `timing::samples_per_step` implements
`sample_rate * 60 / (bpm * 4)` correctly; the Task 21 tests were corrected to 6000 / 5512.5 samples and the
Task 22 clock expectations ("one second crosses 4 steps", "step boundary at sample 24000") must be halved
the same way when that task is executed — one second at 120 BPM crosses **8** steps.

### Task 22 revision: `set_bpm` / `set_swing` rescale the step in flight

Task 22's Step 3 said "`set_bpm` only affects *future* step durations (do not rescale `remaining`)" — but two
of that task's own four tests require the opposite: "at 240 bpm, 6000 samples is two further steps" is
unreachable if the step in flight keeps its old length, and a clock built at `DEFAULT_BPM` (130) would start
its first step at 5538.46 samples instead of 6000 whenever the tempo is set to 120 before playback. As built,
`set_bpm` and `set_swing` rescale `remaining` by the ratio of the new full step duration to the old one, which
keeps the elapsed phase fraction: a change is heard on the step it was made in, and the beat never moves
backwards. Consequence for the tests: advances are kept a few hundred samples off exact step multiples
(50 000 rather than 48 000 samples at 120 BPM), because a duration computed as `sr * 60 / (bpm * 4)` lands
within a nanosecond of a boundary and an assertion made exactly on it is a coin flip.
