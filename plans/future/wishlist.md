# Wishlist — ideas that have not earned a round yet

Nothing here is promised, ordered or designed. An idea lands here when it is worth remembering and not worth a plan; it
leaves when it gets a design doc and a numbered plan in `plans/phase N/`. One entry per idea: what it would do, why it is
not being done now, and what in today's code would have to move for it.

## Decided, waiting for its round

### Save the whole rack

Agreed during phase 2 and deliberately deferred: phase 2 saves the first device only, so **device names and every device past
the first are lost on restart** (the picker card says so).

The round that lands it: `SavedState` gains `#[serde(default)] pub devices: Vec<SavedDevice>` where `SavedDevice` holds
`{ name, kind, inputs, bank, knobs }`; the flat v9 fields (`bank`, `knobs`, `bpm`, `swing`, `midi_input`, `cc_map`) stay as
device 1's mirror so a file written by a v9 build still loads as a one-device song and a file written by this build still
opens in a v9 build with device 1 intact. `SAVE_VERSION` goes to 10, `tests/persist.rs` gains the new pin next to the old
one, and the picker card's footnote goes away. `sanitize` learns to repair a `devices` list the way it repairs a bank:
one device minimum, at most `MAX_DEVICES`, names made distinct.

## Transport and racks

### A global transport, with racks under it

Phase 2 keeps `playing` and `tempo_bpm` on the rack, because there is one rack. The next dimension is several racks, and the
player's shape for it is: **scoop the transport out into a global** — start, stop, tempo, MIDI clock — with each rack keeping
a *sub-transport* of its own, which is where looping would live (a rack looping bars 5–12 while the song runs on).

What would have to move:

- Phase 2 already writes the rack's transport as two atomics (`Rack::playing`, `Rack::tempo_bpm`) plus `sync_transport`,
  which fans them out to the devices. Hoisting them means a `Transport` struct owning those atomics, racks holding a
  reference to it, and `sync_transport` becoming the transport's job for every rack it owns. The fan-out is the seam, which
  is why phase 2 does not scatter `playing` writes through `main`.
- A rack sub-transport (loop in/out, its own play state) means the sequencer learns a loop range. `Clock::restart` and
  `Sequencer::start_at` are the two functions that already exist for seeking; a loop is a seek at the out point.
- The panel gets a second strip: racks across the top, devices inside the selected one. That is the strip in
  `ui/strip.rs` nesting inside itself one level, which is why the strip is one module and not painted inline in `draw_panel`.

### Solo

Mute exists; solo is the other half of a mix, and it is not simply "mute everything else" — it needs a remembered mute state
per device so that un-soloing restores what was muted by hand, not by solo. Cheap once `SharedState::mute` exists: a
`Rack::solo: Option<usize>` and a derived mute in `sync_transport`-adjacent code.

### A master fader

Phase 2 mixes with no master control: each device's VOLUME is the mix, and a `Limiter` on the sum is the only ceiling. A
master fader is one `AtomicF32` on the rack and one more `param::Smooth` in `Mixer::render_mono`, plus a cell in the strip
or the top strip. It waits for a second opinion on where it sits, because the top strip is already full.

## Machine

### A second device type

`device::DeviceKind` is a one-entry catalogue with an exhaustive `match`, on purpose: adding a variant must fail to compile
wherever a decision is owed. The likely second one is a drum machine or a 606, which means `Device` stops owning an
`acid_core::Engine` directly and starts owning a `Box<dyn Voice>`-ish trait object. Do that only when there is a real second
type to build against, not before.

### Per-device audio outs (stems)

One cpal stream and an app-side mix is the phase 2 design, and stems would ride on it: `Mixer` already renders each device
into its own block before summing, so a multi-channel device config plus one channel pair per device is the work. It would
need `audio::Opened` to stop duplicating mono to every channel.

### Pattern length per device

Every pattern is sixteen steps and every clock agrees on the step index. A device with an eight-step pattern would break
"the bar is the same bar for every device", which phase 2 relies on for the pattern swap and `last_position`. It is a
transport change, not a sequencer change.

## Panel

### Drag to reorder devices

Rejected in phase 2 because the rack's selection, the strip's cells and the input filters all key on position and a drag is a
third way to mutate the list. Do it after the save format lands, so a reorder is written once rather than twice.

### Duplicate a device

"Add a device like this one" — the same `Device::new` plus a copy of the source device's bank, knobs and routes. Small, and
only worth it once somebody has built a patch they want a second of.
