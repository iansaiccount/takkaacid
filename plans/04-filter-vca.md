# Phase 04 — Filter, envelope, VCA and accent

> Part of [00-main-plan.md](00-main-plan.md). Tasks **14–19**. Depends on: Phase 03.
> **Delivers:** the part that makes it sound like a 303 — a 4-pole resonant lowpass, an envelope with a
> DECAY knob, a VCA, filter envelope modulation, accent, and a hard safety net.

**Why these tests look the way they do:** none of them assert "sounds good". They pin the properties a
303-ish voice must have — DC passband, ~24 dB/octave rolloff, a resonant peak, an envelope that peaks
and falls in a measurable time, accent louder than non-accent, and no sample outside `-0.999..=0.999`.
The ear check comes last, in Task 19, as a **Listen:** line.

---

### Task 14: Four-pole zero-delay-feedback ladder lowpass

**Files:**
- Create: `rust/crates/acid-core/src/filter.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod filter;`), `src/analysis.rs`
  (add `pub fn brightness(buf: &[f32]) -> f32`)
- Test: `rust/crates/acid-core/tests/filter.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct LadderFilter` with `pub fn new(sample_rate: f32) -> Self`,
    `pub fn set_cutoff_hz(&mut self, hz: f32)` (clamped `20.0 .. 0.45 * sample_rate`),
    `pub fn set_resonance(&mut self, r: f32)` (`0.0..=1.0`), `pub fn process(&mut self, x: f32) -> f32`,
    `pub fn reset(&mut self)`, `pub fn cutoff_hz(&self) -> f32`.
  - `pub const MIN_CUTOFF_HZ: f32 = 20.0;`
  - `analysis::pub fn brightness(buf: &[f32]) -> f32` — `rms(first difference) / rms(buf)`, `0.0` for
    silence; a cheap high-pass proxy used for every brightness assertion in Phase 04.
- Later tasks rely on this exact API: Phase 04 Task 17 (env modulation), Task 19 (voice chain),
  Phase 07 Task 34 (CUT OFF and RESONANCE knobs).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/filter.rs
use acid_core::filter::LadderFilter;

fn run(f: &mut LadderFilter, input: impl Fn(usize) -> f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| f.process(input(i))).collect()
}

fn sine_hz_rms(fc: f32, freq: f32, sr: f32, res: f32) -> f32 {
    let mut f = LadderFilter::new(sr);
    f.set_cutoff_hz(fc);
    f.set_resonance(res);
    let n = 48_000;
    let out = run(&mut f, |i| (core::f32::consts::TAU * freq * i as f32 / sr).sin() * 0.5, n);
    acid_core::analysis::rms(&out[n / 2..])
}

#[test]
fn passes_low_frequencies_and_attenuates_above_cutoff() {
    let sr = 48_000.0;
    let pass = sine_hz_rms(1_000.0, 50.0, sr, 0.0);
    let stop = sine_hz_rms(1_000.0, 8_000.0, sr, 0.0);
    assert!(pass > 0.45, "50 Hz should pass, got {pass}");
    assert!(stop < 0.5 * pass / 8.0, "8 kHz should be far down, got {stop}");
}

#[test]
fn rolloff_is_about_24_db_per_octave() {
    let sr = 48_000.0;
    let a = sine_hz_rms(400.0, 3_200.0, sr, 0.0); // 3 octaves above cutoff
    let b = sine_hz_rms(400.0, 6_400.0, sr, 0.0); // 4 octaves above
    let db_per_oct = 20.0 * (a / b.max(1e-9)).log10();
    assert!((db_per_oct - 24.0).abs() < 8.0, "got {db_per_oct} dB/oct");
}

#[test]
fn resonance_lifts_the_cutoff_region() {
    let sr = 48_000.0;
    let flat = sine_hz_rms(800.0, 800.0, sr, 0.0);
    let peaky = sine_hz_rms(800.0, 800.0, sr, 0.9);
    assert!(peaky > flat * 1.5, "flat {flat} peaky {peaky}");
}

#[test]
fn stays_finite_at_maximum_resonance_and_top_cutoff() {
    let sr = 44_100.0;
    let mut f = LadderFilter::new(sr);
    f.set_cutoff_hz(20_000.0);
    f.set_resonance(1.0);
    let out = run(&mut f, |i| if i % 96 < 48 { 0.9 } else { -0.9 }, 96_000);
    assert!(out.iter().all(|s| s.is_finite() && s.abs() < 50.0), "filter blew up");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test filter`
Expected: FAIL — unresolved import `acid_core::filter`.

- [ ] **Step 3: Implement the filter**

TPT/Zavalishin ladder: per sample `g = tan(π·fc/sr)`, `G = g/(1+g)`, feedback `k = 3.95 · resonance`,
`v = x - k · s[3]`, then four times `vn = (v - s[i]) · G; s[i] += 2·vn; v = s[i] + vn;` and return
`s[3]`. `tan` is computed once per `set_cutoff_hz` call, never per sample. Clamp `k` to `3.95` so the
filter cannot self-oscillate into instability. `brightness` is
`(sum of (x[i]-x[i-1])² / sum of x[i]²).sqrt()`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test filter`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): zero-delay-feedback 4-pole ladder lowpass"
```

---

### Task 15: Envelope with attack, decay, sustain and release

**Files:**
- Create: `rust/crates/acid-core/src/envelope.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod envelope;`)
- Test: `rust/crates/acid-core/tests/envelope.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub enum Stage { Idle, Attack, Decay, Sustain, Release }` (`Copy, PartialEq, Debug`)
  - `pub struct Envelope` with `pub fn new(sample_rate: f32) -> Self`,
    `pub fn set_attack_ms(&mut self, ms: f32)`, `pub fn set_decay_seconds(&mut self, s: f32)`,
    `pub fn set_sustain(&mut self, level: f32)`, `pub fn set_release_ms(&mut self, ms: f32)`,
    `pub fn gate_on(&mut self)`, `pub fn gate_off(&mut self)`, `pub fn next(&mut self) -> f32`,
    `pub fn stage(&self) -> Stage`, `pub fn is_idle(&self) -> bool`.
  - Defaults from `new`: attack 2 ms, decay 0.3 s, sustain 0.35, release 20 ms.
- Later tasks rely on this API: Task 16 (VCA), Task 17 (filter envelope), Task 25 (step retrigger),
  Phase 05 Task 24 (slide must not retrigger).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/envelope.rs
use acid_core::envelope::{Envelope, Stage};

#[test]
fn attack_peaks_in_about_two_ms() {
    let sr = 48_000.0;
    let mut e = Envelope::new(sr);
    e.set_attack_ms(2.0);
    e.gate_on();
    let n = (0.002 * sr) as usize;
    let mut peak = 0.0f32;
    for _ in 0..n {
        peak = peak.max(e.next());
    }
    assert!(peak > 0.95, "peak only {peak}");
}

#[test]
fn decay_reaches_sustain_within_the_requested_time() {
    let sr = 48_000.0;
    let mut e = Envelope::new(sr);
    e.set_attack_ms(1.0);
    e.set_decay_seconds(0.1);
    e.set_sustain(0.35);
    e.gate_on();
    for _ in 0..(0.002 * sr) as usize {
        e.next();
    }
    for _ in 0..(0.1 * sr) as usize {
        e.next();
    }
    let level = e.next();
    assert!((level - 0.35).abs() < 0.05, "level {level}");
    assert_eq!(e.stage(), Stage::Sustain);
}

#[test]
fn release_reaches_silence_and_goes_idle() {
    let sr = 44_100.0;
    let mut e = Envelope::new(sr);
    e.set_release_ms(20.0);
    e.gate_on();
    for _ in 0..(0.05 * sr) as usize {
        e.next();
    }
    e.gate_off();
    for _ in 0..(0.02 * sr) as usize + 16 {
        e.next();
    }
    assert!(e.next() < 0.001);
    assert!(e.is_idle());
}

#[test]
fn retrigger_restarts_from_the_current_level() {
    let sr = 48_000.0;
    let mut e = Envelope::new(sr);
    e.gate_on();
    for _ in 0..(0.05 * sr) as usize {
        e.next();
    }
    e.gate_off();
    for _ in 0..(0.005 * sr) as usize {
        e.next();
    }
    let before = e.next();
    e.gate_on();
    let after = e.next();
    assert!(after >= before - 0.01, "retrigger dropped the level: {before} -> {after}");
    assert_eq!(e.stage(), Stage::Attack);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test envelope`
Expected: FAIL — unresolved import `acid_core::envelope`.

- [ ] **Step 3: Implement the envelope**

Linear segments with per-sample increments computed when a stage starts
(`attack_inc = 1.0 / (attack_ms·0.001·sr)`, `decay_inc = (1.0 - sustain) / (decay_s·sr)`,
`release_inc = level / (release_ms·0.001·sr)`). Release must be measured from the level at gate-off.
`is_idle` is `stage == Idle`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test envelope`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): segmented adsr envelope"
```

---

### Task 16: VCA and accent gain inside `Voice`

**Files:**
- Modify: `rust/crates/acid-core/src/voice.rs` (replace the placeholder gate fade with the envelope)
- Test: `rust/crates/acid-core/tests/voice_vca.rs`

**Interfaces:**
- Consumes: `Envelope`, `param::decay_seconds`, `param::accent_amount`.
- Produces:
  - `Voice`: `pub fn set_decay_knob(&mut self, knob: f32)`, `pub fn set_accent_knob(&mut self, knob: f32)`
  - `Voice`: `pub const ACCENT_GAIN: f32 = 1.4;` — accent multiplies the envelope output.
  - `Voice::note_on(hz, accent)` now runs the envelope: attack 2 ms, decay from the knob (default
    knob 0.35 ≈ 0.15 s), sustain 0.35, release 20 ms.
- Later tasks rely on: `set_decay_knob`, `set_accent_knob` (Phase 07 Task 34).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn an_accented_note_is_about_forty_percent_louder() {
    let sr = 48_000.0;
    let plain = {
        let mut v = Voice::new(sr);
        v.note_on(110.0, false);
        let mut b = vec![0.0; 4_800];
        v.render(&mut b);
        rms(&b[2_400..])
    };
    let accented = {
        let mut v = Voice::new(sr);
        v.set_accent_knob(1.0);
        v.note_on(110.0, true);
        let mut b = vec![0.0; 4_800];
        v.render(&mut b);
        rms(&b[2_400..])
    };
    let ratio = accented / plain;
    assert!(ratio > 1.25 && ratio < 1.55, "accent ratio {ratio}");
}

#[test]
fn a_shorter_decay_settles_lower_by_half_a_second() {
    let sr = 48_000.0;
    let level_at_500ms = |decay_knob: f32| {
        let mut v = Voice::new(sr);
        v.set_decay_knob(decay_knob);
        v.note_on(110.0, false);
        let mut b = vec![0.0; 28_800];
        v.render(&mut b);
        rms(&b[24_000..])
    };
    let short = level_at_500ms(0.0);
    let long = level_at_500ms(0.9);
    assert!(short < long * 0.9, "short {short} long {long}");
}

#[test]
fn the_voice_goes_quiet_after_note_off_without_being_retriggered() {
    let mut v = Voice::new(44_100.0);
    v.note_on(55.0, false);
    v.render(&mut vec![0.0; 4_410]);
    v.note_off();
    let mut b = vec![0.0; 8_820];
    v.render(&mut b);
    assert!(rms(&b[4_410..]) < 0.001);
    assert!(!v.is_active());
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test voice_vca`
Expected: FAIL — no `set_decay_knob`.

- [ ] **Step 3: Implement the VCA**

`Voice::render` becomes: per sample `osc = oscillator.next_sample()`, then keep the sample for Task 17,
then `out = osc * env.next() * accent_gain`. `accent_gain` is `1.0` for a plain note and
`1.0 + ACCENT_GAIN * accent_amount - 0.4`… write it as `1.0 + (accent_amount * (ACCENT_GAIN - 1.0))`
and store `accent_amount` from `set_accent_knob` (default `0.8`, giving the 1.32–1.4 range the test
bounds).

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test voice_vca`
Expected: PASS — 3 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): vca with decay and accent gain"
```

---

### Task 17: Filter envelope modulation and accent-driven brightness

**Files:**
- Modify: `rust/crates/acid-core/src/voice.rs`
- Modify: `rust/crates/acid-core/src/engine.rs` (knob setters; see Task 19 for the full set)
- Test: `rust/crates/acid-core/tests/voice_filter_env.rs`

**Interfaces:**
- Consumes: `LadderFilter`, `Envelope`, `analysis::brightness`, `param::cutoff_hz`, `param::env_mod`.
- Produces:
  - `Voice`: `pub fn set_cutoff_knob(&mut self, knob: f32)`,
    `pub fn set_resonance_knob(&mut self, knob: f32)`, `pub fn set_env_mod_knob(&mut self, knob: f32)`
  - `pub const ENV_MOD_OCTAVES: f32 = 4.0;` — at `env_mod = 1.0`, the filter envelope sweeps the
    cutoff by four octaves at its peak.
  - `Voice`: filter cutoff per sample =
    `param::cutoff_hz(cutoff_knob) * 2^(ENV_MOD_OCTAVES · env_mod · filter_env)`, clamped by the filter.
- Later tasks rely on: the three setters, `ENV_MOD_OCTAVES` (Phase 07 Task 34, Phase 08 Task 40).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn more_env_mod_is_brighter_on_the_attack() {
    let sr = 48_000.0;
    let brightness_for = |env_mod: f32| {
        let mut v = Voice::new(sr);
        v.set_cutoff_knob(0.25);
        v.set_env_mod_knob(env_mod);
        v.note_on(55.0, false);
        let mut b = vec![0.0; 4_800];
        v.render(&mut b);
        brightness(&b[0..2_400])
    };
    let dark = brightness_for(0.0);
    let bright = brightness_for(0.9);
    assert!(bright > dark * 1.2, "dark {dark} bright {bright}");
}

#[test]
fn a_higher_cutoff_knob_is_brighter() {
    let sr = 48_000.0;
    let brightness_for = |cutoff: f32| {
        let mut v = Voice::new(sr);
        v.set_cutoff_knob(cutoff);
        v.set_env_mod_knob(0.0);
        v.note_on(55.0, false);
        let mut b = vec![0.0; 9_600];
        v.render(&mut b);
        brightness(&b[4_800..])
    };
    assert!(brightness_for(0.9) > brightness_for(0.2) * 1.5);
}

#[test]
fn accent_opens_the_filter_further_than_a_plain_note() {
    let sr = 48_000.0;
    let brightness_for = |accent: bool| {
        let mut v = Voice::new(sr);
        v.set_cutoff_knob(0.2);
        v.set_env_mod_knob(0.5);
        v.set_accent_knob(1.0);
        v.note_on(55.0, accent);
        let mut b = vec![0.0; 4_800];
        v.render(&mut b);
        brightness(&b[0..2_400])
    };
    assert!(brightness_for(true) > brightness_for(false) * 1.1);
}

#[test]
fn resonance_knob_highlights_the_cutoff_region() {
    let sr = 48_000.0;
    let brightness_for = |res: f32| {
        let mut v = Voice::new(sr);
        v.set_cutoff_knob(0.4);
        v.set_resonance_knob(res);
        v.set_env_mod_knob(0.0);
        v.note_on(55.0, false);
        let mut b = vec![0.0; 9_600];
        v.render(&mut b);
        brightness(&b[4_800..])
    };
    assert!(brightness_for(0.95) > brightness_for(0.1) * 1.2);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test voice_filter_env`
Expected: FAIL — no `set_env_mod_knob`.

- [ ] **Step 3: Implement the modulation**

`Voice` gains a second `filter_env: Envelope` with `sustain = 0.0`, attack 1 ms, and decay
`0.6 · decay_seconds(decay_knob)` so the filter snaps open faster than the VCA. Per sample compute
`env = filter_env.next()`, `hz = base_cutoff * 2f32.powf(ENV_MOD_OCTAVES * env_mod * env *
accent_multiplier)`, `filter.set_cutoff_hz(hz.min(0.45 * sr))`, then
`out = filter.process(osc) * amp_env.next() * accent_gain`. `accent_multiplier` is `1.0` or `1.5`.
Calling `set_cutoff_hz` per sample is allowed only because it is O(1) arithmetic; if `tan` shows up in
a profile later, cache it per 64-sample block.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test voice_filter_env`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): filter envelope modulation and accent brightness"
```

---

### Task 18: Safety net — finite output, clamped, denormal-free

**Files:**
- Modify: `rust/crates/acid-core/src/voice.rs`, `src/engine.rs`
- Test: `rust/crates/acid-core/tests/safety.rs`

**Interfaces:**
- Consumes: everything above.
- Produces: `Voice::render` guarantees — every sample finite, `|s| <= 0.999`, values with magnitude
  below `1e-20` replaced by `0.0`. `Engine::render` guarantees the same and writes exact `0.0` when
  stopped.
- Later tasks rely on: the clamp being in place before Phase 09 adds drive.

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/safety.rs
use acid_core::engine::Engine;
use acid_core::oscillator::Waveform;

#[test]
fn every_extreme_combination_stays_finite_and_in_range() {
    for sr in [44_100.0, 48_000.0, 96_000.0] {
        for wave in [Waveform::Saw, Waveform::Square] {
            for (cutoff, res, env_mod, decay, accent) in [
                (0.0, 1.0, 1.0, 0.0, 1.0),
                (1.0, 1.0, 1.0, 1.0, 1.0),
                (0.5, 0.0, 0.0, 0.5, 0.0),
                (0.0, 0.0, 1.0, 1.0, 1.0),
            ] {
                let mut e = Engine::new(sr);
                e.set_waveform(wave);
                e.set_cutoff(cutoff);
                e.set_resonance(res);
                e.set_env_mod(env_mod);
                e.set_decay(decay);
                e.set_accent(accent);
                e.note_on(27.5, true);
                let mut buf = vec![0.0; 48_000];
                e.render(&mut buf);
                assert!(buf.iter().all(|s| s.is_finite()), "NaN at sr {sr} {wave:?}");
                assert!(buf.iter().all(|s| s.abs() <= 0.999), "clip at sr {sr} {wave:?}");
            }
        }
    }
}

#[test]
fn a_released_note_renders_exact_zeros() {
    let mut e = Engine::new(48_000.0);
    e.set_resonance(1.0);
    e.note_on(55.0, true);
    e.render(&mut vec![0.0; 4_800]);
    e.note_off();
    e.render(&mut vec![0.0; 4_800]);
    let mut tail = vec![1.0; 4_800];
    e.render(&mut tail);
    assert!(tail.iter().all(|s| *s == 0.0), "tail was not exact silence");
}

#[test]
fn odd_block_sizes_produce_identical_output_to_one_long_block() {
    let render_blocks = |blocks: &[usize]| {
        let mut e = Engine::new(48_000.0);
        e.note_on(110.0, false);
        let mut out = Vec::new();
        for &n in blocks {
            let mut b = vec![0.0; n];
            e.render(&mut b);
            out.extend_from_slice(&b);
        }
        out
    };
    let one = render_blocks(&[16_384]);
    let many = render_blocks(&[1, 2, 3, 511, 1_000, 4_096, 10_771]);
    assert_eq!(one.len(), many.len());
    for (i, (a, b)) in one.iter().zip(many.iter()).enumerate() {
        assert!((a - b).abs() < 1e-6, "sample {i} differs: {a} vs {b}");
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test safety`
Expected: FAIL — at least one combination clips or a tail is not exactly zero, and the block-size test
fails until the render loop is block-size independent.

- [ ] **Step 3: Implement the guards**

Clamp at the end of `Voice::render` (`s = if s.abs() < 1e-20 { 0.0 } else { s.clamp(-0.999, 0.999) }`).
Make all per-sample state advance independent of block boundaries (no per-block smoothing shortcuts).
`Engine::render` short-circuits to `out.fill(0.0)` when `!playing`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test safety`
Expected: PASS — 3 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "fix(core): clamp, denormal flush and block-size-independent rendering"
```

---

### Task 19: The whole voice, wired to `Engine`, and the ear check

**Files:**
- Modify: `rust/crates/acid-core/src/engine.rs`, `rust/crates/acid-core/src/voice.rs`
- Test: `rust/crates/acid-core/tests/voice_integration.rs`

**Interfaces:**
- Consumes: every setter from Tasks 16–17.
- Produces — the seven knobs as `Engine` methods (these names are final and used by Phase 07):
  - `Engine::set_waveform(Waveform)` (from Task 11), `Engine::set_tuning(f32)`,
    `Engine::set_cutoff(f32)`, `Engine::set_resonance(f32)`, `Engine::set_env_mod(f32)`,
    `Engine::set_decay(f32)`, `Engine::set_accent(f32)`, `Engine::set_volume(f32)` (stored, applied in
    Phase 09).
  - `Engine::set_sample_rate(f32)` is **not** provided — the engine is constructed with its rate.
- Later tasks rely on every one of those names.

- [ ] **Step 1: Write the failing test**

```rust
// rust/crates/acid-core/tests/voice_integration.rs
#[test]
fn a_default_patch_produces_a_decaying_resonant_bass_note() {
    let mut e = Engine::new(48_000.0);
    e.set_waveform(Waveform::Saw);
    e.set_tuning(0.5);
    e.set_cutoff(0.35);
    e.set_resonance(0.6);
    e.set_env_mod(0.5);
    e.set_decay(0.35);
    e.set_accent(0.8);
    e.note_on(55.0, false);

    let mut buf = vec![0.0; 48_000];
    e.render(&mut buf);

    let attack = &buf[0..2_400];
    let body = &buf[12_000..14_400];
    assert!(rms(attack) > 0.05, "no attack");
    assert!(rms(body) < rms(attack), "should decay");
    assert!(brightness(attack) > brightness(body), "filter should close as it decays");
    assert!(peak(&buf) <= 0.999);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test voice_integration`
Expected: FAIL — missing setters.

- [ ] **Step 3: Implement the setters and defaults**

Defaults so the app sounds good at launch: waveform `Saw`, tuning knob `0.5`, cutoff `0.35`,
resonance `0.55`, env mod `0.5`, decay `0.35`, accent `0.8`, volume `0.8`. Each setter forwards to
`Voice` and stores its knob so Phase 08 can persist the panel.

- [ ] **Step 4: Run tests, then listen**

Run: `cd rust && cargo test -p acid-core --test voice_integration`
Expected: PASS — 1 passed.
**Listen:** `cargo run -p acid-app` with a temporary held note — it should be a squelchy, resonant saw
bass that drops in brightness as it decays. Sweep CUT OFF by hand and confirm no clicks. If it sounds
thin, the review point is `ENV_MOD_OCTAVES` and the accent multiplier, not the test.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): full 303-style voice wired to engine knobs"
```

---

**Phase 04 done when:** `cargo test --workspace` is green, a held note sounds like a 303 bass, and
`plans/notes/phase04-listening.md` records what was heard and any tuning changes made to
`ENV_MOD_OCTAVES`, `ACCENT_GAIN`, or the envelope defaults.
