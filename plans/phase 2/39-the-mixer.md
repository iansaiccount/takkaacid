# Plan 39 — The mixer: many engines, one sound card

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). One seam, one session: this plan and
> plan 38 share the bus, and plan 40 shares the `ArcSwap` idiom. Steps use checkbox (`- [ ]`) tracking; each task ends in a
> green commit.

**Goal:** `acid_app::audio::mixer::Mixer` — one cpal stream, one `AudioRenderer` per published device, a mute ramp per
device, one sum, one master limiter — plus `Engine::start_at` so a device added mid-run joins the transport in phase.

**Architecture:** `Mixer` loads `BusState` once per block and reconciles its `Vec<Slot>` against it: same length and every
`Arc` pointer-equal is the fast path and costs nothing; anything else rebuilds the list, which is the one allocation on the
audio path and is named where it happens. Each slot renders its device into scratch, the scratch is scaled by that slot's
mute ramp and added to the output, and the sum goes through a `Limiter` because each device's own limiter only holds its own
output, so two can reach twice the ceiling.

**Tech Stack:** Rust 2024, `arc_swap`, `acid_core::{limiter, param, sequencer, engine}`. No new dependency.

**Spec:** `docs/superpowers/specs/2026-11-13-device-list-design.md` — read "The mixer" and "The transport" before starting.

## Global Constraints

Inherited from `plans/phase 2/00-phase-2.md`. This plan is the one that can break the audio-path rule, so:

- **The audio callback allocates nothing in steady state.** The reconcile allocates only when the published list changes, and
  the doc comment on `Mixer::reconcile` says so in the way `audio/renderer.rs` already names its retired `Arc`.
- **No lock on the audio path.** `ArcSwap::load` is the only synchronisation; the `Mutex<AudioStatus>` stays UI-side.
- **One device must sound identical.** Plan 39's first test is a sample-for-sample comparison against the renderer this
  replaces, and it stays in the suite.
- `Rig`, `Opened`, `MAX_BLOCK` and the cpal callback are not modified. The mixer is a `RenderFn`, so `Rig` cannot tell the
  difference between it and what it had.
- Mute is read from `SharedState::mute` (plan 38, task 1), never from `BusState`.

## Review Focus

1. **A per-block rebuild.** If the reconcile's fast path is ever skipped, every engine is recreated each block: the transport
   restarts at step 0 sixty times a second, which sounds like a click and a stuck downbeat. Task 5, with a mutation check.
2. **A click on mute.** Mute must be a ramp, not a store, and a slot born muted must *start* on the muted value. Task 3.
3. **Two devices reaching twice the ceiling.** Each engine limits its own output at `0.98`; the sum has no ceiling unless the
   mixer provides one. Task 4.
4. **A device added while playing that comes in a step ahead or behind.** Without a seek it starts at its own step 0 while
   the rest of the rack is on step 9. Task 6.
5. **A muted device that stops advancing.** Mute is a gain, not a stop: its `position` must keep counting so unmuting rejoins
   in time rather than restarting. Task 3.

---

### Task 1: `start_at` in the core

**Files:**
- Modify: `rust/crates/acid-core/src/sequencer.rs` (`start`, and a new `start_at`)
- Modify: `rust/crates/acid-core/src/engine.rs` (`start`, and a new `start_at`)
- Test: `rust/crates/acid-core/tests/seek.rs` (new file)

**Interfaces:**
- Produces: `Sequencer::start_at(&mut self, step: usize)` and `Engine::start_at(&mut self, step: usize)`.
  `start()` becomes `start_at(0)`. `step` is taken `% STEPS_PER_PATTERN`, so a caller may hand it a long-running count.

- [ ] **Step 1: Write the failing test** in `tests/seek.rs`:

```rust
use std::sync::Arc;

use acid_core::engine::Engine;
use acid_core::factory::starter;
use acid_core::pattern::STEPS_PER_PATTERN;

/// At 130 bpm and 48 kHz a sixteenth is 5_538 samples; a block of it is enough to move the count on.
const SR: f32 = 48_000.0;

#[test]
fn start_is_start_at_zero() {
    let mut a = Engine::new(SR);
    let mut b = Engine::new(SR);
    a.set_pattern(Arc::new(starter()));
    b.set_pattern(Arc::new(starter()));
    a.start();
    b.start_at(0);
    assert_eq!(a.position(), b.position());
    assert_eq!(a.position(), 0);
}

#[test]
fn start_at_puts_the_sequencer_on_the_step_it_was_given() {
    let mut e = Engine::new(SR);
    e.set_pattern(Arc::new(starter()));
    e.start_at(9);
    assert_eq!(e.position(), 9);
    // A step index past the pattern wraps; the rack may hand it a count, not an index.
    e.start_at(STEPS_PER_PATTERN + 3);
    assert_eq!(e.position(), 3);
}

#[test]
fn a_seeked_engine_runs_from_there() {
    let mut e = Engine::new(SR);
    e.set_pattern(Arc::new(starter()));
    e.start_at(15);
    // One sixteenth, plus a sample: the wrap from 15 to 0 must have happened.
    let step_len = (SR * 60.0 / (130.0 * 4.0)).round() as usize;
    let mut buf = vec![0.0; step_len + 64];
    e.render(&mut buf);
    assert_eq!(e.position(), 0, "step 15 was the last of the bar");
}
```

- [ ] **Step 2: Run it to watch it fail**

Run: `cd rust && cargo test -p acid-core --test seek`
Expected: FAIL — `no method named start_at`.

- [ ] **Step 3: Implement**

`Sequencer::start_at(step)` is today's `start` body with `self.position = step % STEPS_PER_PATTERN` instead of `0`. Everything
else — `clock.restart()`, `prev_slide = false`, `trigger_pending = true` — stays: a seek is a start, so the note on the step it
lands on sounds. `Engine::start_at` delegates. Check whether `Engine::position()` exists already (the renderer publishes it);
if it does, use it in the test unchanged.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-core --test seek && cargo test -p acid-core`
Expected: PASS, and the existing sequencer and clock tests are untouched.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-core/src/sequencer.rs rust/crates/acid-core/src/engine.rs rust/crates/acid-core/tests/seek.rs
git commit -m "feat(core): a sequencer can start at a step, not only at the downbeat"
```

---

### Task 2: The mixer, with one device in it

**Files:**
- Create: `rust/crates/acid-app/src/audio/mixer.rs`
- Modify: `rust/crates/acid-app/src/audio/mod.rs` (add `pub mod mixer;` beside `pub mod renderer;`)
- Test: `rust/crates/acid-app/tests/mixer.rs` (new file)

**Interfaces:**
- Consumes: `device::BusState` (plan 38), `audio::renderer::AudioRenderer`, `audio::MAX_BLOCK`.
- Produces:
  ```rust
  pub struct Mixer { /* private */ }
  impl Mixer {
      pub fn new(sample_rate: f32, max_block: usize, bus: Arc<ArcSwap<BusState>>) -> Self;
      pub fn render_mono(&mut self, out: &mut [f32]);
      /// Slots currently rendered. Equals the published slot count after the first block.
      pub fn slots(&self) -> usize;
      /// How many slots have ever been built. A steady rack builds each slot once; this is what proves the
      /// reconcile's fast path works, and it is the only reason it is public.
      pub fn rebuilds(&self) -> usize;
  }
  ```

- [ ] **Step 1: Write the failing test** in `tests/mixer.rs`:

```rust
```rust
use acid_app::audio::MAX_BLOCK;
use acid_app::audio::mixer::Mixer;
use acid_app::audio::renderer::AudioRenderer;
use acid_app::device::{DeviceKind, Rack};
use acid_app::persist;
use acid_core::pattern::STEPS_PER_PATTERN as STEPS;

fn rack_of(n: usize) -> Rack {
    let mut rack = Rack::from_saved(&persist::SavedState::default());
    for _ in 1..n {
        rack.add(DeviceKind::Acid).expect("room");
    }
    rack.publish();
    rack
}

#[test]
fn one_device_through_the_mixer_is_the_device_it_was() {
    // The regression that makes this whole phase safe: nothing about the sound of a single device may change.
    let rack = rack_of(1);
    let state = rack.state_arc();
    rack.set_playing(true);

    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut mixed = vec![0.0; 4_096];
    mixer.render_mono(&mut mixed);

    // The renderer this replaces, driven from the same state and a fresh engine: both start on step 0.
    let mut bare = AudioRenderer::new(48_000.0, 1, state, MAX_BLOCK);
    let mut alone = vec![0.0; 4_096];
    bare.render_mono(&mut alone);

    assert!(mixed.iter().any(|sample| sample.abs() > 0.01), "the mixer must not be silent");
    for (a, b) in mixed.iter().zip(&alone) {
        assert!((a - b).abs() < 1e-6, "mixed {a} differs from bare {b}");
    }
    assert_eq!(mixer.slots(), 1);
}
```

If the two streams drift apart, the cause is the mute ramp starting at something other than 1.0 for an unmuted slot, or the
master limiter engaging when nothing is above the ceiling — fix that, do not loosen the tolerance.

- [ ] **Step 2: Run it to watch it fail**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: FAIL — `unresolved import acid_app::audio::mixer`.

- [ ] **Step 3: Write the mixer**

```rust
struct Slot {
    state: Arc<SharedState>,
    renderer: AudioRenderer,
    mute: param::Smooth,
}
```

`render_mono` loops over `out` in chunks of `max_block` (so a device block larger than `MAX_BLOCK` is fine), and for each chunk:
reconcile, then for each slot render into `scratch[..chunk.len()]` and accumulate `chunk[i] += scratch[i] * slot.mute.process(target)`
where `target` is `0.0` when `state.is_muted()` else `1.0`. Task 4 adds the master limiter; write the loop so it is one extra
statement at the end of the chunk.

`reconcile` is an explicit method so task 5 can test it in isolation. For this task it may rebuild unconditionally; the fast
path arrives in task 5 with its test.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app/src/audio/mixer.rs rust/crates/acid-app/src/audio/mod.rs rust/crates/acid-app/tests/mixer.rs
git commit -m "feat(audio): a mixer that renders a bus of devices"
```

---

### Task 3: Mute is a ramp, and a muted device keeps its place

**Files:**
- Modify: `rust/crates/acid-app/src/audio/mixer.rs`
- Test: `rust/crates/acid-app/tests/mixer.rs`

**Interfaces:**
- Produces: nothing new in the signature. `Slot::mute` is created with `param::Smooth::new(initial, 10.0, sample_rate)` where
  `initial` is the device's mute state at build time — `0.0` for a device born muted, `1.0` otherwise.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn unmuting_a_running_device_is_audible_and_it_kept_time_meanwhile() {
    let rack = rack_of(2);
    let b = std::sync::Arc::clone(&rack.devices()[1].state);
    rack.set_playing(true);
    b.set_mute(true);

    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 4_096];
    // The ramp is 10 ms, so the first block still carries the tail of device B. Render past it before listening.
    for _ in 0..8 {
        mixer.render_mono(&mut out);
    }
    let before = b.position.load(std::sync::atomic::Ordering::Relaxed);
    let with_b_muted = out.clone();

    b.set_mute(false);
    for _ in 0..8 {
        mixer.render_mono(&mut out);
    }
    assert!(
        out.iter().zip(&with_b_muted).any(|(loud, quiet)| (loud - quiet).abs() > 1e-3),
        "unmuting a running device must be audible"
    );

    let after = b.position.load(std::sync::atomic::Ordering::Relaxed);
    assert_ne!(before, after, "a muted device keeps counting steps so unmuting rejoins in time");
}
```

#[test]
fn a_device_born_muted_does_not_click_on_its_first_block() {
    let rack = rack_of(1);
    let state = std::sync::Arc::clone(&rack.devices()[0].state);
    rack.set_playing(true);
    state.set_mute(true);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 512];
    mixer.render_mono(&mut out);
    let peak = out.iter().fold(0.0f32, |acc, sample| acc.max(sample.abs()));
    assert!(peak < 1e-6, "a slot built for a muted device must start on the muted value: {peak}");
}
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: FAIL — the born-muted test hears the device ramp *down* from unity on its first block.

- [ ] **Step 3: Fix the initial value**

Read the device's mute state when the slot is built and hand it to `Smooth::new`. Nothing else changes: the per-sample
`process(target)` call already ramps both ways.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app/src/audio/mixer.rs rust/crates/acid-app/tests/mixer.rs
git commit -m "feat(audio): mute is a ramp, and a muted device still keeps time"
```

---

### Task 4: The master limiter

**Files:**
- Modify: `rust/crates/acid-app/src/audio/mixer.rs`
- Test: `rust/crates/acid-app/tests/mixer.rs`

**Interfaces:**
- Produces: `Mixer::master: acid_core::limiter::Limiter`, applied per sample to the sum. No gain-reduction meter:
  nothing draws one yet, and a `pub fn` nobody calls is a `dead_code` warning waiting to happen.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn two_devices_cannot_reach_twice_the_ceiling() {
    let rack = rack_of(2);
    for device in rack.devices() {
        device.state.volume.set(1.0);
    }
    rack.set_playing(true);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 48_000];
    mixer.render_mono(&mut out);
    let peak = out.iter().fold(0.0f32, |acc, sample| acc.max(sample.abs()));
    assert!(
        peak <= acid_core::limiter::CEILING + 1e-6,
        "the sum must not leave above the ceiling: {peak}"
    );
    assert!(peak > 0.1, "and it must not be silent either");
```

- [ ] **Step 2: Run it to watch it fail**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: FAIL — the peak is above `CEILING`, because two limited devices sum.

- [ ] **Step 3: Insert the limiter**

One statement at the end of each chunk: `for sample in chunk.iter_mut() { *sample = self.master.process(*sample); }`.
`Limiter::new(sample_rate)` at construction. Do not raise or lower `CEILING` — the same ceiling the engine uses is the right
one for the mix, and the design leaves "a softer one tuned for a sum" open only if this test says it is needed. Note in the
commit message which it turned out to be.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: PASS. Re-run `one_device_through_the_mixer_is_the_device_it_was`: the master limiter must not change a single device
that is already under the ceiling.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app/src/audio/mixer.rs rust/crates/acid-app/tests/mixer.rs
git commit -m "feat(audio): the mix bus has a ceiling of its own"
```

---

### Task 5: The reconcile, and its fast path

**Files:**
- Modify: `rust/crates/acid-app/src/audio/mixer.rs`
- Test: `rust/crates/acid-app/tests/mixer.rs`

**Interfaces:**
- Produces: `fn reconcile(&mut self, published: &BusState)` — private, with a doc comment naming the allocation. Builds a slot
  for every device whose `Arc` is not already held, keeps the renderers of those that are, orders the result by `published`,
  drops the rest. The fast path is `published.slots.len() == self.slots.len()` *and* every
  `Arc::ptr_eq(&self.slots[i].state, &published.slots[i])`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_rack_that_did_not_change_is_not_rebuilt() {
    let rack = rack_of(2);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 1_024];
    mixer.render_mono(&mut out);
    let built = mixer.rebuilds();
    assert_eq!(built, 2, "each device is built once");

    for _ in 0..16 {
        mixer.render_mono(&mut out);
    }
    assert_eq!(mixer.rebuilds(), built, "a steady rack reuses its engines");
}

#[test]
fn a_third_device_joins_without_disturbing_the_first_two() {
    let mut rack = rack_of(2);
    rack.set_playing(true);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 4_096];
    for _ in 0..8 {
        mixer.render_mono(&mut out);
    }
    let built = mixer.rebuilds();

    // `add` fans the transport out, so the new device is already playing; only its engine is new.
    rack.add(DeviceKind::Acid).expect("a third");
    mixer.render_mono(&mut out);

    assert_eq!(mixer.slots(), 3);
    assert_eq!(mixer.rebuilds(), built + 1, "only the new device is built; the two already playing are kept");
}

#[test]
fn removing_a_device_stops_its_sound() {
    let mut rack = rack_of(2);
    rack.set_playing(true);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 4_096];
    for _ in 0..8 {
        mixer.render_mono(&mut out);
    }
    rack.remove(1).expect("a second of two may go");
    mixer.render_mono(&mut out);
    assert_eq!(mixer.slots(), 1);
}
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: FAIL — `rebuilds()` grows every block.

- [ ] **Step 3: Implement the reconcile**

Load the bus once per block (`self.bus.load()`), compare, and only on a mismatch rebuild. Keeping the existing renderers is what
test 2 asserts; building a fresh one for a survivor would reset its transport, which is the bug in the Review Focus.

- [ ] **Step 4: Run the tests, then run the mutation check**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: PASS.

Then break it on purpose: delete the `Arc::ptr_eq` half of the fast path so a length match rebuilds. Run the suite. Expected:
`a_rack_that_did_not_change_is_not_rebuilt` FAILS. Restore the line and confirm green again. Record the check in
`## Progress`.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app/src/audio/mixer.rs rust/crates/acid-app/tests/mixer.rs
git commit -m "feat(audio): reconcile the bus once per block, and only when it moved"
```

---

### Task 6: A device added mid-run joins in phase

**Files:**
- Modify: `rust/crates/acid-app/src/audio/renderer.rs` (a `pending_seek` field, `seek`, and the one line in `render_mono`)
- Modify: `rust/crates/acid-app/src/audio/mixer.rs` (seek a newly built slot)
- Test: `rust/crates/acid-app/tests/mixer.rs`

**Interfaces:**
- Produces: `AudioRenderer::seek(&mut self, step: usize)`. It records the step; `render_mono` applies it *after* `apply(...)`,
  and only when the engine has started, so the `start()` that `apply` may issue this block is overridden rather than fought.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_device_added_mid_run_comes_in_on_the_bar_the_others_are_on() {
    let mut rack = rack_of(1);
    rack.set_playing(true);
    rack.set_tempo_bpm(300.0); // a sixteenth every 2_400 samples, so steps pass quickly
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 4_096];
    for _ in 0..16 {
        mixer.render_mono(&mut out);
    }
    let reference = rack.devices()[0]
        .state
        .position
        .load(std::sync::atomic::Ordering::Relaxed);
    assert!(reference > 1, "the reference device should have moved on: {reference}");

    // `add` fans the rack's transport and tempo out, so the join is the only thing under test here.
    rack.add(DeviceKind::Acid).expect("a second");
    mixer.render_mono(&mut out);

    let joined = rack.devices()[1]
        .state
        .position
        .load(std::sync::atomic::Ordering::Relaxed);
    let reference_now = rack.devices()[0]
        .state
        .position
        .load(std::sync::atomic::Ordering::Relaxed);
    // The seek aligns the *step*, not the sample inside it: the joining device's clock restarted, so it may cross
    // into the next step a few samples apart from the reference. One step of slack is the honest assertion.
    let ahead = (joined + STEPS - reference_now) % STEPS;
    assert!(
        ahead <= 1 || ahead == STEPS - 1,
        "a joining device must land within a step of the rack: joined {joined}, reference {reference_now}"
    );
}
```

- [ ] **Step 2: Run it to watch it fail**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: FAIL — `joined` lands on step 0 or 1 while `reference_now` is around 10, far outside the one step of slack the
test allows.

- [ ] **Step 3: Implement the seek**

`AudioRenderer::seek` stores `Some(step)`; `render_mono` takes it after `apply` and, if `self.engine.is_playing()`, calls
`self.engine.start_at(step)`. In `reconcile`, a slot built for a device other than the *reference* (`published.slots[0]`),
while the reference is playing, is seeked to the reference's published `position` — read straight off
`published.slots[0].position` before the slot renders, so both devices land on the same step in the same block.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test mixer`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app/src/audio/renderer.rs rust/crates/acid-app/src/audio/mixer.rs rust/crates/acid-app/tests/mixer.rs
git commit -m "feat(audio): a device added while playing joins on the bar, not on step 0"
```

---

### Task 7: `main` hands the bus to the mixer

**Files:**
- Modify: `rust/crates/acid-app/src/main.rs` (the `Rig::new` closure only)

**Interfaces:**
- Consumes: `Mixer::new`, `Rack::bus()`, `audio::MAX_BLOCK`.
- Produces: the app now renders the whole rack. With one device, nothing about it changes.

- [ ] **Step 1: Replace the closure**

```rust
let bus = rack.bus();
let rig = Rig::new(move |sample_rate| {
    let mut mixer = audio::mixer::Mixer::new(sample_rate, audio::MAX_BLOCK, Arc::clone(&bus));
    Box::new(move |buf: &mut [f32]| mixer.render_mono(buf))
});
```

`Rig::new` takes `Fn`, so the `Arc::clone` happens inside the closure body and never moves `bus`. Delete the old
`AudioRenderer` construction; `audio::renderer::AudioRenderer` is now reached only through the mixer.

- [ ] **Step 2: Run the whole gate**

```bash
cd rust && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: PASS. `cargo run -p acid-app --bin takka-acid` sounds exactly as it did, and the status line still names the device
and rate.

- [ ] **Step 3: Commit**

```bash
git add rust/crates/acid-app/src/main.rs
git commit -m "refactor(main): the audio callback renders the rack"
```

---

## Progress

- [ ] Task 1 — `start_at` in the core
- [ ] Task 2 — the mixer, one device
- [ ] Task 3 — mute is a ramp
- [ ] Task 4 — the master limiter
- [ ] Task 5 — the reconcile and its fast path (mutation check recorded here)
- [ ] Task 6 — join in phase
- [ ] Task 7 — `main` hands the bus to the mixer

Rulings made while writing this plan, and their cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| `Mixer::rebuilds()` is public for one test and one future meter | A public counter on a production type is a small smell; the alternative is an untestable fast path, which is the click-from-a-rebuilt-transport bug. |
| The master limiter uses `limiter::CEILING`, the same 0.98 | Two devices under one ceiling duck each other slightly. Accepted until a master fader exists; a softer sum limiter is a tuning change, not a redesign. |
| A new slot is seeked only when the reference device is playing | A rack that starts playing on the same block a device joins starts everyone at 0 together, which is right; the seek is for a join into a *running* transport. |
| Mute ramps over 10 ms, the engine's own `Smooth` default for volume | A very short mute still clicks. 10 ms is what the VOLUME knob uses, so muting and turning the knob sound alike. |
