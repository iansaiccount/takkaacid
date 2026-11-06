# Phase 02 — Audio output and engine skeleton

> Part of [00-main-plan.md](00-main-plan.md). Tasks **5–9**. Depends on: Phase 01.
>
> **Revisions from execution (2026-11-06).** Audio is two-phase:
> `audio::open_default() -> Opened` resolves device + config, then `Opened::spawn(render_mono, errors)`
> builds the stream once the caller knows the sample rate; `audio::Rig` wraps both, drains stream errors,
> reopens after `RETRY_DELAY` and exposes a status string. `start` takes a mono render closure plus a
> `Sender<()>`, never an `Arc<Mutex<Engine>>`. cpal 0.18 here uses `DeviceTrait::description()`,
> `SampleRate = u32`, `build_output_stream(config, cb, err, timeout)` and a unified `cpal::Error`.
> **Delivers:** `Engine::render(&mut [f32])` that is silent when stopped, a real cpal output stream
> picking a sane device/config, a `Voice` with its final public API (its DSP is a placeholder sine),
> device-error recovery, and a transport flag the UI can flip.

**Why this phase:** the riskiest platform work is here — opening a stream on PipeWire at a known
sample rate, and keeping the callback free of allocation and locks. Everything after this is DSP and
drawing, tested offline. The `Voice` and `Engine` APIs introduced here are final; later phases swap
implementations behind them.

---

### Task 5: `Engine` renders silence when stopped, at any rate and block size

**Files:**
- Create: `rust/crates/acid-core/src/engine.rs`, `rust/crates/acid-core/src/analysis.rs`
- Modify: `rust/crates/acid-core/src/lib.rs` (`pub mod engine; pub mod analysis;`)
- Test: `rust/crates/acid-core/tests/engine_silence.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct Engine` with `pub fn new(sample_rate: f32) -> Self`, `pub fn sample_rate(&self) -> f32`,
    `pub fn render(&mut self, out: &mut [f32])`, `pub fn is_playing(&self) -> bool`.
  - `analysis::pub fn dominant_hz(buf: &[f32], sample_rate: f32) -> f32` — autocorrelation pitch
    estimate over `40..=2000 Hz`, valid for `buf.len() >= 1024`; returns `0.0` when the buffer is
    silent. Deliberately waveform-agnostic: a zero-crossing counter reports half the true pitch for a
    sawtooth, so it must not be used.
  - `analysis::pub fn rms(buf: &[f32]) -> f32`, `analysis::pub fn peak(buf: &[f32]) -> f32`.
- Later tasks rely on `Engine::render`/`sample_rate`; every DSP assertion in Phases 03–09 uses the
  three `analysis` helpers.

- [ ] **Step 1: Write the failing test**

```rust
// rust/crates/acid-core/tests/engine_silence.rs
use acid_core::analysis::{peak, rms};
use acid_core::engine::Engine;

#[test]
fn stopped_engine_renders_silence_at_every_rate_and_block_size() {
    for sr in [44_100.0, 48_000.0, 96_000.0] {
        let mut e = Engine::new(sr);
        for block in [1usize, 7, 64, 511, 4096] {
            let mut buf = vec![0.5f32; block];
            e.render(&mut buf);
            assert_eq!(peak(&buf), 0.0, "sr {sr}, block {block}");
            assert_eq!(rms(&buf), 0.0, "sr {sr}, block {block}");
        }
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test engine_silence`
Expected: FAIL — unresolved import `acid_core::engine`.

- [ ] **Step 3: Implement `engine.rs` and `analysis.rs`**

`Engine` holds `sample_rate: f32` and a private `voice_placeholder: f32`; `render` fills `out` with
`0.0`. `analysis::dominant_hz` normalised-autocorrelation over lags `sr/2000..=sr/40`, picking the
highest peak after the first zero crossing and refining it parabolically (accuracy ±1 %);
`peak` = max absolute value; `rms` = `sqrt(mean of squares)`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test engine_silence`
Expected: PASS — 1 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): engine skeleton rendering silence plus analysis helpers"
```

---

### Task 6: cpal output stream with device and config selection

**Files:**
- Create: `rust/crates/acid-app/src/audio.rs`
- Modify: `rust/crates/acid-app/Cargo.toml` (`cargo add -p acid-app cpal`), `src/main.rs`
- Test: `rust/crates/acid-app/src/audio.rs` (`#[cfg(test)] mod tests`, no device needed)

**Interfaces:**
- Consumes: `acid_core::engine::Engine`.
- Produces:
  - `pub fn pick_config(configs: &[cpal::SupportedStreamConfig]) -> Option<cpal::SupportedStreamConfig>`
  - `pub fn pick_rate(rates: &[u32]) -> Option<u32>` — the preference order `pick_config` uses
    (`48_000`, `44_100`, `96_000`, then whatever is offered); tested directly because it needs no device.
  - `pub fn describe(cfg: &cpal::SupportedStreamConfig) -> String`
  - `pub struct AudioHandle { stream: cpal::Stream }` — `pub fn sample_rate(&self) -> f32`, kept alive
    by the app.
  - `pub fn start(engine: std::sync::Arc<std::sync::Mutex<…>>) -> anyhow::Result<AudioHandle>` —
    superseded in Task 9 by the lock-free version; Task 6 may use the mutex-free shape directly
    (`Arc<EngineHandle>` per Task 9). If Task 9's `SharedState` is written first, use it here.
- Later tasks rely on `pick_config`'s preference order.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn prefers_48k_then_44k1_then_the_rest() {
    let rates = [96_000u32, 44_100, 48_000];
    let picked = pick_rate(&rates);
    assert_eq!(picked, Some(48_000));
    assert_eq!(pick_rate(&[96_000, 44_100]), Some(44_100));
    assert_eq!(pick_rate(&[192_000, 96_000]), Some(96_000));
    assert_eq!(pick_rate(&[]), None);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app pick_rate`
Expected: FAIL — cannot find function `pick_rate`.

- [ ] **Step 3: Implement selection and the stream**

`pick_rate(&[u32]) -> Option<u32>`: return the first of `[48_000, 44_100, 96_000]` present, else the
first entry. `pick_config`: filter configs with `channels() >= 1`, prefer `pick_rate` over the
available rates, keep cpal's default buffer size. `start`: `cpal::default_host().default_output_device()`,
log device name / rate / channels, build `StreamConfig { channels: cfg.channels(), sample_rate:
cfg.sample_rate(), buffer_size: BufferSize::Default }`, `build_output_stream` with a callback that
calls `engine.render(mono)` into a preallocated `Vec<f32>` sized at stream open (no allocation inside),
then copies that sample to every channel. Wire `main.rs` to start it and keep the handle.

- [ ] **Step 4: Run tests, then confirm a real stream opens**

Run: `cd rust && cargo test -p acid-app pick_rate`
Expected: PASS — 1 passed.
Run: `cd rust && cargo run -p acid-app`
Expected: header lines print the device name, rate and channels; window still opens; no stream error.
**Listen:** silence at every channel.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(audio): cpal output stream with 48k/44k1 config preference"
```

---

### Task 7: `Voice` with its final API (sine placeholder DSP)

**Files:**
- Create: `rust/crates/acid-core/src/voice.rs`
- Modify: `rust/crates/acid-core/src/lib.rs`, `src/engine.rs` (own a `Voice`)
- Test: `rust/crates/acid-core/tests/voice_placeholder.rs`

**Interfaces:**
- Consumes: `analysis::{dominant_hz, rms}`.
- Produces:
  - `pub struct VoiceParams { pub gate: f32, pub accent: f32 }` — reserved for later phases.
  - `pub struct Voice` with `pub fn new(sample_rate: f32) -> Self`,
    `pub fn note_on(&mut self, hz: f32, accent: bool)`,
    `pub fn set_pitch(&mut self, hz: f32)`,
    `pub fn note_off(&mut self)`,
    `pub fn is_active(&self) -> bool`,
    `pub fn render(&mut self, out: &mut [f32])`.
  - `Engine`: `pub fn note_on(&mut self, hz: f32, accent: bool)`, `pub fn note_off(&mut self)`.
- Later tasks keep this API exactly: Phase 03 Task 12 replaces `set_pitch`'s body with glide,
  Phase 04 Task 19 replaces `render`'s body with the full chain, Phase 05 Task 25 calls `note_on` from
  the sequencer.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_held_note_sounds_at_the_requested_pitch() {
    let mut v = Voice::new(48_000.0);
    v.note_on(440.0, false);
    let mut buf = vec![0.0; 9_600];
    v.render(&mut buf);
    let hz = dominant_hz(&buf[4_800..], 48_000.0);
    assert!((hz - 440.0).abs() < 8.0, "got {hz}");
    assert!(rms(&buf) > 0.05);
}

#[test]
fn note_off_reaches_silence() {
    let mut v = Voice::new(48_000.0);
    v.note_on(220.0, false);
    v.render(&mut vec![0.0; 2_400]);
    v.note_off();
    let mut tail = vec![0.0; 4_800];
    v.render(&mut tail);
    assert!(rms(&tail[tail.len() / 2..]) < 0.001);
    assert!(!v.is_active());
}

#[test]
fn output_never_clips_or_contains_nan() {
    let mut v = Voice::new(44_100.0);
    v.note_on(55.0, true);
    let mut buf = vec![0.0; 44_100];
    v.render(&mut buf);
    assert!(buf.iter().all(|s| s.is_finite() && s.abs() <= 0.999), "bad sample");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-core --test voice_placeholder`
Expected: FAIL — no `Voice` type.

- [ ] **Step 3: Implement the placeholder voice**

`phase: f64` advanced by `hz / sr` per sample, wrapped to `0.0..1.0`; sine via `libm`-free
`(phase * TAU).sin()`. Amplitude `0.3 * (1.0 + 0.4 * accent as u8 as f32)`. `note_off` starts a 20 ms
one-pole fade (`acid_core::param::Smooth`); `is_active` is false once the fade is below `1e-4` and the
gate is closed. `Engine::render` delegates to `voice.render`. This is throwaway DSP with a permanent
API.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-core --test voice_placeholder`
Expected: PASS — 3 passed.
**Listen:** `cargo run -p acid-app` is still silent — nothing calls `note_on` yet.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core
git commit -m "feat(core): voice api with placeholder sine oscillator"
```

---

### Task 8: Stream failure recovery without hanging the UI

**Files:**
- Create: `rust/crates/acid-app/src/audio/supervisor.rs` (make `audio.rs` a folder module:
  `src/audio/mod.rs` + `src/audio/supervisor.rs`)
- Modify: `src/main.rs`
- Test: `rust/crates/acid-app/src/audio/supervisor.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub const MAX_RETRIES: u32 = 5;` `pub const RETRY_DELAY: std::time::Duration = 250 ms`;
  - `pub enum Action { Open, Retry, GiveUp }`
  - `pub struct Supervisor` with `pub fn new() -> Self`, `pub fn on_error(&mut self) -> Action`,
    `pub fn on_success(&mut self)`, `pub fn attempts(&self) -> u32`.
  - `pub enum AudioStatus { Starting, Running { device: String, sample_rate: u32 }, Failed { reason: String } }`
    stored in `SharedState` (Task 9) and read by the UI in Phase 11 Task 55.
- Later tasks rely on `AudioStatus`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn retries_five_times_then_gives_up() {
    let mut s = Supervisor::new();
    for _ in 0..MAX_RETRIES {
        assert_eq!(s.on_error(), Action::Retry);
    }
    assert_eq!(s.on_error(), Action::GiveUp);
}

#[test]
fn a_success_resets_the_counter() {
    let mut s = Supervisor::new();
    assert_eq!(s.on_error(), Action::Retry);
    s.on_success();
    assert_eq!(s.attempts(), 0);
    assert_eq!(s.on_error(), Action::Retry);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app supervisor`
Expected: FAIL — module `audio::supervisor` not found.

- [ ] **Step 3: Implement the supervisor and use it**

The cpal error callback sends on an `std::sync::mpsc::Sender<()>`; the app's `update` drains it, calls
`on_error`, and on `Retry` re-runs `audio::start` after `RETRY_DELAY` (checked against `Instant`, never
`sleep` on the UI thread), publishing `AudioStatus::Failed` on `GiveUp`. `on_success` fires when the
stream reports `Playing`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app supervisor`
Expected: PASS — 2 passed.
Also run `cd rust && cargo run -p acid-app` with PipeWire stopped (`systemctl --user stop pipewire`) —
expected: window still opens, status shows a failure, no hang, then `systemctl --user start pipewire`
and the app recovers without restart.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(audio): supervised stream open with bounded retries"
```

---

### Task 9: Lock-free transport between UI and audio thread

**Files:**
- Create: `rust/crates/acid-app/src/state.rs`
- Modify: `src/main.rs`, `src/audio/mod.rs`, `src/ui/mod.rs` (space bar later; here a temporary
  on-screen RUN/STOP pair is allowed)
- Test: `rust/crates/acid-app/src/state.rs` (`#[cfg(test)] mod tests`), and
  `rust/crates/acid-app/tests/transport.rs`

**Interfaces:**
- Consumes: `Engine`, `AudioStatus`.
- Produces:
  - `pub struct AtomicF32(std::sync::atomic::AtomicU32)` with `pub fn new(f32) -> Self`,
    `pub fn get(&self) -> f32`, `pub fn set(&self, f32)` (bit-cast, `Ordering::Relaxed`).
  - `pub struct SharedState` with `pub fn new() -> std::sync::Arc<Self>`, fields
    `pub playing: std::sync::atomic::AtomicBool`, `pub tempo_bpm: AtomicF32`,
    `pub volume: AtomicF32`, `pub status: std::sync::Mutex<AudioStatus>` (UI thread only, short holds).
  - `Engine`: `pub fn start(&mut self)`, `pub fn stop(&mut self)`; `stop` closes the gate and is silent
    within 20 ms.
- Later tasks rely on `AtomicF32` and `SharedState`'s shape: Phase 07 Task 33 adds the seven knob
  atomics and the `ArcSwap<Pattern>`.

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-app/tests/transport.rs
#[test]
fn stopping_silences_the_engine_within_twenty_ms() {
    let mut e = Engine::new(48_000.0);
    e.start();
    let mut warm = vec![0.0; 9_600];
    e.render(&mut warm);
    assert!(acid_core::analysis::rms(&warm) > 0.02);

    e.stop();
    let mut tail = vec![0.0; 1_920]; // 40 ms
    e.render(&mut tail);
    let last_20ms = &tail[960..];
    assert!(acid_core::analysis::rms(last_20ms) < 0.001);
    assert!(!e.is_playing());
}

#[test]
fn shared_state_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<acid_app::state::SharedState>();
}

#[test]
fn atomic_f32_round_trips_negative_and_fractional_values() {
    let a = acid_app::state::AtomicF32::new(0.0);
    for v in [-1.0, -0.25, 0.0, 0.333, 1.0] {
        a.set(v);
        assert!((a.get() - v).abs() < 1e-6);
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test transport`
Expected: FAIL — no `state` module / no `Engine::start`.

- [ ] **Step 3: Implement `state.rs` and `Engine::start`/`stop`**

`Engine::start` sets `playing = true` and calls `voice.note_on(step_hz(Pitch::C, 0), false)` as a
placeholder until Phase 05's sequencer. `stop` calls `voice.note_off()` and clears `playing`. The audio
callback reads `playing` once per block (cheap, `Relaxed`) and calls `start`/`stop` when it changes.
Add `lib.rs` to `acid-app` (`src/lib.rs` with `pub mod state; pub mod audio; pub mod ui;`) so tests can
import it, and keep the binary thin.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app --test transport`
Expected: PASS — 3 passed.
**Listen:** with `cargo run -p acid-app`, the temporary RUN button plays a 55 Hz sine; STOP silences it
with no click or stuck note. This is the first audible output of the project.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(app): lock-free transport flags and engine start/stop"
```

---

**Phase 02 done when:** `cargo test --workspace` is green, `cargo run -p acid-app` opens the panel and
a temporary RUN button produces a tone that stops cleanly, and stopping PipeWire mid-run neither hangs
nor panics.
