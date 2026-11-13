# Phase 07 — Wiring the panel to the engine

> Part of [00-main-plan.md](00-main-plan.md). Tasks **33–37**. Depends on: Phase 06.

> **Revisions from execution (2026-11-06).** These override the task text below where they differ:
>
> - **Audio interface.** There is no `AudioRenderer::render_into(interleaved, channels)`. The audio module
>   exposes `audio::Rig::new(|sample_rate: f32| -> audio::RenderFn)` where
>   `RenderFn = Box<dyn FnMut(&mut [f32]) + Send>` renders **mono**; `audio::Opened::spawn` owns the
>   interleaving and duplicates each mono sample to every channel. So `AudioRenderer` should expose
>   `render_mono(&mut [f32])` behind `RenderFn`, and the tests in Tasks 34–37 use mono buffers
>   (`vec![0.0; n]`, not `n * channels`).
> - **Engine lifetime.** The engine is created inside the `Rig` factory, once per successful open, at the
>   device's sample rate — never behind a mutex.
> - **eframe/egui 0.37.** `eframe::App` requires `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)`
>   (there is no `update(ctx, frame)`); optional `fn logic(&mut self, ctx: &egui::Context, frame: &mut Frame)`
>   runs every frame even when hidden — that is where `Rig::poll` belongs. Panels take `&mut Ui`.
> - **Transport.** `Engine::start/stop` already exist and the audio callback drives them from
>   `SharedState::is_playing()` once per block; Task 36 only needs the UI side and the position publish.
> **Delivers:** a live instrument. Knob moves are heard immediately, the step LEDs chase the audio
> position, RUN/STOP/TEMPO work, and the audio callback is proven allocation-free.

**The seam, in one picture:**

```
UI thread                                  audio thread (cpal callback)
─────────                                  ────────────────────────────
knob drag → SharedState.tuning.set()  ──▶  read atomics once per block
step click → pattern.rstore(Arc<new>) ──▶  pattern.load()  (wait-free)
                                            engine.set_*   → engine.render(buf)
LEDs      ← state.position.load()     ◀──  position.store(step)
                                            state.silent.store(...)  (on stop)
```

No mutex is shared with the audio thread. `SharedState.status` is a mutex, but only the UI thread
touches it.

---

### Task 33: `SharedState` grows the whole instrument surface

**Files:**
- Modify: `rust/crates/acid-app/src/state.rs`
- Test: `rust/crates/acid-app/src/state.rs` (`#[cfg(test)] mod tests`), `rust/crates/acid-app/tests/state_live.rs`

**Interfaces:**
- Consumes: `acid_core::pattern::Pattern`, `acid_core::oscillator::Waveform`, `arc-swap`.
- Produces:
  - `pub struct SharedState` fields (final names):
    `playing: AtomicBool`, `position: AtomicUsize`, `tempo_bpm: AtomicF32`, `swing: AtomicF32`,
    `waveform: AtomicUsize` (0 = saw, 1 = square), `tuning: AtomicF32`, `cutoff: AtomicF32`,
    `resonance: AtomicF32`, `env_mod: AtomicF32`, `decay: AtomicF32`, `accent: AtomicF32`,
    `volume: AtomicF32`, `pattern: arc_swap::ArcSwap<Pattern>`, `status: Mutex<AudioStatus>`.
  - `pub struct Parameters { pub waveform: Waveform, pub tuning: f32, pub cutoff: f32,
    pub resonance: f32, pub env_mod: f32, pub decay: f32, pub accent: f32, pub volume: f32,
    pub tempo_bpm: f32, pub swing: f32, pub playing: bool }` — a plain `Copy` struct.
  - `pub fn snapshot(&self) -> Parameters` — eleven relaxed atomic loads, no allocation.
  - `pub fn set_waveform(&self, w: Waveform)`, `pub fn waveform(&self) -> Waveform`.
  - `pub fn set_pattern(&self, pattern: std::sync::Arc<Pattern>)`,
    `pub fn pattern(&self) -> std::sync::Arc<Pattern>`.
  - `pub fn set_step(&self, index: usize, step: acid_core::pattern::Step)` — clone-on-write through
    `ArcSwap`; never blocks the audio thread.
- Later tasks rely on every name: Task 34 (apply), Task 35 (edits), Task 36 (LEDs and transport),
  Phase 08 Task 40 (persistence), Phase 10 Task 50 (CC map).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-app/tests/state_live.rs
use acid_app::state::SharedState;
use acid_core::oscillator::Waveform;
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

#[test]
fn every_knob_survives_a_snapshot() {
    let s = SharedState::new();
    s.tuning.set(0.25);
    s.cutoff.set(0.75);
    s.resonance.set(0.6);
    s.env_mod.set(0.4);
    s.decay.set(0.33);
    s.accent.set(0.9);
    s.volume.set(0.7);
    s.tempo_bpm.set(142.0);
    s.swing.set(0.5);
    s.set_waveform(Waveform::Square);
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);

    let p = s.snapshot();
    assert_eq!(p.waveform, Waveform::Square);
    assert!((p.tuning - 0.25).abs() < 1e-6);
    assert!((p.cutoff - 0.75).abs() < 1e-6);
    assert!((p.tempo_bpm - 142.0).abs() < 1e-6);
    assert!(p.playing);
}

#[test]
fn editing_one_step_keeps_the_other_fifteen() {
    let s = SharedState::new();
    let mut p = Pattern::empty();
    p.steps[2] = Step::note(Pitch::E, 0);
    s.set_pattern(std::sync::Arc::new(p));
    s.set_step(2, Step { accent: true, ..Step::note(Pitch::G, 0) });
    let read = s.pattern();
    assert!(read.steps[2].accent);
    assert_eq!(read.steps[2].pitch, Pitch::G);
    assert_eq!(read.steps.len(), 16);
    assert!(read.steps.iter().enumerate().all(|(i, st)| i == 2 || !st.on));
}

#[test]
fn switching_waveform_round_trips_both_ways() {
    let s = SharedState::new();
    s.set_waveform(Waveform::Square);
    assert_eq!(s.waveform(), Waveform::Square);
    s.set_waveform(Waveform::Saw);
    assert_eq!(s.waveform(), Waveform::Saw);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test state_live`
Expected: FAIL — `SharedState` has no `snapshot`, `set_step`, or `set_waveform`.

- [ ] **Step 3: Implement**

`cargo add -p acid-app arc-swap`. `AtomicF32` from Phase 02 is reused. `Parameters` derives
`Copy, Clone, Debug`. `snapshot` reads with `Ordering::Relaxed` and decodes `waveform` with
`if raw == 1 { Square } else { Saw }`. `set_step` ignores indices `>= 16`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app --test state_live`
Expected: PASS — 3 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(app): shared state carries every knob, pattern and transport flag"
```

---

### Task 34: The audio callback applies `Parameters` to the engine

**Files:**
- Create: `rust/crates/acid-app/src/audio/renderer.rs`
- Modify: `rust/crates/acid-app/src/audio/mod.rs`, `src/main.rs`
- Test: `rust/crates/acid-app/tests/renderer.rs`

**Interfaces:**
- Consumes: `SharedState`, `Parameters`, `Engine`.
- Produces:
  - `pub struct AudioRenderer { engine: Engine, state: std::sync::Arc<SharedState>, mono: Vec<f32> }`
        with `pub fn new(sample_rate: f32, channels: usize, state: std::sync::Arc<SharedState>,
    max_block: usize) -> Self`,
    `pub fn render_into(&mut self, out: &mut [f32])` (interleaved, mono duplicated to every channel),
    `pub fn engine(&self) -> &Engine`.
  - `pub fn apply(params: &Parameters, engine: &mut Engine)` — the one place params reach the engine;
    pures so it is unit-testable.
- Later tasks rely on: `AudioRenderer::render_into` (Task 37 allocation test, Phase 09 Task 44,
  Phase 10 Task 48).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-app/tests/renderer.rs
use acid_app::audio::renderer::{apply, AudioRenderer};
use acid_app::state::SharedState;
use acid_core::analysis::peak;
use acid_core::oscillator::Waveform;

#[test]
fn applying_parameters_reaches_the_engine_knobs() {
    let s = SharedState::new();
    s.set_waveform(Waveform::Square);
    s.tuning.set(0.75);
    s.cutoff.set(0.9);
    let mut e = acid_core::engine::Engine::new(48_000.0);
    apply(&s.snapshot(), &mut e);
    // The engine exposes readback for every knob it stores:
    assert_eq!(e.waveform(), Waveform::Square);
    assert!((e.tuning() - 0.75).abs() < 1e-6);
    assert!((e.cutoff() - 0.9).abs() < 1e-6);
}

#[test]
fn stopped_transport_renders_silence_into_every_channel() {
    let s = SharedState::new();
    let mut r = AudioRenderer::new(48_000.0, 2, s, 2_048);
    let mut out = vec![1.0f32; 4_096]; // stereo
    r.render_into(&mut out);
    assert_eq!(peak(&out), 0.0);
}

#[test]
fn playing_duplicates_the_mono_signal_to_both_channels() {
    let s = SharedState::new();
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);
    s.tempo_bpm.set(120.0);
    s.set_pattern(std::sync::Arc::new(acid_core::pattern::Pattern::from_steps(
        [acid_core::pattern::Step::note(acid_core::pitch::Pitch::A, -1); 16],
    )));
    let mut r = AudioRenderer::new(48_000.0, 2, s, 2_048);
    let mut out = vec![0.0f32; 8_192];
    r.render_into(&mut out);
    let left = &out[0..8_192];
    for i in (0..8_192).step_by(2) {
        assert_eq!(out[i], out[i + 1], "channels differ at {i}");
    }
    assert!(peak(left) > 0.01);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test renderer`
Expected: FAIL — no `audio::renderer` module; `Engine` lacks readback getters.

- [ ] **Step 3: Implement**

Add readback getters to `Engine` for every knob (`waveform()`, `tuning()`, `cutoff()`, `resonance()`,
`env_mod()`, `decay()`, `accent()`, `volume()`, `bpm()`, `position()`). `AudioRenderer::render_into`
`render_into` divides `out` by the stored channel count, renders that many mono frames in chunks of at
most `max_block` (never allocating: `mono` is preallocated to `max_block`), then copies each frame to every
channel. The third test above passes `2` channels and checks both sides match.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app --test renderer`
Expected: PASS — 3 passed.
**Listen:** run the app, drag CUT OFF while the factory pattern plays — the sweep must be audible with
no zipper noise or clicks (the engine smooths; the UI does not need to).

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(audio): audio renderer applies shared parameters every block"
```

---

### Task 35: Editing a step during playback

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (step click → `SharedState::set_step`)
- Modify: `rust/crates/acid-core/src/sequencer.rs` (read the newest `Arc<Pattern>` at every boundary)
- Test: `rust/crates/acid-app/tests/live_edit.rs`

**Interfaces:**
- Consumes: `SharedState::set_step`, `Sequencer`.
- Produces: `Sequencer` must accept a new pattern without disturbing position, gate or clock — already
  true after Phase 05 Task 26 — and the audio thread must pick up a *new* `Arc` at the next step
  boundary. `AudioRenderer` stores its last loaded `Arc` pointer and reloads when
  `Arc::ptr_eq(&cached, &state.pattern())` is false.
- Later tasks rely on: nothing new.

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-app/tests/live_edit.rs
#[test]
fn an_edit_made_while_rendering_is_heard_on_the_next_pass() {
    let s = SharedState::new();
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);
    s.tempo_bpm.set(240.0); // 6000 samples per step at 48k
    s.set_pattern(std::sync::Arc::new(Pattern::empty())); // everything silent
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 4_096);

    let mut silent = vec![0.0f32; 8_192];
    r.render_into(&mut silent);
    assert!(peak(&silent) < 1e-6);

    // Turn step 0 into a note while the loop is running.
    s.set_step(0, Step::note(Pitch::A, -1));
    // Render a full pattern plus a little more, so step 0 comes round again.
    let mut buf = vec![0.0f32; 48_000 * 2];
    r.render_into(&mut buf);
    assert!(peak(&buf) > 0.01, "the edited step was never heard");
}

#[test]
fn a_thousand_edits_from_another_thread_never_break_the_pattern() {
    let s = SharedState::new();
    s.set_pattern(std::sync::Arc::new(Pattern::empty()));
    let writer = {
        let s = s.clone();
        std::thread::spawn(move || {
            for i in 0..1_000 {
                s.set_step(i % 16, Step::note(Pitch::ALL[i % 13], 0));
            }
        })
    };
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 1_024);
    let mut out = vec![0.0f32; 2_048];
    while !writer.is_finished() {
        r.render_into(&mut out);
        assert_eq!(s.pattern().steps.len(), 16);
    }
    writer.join().unwrap();
    assert!(s.pattern().steps.iter().filter(|st| st.on).count() >= 1);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test live_edit`
Expected: FAIL — the renderer caches the first `Arc` and never sees the edit.

- [ ] **Step 3: Implement**

`AudioRenderer::render_into` calls `state.snapshot()` once, `apply`, then
`let pat = state.pattern(); if !Arc::ptr_eq(&pat, &self.cached) { self.engine.set_pattern(pat.clone());
self.cached = pat; }`. Wire the step keys in the UI to `set_step` (gestures refined in Phase 08).

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app --test live_edit`
Expected: PASS — 2 passed.
**Listen:** with the loop running, toggle steps on and off — each change is heard on the next pass of
that step, never a beat later, and never with a click.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app rust/crates/acid-core
git commit -m "feat(app): live pattern edits reach the audio thread at the next step"
```

---

### Task 36: Transport and chasing LEDs

**Files:**
- Modify: `rust/crates/acid-app/src/audio/renderer.rs` (publish position), `src/ui/mod.rs`
  (RUN/STOP/TEMPO buttons and LED chase)
- Test: `rust/crates/acid-app/tests/transport_live.rs`

**Interfaces:**
- Consumes: `SharedState::{playing, position, tempo_bpm}`, `ui::step_key::step_view`.
- Produces:
  - `AudioRenderer` stores `state.position.store(engine.position(), Relaxed)` after every
    `render_into` call.
  - `pub fn transport_button(ui: &mut egui::Ui, rect: Rect, label: &str, lit: bool) -> Response` in
    `ui/transport.rs` — a painted rectangular key like the real 303's RUN/STOP, `SILKSCREEN` legend,
    lit when active.
  - `pub fn nudge_tempo(bpm: f32, direction: i8) -> f32` — `±1` BPM steps, clamped by
    `timing::clamp_bpm`.
- Later tasks rely on: `transport_button` (Phase 09 Task 45 VOLUME, Phase 11 Task 55).

- [ ] **Step 1: Write the failing tests**

```rust
// rust/crates/acid-app/tests/transport_live.rs
#[test]
fn position_is_published_and_never_exceeds_fifteen() {
    let s = SharedState::new();
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);
    s.tempo_bpm.set(300.0);
    s.set_pattern(std::sync::Arc::new(Pattern::from_steps([Step::note(Pitch::A, -1); 16])));
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 2_048);
    let mut out = vec![0.0f32; 4_096];
    for _ in 0..200 {
        r.render_into(&mut out);
        assert!(s.position.load(std::sync::atomic::Ordering::Relaxed) < 16);
    }
    assert!(s.position.load(std::sync::atomic::Ordering::Relaxed) > 0);
}

#[test]
fn stopping_clears_the_chase() {
    let s = SharedState::new();
    s.playing.store(true, std::sync::atomic::Ordering::Relaxed);
    s.set_pattern(std::sync::Arc::new(Pattern::from_steps([Step::note(Pitch::A, -1); 16])));
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 2_048);
    r.render_into(&mut vec![0.0f32; 4_096]);
    s.playing.store(false, std::sync::atomic::Ordering::Relaxed);
    r.render_into(&mut vec![0.0f32; 4_096]);
    let position = s.position.load(std::sync::atomic::Ordering::Relaxed);
    let playing = s.playing.load(std::sync::atomic::Ordering::Relaxed);
    let view = acid_app::ui::step_key::step_view(&Step::rest(), position, position, playing);
    assert!(!view.current, "an LED must not stay lit after stop");
}

#[test]
fn tempo_nudges_land_on_whole_bpm_within_range() {
    assert_eq!(nudge_tempo(130.0, 1), 131.0);
    assert_eq!(nudge_tempo(300.0, 1), 300.0);
    assert_eq!(nudge_tempo(40.0, -1), 40.0);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test transport_live`
Expected: FAIL — `position` is never written; no `ui::transport`.

- [ ] **Step 3: Implement**

Create `src/ui/transport.rs` with `transport_button` and `nudge_tempo`, publish `position` at the end of
`render_into`, and give `TakkaApp` a real RUN/STOP pair plus two tempo buttons that write
`state.playing` / `state.tempo_bpm`. Paint RUN lit in `LED_ACCENT` while playing.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app --test transport_live`
Expected: PASS — 3 passed.
**Listen:** press RUN — the chase starts at step 1 and moves in time with the audio; press STOP — the
LEDs go dark and the note stops in the same instant, with no click.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(app): transport buttons and led chase driven by audio position"
```

---

### Task 37: The audio callback allocates nothing

**Files:**
- Modify: `rust/crates/acid-app/src/audio/renderer.rs`, `src/audio/mod.rs`
- Test: `rust/crates/acid-app/tests/no_alloc.rs` (a dedicated test binary with a counting allocator)

**Interfaces:**
- Consumes: `AudioRenderer`.
- Produces: `pub struct CountingAllocator` test-only — in the test binary as
  `#[global_allocator] static ALLOC: Counting = Counting;` with
  `pub fn allocations() -> usize` / `pub fn reset()`.
- Later tasks rely on: nothing, but every later audio change must keep this test green.

- [ ] **Step 1: Write the failing test**

```rust
// rust/crates/acid-app/tests/no_alloc.rs
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

#[test]
fn rendering_a_thousand_blocks_does_not_allocate() {
    let s = SharedState::new();
    s.playing.store(true, Ordering::Relaxed);
    s.set_pattern(std::sync::Arc::new(Pattern::from_steps(
        [Step::note(Pitch::A, -1); 16],
    )));
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 1_024);
    let mut out = vec![0.0f32; 2_048];
    r.render_into(&mut out); // warm-up: first block may load the pattern Arc
    COUNT.store(0, Ordering::Relaxed);
    for _ in 0..1_000 {
        r.render_into(&mut out);
    }
    assert_eq!(COUNT.load(Ordering::Relaxed), 0, "the audio path allocated");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test no_alloc`
Expected: FAIL — `set_pattern` / `Arc` cloning inside `render_into` allocates, and `apply` may box.

- [ ] **Step 3: Implement**

Only clone the `Arc` when the pattern pointer changed (an occasional allocation, once per pattern
switch, never steady-state). Keep `mono` preallocated at `max_block`; when `out` is larger than
`max_block`, render in `max_block` chunks. Replace `Pattern::clone` in the sequencer with `Arc::clone`.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app --test no_alloc`
Expected: PASS — 1 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "perf(audio): allocation-free audio callback"
```

---

**Phase 07 done when:** the app is a playable instrument — RUN starts the chase, knobs are heard
immediately, editing a step is heard on the next pass, and `no_alloc` is green.
