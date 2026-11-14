# Plan 40 — MIDI routing: which device hears which controller

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). This plan changes a message's shape
> (`MidiEvent` grows a channel), so a fresh implementer per task would fight its own earlier compile errors. Steps use
> checkbox (`- [ ]`) tracking; each task ends in a green commit.
>
> **Review:** every task ends with the review gate in `00-phase-2.md` — the diff is reviewed against this task's **Files** and
> **Interfaces** blocks, and every finding is actioned, before the commit. No subagents elsewhere: this plan runs inline, in one
> session, by one pair of hands.

**Goal:** every MIDI message carries the channel it arrived on, every device has an `InputFilter` (ports, channel, role), one
`Router` per open port fans a message out to the devices it reaches, the app can hold several ports open at once, and the
machine's tempo comes from wherever the clock does.

**Architecture:** the filter lives on the device, on the UI thread, and is *derived* into `BusState` by `Rack::publish`, so
the MIDI handlers and the mixer read one published truth. Each port's handler owns a `Router` — the bootleg gate and the tempo
follower that used to be inline in `main` — loads `BusState` once per message, and writes only atomics. Learn stays one map
per device: the filter decides *whether* a controller reaches a device, the map decides *what it drives*.

**Tech Stack:** Rust 2024, `midir` (already a dependency), `arc_swap`, `acid_core::timing::clamp_bpm`.

**Spec:** `docs/superpowers/specs/2026-11-13-device-list-design.md` — read "Routing" and the "Why the filter and not a second
learn map" paragraph before starting. Also re-read the answer to the player's own question in the spec's opening: a keyboard
for notes, controllers for cutoff and resonance.

## Global Constraints

Inherited from `plans/phase 2/00-phase-2.md`. This plan is the one that can break the MIDI thread's rules:

- **The handler writes atomics and returns.** No lock, no allocation, no `format!` outside `trace::note`'s existing
  closure. `BusState` is loaded with `ArcSwap::load`, never `load_full`.
- **A missing or unplugged interface is a status line, never a dialog, and never a re-route.** A filter naming a port that has
  gone away keeps naming it; the device falls silent rather than quietly listening to the remaining port.
- **Device 1 hears everything until a filter says otherwise.** `InputFilter::everything()` is the factory, so a player who
  never opens the card cannot tell phase 2 happened.
- **No `_` arm** on `Role`, `Channel` or `Ports`.
- `midi::cc::apply_cc`, `midi::message::handle` and the pickup/learn machinery keep their signatures. Per-device learning is a
  consequence of `SharedState` being per device, not a change to those functions.

## Review Focus

1. **A note that plays two devices because nothing stopped it.** A device added by `+` must start with an empty port list, not
   "all". Task 3, and task 6's live test through the card.
2. **The same note delivered twice, from one port to one device.** The note gate is per port and shared by every device that
   port reaches — not per (port, device), not per device. Task 5.
3. **An unplugged controller re-routed to "all".** `Ports::Named` is never rewritten by a failed open. Task 3.
4. **A MIDI clock that moves one device and not the others.** The follower writes the rack's tempo cell; the fan-out carries
   it. Task 4.
5. **Channel off by one.** A controller sending on channel 1 means `Channel::One(0)`; the panel shows `1..16`. Task 2.

---

### Task 1: A message knows its channel

**Files:**
- Modify: `rust/crates/acid-app/src/midi/message.rs` (the `MidiEvent` enum, `parse`, `handle`, and the tests that name a
  variant)
- Modify: `rust/crates/acid-app/src/main.rs` (the handler's `match`, until task 5 replaces it)
- Test: `rust/crates/acid-app/src/midi/message.rs` (unit tests, updated)

**Interfaces:**
- Produces:
  ```rust
  pub enum MidiEvent {
      NoteOn { channel: u8, note: u8, velocity: u8 },
      NoteOff { channel: u8, note: u8 },
      ControlChange { channel: u8, cc: u8, value: u8 },
      Clock,
      Start,
      Stop,
      Continue,
  }
  impl MidiEvent {
      /// `None` for the four real-time bytes, which carry no channel.
      pub fn channel(&self) -> Option<u8>;
      /// True for the two channel messages that carry a note.
      pub fn carries_notes(&self) -> bool;
  }
  ```
  `channel` is `status & 0x0F`, so it is `0..=15` and channel 1 on a keyboard's own display is `0` here.

- [ ] **Step 1: Write the failing tests** in `message.rs`'s test module:

```rust
#[test]
fn a_message_carries_the_channel_it_arrived_on() {
    assert_eq!(
        parse(&[0x92, 60, 100]),
        Some(MidiEvent::NoteOn { channel: 2, note: 60, velocity: 100 })
    );
    assert_eq!(
        parse(&[0x8F, 36, 0]),
        Some(MidiEvent::NoteOff { channel: 15, note: 36 })
    );
    assert_eq!(
        parse(&[0xB0, 74, 127]),
        Some(MidiEvent::ControlChange { channel: 0, cc: 74, value: 127 })
    );
    assert_eq!(parse(&[0xF8]).and_then(|event| event.channel()), None, "clock has no channel");
}

#[test]
fn only_the_channel_messages_can_carry_notes() {
    assert!(MidiEvent::NoteOn { channel: 0, note: 60, velocity: 1 }.carries_notes());
    assert!(MidiEvent::NoteOff { channel: 0, note: 60 }.carries_notes());
    assert!(!MidiEvent::ControlChange { channel: 0, cc: 1, value: 1 }.carries_notes());
    assert!(!MidiEvent::Clock.carries_notes());
}
```

Update the existing `note_on_and_off_parse_on_any_channel` and `control_clock_and_transport_parse` expectations to include
`channel:`. The "on any channel" test keeps its point — it now shows the channel *and* proves the parse does not filter on it.

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --lib midi::message`
Expected: FAIL — `missing field channel`.

- [ ] **Step 3: Add the channel**

`parse` masks `status & 0x0F` for the three channel arms. `handle` ignores the channel: it applies to whichever device it is
handed, and the routing has already decided that. Rewrite `handle`'s match arms with `channel: _` — or, better, bind nothing by
using `..` in the pattern (`MidiEvent::NoteOn { velocity: 0, .. }`), which reads as "the channel is not my business".

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --lib midi && cargo test -p acid-app`
Expected: PASS. `main`'s handler still compiles because it matches `ControlChange { cc, value }` — add `..` where it needs to.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/midi/message.rs rust/crates/acid-app/src/main.rs
git commit -m "feat(midi): a message carries the channel it arrived on"
```

---

### Task 2: The filter

**Files:**
- Create: `rust/crates/acid-app/src/midi/route.rs`
- Modify: `rust/crates/acid-app/src/midi/mod.rs` (add `pub mod route;`)
- Test: `rust/crates/acid-app/src/midi/route.rs` (unit tests)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, PartialEq, Eq, Debug)]
  pub enum Ports { All, Named(Vec<String>) }
  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub enum Channel { All, One(u8) }          // 0..=15 in, 1..=16 on the panel
  #[derive(Copy, Clone, PartialEq, Eq, Debug)]
  pub enum Role { PlayAndControl, ControlOnly }

  #[derive(Clone, PartialEq, Eq, Debug)]
  pub struct InputFilter { pub ports: Ports, pub channel: Channel, pub role: Role }

  impl InputFilter {
      pub fn everything() -> Self;   // All / All / PlayAndControl — the factory device
      pub fn silent() -> Self;       // Named(vec![]) / All / PlayAndControl — a device just added
      pub fn hears(&self, port: &str, channel: u8) -> bool;
  }
  ```

- [ ] **Step 1: Write the failing tests** in `route.rs`:

```rust
#[test]
fn the_factory_device_hears_every_port_and_channel() {
    let filter = InputFilter::everything();
    for port in ["Launchkey MIDI", "Arturia KeyStep", "nanoKONTROL"] {
        for channel in 0..=15 {
            assert!(filter.hears(port, channel), "{port} ch{channel}");
        }
    }
}

#[test]
fn a_new_device_hears_nothing_until_it_is_given_a_port() {
    let filter = InputFilter::silent();
    assert!(!filter.hears("Launchkey MIDI", 0));
    assert!(!filter.hears("Launchkey MIDI", 9));
}

#[test]
fn a_named_port_and_a_named_channel_both_have_to_match() {
    let filter = InputFilter {
        ports: Ports::Named(vec!["nanoKONTROL".to_string()]),
        channel: Channel::One(3),
        role: Role::ControlOnly,
    };
    assert!(filter.hears("nanoKONTROL", 3));
    assert!(!filter.hears("nanoKONTROL", 2), "the channel is the one it was given");
    assert!(!filter.hears("Launchkey MIDI", 3), "no other port may address it");

    let any_channel = InputFilter { channel: Channel::All, ..filter };
    assert!(any_channel.hears("nanoKONTROL", 0));
    assert!(any_channel.hears("nanoKONTROL", 15));
}

#[test]
fn a_controller_only_device_still_hears_but_may_not_play() {
    let filter = InputFilter { role: Role::ControlOnly, ..InputFilter::everything() };
    assert!(filter.hears("Launchkey MIDI", 0), "its knobs still arrive");
    assert_eq!(filter.role, Role::ControlOnly);
}
```

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --lib midi::route`
Expected: FAIL — the module does not exist.

- [ ] **Step 3: Write the filter**

`hears` is two `match`es, both written out in full:

```rust
let port_ok = match &self.ports {
    Ports::All => true,
    Ports::Named(names) => names.iter().any(|name| name == port),
};
let channel_ok = match self.channel {
    Channel::All => true,
    Channel::One(only) => only == channel,
};
port_ok && channel_ok
```

The role is deliberately *not* consulted by `hears`: a `ControlOnly` device still receives its controller messages, and the
caller asks the role only about messages that carry notes. One question per function.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --lib midi::route`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/midi/route.rs rust/crates/acid-app/src/midi/mod.rs
git commit -m "feat(midi): a device says which ports and which channel it hears"
```

---

### Task 3: The filter reaches the bus

**Files:**
- Modify: `rust/crates/acid-app/src/device.rs` (`Device::inputs`, `BusState::routes`, `Rack::publish`, `Rack::set_inputs`)
- Test: `rust/crates/acid-app/tests/rack.rs`

**Interfaces:**
- Consumes: task 2's `InputFilter`.
- Produces:
  ```rust
  impl Device {
      pub fn adopted(..) -> Self;      // gains `inputs: InputFilter::everything()`
      pub fn new(..) -> Self;          // gains `inputs: InputFilter::silent()`
  }
  pub struct BusState {
      pub slots: Vec<Arc<SharedState>>,
      pub routes: Vec<crate::midi::route::InputFilter>,   // same index as `slots`
  }
  impl Rack {
      pub fn set_inputs(&mut self, index: usize, inputs: InputFilter);
      pub fn inputs(&self, index: usize) -> Option<&InputFilter>;
  }
  ```

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_bus_carries_the_route_beside_the_slot() {
    use acid_app::midi::route::{Channel, InputFilter, Ports, Role};
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    rack.publish();
    let held = rack.hold();
    assert_eq!(held.routes.len(), held.slots.len());
    assert_eq!(held.routes[0], InputFilter::everything(), "the factory device hears everything, as it always did");
    assert_eq!(held.routes[1], InputFilter::silent(), "a device just added hears nothing");

    rack.set_inputs(1, InputFilter {
        ports: Ports::Named(vec!["nanoKONTROL".into()]),
        channel: Channel::All,
        role: Role::ControlOnly,
    });
    assert_eq!(rack.hold().routes[1].role, Role::ControlOnly, "setting a filter publishes it");
    assert_eq!(rack.inputs(1), Some(&rack.hold().routes[1]));
    assert_eq!(rack.inputs(9), None);
}
```

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: FAIL — `no field routes`.

- [ ] **Step 3: Add the field, the derivation and the setter**

`Device::adopted` gets `InputFilter::everything()`, `Device::new` gets `InputFilter::silent()` — that difference is the whole
of Review Focus 1. `Rack::publish` builds `routes` in the same `map` that builds `slots`, so the two lists cannot drift.
`set_inputs` writes the filter and publishes; `inputs` is a bounds-checked read.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test rack && cargo test -p acid-app --test mixer`
Expected: PASS. The mixer ignores `routes` entirely — it is in the same struct because both readers want one truth, not
because both read all of it.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/device.rs rust/crates/acid-app/tests/rack.rs
git commit -m "feat(device): a device's route travels with it on the bus"
```

---

### Task 4: The router — one per port, writing atomics

**Files:**
- Modify: `rust/crates/acid-app/src/midi/route.rs`
- Test: `rust/crates/acid-app/src/midi/route.rs`

**Interfaces:**
- Produces:
  ```rust
  /// One open port's point of view: its name, its clock follower, its note gate and the tempo cell it writes.
  pub struct Router { /* private */ }

  impl Router {
      pub fn new(port: impl Into<String>, tempo: std::sync::Arc<crate::state::AtomicF32>) -> Self;
      /// One message from this port. Runs on the MIDI thread: no lock, no allocation, atomics only.
      pub fn on_message(&mut self, bus: &crate::device::BusState, bytes: &[u8]);
      pub fn port(&self) -> &str;
  }
  ```
  `on_message` parses, then applies the message to every device the bus routes it to.

- [ ] **Step 1: Write the failing tests**

```rust
use std::sync::Arc;

use acid_app::device::BusState;
use acid_app::midi::route::Router;
use acid_app::state::{AtomicF32, SharedState};

fn bus_of(filters: Vec<acid_app::midi::route::InputFilter>) -> BusState {
    let slots: Vec<Arc<SharedState>> = filters.iter().map(|_| SharedState::new()).collect();
    BusState { slots, routes: filters }
}

#[test]
fn a_note_reaches_every_device_whose_filter_names_its_channel() {
    let everything = acid_app::midi::route::InputFilter::everything();
    let bus = bus_of(vec![
        everything.clone(),
        acid_app::midi::route::InputFilter {
            channel: acid_app::midi::route::Channel::One(2),
            ..everything
        },
    ]);
    let tempo = Arc::new(AtomicF32::new(130.0));
    let mut router = Router::new("Launchkey MIDI", tempo);
    router.on_message(&bus, &[0x92, 60, 100]); // channel 2, note 60

    assert_eq!(bus.slots[0].midi_velocity.load(std::sync::atomic::Ordering::Relaxed), 100);
    assert_eq!(bus.slots[1].midi_velocity.load(std::sync::atomic::Ordering::Relaxed), 100);
    assert!((bus.slots[0].midi_hz.get() - bus.slots[1].midi_hz.get()).abs() < 1e-3);
}

#[test]
fn a_channel_filter_keeps_a_note_off_its_neighbours() {
    let everything = acid_app::midi::route::InputFilter::everything();
    let bus = bus_of(vec![
        acid_app::midi::route::InputFilter { channel: acid_app::midi::route::Channel::One(1), ..everything.clone() },
        everything,
    ]);
    let mut router = Router::new("Launchkey MIDI", Arc::new(AtomicF32::new(130.0)));
    router.on_message(&bus, &[0x92, 60, 100]); // channel 2
    assert!(!bus.slots[0].midi_gate.load(std::sync::atomic::Ordering::Relaxed), "device 1 is not on channel 2");
    assert!(bus.slots[1].midi_gate.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn a_controller_only_device_gets_its_knobs_and_not_its_notes() {
    use acid_app::midi::route::{InputFilter, Role};
    let controller = InputFilter { role: Role::ControlOnly, ..InputFilter::everything() };
    let keys = InputFilter::everything();
    let bus = bus_of(vec![controller, keys]);
    let mut router = Router::new("Launchkey MIDI", Arc::new(AtomicF32::new(130.0)));

    router.on_message(&bus, &[0x90, 60, 100]); // a note
    assert!(!bus.slots[0].midi_gate.load(std::sync::atomic::Ordering::Relaxed), "control-only hears no notes");
    assert!(bus.slots[1].midi_gate.load(std::sync::atomic::Ordering::Relaxed), "the playing device hears it");

    router.on_message(&bus, &[0xB0, 7, 0]); // volume, CC 7
    assert_eq!(bus.slots[0].volume.get(), 0.0, "but its knobs still arrive");
}

#[test]
fn a_port_that_is_not_named_reaches_nothing() {
    use acid_app::midi::route::{InputFilter, Ports};
    let bus = bus_of(vec![InputFilter {
        ports: Ports::Named(vec!["nanoKONTROL".into()]),
        ..InputFilter::everything()
    }]);
    let mut router = Router::new("Launchkey MIDI", Arc::new(AtomicF32::new(130.0)));
    router.on_message(&bus, &[0x90, 60, 100]);
    assert!(!bus.slots[0].midi_gate.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn a_clock_sets_the_tempo_the_rack_cell_holds() {
    use acid_app::midi::tempo::PPQN;
    let bus = bus_of(vec![acid_app::midi::route::InputFilter::everything()]);
    let tempo = Arc::new(AtomicF32::new(130.0));
    let mut router = Router::new("Launchkey MIDI", Arc::clone(&tempo));
    // Two ticks a known distance apart: the follower needs an interval before it will name a tempo.
    router.on_message(&bus, &[0xF8]);
    std::thread::sleep(std::time::Duration::from_millis(
        (60_000 / (PPQN as u64 * 120)) as u64,
    ));
    router.on_message(&bus, &[0xF8]);
    let named = tempo.get();
    assert!((named - 120.0).abs() < 12.0, "the rack's cell should be near 120 bpm, was {named}");
}
```

The clock test's tolerance is wide on purpose: a two-tick measurement on a loaded machine is an estimate. If the follower
needs a third tick to name a tempo at all, feed it one more rather than widening the window.

- [ ] **Step 2: Run to watch them fail**

Run: `cd rust && cargo test -p acid-app --lib midi::route`
Expected: FAIL — `no struct named Router`.

- [ ] **Step 3: Write the router**

`on_message` does, in order: `trace::note` the raw bytes (moved out of `main`'s closure, unchanged); `message::parse`; then

- `Clock`: `self.follow.on_clock(Instant::now())`, and on `Some(bpm)` write `self.tempo.set(clamp_bpm(bpm))`;
- `ControlChange { channel, cc, value }`: for every `(slot, filter)` where `filter.hears(self.port, channel)`,
  `cc::apply_cc(slot, cc, value)`;
- a note event: `gate::passes(&mut self.gate, event)` first — one gate per port, shared by every device it reaches — and then,
  for every `(slot, filter)` where `filter.hears(..) && filter.role == Role::PlayAndControl`, `message::handle(event, slot)`;
- `Start`/`Stop`/`Continue`: nothing, yet, exactly as today. The transport is the rack's and a MIDI transport byte has no rack
  to reach from the MIDI thread; wiring it is its own round (see `plans/future/wishlist.md`). The router parses them and drops
  them, and the plan does not pretend otherwise.

`follow_clock` in `midi::tempo` has no caller after this. Delete it and its test, or keep it as the documented helper if a
test uses it; do not leave a `pub fn` with no caller.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --lib midi`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/midi/route.rs rust/crates/acid-app/src/midi/tempo.rs
git commit -m "feat(midi): one router per port, fanning a message out to the devices that hear it"
```

---

### Task 5: Several ports open at once

**Files:**
- Modify: `rust/crates/acid-app/src/main.rs` (`Midi`, its probe, `poll`, `choose`, and the handler closure)
- Modify: `rust/crates/acid-app/src/midi/ports.rs` (a `list_ports` caller that answers names only — check what is needed)
- Modify: `rust/crates/acid-app/src/ui/settings.rs` (`midi_enabled: Vec<String>`, `Action::TogglePort(String, bool)`,
  `Live::midi_current: &[String]`, and `Rows::ports: Vec<Rect>` — one rect per port row, so a test can click one)
- Test: `rust/crates/acid-app/tests/settings_live.rs`

**Interfaces:**
- Produces:
  ```rust
  struct Midi {
      enabled: Vec<String>,                                  // ports to open, by name
      inputs: Vec<(String, midi::ports::MidiIn)>,             // the live connections
      supervisor: midi::supervisor::MidiSupervisor,           // one for the app, as before
      pending: Option<Receiver<Result<(String, MidiIn), String>>>,
  }
  impl Midi {
      fn enable(&mut self, port: String);      // open it now
      fn disable(&mut self, port: &str);       // drop the connection, keep the preference
      fn enabled(&self) -> &[String];
  }
  ```
  `settings::Pick::Midi(Option<String>)` becomes `Action::TogglePort(String, bool)`; the card's port list becomes a checkbox
  list instead of a picker.

- [ ] **Step 1: Write the failing test** in `tests/settings_live.rs`:

```rust
#[test]
fn the_card_lists_every_port_with_its_own_toggle() {
    let state = SharedState::new();
    let mut panel = PanelState { settings_open: true, ..Default::default() };
    panel.settings.offer(
        vec!["Launchkey MIDI".into(), "nanoKONTROL".into(), "KeyStep".into()],
        vec!["Speakers".into()],
    );
    let ctx = Context::default();
    run_on(&ctx, &mut panel, &state, vec![]);

    // Three ports, three toggles: what the player turns on is a set, not a choice.
    let rows = acid_app::ui::settings::rows(layout(screen()), scale(screen()), &panel.settings);
    assert_eq!(rows.ports.len(), 3);

    click(&ctx, &mut panel, &state, rows.ports[1].center());
    let request = panel.settings_request.take().expect("the row asks for something");
    assert_eq!(
        request.action(),
        acid_app::ui::settings::Action::TogglePort("nanoKONTROL".into(), true)
    );
}
```

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --test settings_live`
Expected: FAIL — `no field ports` on `Rows`, or `no variant TogglePort`.

- [ ] **Step 3: Teach `Midi` to hold several connections**

One supervisor for the app (it answers "is there a MIDI input at all", and that question did not change). One `MidiIn` per
enabled name, opened through the existing probe-thread machinery — the probe now reports the name it opened, so a
disconnection can be attributed to the right entry. `enabled` survives a failed open: a controller that is unplugged is still
in the list, and the device's filter still names it (Review Focus 3).

On a first run `enabled` is empty and the saved session has no port name; the first successful enumeration fills `enabled` with
every port it found, which is how a rig that works today keeps working. A saved `midi_input: Some(name)` becomes
`enabled: vec![name]`.

- [ ] **Step 4: Run the tests and the gate**

```bash
cd rust && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "feat(midi): the app can hold several input ports open at once"
```

---

### Task 6: The app routes, and a load leaves the rack alone

**Files:**
- Modify: `rust/crates/acid-app/src/main.rs` (`TakkaApp`'s handler wiring, and `save`/`session_for_disk` for `midi_input`)
- Test: `rust/crates/acid-app/tests/rack.rs` (one unit-level assertion) and the manual check

**Interfaces:**
- Consumes: task 4's `Router`, task 5's `Midi`, plan 39's mixer.
- Produces: no new API. The handlers are built from `bus` + `tempo_cell`, and `main` is out of the message path.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_rack_hands_each_router_the_tempo_cell_it_shares() {
    // The MIDI thread gets a cell, not the rack: two routers on two ports must write the same tempo.
    let rack = one();
    let a = rack.tempo_cell();
    let b = rack.tempo_cell();
    a.set(144.0);
    assert_eq!(b.get(), 144.0);
    rack.sync_transport();
    assert_eq!(rack.devices()[0].state.tempo_bpm.get(), 144.0);
}
```

- [ ] **Step 2: Run it**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: PASS if plan 38 task 5 is done (this is a check, not a red test); say so in the commit message.

- [ ] **Step 3: Wire the handlers**

Order in `main`: the rack is built first, then `Midi::new(preference, rack.tempo_cell(), rack.bus())` — the MIDI side takes the
two cells it needs and never a reference to the rack. Each opened port's handler owns a `Router::new(port_name,
Arc::clone(&tempo_cell))`, built inside the probe thread with `Arc::clone(&bus)` captured alongside. The closure becomes:

```rust
move |_, bytes| router.on_message(&bus.load(), bytes)
```

Delete the inline `match event { .. }` from `main` — the routing table is the only thing that decides what a message means
now. `main::save` writes `saved.midi_input` from the *first* enabled port (the field is one string and stays one string until
the save format lands); note that in the plan's Progress section rather than inventing a field.

- [ ] **Step 4: Run the tests and the manual check**

```bash
cd rust && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Then, with two ports attached if the machine has them: `cargo run -p acid-app --bin takka-acid -- --midi-map` still prints the
same table, and playing a note on a port the device's filter does not name produces silence on that device and nothing in the
trace beyond the raw bytes.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "refactor(midi): the routing table decides, and main is out of the message path"
```

### Task 7: the tempo fader moves the rack, not one device

Task 6's review found this and the previous commit records it as deferred. It is task 7 because it belongs to this round: the
routing table is what delivers a CC to a device, and this is the one message whose meaning is the rack's.

**What is wrong today.** `midi::cc::apply_cc`'s `CcTarget::Tempo` arm writes the *device's* `tempo_bpm`, and `main::logic`
calls `Rack::sync_transport` every frame, which writes the *rack's* cell into every device. A learned tempo fader therefore
moves one device's tempo for less than a frame and the fan-out puts it back: factory CC 72 does nothing at all, silently, and
so does any learned tempo controller. It predates this plan — the fan-out came with plan 38.

**Root cause.** Tempo is the rack's: one cell (`Rack::tempo_bpm`) that the MIDI thread already holds a clone of for the
clock. The CC path writes a device's copy of it instead, and `apply_cc` has no way to reach the rack — its signature is
frozen by this plan's Global Constraints.

**Design.** The carrier is an atomic on the device that heard the message, and the router drains it. `SharedState::tempo_request`
holds `-1.0` when nothing is pending; the Tempo arm writes it, and `Router::on_message`'s CC arm takes it and writes the rack's
cell — in the same message, because the router already holds that cell for the clock. This is the `pattern_request` shape, and
it keeps `main` out of the message path.

The Tempo arm still writes the device's own `tempo_bpm` as well, at the same value. The pickup measures the knob against that
copy (`cc::knob_value`), and a fader that outran a frame would otherwise lose the knob it is on — the tolerance is two and a
half of 128 steps. The two writes agree, and the fan-out writes the same value back to every device on the next frame.

**Files:**
- Modify: `rust/crates/acid-app/src/state.rs` (`AtomicF32::swap`, `SharedState::tempo_request`, `request_tempo`, `take_tempo_request`)
- Modify: `rust/crates/acid-app/src/midi/cc.rs` (the `CcTarget::Tempo` arm, and the module doc's claim about "the same atomics
  the mouse writes")
- Modify: `rust/crates/acid-app/src/midi/route.rs` (the CC arm drains the request)
- Test: the three files' own test modules, and `rust/crates/acid-app/tests/rack.rs` — the router, the rack's cell and the
  fan-out joined the way `main` joins them, which is the smallest frame in which this bug is visible

**Interfaces:**
- Produces:
  ```rust
  impl AtomicF32 { pub fn swap(&self, value: f32) -> f32; }
  impl SharedState {
      /// Record the tempo a learned controller asked for; the router carries it to the rack's cell.
      pub fn request_tempo(&self, bpm: f32);
      /// `None` when nothing is pending.
      pub fn take_tempo_request(&self) -> Option<f32>;
  }
  ```

- [ ] **Step 1: Write the failing tests**

In `cc.rs`'s test module:

```rust
#[test]
fn a_tempo_fader_asks_the_rack_and_a_parked_one_does_not() {
    // The bug this pins: the fader wrote one device's tempo and the rack's fan-out put it back, so CC 72 did nothing.
    let s = SharedState::new();
    assert!(apply_cc(&s, 72, 0));
    assert_eq!(
        s.take_tempo_request(),
        None,
        "a fader that has not reached the knob asks for nothing"
    );

    reach(&s, 72);
    assert_eq!(s.take_tempo_request(), Some(MAX_BPM));
    assert_eq!(s.take_tempo_request(), None, "a request is taken once");
}
```

In `route.rs`'s test module:

```rust
#[test]
fn a_tempo_fader_moves_the_rack_not_one_device() {
    let bus = bus_of(vec![InputFilter::everything()]);
    let tempo = Arc::new(AtomicF32::new(130.0));
    let mut router = Router::new("Launchkey MIDI", Arc::clone(&tempo));

    // CC 72 is the factory tempo controller, and it takes pickup: the fader is swept up to the knob before it may turn it.
    router.on_message(&bus, &[0xB0, 72, 0]);
    assert_eq!(tempo.get(), 130.0, "a parked fader leaves the tempo alone");
    router.on_message(&bus, &[0xB0, 72, 127]);

    assert_eq!(tempo.get(), acid_core::timing::MAX_BPM, "the rack's cell moved");
    assert_eq!(
        bus.slots[0].take_tempo_request(),
        None,
        "the router drained the request in the same message"
    );
}
```

- [ ] **Step 2: Run to watch them fail**

Run: `cd rust && cargo test -p acid-app --lib midi`
Expected: FAIL — no method `take_tempo_request`, and the rack's cell never moves.

- [ ] **Step 3: Add the carrier, the drain and the request**

`AtomicF32::swap` is the same relaxed swap on the bits. `tempo_request` starts at `-1.0`; `request_tempo` stores,
`take_tempo_request` swaps in `-1.0` and answers `None` for anything not positive, so a tempo of `0` can never be mistaken
for "nothing pending". The router drains inside its existing CC loop, right after `apply_cc`, so a message costs one swap per
device that hears it and no second pass over the bus.

- [ ] **Step 4: Run the tests and the gate**

Run: `cd rust && cargo test -p acid-app --lib midi && cargo test --workspace`
Then the three gates. Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "fix(midi): a learned tempo fader moves the rack, not one device"
```

**Non-goal:** the tempo keys and the tempo knob already go through `Rack::set_tempo_bpm` and are untouched, and the fan-out's
≤ 200 ms latency for a followed clock is plan 38's design — this task makes the fader land in the same cell, not sooner.

### Task 8: one tempo cell, so the knob can be moved quickly and safely

Task 7 made the fader reach the rack and recorded the cost it could not pay: the device's `tempo_bpm` is a *mirror* that
`main` rewrites every frame, so a fan-out landing between two writes can put the old tempo back for a frame, and a fast sweep
loses the pickup pin. It also means the engine hears a tempo change at the next frame (up to 200 ms when nothing else is
happening), not at the next block. This task removes the mirror, which removes both.

**What is wrong today.** Tempo lives in two places: `Rack::tempo_bpm` (the truth) and each `SharedState::tempo_bpm` (a copy
`Rack::sync_transport` writes into every device, called once per frame from `main::logic`). Every reader — the engine, the
panel readout, the pickup — reads the copy. So a tempo is fast only if a frame runs, safe only if no fan-out interleaves, and
`apply_cc` needed a `tempo_request` carrier (task 7) precisely because it could only see a copy.

**Design.** One atomic. `SharedState::tempo_bpm` becomes `Arc<AtomicF32>`, and every device in a rack is built holding the
rack's own cell: `Rack::with_device` adopts the first device's cell, `Rack::add` builds the new device on
`SharedState::with_tempo(Arc::clone(&self.tempo_bpm))`. Then:

- `Rack::sync_transport` fans out `playing` only — there is nothing to fan out for tempo.
- `main::logic`'s per-frame `sync_transport()` goes: it existed only to carry a followed tempo.
- `CcTarget::Tempo` is a one-line write again, and `tempo_request` / `request_tempo` / `take_tempo_request` /
  `AtomicF32::swap` / the router's drain are deleted. The pickup reads the same cell the fader writes, so it can never be
  stale; the renderer reads it at the next block, so the tempo is heard immediately.
- `Deref` means every existing `.get()`/`.set()` call site is unchanged; the change is in how the cell is *made*.

**Files:**
- Modify: `rust/crates/acid-app/src/state.rs` (`tempo_bpm: Arc<AtomicF32>`, `with_tempo`, and the deletions)
- Modify: `rust/crates/acid-app/src/device.rs` (`Device::on`, `Rack::with_device`, `Rack::add`, `sync_transport`)
- Modify: `rust/crates/acid-app/src/midi/cc.rs` (the Tempo arm goes back to one write; the module doc's exception goes)
- Modify: `rust/crates/acid-app/src/midi/route.rs` (the drain goes; the clock arm already writes the same cell)
- Modify: `rust/crates/acid-app/src/main.rs` (drop the per-frame `sync_transport`)
- Test: `rust/crates/acid-app/tests/rack.rs`, and the touched files' own test modules

**Interfaces:**
- Produces:
  ```rust
  impl SharedState {
      /// A state with its own tempo cell: a device on its own, or a test.
      pub fn new() -> Arc<Self>;
      /// A state sharing `tempo`: how a device joins a rack, where there is one tempo for every engine.
      pub fn with_tempo(tempo: Arc<AtomicF32>) -> Arc<Self>;
  }
  impl Device {
      /// A new device built on somebody else's state — the rack's tempo cell, in `Rack::add`.
      pub fn on(kind: DeviceKind, number: usize, state: Arc<SharedState>) -> Self;
  }
  ```
- Removes: `SharedState::tempo_request`, `request_tempo`, `take_tempo_request`, `AtomicF32::swap`.

- [ ] **Step 1: Write the failing tests** in `tests/rack.rs`

```rust
#[test]
fn every_device_in_a_rack_holds_the_racks_tempo_cell() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    let cell = rack.tempo_cell();
    for device in rack.devices() {
        assert!(
            std::sync::Arc::ptr_eq(&cell, &device.state.tempo_bpm),
            "{} has its own tempo, which is a copy somebody has to fan out",
            device.name
        );
    }
}

#[test]
fn a_tempo_the_midi_thread_writes_is_heard_without_a_frame() {
    // The engine reads `SharedState::tempo_bpm` at the next block; the panel reads it every frame. If it is the rack's own
    // cell, a fader or a clock is heard at once and no fan-out can put an older tempo back.
    let rack = one();
    let mut router = acid_app::midi::route::Router::new("Launchkey MIDI", rack.tempo_cell());
    let bus = rack.bus();
    router.on_message(&bus.load(), &[0xB0, 72, 0]);
    router.on_message(&bus.load(), &[0xB0, 72, 127]);
    assert_eq!(rack.devices()[0].state.tempo_bpm.get(), acid_core::timing::MAX_BPM);
}
```

- [ ] **Step 2: Run to watch them fail**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: FAIL — `Arc::ptr_eq` over two different types (the device's tempo is not an `Arc` yet), which is the compile-red part.
      The behavioural pin came with the implementation: `a_followed_tempo_reaches_the_engine_at_the_next_block` reads `130.0`
      under the mirror and the clock's tempo under the shared cell, which is why it is the test that would have caught task 7's
      residual bug — the fader-driven tests pass either way, because the deleted arm wrote the copy as well as the request.

- [ ] **Step 3: Make the cell one cell, and delete what existed for the copy**

`Rack::with_device` keeps the first device's own cell (setting it to the saved tempo) instead of allocating a second one;
`Rack::add` builds the device on the rack's cell through `Device::on`. `sync_transport` writes `playing` only. The task-7
carrier and its drain go; `CcTarget::Tempo` is `state.tempo_bpm.set(..)` again, because now that is the rack's cell.

- [ ] **Step 4: Run the tests and the gate**

Run: `cd rust && cargo test --workspace`, then the three gates. Expected: PASS, and the deleted machinery's tests with it.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "refactor(tempo): one tempo cell shared by the rack and every device"
```

**Non-goal:** the panel's *readout* still follows on the next frame, so it can trail a fader by up to 200 ms when nothing
else is repainting; the sound does not, because the engine reads the cell at the next block. Waking the UI thread from MIDI is
a separate question (it has no handle on the egui context, and inventing one is a bigger change than this).

---

## Progress

- [x] **Task 1 — a message knows its channel.** `39e6575`. `channel` on the three channel messages, `channel()` and
      `carries_notes()`; `handle` ignores the channel because the routing has already decided. `gate.rs`'s patterns take `..`.
- [x] **Task 2 — the filter.** `34023c5`. `midi/route.rs`: `Ports`/`Channel`/`Role`, `InputFilter::{everything, silent, hears}`.
      `hears` deliberately ignores the role — one question per function, and the router asks the role only about note events.
- [x] **Task 3 — the filter reaches the bus.** `34023c5`. `Device.inputs` (`everything()` for a loaded device, `silent()` for a
      new one), `BusState.routes` built in the same pass as `slots`, `Rack::set_inputs`/`inputs`. 12 tests in `tests/rack.rs`.
- [x] **Task 4 — the router.** `34023c5`. One `Router` per port: raw-bytes trace, clock to the tempo cell, CC and notes to every
      device that hears them, one note gate per port so a duplicate note-on is a duplicate for everyone. Transport bytes are
      parsed and dropped, exactly as before this plan — a MIDI transport has no rack to reach from midir's thread, and the
      plan does not pretend otherwise.
- [x] **Task 5 — several ports at once.** `main::Midi` is `{ enabled, inputs, supervisor, pending, filled }`: one `MidiIn`
      per enabled name, opened by one probe thread per round that reports **every** port it tried, so one dead port cannot
      starve the rest; each result is installed as it arrives and only while its port is still wanted. `ui::settings`'s MIDI
      list is a checkbox list — `Request::MidiPort(String, bool)` → `Action::TogglePort(String, bool)`, `Rows::ports:
      Vec<Rect>`, no `none` row, and a held-but-gone port keeps its row marked `(missing)`. `PortChoice`, `pick_port`,
      `MidiIn::open`/`port_name` and `PickerState::input_choice` are gone: a name the app holds came from the machine, so
      `open_named` answers it exactly or not at all. `supervisor::status_line` takes the ports wanted, the names connected
      and the retries, so "off" is a state and not a retry. New tests: 3 in `settings_live.rs`, 2 in `main.rs` (the roster,
      and the in-flight result for a port turned off), 6 in `midi/route.rs` (the router, which had none — the debt this
      plan's own ruling named), 2 in `midi/ports.rs` (the exact-name rule and the first-run fill).
- **Task 5 review** (fresh `reviewer` subagent, 10 findings: 8 actioned, 2 ruled). Actioned: a probe result landing after
      the player turned that port off would have reconnected it with nothing to remove it (`still_wanted`, tested); a first
      run filling `enabled` with every port can deliver one press twice across two ports of one instrument (now the first
      port only — `opened_by_default`, tested); the `Router` tests the plan owed; `midi_given_up` offered RETRY to a player
      who had turned everything off; the clock test slept through a real-time stream, so `on_message_at` takes the arrival
      time; a duplicated doc line; an orphaned comment the deleted picker left behind; `MidiIn::port_name` dead API; and three
      places whose prose still described the `none` row (`persist::SavedState::midi_input` and `docs/guide/src/13-settings.md`,
      `14-midi.md`). Ruled, with the cost: a session that turned every port off opens one again on the next launch, because
      `midi_input: Option<String>` cannot tell "none chosen" from "nothing saved yet" and the format is frozen — cost, one
      untick per launch, and the save-a-list round is where it gets a field; and the probe thread is checked by hand with two
      ports rather than in a test, because constructing a `MidiIn` needs a device (the roster around it is tested). The
      scrolling-list test's rect filter also now rejects a sliver, so an animating scroll-bar handle can no longer be counted
      as a row face.
- [x] **Task 6 — the app routes.** `Midi::new(enabled, tempo_cell, bus)` takes the two cells before the rack moves into the
      app; each opened port's handler is a `Router::new(name, Arc::clone(&tempo))` built inside the probe thread and closed
      over `Arc::clone(&bus)`, and the closure is `move |_, bytes| router.on_message(&bus.load(), bytes)` — `load`, never
      `load_full`. The inline `match event` (`port_handler`) is gone, and with it the last thing that opened a port: one caller
      of `open_named` remains in the workspace. `save`/`session_for_disk` already wrote `midi_input` from the first enabled
      port (task 5), so the Files block's second half needed nothing. `follow_clock` and `SharedState::external_clock` are
      deleted: the only writer of the flag went with the inline handler, nothing in the workspace (code, tests, `acid-core`,
      the guide) ever read it, and a field that always reads `false` is worse than none. `tests/rack.rs` gains
      `a_rack_hands_each_router_the_tempo_cell_it_shares` — green on arrival, as the plan said it would be: it is a check,
      not a red test.
- **Task 6 review** (fresh `reviewer` subagent, 4 findings: 3 actioned, 1 recorded). It found **no code defect in the diff**;
      it verified parity with the deleted handler for clock, CC and notes, that a followed clock still moves the whole rack
      through `sync_transport`, that one `Router` and therefore one note gate is built per connection, and that no reader of
      `external_clock` and no other caller of `follow_clock` existed anywhere. Actioned: a stale comment in `main` still said a
      fresh session opens "every port" (task 5's ruling made it the first); Review Focus 1's live test through the card had no
      home, so it is now in `plans/phase 2/42-the-device-card.md` task 4 step 1 — the card that sets a filter is what plan 42
      builds, and until then the rack-level half in `tests/rack.rs` is the only place it can be pinned.
- **Task 7 — the tempo fader.** (Its carrier and its mirror were both removed by task 8; this line is the record of why it
      existed.) `CcTarget::Tempo` used to write only the *device's* `tempo_bpm`, which `sync_transport` overwrote from the rack
      cell every frame, so factory CC 72 did nothing. Task 7 gave the arm a `tempo_request` carrier the router drained into the
      rack's cell; task 8 removed the need for both by making the device's cell the rack's.
- **Deferred (Minor), deliberate:** the probe thread's closure — `Router` over `bus.load()` — is exercised by hand
      (`--midi-map`, and a note on a port the device's filter does not name produces silence), because constructing a
      `MidiIn` needs a device. What it hands messages to is covered by six `Router` tests and the roster around it by two.
- [x] **Task 7 — the tempo fader.** `SharedState::tempo_request` (`-1.0` when nothing is pending) with `request_tempo` and
      `take_tempo_request` (a positive tempo only, so no real tempo can be mistaken for "nothing"), written by
      `CcTarget::Tempo` **and** the device's own `tempo_bpm`; the router takes it in the same CC iteration and writes the
      rack's cell, so `sync_transport` carries it to every device instead of undoing it. Removal-checked: delete the request
      and three tests fail with `130.0` where `300.0` belongs (the cc unit test, the router test and the rack-level one).
      New tests: the parked-fader-and-request case and the `-1.0`/take-once semantics in `cc.rs`, three in `route.rs` (one
      device, two devices hearing one fader, and a tempo learned onto another controller), and two in `tests/rack.rs` (the
      router + rack cell + fan-out joined the way `main` joins them, and a fan-out *between* two fader messages). **Superseded by
      task 8**, which removed the carrier, the device copy and three of those tests; the record stays because it is why task 8
      exists.
- **Task 7 review** (fresh `reviewer` subagent, 5 findings, all Minor, all actioned). It confirmed the Interfaces block
      exactly, no convention break, no lock or allocation added on the MIDI path, the sentinel safe (`MIN_BPM` is 40, so
      nothing real is ≤ 0), no pending request able to outlive its own message, and that removing the fix fails the new
      tests. Actioned: the Files block now names `tests/rack.rs`; a test that a fan-out between two fader messages leaves the
      tempo where the fader put it; the two-device and learned-controller cases; a blank line.
- [x] **Task 8 — one tempo cell.** `SharedState::tempo_bpm` is an `Arc<AtomicF32>`; `Rack::with_device` adopts the first
      device's cell as the rack's and `Rack::add` builds each new device on it through `Device::on(kind, number,
      SharedState::with_tempo(..))`. `Rack::sync_transport` now fans out `playing` only, `main`'s per-frame call is gone, and
      task 7's carrier (`tempo_request`/`request_tempo`/`take_tempo_request`/`AtomicF32::swap`) and the router's drain are
      deleted. `Rack::from_device` went with it: no caller, and it now wrote a cell it had not created. Removal-checked: make
      `with_device` allocate a cell again and three tests fail — `every_device_in_a_rack_holds_the_racks_tempo_cell`,
      `a_tempo_the_midi_thread_writes_is_heard_without_a_frame` and `a_followed_tempo_reaches_the_engine_at_the_next_block`,
      the last one with `was 130` where the clock's tempo belongs, which is precisely the bug this task retires.
- **Task 8 review** (fresh `reviewer` subagent, 4 findings, all Minor, all actioned). It confirmed the Files/Interfaces blocks
      exactly, no convention break, no allocation or lock added on the audio path, no path that can build a rack with a second
      cell, and that nothing else still needs the per-frame `sync_transport`. Actioned: the review showed that the
      fader-driven tests pass under the *old* design too (the deleted arm wrote the copy as well as the request), so the round
      gained `a_followed_tempo_reaches_the_engine_at_the_next_block` — the clock path, the one that really did leave the engine
      a frame behind — and the plan's Step 2 now says which test is red and why; four comments that still described the deleted
      fan-out; `Rack::from_device` deleted rather than left as an untested constructor that writes someone else's cell.
- **Ruling (task 7): the device's `tempo_bpm` is a mirror, and it is written twice for one message.** The fader writes it
      and the router writes the rack's cell; if `main`'s fan-out ran between those two writes it could put the *old* cell into
      the mirror for one frame, and a fader that moved more than the pickup's 2.5/128 would lose the knob until it crossed
      again. Closing that would mean the pickup reading the rack's cell instead of the device's — a change to what
      `SharedState` is. Cost if wrong: a fast sweep can stall for a frame and needs a re-cross; the cell — the thing the
      engine follows — is always right, and the test above pins that a fan-out does not undo the fader. **Superseded by task 8**,
      which removes the mirror this ruling is about.
- **Ruling (paid in task 5):** the router was written and tested by its filters but had no `on_message` test until task 5
      added six, which is what this ruling asked of whoever took it: task 4 shipped `Router::on_message` with no caller and no
      routing test, and it is exactly the function 40.6 turns into the only message path.
- **Ruling:** transport bytes (`Start`/`Stop`/`Continue`) stay unhandled, as they were before this plan. Wiring them needs the
      rack, which the MIDI thread cannot reach; it belongs with the global-transport round in `plans/future/wishlist.md`.

Rulings made while writing this plan, and their cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| The note gate is per port, not per device | A device with a channel filter that matches *some* of a port's notes still shares the gate's "already down" set, which is the right answer: a duplicate note-on is a duplicate for everyone. |
| `hears` ignores the role | A caller that forgets to check the role on a note event routes notes to a `ControlOnly` device. Mitigated: the router is the only caller and its note arm checks. |
| Transport bytes go to `ControlOnly` devices too | A controller-only device starts and stops with the rack. That is what "controllers only" means — it is about notes, not about the transport. |
| `enabled` keeps a name whose port is gone | The device stays silent instead of quietly listening to the survivors. The card shows the port as missing; a removal is the player's decision. |
| `SavedState::midi_input` keeps holding one name | A rig with three enabled ports remembers one of them across a restart, and a session that turned every port off opens one again the next time — the field cannot tell "none chosen" from "nothing saved yet". Accepted: it is part of the frozen format, and the save-the-rack round replaces it with a list. |
| A fresh session opens the first port the machine reports, not every one (task 5) | A rig with two controllers ticks the second row once. The alternative delivers one press twice when one instrument publishes two ports — USB and DIN, or a DAW forwarding its input — which is the bug this repo has already paid for. |
| An unplugged port keeps its card row, marked `(missing)` (task 5) | The list can carry a name the machine does not report. Without the row, the only way to turn a gone port off is to plug the cable back in. |
| A song load adds its saved port to the set instead of replacing it (task 5) | A song that named one port leaves the player's others enabled. Replacing would drop controllers the file cannot name. |
| `Router::on_message_at` takes the arrival time (task 5) | One public method whose only caller is a test. The alternative is a clock test that sleeps through a real-time stream and flakes on a loaded machine. |
| ~~A tempo CC leaves a request and the router carries it to the rack's cell~~ (task 7) | Superseded by task 8: the fader writes the cell every device already holds, so there is no request to carry. Kept struck through because the ruling explains why task 7 existed. |
| ~~The device's `tempo_bpm` is written twice for one fader message~~ (task 7) | Superseded by task 8, which removes the mirror the second write was feeding. |
| One tempo cell, shared by the rack and every device (task 8) | `SharedState::tempo_bpm` is an `Arc<AtomicF32>`, so any future code that wants a device to follow a *different* tempo cannot have it without splitting the cell — which is the point: tempo is the rack's. A device built on its own (`SharedState::new()`) gets its own cell, so a test or a standalone state still works. |
