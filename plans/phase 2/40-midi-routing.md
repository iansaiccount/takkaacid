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

---

## Progress

- [ ] Task 1 — a message knows its channel
- [ ] Task 2 — the filter
- [ ] Task 3 — the filter reaches the bus
- [ ] Task 4 — the router
- [ ] Task 5 — several ports at once
- [ ] Task 6 — the app routes

Rulings made while writing this plan, and their cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| The note gate is per port, not per device | A device with a channel filter that matches *some* of a port's notes still shares the gate's "already down" set, which is the right answer: a duplicate note-on is a duplicate for everyone. |
| `hears` ignores the role | A caller that forgets to check the role on a note event routes notes to a `ControlOnly` device. Mitigated: the router is the only caller and its note arm checks. |
| Transport bytes go to `ControlOnly` devices too | A controller-only device starts and stops with the rack. That is what "controllers only" means — it is about notes, not about the transport. |
| `enabled` keeps a name whose port is gone | The device stays silent instead of quietly listening to the survivors. The card shows the port as missing; a removal is the player's decision. |
| `SavedState::midi_input` keeps holding one name | A rig with three enabled ports remembers one of them across a restart. Accepted: the field is part of the frozen format, and the wishlist's save-the-rack round replaces it with a list. |
