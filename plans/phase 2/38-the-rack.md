# Plan 38 — The rack: a device you can name, and a list of them

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). This plan and plan 39 share one seam —
> the bus — so a fresh implementer per task would re-derive the interfaces. Steps use checkbox (`- [ ]`) tracking and each
> task ends in a green commit.
>
> **Review:** every task ends with the review gate in `00-phase-2.md` — the diff is reviewed against this task's **Files** and
> **Interfaces** blocks, and every finding is actioned, before the commit. No subagents elsewhere: this plan runs inline, in one
> session, by one pair of hands.

**Goal:** `acid_app::device` — a `Device` (its own state, bank, number and name), a `Rack` (the list, the selection, the
transport and the published bus), and `SharedState::mute` — with `main` running on a rack of one and behaving exactly as it
does today.

**Architecture:** a `Rack` owns `Vec<Device>` and three pieces of rack-level transport state; its only exports to the other
threads are an `ArcSwap<BusState>` (the slot list, in rack order) which the mixer and the MIDI handlers will read. Nothing on
the audio path changes in this plan: `Rig` still renders device 1's `SharedState` exactly as it does today, and plan 39
replaces that closure with the mixer.

**Tech Stack:** Rust 2024, `arc_swap` 1.x (already a dependency), `serde` untouched, egui/eframe 0.37 untouched.

**Spec:** `docs/superpowers/specs/2026-11-13-device-list-design.md` — read "The device and the rack", "The transport" and
"What is deliberately not saved" before starting.

## Global Constraints

Inherited from `plans/phase 2/00-phase-2.md`. Restated where this plan is the one that can break them:

- `SavedState` does not change. `tests/persist.rs` stays green with no edit.
- `SharedState` gains exactly one field in this plan: `mute`.
- Device 1 is the regression: after task 6 the app sounds, times and saves exactly as before.
- `AddError`/`RemoveError` are enums with no `_` arm where they are matched — `match` is written out in full at every call
  site so a new refusal reason has to be handled.
- `DeviceKind::ALL` is the catalogue; `DeviceKind::name()` is its only caption. Nothing writes `"ACID"` a second time.

## Review Focus

The failure modes this plan can leave behind, most likely first. Each has its test in the task that owns the code.

1. **A removal that leaves `selected` out of range.** `selected` is an index into `devices`; removing the last or the selected
   device must leave the invariant `selected < devices.len()` true, and every accessor assumes it. Task 3.
2. **A number reused after a removal.** Two devices called `ACID 2` is the bug the `next_number` counter exists to prevent;
   it must be a high-water mark, not "lowest free". Task 2 and task 4.
3. **A published bus that disagrees with the rack.** The mixer renders `slots` and the strip draws `devices`; if `publish`
   is ever skipped after a mutation, one device is heard and another is drawn. Task 3.
4. **A device added while playing that stays silent,** because it was created with `playing: false` while the transport runs.
   It must be given the rack's current playing state and tempo at the moment it joins. Task 5.
5. **Two names that collide,** making "which one is muted" unanswerable in the strip and in a routing row. Task 4.

---

### Task 1: The mute flag

**Files:**
- Modify: `rust/crates/acid-app/src/state.rs` (the `SharedState` struct, its `new`, and a new test)

**Interfaces:**
- Produces: `SharedState::mute: AtomicBool`, `pub fn is_muted(&self) -> bool`, `pub fn set_mute(&self, muted: bool)`.
  Relaxed ordering both ways: the mixer reads it once per block and does not pair it with any other write.

- [ ] **Step 1: Write the failing test** in `state.rs`'s test module:

```rust
#[test]
fn a_device_starts_unmuted_and_remembers_being_muted() {
    let s = SharedState::new();
    assert!(!s.is_muted(), "a fresh device is audible");
    s.set_mute(true);
    assert!(s.is_muted());
    s.set_mute(false);
    assert!(!s.is_muted());
}
```

- [ ] **Step 2: Run it to watch it fail**

Run: `cd rust && cargo test -p acid-app --lib state::tests::a_device_starts`
Expected: FAIL — `no method named is_muted`.

- [ ] **Step 3: Add the field and the pair of accessors**

`pub mute: AtomicBool` beside `playing`, `AtomicBool::new(false)` in `new`, and the two methods in `impl SharedState`,
written like `is_playing`/`set_playing` with `Ordering::Relaxed`.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --lib`
Expected: PASS, including every existing state test.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/state.rs
git commit -m "feat(state): a device knows when it is muted"
```

---

### Task 2: The device and its kind

**Files:**
- Create: `rust/crates/acid-app/src/device.rs`
- Modify: `rust/crates/acid-app/src/lib.rs` (add `pub mod device;` in the alphabetical run of `pub mod`s)
- Test: `rust/crates/acid-app/tests/rack.rs` (new file)

**Interfaces:**
- Consumes: `crate::state::SharedState`, `acid_core::bank::PatternBank`, `acid_core::factory::starter`.
- Produces:
  ```rust
  pub const MAX_DEVICES: usize = 4;

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub enum DeviceKind { Acid }
  impl DeviceKind {
      pub const ALL: [DeviceKind; 1] = [DeviceKind::Acid];
      pub fn name(self) -> &'static str;      // "ACID"
  }

  pub struct Device {
      pub kind: DeviceKind,
      pub state: std::sync::Arc<SharedState>,
      pub bank: PatternBank,
      pub number: usize,
      pub name: String,
  }
  impl Device {
      pub fn new(kind: DeviceKind, number: usize) -> Self;
      pub fn adopted(kind: DeviceKind, number: usize, state: Arc<SharedState>, bank: PatternBank) -> Self;
      pub fn default_name(kind: DeviceKind, number: usize) -> String;   // "ACID 3"
  }
  ```

- [ ] **Step 1: Write the failing test** in `tests/rack.rs`:

```rust
use acid_app::device::{Device, DeviceKind, MAX_DEVICES};

#[test]
fn a_new_device_is_numbered_named_and_has_something_to_play() {
    let device = Device::new(DeviceKind::Acid, 2);
    assert_eq!(device.number, 2);
    assert_eq!(device.name, "ACID 2");
    assert_eq!(Device::default_name(DeviceKind::Acid, 3), "ACID 3");
    // A device that arrives silent is a bug report; the starter phrase is what a new song opens on.
    assert!(device.bank.current_pattern().steps.iter().any(|step| step.on));
    // Its own engine state, not a shared one: muting it cannot mute its neighbour.
    assert!(!device.state.is_muted());
    assert_eq!(MAX_DEVICES, 4);
    assert_eq!(DeviceKind::ALL, [DeviceKind::Acid]);
}
```

- [ ] **Step 2: Run it to watch it fail**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: FAIL — `unresolved import acid_app::device`.

- [ ] **Step 3: Write `device.rs`**

`DeviceKind::name` is a full `match` on the variant, no `_`. `Device::new` builds
`SharedState::new()`, `PatternBank::new_empty()` with `acid_core::factory::starter()` written into slot 0 — the same pair
`persist::new_song` builds, so a new device and a new song open on the same phrase. `default_name` is
`format!("{} {number}", kind.name())`.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/device.rs rust/crates/acid-app/src/lib.rs rust/crates/acid-app/tests/rack.rs
git commit -m "feat(device): a device kind, a device, and its number"
```

---

### Task 3: The rack, the selection and the bus

**Files:**
- Modify: `rust/crates/acid-app/src/device.rs`
- Test: `rust/crates/acid-app/tests/rack.rs`

**Interfaces:**
- Consumes: `Task 2`'s `Device`, `persist::saved_to_shared`, `persist::SavedState`, `arc_swap::ArcSwap`.
- Produces:
  ```rust
  /// What the audio callback and the MIDI handlers read. One entry per device, in rack order.
  #[derive(Debug)]
  pub struct BusState {
      pub slots: Vec<std::sync::Arc<SharedState>>,
  }

  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub enum AddError { AtCapacity }
  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub enum RemoveError { LastDevice, OutOfRange }

  pub struct Rack { /* private */ }
  impl Rack {
      pub fn from_saved(saved: &crate::persist::SavedState) -> Self;
      pub fn from_device(device: Device) -> Self;
      pub fn bus(&self) -> std::sync::Arc<ArcSwap<BusState>>;
      pub fn publish(&self);
      pub fn devices(&self) -> &[Device];
      pub fn selected(&self) -> usize;
      pub fn select(&mut self, index: usize) -> bool;
      pub fn state(&self) -> &SharedState;
      pub fn state_arc(&self) -> Arc<SharedState>;
      pub fn bank(&self) -> &PatternBank;
      pub fn bank_mut(&mut self) -> &mut PatternBank;
      pub fn add(&mut self, kind: DeviceKind) -> Result<usize, AddError>;
      pub fn remove(&mut self, index: usize) -> Result<(), RemoveError>;
      pub fn hold(&self) -> Arc<BusState>;      // ArcSwap::load_full, for tests and callers
  }
  ```

- [ ] **Step 1: Write the failing tests** in `tests/rack.rs`:

```rust
use acid_app::device::{AddError, DeviceKind, Rack, RemoveError};
use acid_app::persist;

fn one() -> Rack {
    Rack::from_saved(&persist::SavedState::default())
}

#[test]
fn a_rack_opens_on_one_device_and_publishes_it() {
    let mut rack = one();
    assert_eq!(rack.devices().len(), 1);
    assert_eq!(rack.selected(), 0);
    assert_eq!(rack.devices()[0].name, "ACID 1");
    rack.publish();
    assert_eq!(rack.hold().slots.len(), 1);
    assert!(std::sync::Arc::ptr_eq(&rack.hold().slots[0], &rack.devices()[0].state));
}

#[test]
fn adding_publishes_in_rack_order_and_the_bus_follows_the_list() {
    let mut rack = one();
    let second = rack.add(DeviceKind::Acid).expect("room for a second");
    assert_eq!(second, 1);
    rack.publish();
    let held = rack.hold();
    assert_eq!(held.slots.len(), 2);
    for (index, slot) in held.slots.iter().enumerate() {
        assert!(std::sync::Arc::ptr_eq(slot, &rack.devices()[index].state));
    }
}

#[test]
fn the_selection_never_leaves_the_list() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second device");
    assert!(rack.select(1));
    assert_eq!(rack.selected(), 1);
    assert!(!rack.select(7), "an out-of-range pick is refused, not clamped");
    assert_eq!(rack.selected(), 1);

    // Removing the selected device leaves the selection on a device that exists.
    rack.remove(1).expect("a first of two may go");
    assert_eq!(rack.devices().len(), 1);
    assert_eq!(rack.selected(), 0);
    let held = rack.hold();
    assert_eq!(held.slots.len(), 1);
    assert!(std::sync::Arc::ptr_eq(&held.slots[0], &rack.devices()[0].state));
}

#[test]
fn the_last_device_cannot_go_and_the_cap_holds() {
    let mut rack = one();
    assert_eq!(rack.remove(0), Err(RemoveError::LastDevice));
    assert_eq!(rack.remove(9), Err(RemoveError::OutOfRange));

    for _ in 1..4 {
        rack.add(DeviceKind::Acid).expect("up to four");
    }
    assert_eq!(rack.devices().len(), 4);
    assert_eq!(rack.add(DeviceKind::Acid), Err(AddError::AtCapacity));
    assert_eq!(rack.devices().len(), 4, "a refusal changes nothing");
}

#[test]
fn a_rack_from_a_saved_session_is_the_session_it_was() {
    use acid_core::bank::PatternBank;
    let mut saved = persist::SavedState::default();
    saved.bpm = 141.0;
    saved.bank = PatternBank::factory();
    saved.knobs.cutoff = 0.25;
    let rack = Rack::from_saved(&saved);
    assert_eq!(rack.devices().len(), 1);
    assert!((rack.state().cutoff.get() - 0.25).abs() < 1e-6);
    assert_eq!(rack.bank().current(), saved.bank.current());
}
```

The tempo this rack was built at is proved in task 5, where `Rack::tempo_bpm` exists; do not stub a method that does not
exist yet.

- [ ] **Step 2: Run them to watch them fail**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: FAIL — `no struct named Rack` at the import.

- [ ] **Step 3: Write the rack**

`Rack::from_saved` builds one device with `Device::adopted(DeviceKind::Acid, 1, persist::saved_to_shared(saved),
saved.bank.clone())`, `selected: 0`, `next_number: 2`, and publishes once at the end. `from_device` starts `next_number` at
`device.number + 1` and publishes once.

`select` returns false and changes nothing when `index >= devices.len()`. `remove` returns `OutOfRange` for a bad index and
`LastDevice` when `devices.len() == 1`, and on success pulls `selected` down to `devices.len() - 1` when it now points past
the end. Every mutating method ends with `self.publish()` — that is the invariant tests 3 and 4 lean on, and the reason
`publish` is safe to call from `add`: an `ArcSwap::store` takes the write lock for a store, never for a load.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/device.rs rust/crates/acid-app/tests/rack.rs
git commit -m "feat(device): a rack of devices with a selection and a published bus"
```

---

### Task 4: Names are the identity, numbers only seed them

**Files:**
- Modify: `rust/crates/acid-app/src/device.rs`
- Test: `rust/crates/acid-app/tests/rack.rs`

**Interfaces:**
- Produces: `Rack::rename(&mut self, index: usize, typed: &str) -> String` — the name the device ended up with. It never
  fails: blank falls back to the default, a collision is suffixed, and `"BASS (2)"` is what you get for a second `BASS`,
  the same rule `persist::free_song_name` uses for songs.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_device_number_is_never_reused() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");       // ACID 2
    rack.add(DeviceKind::Acid).expect("a third");        // ACID 3
    rack.remove(1).expect("a device may go");
    let index = rack.add(DeviceKind::Acid).expect("room again");
    assert_eq!(rack.devices()[index].number, 4, "the gap at 2 is not filled");
    assert_eq!(rack.devices()[index].name, "ACID 4");
}

#[test]
fn renaming_lands_on_one_device_and_keeps_names_distinct() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");

    assert_eq!(rack.rename(0, "  BASS  "), "BASS", "the typed name is trimmed");
    assert_eq!(rack.devices()[0].name, "BASS");
    assert_eq!(rack.devices()[1].name, "ACID 2", "the neighbour is untouched");

    assert_eq!(rack.rename(1, "BASS"), "BASS (2)", "a collision is suffixed, not refused");
    assert_eq!(rack.devices()[1].name, "BASS (2)");

    assert_eq!(rack.rename(1, "   "), "ACID 2", "blank falls back to the device's own name");
    assert_eq!(rack.rename(0, "BASS"), "BASS", "renaming back to your own name is not a collision");
}
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: FAIL — `no method named rename`.

- [ ] **Step 3: Implement `rename`, and stop `add` from filling gaps**

`add` must take `self.next_number` and increment it, never search for a free number. `rename` trims, falls back to
`Device::default_name`, then, while another device already holds the candidate, appends `" (n)"` counting from 2 with the
suffix allowed to push the name past any length the strip can show — truncation is `ui::text::fit`'s job at draw time, not
the store's.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/device.rs rust/crates/acid-app/tests/rack.rs
git commit -m "feat(device): names are distinct, and a number is never reused"
```

---

### Task 5: One transport over every device

**Files:**
- Modify: `rust/crates/acid-app/src/device.rs`
- Modify: `rust/crates/acid-app/src/midi/tempo.rs` (only the `follow_clock` test's call site, if it names a `SharedState`)
- Test: `rust/crates/acid-app/tests/rack.rs`

**Interfaces:**
- Produces:
  ```rust
  impl Rack {
      pub fn is_playing(&self) -> bool;
      pub fn set_playing(&self, playing: bool);
      pub fn toggle_playing(&self) -> bool;
      pub fn tempo_bpm(&self) -> f32;
      pub fn set_tempo_bpm(&self, bpm: f32);          // clamped by acid_core::timing::clamp_bpm
      pub fn tempo_cell(&self) -> Arc<state::AtomicF32>;   // the cell the MIDI thread writes
      pub fn sync_transport(&self);                   // fan playing + tempo out to every device
  }
  ```
  `playing: AtomicBool` and `tempo_bpm: Arc<AtomicF32>` are the rack's; the devices' atomics are mirrors kept up to date by
  `sync_transport`, which `set_playing`/`set_tempo_bpm` call and which `main` calls once per frame so a tempo the MIDI
  thread wrote is carried to the devices.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_transport_is_the_racks_and_the_devices_follow_it() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");

    assert!(!rack.is_playing());
    rack.set_playing(true);
    assert!(rack.is_playing());
    for device in rack.devices() {
        assert!(device.state.is_playing(), "every device starts on the same frame");
    }
    rack.set_playing(false);
    for device in rack.devices() {
        assert!(!device.state.is_playing());
    }

    rack.set_tempo_bpm(5.0);
    assert_eq!(rack.tempo_bpm(), acid_core::timing::MIN_BPM, "out of range is clamped, not stored");
    rack.set_tempo_bpm(141.0);
    for device in rack.devices() {
        assert_eq!(device.state.tempo_bpm.get(), 141.0);
    }
}

#[test]
fn a_device_added_while_the_transport_runs_starts_in_it() {
    let mut rack = one();
    rack.set_playing(true);
    rack.set_tempo_bpm(152.0);
    let index = rack.add(DeviceKind::Acid).expect("a second");
    let device = &rack.devices()[index];
    assert!(device.state.is_playing(), "a new device joins the transport, it does not sit silent");
    assert_eq!(device.state.tempo_bpm.get(), 152.0);
}

#[test]
fn a_tempo_written_straight_into_the_cell_reaches_every_device() {
    // This is the MIDI thread's path: the clock follower writes the cell, and the next sync carries it.
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    rack.tempo_cell().set(129.0);
    rack.sync_transport();
    for device in rack.devices() {
        assert_eq!(device.state.tempo_bpm.get(), 129.0);
    }
}
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: FAIL — `no method named is_playing`.

- [ ] **Step 3: Implement the transport**

`set_playing` stores the rack's flag then calls `sync_transport`. `sync_transport` writes `self.playing.load(Relaxed)` into
every `device.state.playing` and `self.tempo_bpm.get()` into every `device.state.tempo_bpm`. `add` finishes with
`self.sync_transport()` before `self.publish()`, which is what test 2 pins. `tempo_cell` clones the `Arc`, which is how
`main` hands the MIDI thread a cell without handing it the rack.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/device.rs rust/crates/acid-app/tests/rack.rs
git commit -m "feat(device): one transport on the rack, fanned out to every device"
```

---

### Task 6: `main` runs on a rack of one

**Files:**
- Modify: `rust/crates/acid-app/src/main.rs` (the `TakkaApp` struct, its constructor, every `self.state`/`self.bank` read,
  and the song/demo/new load paths)
- Modify: `rust/crates/acid-app/src/persist.rs` — `saved_to_shared` becomes a thin wrapper over `Rack::from_saved` or is
- Modify: nothing in `persist.rs` — `Rack::from_saved` is built on `persist::saved_to_shared`, so the session is still read
  by the one function that reads it. If the compiler says a `persist` item is now unused, delete the item rather than
  keeping it for nobody; `saved_to_shared` is used by `Rack::from_saved` and stays.
- Test: `rust/crates/acid-app/tests/persist.rs` (unchanged, must still pass) and the whole workspace

**Interfaces:**
- Consumes: task 3's `Rack`.
- Produces: `TakkaApp { rack: Rack, .. }` replacing `state: Arc<SharedState>` and `bank: PatternBank`.

- [ ] **Step 1: Write the failing test** — this task's test is the existing suite, and one new assertion that the seam holds:

```rust
// in tests/persist.rs, at the end of the file
#[test]
fn a_rack_built_from_a_session_carries_the_session_into_the_mixer() {
    // `main` hands the bus to the audio thread; a rack of one must publish exactly the session it was built from.
    let saved = acid_app::persist::SavedState::default();
    let rack = acid_app::device::Rack::from_saved(&saved);
    rack.publish();
    let held = rack.hold();
    assert_eq!(held.slots.len(), 1);
    assert_eq!(held.slots[0].tempo_bpm.get(), saved.bpm);
}
```

- [ ] **Step 2: Run it to watch it fail**

Run: `cd rust && cargo test -p acid-app --test persist`
Expected: PASS already if plans 38.1–38.5 are done — this step is a check, not a red test. If it is green, say so in the
commit message rather than inventing a failure.

- [ ] **Step 3: Move `main` onto the rack**

`TakkaApp` holds `rack: Rack`. Add two accessors on the app to keep the diff mechanical:
`fn state(&self) -> &SharedState { self.rack.state() }` and `fn bank_mut(&mut self) -> &mut PatternBank
{ self.rack.bank_mut() }`, then let the compiler walk you through the call sites. Two rules:

- **The `Rig` closure keeps rendering device 1.** Capture `Arc::clone(&self.rack.devices()[0].state)` before the closure and
  leave `Rig::new`'s body as it is. Plan 39 replaces it.
- **A song, a demo and NEW apply to device 1 only.** `persist::apply_saved(&saved, self.rack.state(), self.rack.bank_mut())`
  is already the call shape; nothing else in the load paths changes. Devices beyond the first keep playing through a load,
  which is the whole point of not saving them yet.

`main::logic` gains one line after the transport and pattern handling: `self.rack.sync_transport();`, so a tempo the MIDI
thread wrote reaches the devices on the next frame.

- [ ] **Step 4: Run the whole gate**

```bash
cd rust && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: PASS — 526 tests plus the new ones, no clippy warning, and `cargo run -p acid-app --bin takka-acid` sounds and
saves exactly as before (`--midi-map` prints the same table).

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "refactor(main): the app runs on a rack of one device"
```

---

## Progress

- [ ] Task 1 — the mute flag
- [ ] Task 2 — the device and its kind
- [ ] Task 3 — the rack, the selection and the bus
- [ ] Task 4 — names and numbers
- [ ] Task 5 — one transport over every device
- [ ] Task 6 — `main` runs on a rack of one

Rulings made while writing this plan, and their cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| `Rack::select` refuses an out-of-range index rather than clamping | A caller that expects a clamp gets `false` and must handle it; the alternative silently moves the player's selection. |
| Every mutating `Rack` method publishes | A `publish` per knob turn would be wasteful, so only list/name/filter changes publish; a future mutation that forgets to publish desynchronises the mixer, which is why the tests assert the bus after `add` and `remove`. |
| A song load targets device 1 and leaves the others alone | A player expecting a song to restore a whole rack will find three devices still there; phase 3's save format is where a song becomes a rack. |
| `tempo_bpm` is an `Arc<AtomicF32>` on the rack | The MIDI thread holds a clone; a later "global transport" hoists this cell into a `Transport` and the rack keeps a reference (see `plans/future/wishlist.md`). |
