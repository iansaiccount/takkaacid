# A device list: two acid machines, one transport, one window

**Status:** approved 2026-11-13. **Branch:** `phase-2`. **Plans:** `plans/phase 2/00-phase-2.md`.

## Intent

`takka-acid` is one instrument: one `SharedState`, one `Engine`, one `PatternBank`, one MIDI connection, one panel filling
the window. The player wants to play two at once — two acid lines in the same window, from the same transport, each with its
own pattern bank and its own shuffle, mixed to the one sound card. That is the first step towards a DAW, and the step that
decides what a "device" is; everything after it (arrangement, automation, more device types) hangs off that answer.

The reason to do it now, in this shape, is that the *routing* question has a cheap correct answer and the *storage*
question does not. The player's rig is a keyboard for notes plus controllers for cutoff and resonance. Today a learn binds
one CC to one target globally, and a note is a note on any channel, so with two devices there is no way to say which
machine the keyboard plays and which the knobs drive. Getting that wrong now means rebuilding every learn slot later.

Decisions the player made, which this document designs to rather than reopens:

- Keyboard-vs-controller ambiguity is solved by a **per-device role**: a device is `Keyboard + controllers` or
  `Controllers only`; a controllers-only device can never trigger notes.
- A second device is **its own `Engine`** — own sequencer, own drive, own limiter, own master volume — because that is what
  a DAW does and it is the model that survives a third device.
- **Tempo is global, swing is per device.** One transport, one clock rate, each machine's shuffle its own.
- **Pattern banks are per device.** Each device keeps its own `A1..H8`, its own selection, its own learned pattern fader.
- **The panel shows one device at a time and the strip shows the rest.** Selecting a cell is how a device is displayed;
  the unselected ones keep playing and stay uneditable.
- **MUTE silences the volume, not the transport.** A muted device's steps keep running, so unmuting rejoins in time.
- **The save-file format is not changed this round.** See "What is deliberately not saved".

## Goals

- Any number of devices from one to four, each a 303 clone, in one window.
- Each device has a name of its own, editable on its card, distinct from its neighbours, and the name — not its position — is
  what the strip, the routing rows and the player say out loud.
- One transport: RUN/STOP and tempo are global; every device follows. Swing is per device.
- A strip along the top of the panel: one cell per device, a `+` to add, the selected cell lit.
- Selecting a cell points the panel at that device — its knobs, its pattern, its bank, its step selection.
- Each device has its own input routing: which MIDI ports it listens to, and which channel on them, and a role that can
  make it ignore note events entirely.
- A single sound card: one cpal stream, all devices mixed by the app, with a master limiter on the sum.
- MIDI learn stays one map per device. Which controllers reach which device is decided by the routing filter, not by a
  second map.

## Non-goals

- No arrangement, timeline, song mode, automation, plugins, per-device audio outputs, sends or master effects.
- No second device *type*. The picker exists and offers one type; the catalogue has one entry.
- No key shortcut for renaming and no names in the save file: a device's name lives and dies with the session, like the devices
  beyond the first.
- No drag-to-reorder, no duplicate-a-device, no per-device tempo or per-device transport.
- `--midi-map` keeps its current meaning: what the app makes of a note number, device-independent.
- No change to the demo songs, the song card, or `persist::SAVE_VERSION`.

## Anti-goals

- **Two cpal streams.** It would half this work and be wrong: two streams are two clocks with no shared transport, and the
  first device to claim an exclusive-mode interface locks the other out.
- **One engine, two voices.** Cheaper for mixing, but the player asked for what a DAW does, and two acid lines through one
  drive/limiter pump each other. It also makes "which device's volume knob" ambiguous forever.
- **Layered notes by default.** A new device that listens to everything makes one key press play two machines, and the
  player would spend an hour finding out why. See the routing default below.
- **A per-device learn *and* a per-port learn table.** Two maps over one CC is where "which one owns CC 74" gets
  unanswerable. The routing filter already scopes a controller to a device; a second scope would be a second truth.
- **Silently dropping device 2 on restart** without saying so anywhere.

## Constraints

- **AGENTS.md:** no `_` arm on a catalogue enum — `DeviceKind`, `Role`, `Channel` and the routing enums are written out in
  full. Every order and length is derived from its catalogue. No font glyphs on the face: the strip's play lamp, the `+`
  and the `−` are painter shapes and ASCII text.
- **Two threads, two rules.** The mixer runs in the audio callback: no lock, no allocation in steady state. The panel
  writes atomics; the device list is published to the audio thread by `ArcSwap`.
- **The audio thread never rebuilds an engine mid-block.** Adding or removing a device swaps a whole rendered graph at a
  block boundary; that swap allocates, and is named as the one exception, like the pattern `Arc` retirement already is.
- **The save file is a contract.** `SavedState`'s field names and order are unchanged and `tests/persist.rs` keeps pinning
  the same bytes, so a session written by this build is a session the previous build reads.
- **Panel behaviour is tested through `draw_panel`** with real egui frames, in `tests/*_live.rs`, not through helpers.
- **`egui::Id`s are behaviour.** The strip's cells, the picker's rows and the new keys get names that will not be renamed
  casually; a knob drag is routed by id.
- Both gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`.

## Approaches

**A. One stream, a mixer in the app, an `Engine` per device (picked).** `AudioRenderer` already renders one device's mono
block from one `SharedState`. A new `audio::mixer::Mixer` owns `Vec<AudioRenderer>` plus a master `Limiter`, reconciles that
list against the published slot list once per block, sums the devices with a per-device mute ramp, and hands the result to
the existing `Rig` closure unchanged. Adding a device is a `Vec` push on the UI thread and a rebuild at the next block;
removing one likewise. The transport stays one `Clock` rate because every engine is fed the same global tempo, and each
engine keeps its own swing. Nothing about `Rig`, `Opened` or the cpal callback changes.

**B. One cpal stream per device.** Rejected above: no shared transport, clock drift between streams, and interfaces that
grant exclusive access to the first opener. It also puts the mix bus in the OS, where the app cannot limit it.

**C. One engine, two voices, one master chain.** Rejected: it is not what a DAW does, both lines hit one limiter, and the
`Engine`'s per-device knobs stop being per-device. It would also make a third device a rewrite rather than a push.

**D. Device lists in the panel, mixed by `main`.** Rejected: a second audio path in the UI thread. The mixer belongs where
the callback is.

## Design

### The device and the rack

`acid-app/src/device.rs` — new, and the only place a device's identity lives.

```rust
/// What a device is. One entry today; the picker reads this catalogue, so a second kind is a variant and a `match`.
pub enum DeviceKind { Acid }

impl DeviceKind {
    pub const ALL: [DeviceKind; 1] = [DeviceKind::Acid];
    pub fn name(self) -> &'static str;   // "ACID", the caption on the picker row
}

/// One machine: its own sound, its own patterns, its own inputs.
pub struct Device {
    pub kind: DeviceKind,
    pub state: Arc<SharedState>,   // knobs, pattern, mute, cc map, pickup, position — all per device
    pub bank: PatternBank,         // its own A1..H8
    pub inputs: InputFilter,       // which ports and channel it hears
    /// The device's own number, stable for its life: the next unused one is taken when a device is created and is never
    /// reused or renumbered. It is the seed of the default name and the reason a removal leaves a gap.
    pub number: usize,
    /// What the strip and the cards call it. Editable, distinct from its neighbours, `ACID 1` until a hand says otherwise.
    pub name: String,
}
```

`Rack` replaces the loose fields `main` holds today:

```rust
pub struct Rack {
    pub devices: Vec<Device>,
    pub selected: usize,   // always < devices.len()
}

/// The most devices a rack may hold. Four cells of ~190 px fill the strip, and four acid lines is already a lot of voice.
///
/// A hard cap rather than a growing list: the strip has to show every device at once, and a rack that outgrows its own
/// strip is a layout problem this round does not want. `+` greys out at the cap instead of failing silently.
pub const MAX_DEVICES: usize = 4;
```

`Rack` is `main`'s, on the UI thread. Its methods are the only writers of the list: `select`, `add(kind)`, `remove(i)`
(refuses the last one, and refuses past `MAX_DEVICES`), and `publish()` which hands the audio thread a fresh `BusState` — the
slot list *and* the routing table derived from it, so the mixer and the MIDI handlers read one published truth instead of
two that could disagree.

`SharedState` gains exactly one field, `mute: AtomicBool`. Everything else stays, and because it is per device, `cc_map` and
`pickup` become per device for free — that is what makes two controllers able to drive two devices' cutoff with CC 74.

### The transport

Global: `playing` and `tempo_bpm` are **the rack's**, and the rack is the source of truth. A press of RUN sets `Rack::playing`
and fans one write out to every device's `playing` atomic in the same frame, so every engine's `start()` lands inside one
callback and the downbeats coincide. Tempo is the same: `Rack::tempo_bpm` is written through to every device's `tempo_bpm`,
so a knob sweep or an incoming MIDI clock moves all devices at once. A device added while the transport runs is given the
rack's current `playing` and `tempo_bpm` at the moment it joins — that is how it starts playing rather than sitting silent.

The panel's RUN lamp and tempo readout draw from the *selected* device's own atomic. The fan-out is what guarantees that
mirror and the rack cannot disagree; nothing reads the rack's copy for display.

`midi::tempo::follow_clock` currently writes one `SharedState`. It writes the rack's tempo now, and the fan-out carries it,
so an external clock drives every device together.

Per device: `swing`, and the engine's position. Swing moving per device is *why* the design cannot use one shared clock:
device A's 16ths land on the straight grid and device B's on a shuffled one, each from its own `Clock`, while both agree on
which step they are on.

A device added while the transport runs joins in phase, not at step 0. That needs one small addition to the core:

```rust
// acid-core/src/engine.rs
pub fn start_at(&mut self, step: usize);   // start() == start_at(0)
// acid-core/src/sequencer.rs
pub fn start_at(&mut self, step: usize);   // start() == start_at(0); seeks the clock and sets position
```

Its cost if wrong: a device added mid-bar plays a step out of place for one bar and then locks. Cheap to notice, cheap to
fix by deleting the seek.

### The mixer

`acid-app/src/audio/mixer.rs` — new.

```rust
pub struct Mixer {
    sample_rate: f32,
    slots: Vec<Slot>,              // one per device, in rack order
    master: Limiter,               // the same soft ceiling the engine already uses
    scratch: Vec<f32>,             // sized once at open
}
struct Slot {
    state: Arc<SharedState>,
    renderer: AudioRenderer,       // owns the Engine
    mute: param::Smooth,           // a ramp, or muting clicks
}
```

Once per block, before rendering: reconcile. Compare the published `Arc<[Arc<SharedState>]>` with `slots` by length and
`Arc::ptr_eq` per entry; on a mismatch, build the new list — new `AudioRenderer` for each added device, drop the renderers
of removed ones, and order the survivors by the published list so the strip and the mix agree. Then render each slot into
`scratch`, multiply by its mute ramp, accumulate, and run the sum through `master`.

The master limiter is new and necessary: the per-device limiters each hold their own device, so two summing devices can
reach twice the ceiling. Master volume stays each device's VOLUME knob (that is the mix) — there is no second master fader
this round, so the ceiling is the limiter's alone.

MUTE is read from the device's own atomic once per block, *not* from `BusState`, so muting a device publishes nothing and
costs one relaxed load. A slot created for a device that is already muted starts its ramp **on** the muted value rather than
ramping down from unity, or adding a device while others are muted would click.

`BusState` is published only when the rack changes — an add, a removal, a reselect, a route edit — never per frame. A
one-device app therefore publishes once at startup and never again.

### Routing

`acid-app/src/midi/route.rs` — new, and the answer to the player's actual question.

```rust
/// What a device hears.
pub struct InputFilter {
    /// Which of the opened ports may address it. Empty means it hears nothing.
    ports: Ports,             // All | Named(Vec<String>)
    channel: Channel,         // All | One(u8)   // 0..15, shown as 1..16
    role: Role,               // PlayAndControl | ControlOnly
}
```

For every message that arrives on a port, each device whose filter matches gets it: `ControlChange` when the role allows
either, note events and computer-keyboard-style triggers **only** when the role is `PlayAndControl`. A device created by
`+` starts with `Ports::Named(vec![])` — hears nothing — so adding a device never silently layers it under the keyboard.
Device 1, the factory device, starts as `All / All / PlayAndControl`, which is exactly today's behaviour, so a session that
never opens the picker behaves as it always did.

Every device's routing lives on that device's **own card**, not as a row in the settings card. `F3` — or the caret on the
selected cell — opens `DEVICE <name>`: a name field, the role switch, the port checkboxes, a channel stepper, the MUTE key,
and the remove (asked twice). The settings card stays about the machine — tooltips, autosave, which ports exist, which sound
card — and never grows a row per device. One card, one subject, and the settings card does not have to scroll for a fourth
device.

**Why the filter and not a second learn map.** Learn stays one map per device (`SharedState::cc_map`, as today). The filter
decides *whether a controller reaches a device at all*; the map decides what it drives once it does. That is one question
per function: "which device is this controller talking to" is the filter, "which knob is this CC" is the map. Anything the
player wants from a per-port learn table is reachable by routing the port, and nothing needs a second mapping that could
disagree with the first.

**Where the filter lives, and how the MIDI thread reaches it.** The filter is `Device::inputs`, on the UI thread, beside the
bank it belongs to — that is where a device's identity is edited. `Rack::publish` derives the routing table from it into the
same `BusState` the mixer reads: one entry per device, `(index, ports, channel, role)`, already resolved. Each open port's
handler holds that `Arc<ArcSwap<BusState>>` and loads it once per message — a refcount bump, no lock, no allocation — then
asks `route::reaches(...)` which devices a message is addressed to. Teaching, selecting and editing a filter never touch a
handler; they change what the bus publishes, and the handlers see it on the next message.

The port dimension is per *connection*, not per message: `main` opens one `midi::ports::MidiIn` per enabled port (today it
opens one), and the handler closes over the port's name so both the trace and the filter know where a message came from.
Multiple enabled ports is what the player asked for, and it is a `Vec` in the settings card, not a new mechanism.

**Ports and the supervisor.** The supervisor's question was "is there a MIDI input at all", and it stays that: one
supervisor for the app, deciding when to re-enumerate. Enumerating fills the settings card's port list; one `MidiIn` is
opened per *enabled* port, and an enabled port that will not open is a line in the card, not a retry loop of its own. The
first device's factory filter is "all ports", which on a machine with one port is exactly today's single connection — so a
rig that works today keeps working, and its supervisor keeps its current behaviour.

### The panel

`theme::DESIGN_SIZE` grows from 980 × 640 to 980 × 670 and `layout()` gains a strip band above the top strip. Each existing
band keeps its pixel height and moves down exactly 30 px — 46, 114, 342, 416, 532 and 600, the same heights as 16, 84, 312,
386, 502 and 570 — so the bottom margin stays the 26 px it always was and every layout test's *arithmetic* still holds. Only
its window size changes (`WINDOW_SIZE` moves with `DESIGN_SIZE`).

```
┌──────────────────────────────────────────────────────────────────────────┐
│ ▸ACID 1 ● M      ACID 2  ○ M      ACID 3  ○ M                        +  │   strip, 30 px
├──────────────────────────────────────────────────────────────────────────┤
│  ◯ vol   ACID (?) (gear) ●        SONG Acid 1         A1   ▮▮▮   137    │
│  …the six bands as they are today…                                       │
└──────────────────────────────────────────────────────────────────────────┘
```

- A cell is the device's name (`ui::text::fit` to the cell), a play lamp lit while that device's transport runs, a `MUTE`
  key, and a border in `theme::LED_ACCENT` while it is selected. A strip of one cell still draws — it is the handle for
  `+`, and hiding it would make the feature undiscoverable.
- **The journey:** click a cell, that device is on the panel. Its knobs, its pattern, its bank, its selected step. This is
  the whole of "minimise or display": a device is displayed by being selected, and there is no second way to fold one,
  because nothing is lost by not displaying it. `Ctrl+1..4` selects too, so a two-device rig is playable without the mouse.
- The `+` opens the device-type picker — a card in the shape of the settings card, with one row per `DeviceKind::ALL`
  entry and a footnote: *device names and extra devices are not saved yet*. The caret on the selected cell opens that
  device's card, where the name is typed, the routing is set and the removal is asked for twice, the way CLEAR is. The last
  device cannot be removed, and `+` greys out at `MAX_DEVICES`.
- **Names are the identity, the number is only the seed.** A new device takes the next unused number and is called
  `ACID <number>`. Typing a name replaces it; the number stays, so a removal leaves a gap in the numbers and no device is
  ever silently renumbered. Names must be distinct — a rename that collides lands as `BASS (2)`, the same `free_song_name`
  rule the song library already uses — because the input filters and the strip name devices by them, and two `BASS` cells
  would make "which one is muted" unanswerable. A blank name falls back to `ACID <number>`.

### Panel state

Most of `PanelState` is a *mirror*: `knobs`, `volume`, `pattern`, `tempo`, `playing`, `position` are all re-read from
`SharedState` at the top of `draw_panel` every frame. Those need no per-device storage — switching device re-reads them.

The editor state is not a mirror, and is what must follow a device:

```rust
/// What the panel remembers about one device while another is displayed.
pub struct PanelEdit {
    pattern_id: PatternId,
    selected: usize,
    step_mode: bool,
    placed: Vec<egui::Key>,
    last_pitch: Pitch,
    last_octave: i8,
    density: f32,
    scale: Scale,
    scale_knob: f32,
}

impl PanelState {
    pub fn save_edit(&mut self) -> PanelEdit;      // out of the flat fields, into the slot
    pub fn load_edit(&mut self, edit: PanelEdit);  // into the flat fields, from the slot
}
```

`PanelState` keeps its flat fields as "the selected device's editor", so no row module and no test changes shape, and
`PanelState` gains `edit_slots: Vec<PanelEdit>` and `strip: Strip`. A switch is `save_edit` for the outgoing index,
`load_edit` for the incoming, and a cancelled learn (`learn::cancel`) — a learn in progress belongs to the device it was
started on. One test pins that switching away and back keeps the step you were on.

The strip's own mirror, filled by `main` each frame like `state.pattern` is:

```rust
pub struct Strip { pub devices: Vec<StripCell> }   // name, kind, running, muted, selected
```

`PanelState` keeps `edit_slots` and `strip` in step with the rack the only way a painter can — by length, at the top of
`draw_panel`: truncate `edit_slots` to `strip.devices.len()` and clamp `strip.selected` into range. The panel may see a
removal one frame after `main` made it, so deriving the length rather than trusting it is what keeps that frame from
indexing out of bounds. It costs two comparisons.

Panel → `main` requests ride the existing queue shape: `PanelState::device_request: Option<device::Request>` with
`Select(usize)`, `SetMute(usize, bool)`, `Add(DeviceKind)`, `Rename(usize, String)`, `SetRole(usize, Role)`, `SetPorts(usize,
Ports)`, `SetChannel(usize, Channel)`, `AskRemove(usize)`, `ConfirmRemove(usize)`, and `Open(usize)` / `Close` for the two
cards. `Open` is a request rather than panel state for the same reason the settings card's lists are: what a card shows is a
reading of the machine, and `main` is the only thing allowed to take that reading.

### `main`

`TakkaApp` holds `rack: Rack` instead of `state`/`bank`, and `midi: Midi` holds `Vec<MidiIn>` instead of one. Every place
that reads `self.state` becomes `self.rack.selected()` or a loop over `self.rack.devices` — including the settings card's
`Live` mirror, which grows an input row per device. `save()`/`session_for_disk()` write the selected device, as below.

Two pieces of `main` become per device rather than one. `pending_pattern: Option<usize>` becomes a slot per device, because
two devices can each have a pattern waiting for the bar; the swap check runs over all of them in the same loop, and
`last_position` stays single because the bar is the same bar for every device. `midi_given_up` and the supervisor stay
single for the same reason: there is one machine's worth of ports, not one per device.

### What is deliberately not saved

`SavedState` is untouched: same fields, same order, `SAVE_VERSION` stays 9, and `tests/persist.rs` keeps pinning the same
bytes. A session saves **the first device** — its bank, knobs, tempo, swing, inputs and learn map — exactly as it does
today. Devices beyond the first live in memory and are gone on restart.

That is the player's ruling, and its cost is stated where the player will meet it: the picker card's footnote. **Device names
are not saved either**, which is the more visible half — a second device that vanishes on restart is at least a device the
player only just made, where a name they typed is gone from the one machine they have been using all along. The footnote says
both. The next round adds `devices: Vec<SavedDevice>` (name, kind, routes, bank, knobs) behind `#[serde(default)]` with a
`SAVE_VERSION` bump, keeps the flat fields as device 1's mirror so a v9 file still loads as a one-device song, and deletes the
footnote.

## Rulings, and what each costs if it is wrong

| Ruling | Cost if wrong |
|---|---|
| One cpal stream with an app-side mixer | A per-device output (stems) would need the stream rebuilt per device later; but the mix bus is where a DAW puts it anyway. |
| Tempo global, swing per device | A device that wanted its own tempo is a redesign of the transport. Accepted: the player asked for one transport. |
| Routes rather than a second learn map | "One port, two devices, different CC mappings" is already possible; "one port, one CC, two devices" is not without teaching each. Accepted: that is a learn, not a routing. |
| A new device hears nothing until routed | A player who adds a device and presses a key hears one machine and may think the add failed. Mitigated: the strip shows the new cell, and the card says `IN: —`. |
| Master limiter on the mix, no master fader | Two loud devices squash each other; there is no single knob to pull down. Accepted: each device's VOLUME is the mix, and the ceiling is not a control yet. |
| Editor state saved and restored per device, mirrors re-read | A field that is both (nothing today) would need a decision. Accepted: the split is visible in the struct. |
| Panel design size grows to 670 | Every screenshot in `docs/guide` is stale. Accepted: the guide's check only proves images exist. |
| Names are the identity, numbers only seed them | A player who renames everything loses the numbering they used to reason about; gaps in the numbers look like a bug until the cell's caption explains them. Accepted: a stable identity is worth more than a tidy one. |
| Four devices per rack, not per app | "Multiple racks later" is a second dimension this round does not model. Every rack-level field (`selected`, `playing`, `tempo_bpm`, `publish`) is written as if a rack were the whole world, so a second rack would be a list of `Rack`s and not a redesign. |
| Routing on the device card, not the settings card | One more card to learn. Accepted: the settings card would have to scroll past four devices to reach tooltips, and "which device is this?" is not a machine setting. |

## Tests that make this safe

- `acid-core/tests/seek.rs`: `start_at(n)` puts the sequencer on step `n`, and `start()` is `start_at(0)`.
- `audio/mixer.rs` tests: two devices sum; a muted device contributes exactly nothing while its position keeps advancing; a
  third device added between blocks is heard from the next block and the first two are not disturbed; the master limiter
  holds the ceiling on a two-device sum; the reconcile is a no-op when the published list is unchanged (`Arc::ptr_eq`).
- `midi/route.rs` tests: channel filter, port filter, `ControlOnly` swallowing notes but not CCs, an empty port list
  hearing nothing, the factory device hearing everything, and a `ControlChange` reaching a device-scope learn only through
  its own map.
- `device.rs` tests: a new device takes the next unused number and is named `ACID <n>`; a removal frees no number; a rename
  that collides lands as `BASS (2)`; a blank rename falls back to `ACID <number>`; a rename past the cell's budget is
  truncated by `ui::text::fit` and not by the store.
- `tests/devices_live.rs`, through `draw_panel` and real frames: one cell per device; clicking a cell selects it; the
  selected cell's border moves; `Ctrl+2` selects the second; `M` sets that device's `mute` atomic and only that one; the
  `+` opens the picker and its row adds a device; `−` on the last device is refused; the picker card says the extra
  devices are not saved; switching away and back keeps the selected step.
- `tests/persist.rs`: unchanged and still green — the file this build writes is the file the last build wrote.
- `tests/device_card_live.rs`: `F3` and the caret both open the selected device's card; typing a name and confirming lands
  it on that device and on the strip's cell; a colliding name lands suffixed; the channel stepper and a port checkbox write
  the device's filter; the card's remove asks twice; `tests/settings_live.rs` proves the settings card mentions devices only
  by way of the port list, and has no row per device.
- Mutation check on the reconcile: deleting the `Arc::ptr_eq` short-circuit must fail a test (a per-block rebuild would
  reset every engine's transport), and giving the mute ramp a target of `1.0` for a muted slot must fail another.

## What is left to the builder

- The documents that follow: `ui/help.rs`'s key list gains `F3` and the caret, `README.md` says where a device's name and
  routing live and that neither is saved yet, and `docs/guide`'s screenshots are stale because the panel grew 30 px.
- The strip's exact cell widths, the `+`/`−` glyph geometry (painter shapes, not characters), and the picker card's rows.
- Whether the master limiter is the same `Limiter` type at the same ceiling, or a softer one tuned for a sum — and where the
  master ceiling itself should live, given there is no master fader to place it beside.
- How `main` names a port inside its closure so a message and its trace agree on where it came from, and that a filter entry
  for a port that has gone away is *kept* rather than resolved away — unplugging a controller must not silently re-route its
  device to "all ports".
