# Phase 09 — Master section: drive, limiter, level

> Part of [00-main-plan.md](00-main-plan.md). Tasks **43–46**. Depends on: Phase 08.
> **Delivers:** the last stage before the sound card — a drive that makes it dirty, a limiter that makes
> it safe, the VOLUME knob, and a gain structure that puts the factory pattern in a sensible range.

**Why after the panel works:** these are the controls that matter most once you can hear a loop and want
it to sound like a record. Doing them now keeps Phase 10–11 (MIDI, polish) from having to re-tune levels.

---

### Task 43: Drive with soft clipping

**Files:**
- Create: `rust/crates/acid-core/src/drive.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod drive;`), `src/engine.rs`
  (add `set_drive(f32)` / `drive()` and pass the master input through it)
- Test: `rust/crates/acid-core/tests/drive.rs`

**Interfaces:**
- Consumes: `param::Smooth`.
- Produces:
  - `pub const MAX_DRIVE_GAIN: f32 = 8.0;`
  - `pub fn soft_clip(x: f32) -> f32` — odd-symmetric, monotonic, `|soft_clip(x)| < 1.0` for all finite
    `x`, `soft_clip(0.0) == 0.0`, `soft_clip(x) ~ x` for `|x| < 0.1`.
  - `pub struct Drive { gain: Smooth }` with `pub fn new(sample_rate: f32) -> Self`,
    `pub fn set_amount(&mut self, knob: f32)`
    (gain `1.0 + (MAX_DRIVE_GAIN - 1.0) · knob²`, smoothed over 10 ms), `pub fn process(&mut self, x:
    f32) -> f32` — `soft_clip(x · gain) / soft_clip(gain)` so full-scale input stays near full-scale.
  - `Engine::set_drive(f32)`, `Engine::drive() -> f32` — knob domain; **default `0.0`** (clean).
- Later tasks rely on: `Engine::set_drive` (Task 46 gain staging, Phase 11 Task 56 tooltips).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/drive.rs
use acid_core::drive::{soft_clip, Drive, MAX_DRIVE_GAIN};

#[test]
fn soft_clip_is_monotonic_and_bounded() {
    let mut last = soft_clip(-50.0);
    for i in -1_000..=1_000 {
        let y = soft_clip(i as f32 * 0.05);
        assert!(y >= last - 1e-6, "not monotonic at {i}");
        assert!(y.abs() < 1.0, "escaped the bound at {i}: {y}");
        last = y;
    }
    assert_eq!(soft_clip(0.0), 0.0);
    assert!((soft_clip(0.01) - 0.01).abs() < 1e-3);
}

#[test]
fn amount_zero_is_transparent() {
    let mut d = Drive::new(48_000.0);
    d.set_amount(0.0);
    for _ in 0..480 {
        d.process(0.0);
    }
    for x in [-0.9, -0.5, 0.0, 0.25, 0.9] {
        assert!((d.process(x) - x).abs() < 0.02, "input {x}");
    }
}

#[test]
fn more_drive_means_more_harmonics_on_the_same_sine() {
    let brightness_after = |amount: f32| {
        let sr = 48_000.0;
        let mut d = Drive::new(sr);
        d.set_amount(amount);
        for _ in 0..4_800 {
            d.process(0.0);
        }
        let buf: Vec<f32> = (0..48_000)
            .map(|i| d.process((core::f32::consts::TAU * 110.0 * i as f32 / sr).sin() * 0.8))
            .collect();
        acid_core::analysis::brightness(&buf[24_000..])
    };
    let clean = brightness_after(0.0);
    let dirty = brightness_after(1.0);
    assert!(dirty > clean * 1.5, "clean {clean} dirty {dirty}");
}

#[test]
fn maximum_gain_never_exceeds_unity_output() {
    let mut d = Drive::new(44_100.0);
    d.set_amount(1.0);
    for _ in 0..4_410 {
        d.process(0.0);
    }
    let peak = (0..44_100)
        .map(|_| d.process(0.99).abs())
        .fold(0.0f32, f32::max);
    assert!(peak <= 0.999, "peak {peak}");
    assert!((MAX_DRIVE_GAIN - 8.0).abs() < 1e-6);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test drive`
Expected: FAIL — unresolved import `acid_core::drive`.

- [ ] **Step 3: Implement**

`soft_clip(x) = x / (1.0 + x.abs())` (bounded by 1, monotonic, cheap) — then the normaliser
`1.0 / soft_clip(MAX_DRIVE_GAIN)` keeps unity near full scale. `Engine::render` order becomes
voice → drive → limiter (Task 44) → volume.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test drive`
Expected: PASS — 4 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): soft-clipping drive stage"
```

---

### Task 44: Limiter that cannot be exceeded

**Files:**
- Create: `rust/crates/acid-core/src/limiter.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod limiter;`), `src/engine.rs`
  (insert the limiter between drive and volume)
- Test: `rust/crates/acid-core/tests/limiter.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub const CEILING: f32 = 0.98;` `pub const RELEASE_MS: f32 = 200.0;`
  - `pub struct Limiter` with `pub fn new(sample_rate: f32) -> Self`, `pub fn process(&mut self, x: f32)
    -> f32`, `pub fn gain(&self) -> f32` — instant attack, `RELEASE_MS` release, output
    `|y| <= CEILING` for every finite `x`.
  - `Engine::render` applies the limiter **after** the drive and **before** the volume knob, and states
    that in a comment.
- Later tasks rely on: the guarantee, not the API (Phase 10 Task 48 feeds MIDI notes through it).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/limiter.rs
use acid_core::limiter::{Limiter, CEILING, RELEASE_MS};

#[test]
fn a_pathological_input_never_escapes_the_ceiling() {
    let sr = 48_000.0;
    let mut l = Limiter::new(sr);
    for i in 0..(sr as usize * 5) {
        let x = if i % 4_800 < 100 { 12.0 } else { -0.9 };
        let y = l.process(x);
        assert!(y.is_finite(), "NaN at {i}");
        assert!(y.abs() <= CEILING + 1e-6, "leaked {y} at {i}");
    }
}

#[test]
fn gain_recovers_after_a_burst() {
    let sr = 48_000.0;
    let mut l = Limiter::new(sr);
    for _ in 0..4_800 {
        l.process(4.0);
    }
    assert!(l.gain() < 0.5);
    // 5 release times of quiet input
    for _ in 0..(RELEASE_MS * 0.001 * sr * 5.0) as usize {
        l.process(0.1);
    }
    assert!(l.gain() > 0.95, "gain stuck at {}", l.gain());
}

#[test]
fn quiet_material_passes_with_unity_gain() {
    let mut l = Limiter::new(48_000.0);
    for _ in 0..48_000 {
        assert!((l.process(0.3) - 0.3).abs() < 1e-6);
    }
    assert!((l.gain() - 1.0).abs() < 1e-6);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test limiter`
Expected: FAIL — unresolved import `acid_core::limiter`.

- [ ] **Step 3: Implement**

Track `env` (peak follower) and `gain = if env > CEILING { CEILING / env } else { 1.0 }` with a release
one-pole on `gain` upward only (attack is instantaneous, so the ceiling holds on the same sample).
Update `env = env.max(|x|)` then decay `env *= release_coeff` each sample. No lookahead — the guarantee
comes from attack being instantaneous, not from delay.

- [ ] **Step 4: Run to confirm passing, then listen**

Run: `cd rust && cargo test -p acid-core --test limiter`
Expected: PASS — 3 passed.
**Listen:** crank RESONANCE to maximum with DECAY long — the sound gets loud and resonant but never
crackles or clips, and it recovers after the loud part.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): instant-attack limiter with 200ms release"
```

---

### Task 45: VOLUME knob on the panel, applied last

**Files:**
- Modify: `rust/crates/acid-core/src/engine.rs` (apply `param::master_volume`), `src/ui/mod.rs`
  (a VOLUME knob in the transport row)
- Test: `rust/crates/acid-core/tests/volume.rs`

**Interfaces:**
- Consumes: `param::master_volume`, `Engine::set_volume` (stored since Task 19).
- Produces: `Engine::set_volume(knob)` applies the taper **after** the limiter, so a volume change cannot
  push the output over the ceiling. `Engine::volume(&self) -> f32` returns the knob.
- Later tasks rely on: nothing.

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-core/tests/volume.rs
#[test]
fn volume_zero_is_exact_silence() {
    let mut e = Engine::new(48_000.0);
    e.set_pattern(std::sync::Arc::new(acid_core::pattern::Pattern::from_steps(
        [acid_core::pattern::Step::note(acid_core::pitch::Pitch::A, -1); 16],
    )));
    e.set_volume(0.0);
    e.start();
    let mut buf = vec![0.0; 24_000];
    e.render(&mut buf);
    assert!(buf.iter().all(|s| *s == 0.0));
}

#[test]
fn volume_quarter_power_is_about_twelve_db_down() {
    let rms_at = |knob: f32| {
        let mut e = Engine::new(48_000.0);
        e.set_pattern(std::sync::Arc::new(acid_core::pattern::Pattern::from_steps(
            [acid_core::pattern::Step::note(acid_core::pitch::Pitch::A, -1); 16],
        )));
        e.set_volume(knob);
        e.start();
        let mut buf = vec![0.0; 48_000];
        e.render(&mut buf);
        acid_core::analysis::rms(&buf[24_000..])
    };
    let full = rms_at(1.0);
    let half = rms_at(0.5);
    let db = 20.0 * (half / full).log10();
    assert!((db + 12.04).abs() < 0.5, "half knob was {db} dB");
}

#[test]
fn volume_cannot_push_past_the_ceiling() {
    let mut e = Engine::new(48_000.0);
    e.set_resonance(1.0);
    e.set_drive(1.0);
    e.set_volume(1.0);
    e.note_on(27.5, true);
    let mut buf = vec![0.0; 96_000];
    e.render(&mut buf);
    assert!(acid_core::analysis::peak(&buf) <= acid_core::limiter::CEILING + 1e-6);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test volume`
Expected: FAIL — volume is stored but never applied.

- [ ] **Step 3: Implement**

`Engine::render` ends with `out[i] = limiter.process(drive.process(voice)) * master_volume(volume_knob)`.
Add the VOLUME knob to the transport row using `ui::knob::knob` at 2/3 the diameter of the panel knobs,
indicator-only (no numeric readout on the panel; the tooltip shows the dB).

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test volume`
Expected: PASS — 3 passed.
**Listen:** sweep VOLUME from zero to full while the loop runs — smooth, no steps, silent at zero.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core rust/crates/acid-app
git commit -m "feat(core): master volume applied after the limiter"
```

---

### Task 46: Gain staging to a target loudness

**Files:**
- Modify: `rust/crates/acid-core/src/engine.rs` (internal voice gain constant), `src/factory.rs`
  (pattern velocities if needed)
- Test: `rust/crates/acid-core/tests/gain_staging.rs`

**Interfaces:**
- Consumes: everything above.
- Produces: `pub const TARGET_RMS_DB: f32 = -14.0;` and the internal gain that makes the factory pattern
  land there at default knobs (`volume = 1.0`, `drive = 0.0`). The implementer chooses the constant; the
  test fixes the window the result must fall in.
- Later tasks rely on: nothing.

- [ ] **Step 1: Write the failing test**

```rust
// rust/crates/acid-core/tests/gain_staging.rs
use acid_core::analysis::rms;

#[test]
fn the_factory_pattern_lands_between_minus_eighteen_and_ten_db() {
    let mut e = acid_core::engine::Engine::new(48_000.0);
    e.set_pattern(std::sync::Arc::new(acid_core::factory::factory_patterns()[0]));
    e.set_volume(1.0);
    e.start();
    let mut buf = vec![0.0; 48_000 * 4];
    e.render(&mut buf);
    let level = rms(&buf[48_000..]);
    let db = 20.0 * level.log10();
    assert!((-18.0..=-10.0).contains(&db), "factory pattern sits at {db} dBFS");
    assert!(acid_core::analysis::peak(&buf[48_000..]) < 0.9, "peaks too hot");
}

#[test]
fn drive_and_resonance_at_maximum_stay_below_the_ceiling() {
    let mut e = acid_core::engine::Engine::new(48_000.0);
    e.set_pattern(std::sync::Arc::new(acid_core::factory::factory_patterns()[0]));
    e.set_drive(1.0);
    e.set_resonance(1.0);
    e.set_env_mod(1.0);
    e.set_volume(1.0);
    e.start();
    let mut buf = vec![0.0; 48_000 * 4];
    e.render(&mut buf);
    assert!(acid_core::analysis::peak(&buf) <= acid_core::limiter::CEILING + 1e-6);
    assert!(buf.iter().all(|s| s.is_finite()));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test gain_staging`
Expected: FAIL — the level is either under −18 dB (thin) or above −10 dB (hot).

- [ ] **Step 3: Implement**

Adjust the voice's internal amplitude constant (one number in `Voice::render`), re-run, and stop when the
test passes. Record the chosen value and the measured level in a comment next to it.

- [ ] **Step 4: Run to confirm passing, then listen**

Run: `cd rust && cargo test -p acid-core --test gain_staging`
Expected: PASS — 2 passed.
**Listen:** compare against a reference acid record at a comfortable volume — the loop should not need
the system volume moved. If it does, the review point is `TARGET_RMS_DB`, not the pattern.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): gain staging to a -14 dBFS target"
```

---

**Phase 09 done when:** a maximally resonant, fully driven loop stays inside the ceiling, the VOLUME knob
works, and the factory pattern sits in the −18…−10 dBFS window.
