# Wishlist — ideas that have not earned a round yet

Nothing here is promised or designed. An idea lands here when it is worth remembering and not worth a plan; it leaves when it
gets a design doc and a numbered plan in `plans/phase N/`. One entry per idea: what it would do, why it is not being done now,
and what in today's code would have to move for it.

The player has since put the next five rounds in an order — the section at the end records it. Ordered is not designed: each of
those entries says what the phase is and what would have to move, and none of them is a commitment to a shape.

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
wherever a decision is owed. The likely second one is a drum machine or a 606, which means the thing that renders a device stops
being an `acid_core::Engine` and becomes a `Box<dyn Voice>`-ish trait object. Note where that lives: not on `device::Device`
(`kind`, `state`, `bank`, `inputs`, `number`, `name`, `scheme`) but in `audio::renderer::AudioRenderer`, one per mixer slot.
Do it only when there is a real second type to build against, not before.

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

## The road ahead, as the player ordered it

The next five rounds, in the player's order. Each entry points at the wishlist entry that already holds the detail and then says
only what that entry does not — restating a plan is the bug this repo has paid for twice.

Where they will live: **2a is a plan in `plans/phase 2/`** (`47-…`), because it is phase 2's transport, and each of 3 to 6 opens
its own `plans/phase N/` directory with a `00-` overview when it earns a plan. Naming a phase here is not opening it.

### Phase 2a — the transport at the top, seek, reverse, transpose

**What it would do.** Play, stop, back and forward in the transport row, for the whole instrument. Reverse per pattern, so one
device can run its line backwards while the others run forwards. A semitone transpose for the whole synth from one control.

**Why not now — and what already exists.** More of this is built than it looks, which is why the entry says where each piece
would ride rather than proposing new seams:

- **Play and stop** exist and are already rack-wide: `Rack::playing` and `Rack::tempo_bpm` (`state.rs`), fanned out by
  `Device::sync_transport`. Moving them to the transport row is the first bullet of *A global transport, with racks under it*.
- **Transpose** is the surprise: `param::tuning_semitones` is exactly ±12 semitones (`param.rs`), and it is applied at the voice
  by `Voice::set_tuning_semitones` in `apply_pitch`, under everything `base_hz` came from — so it transposes sequencer notes and
  MIDI alike. The tuning path is the transpose; what is missing is a *quantised, rack-wide* control on it rather than a knob per
  device.
- **Seek** exists end to end per device: `AudioRenderer::seek` sets a `pending_seek`, which is how a device that joins a
  running rack lands on the bar everybody else is on (`Engine::start_at`, `Sequencer::start_at`). What is missing is a
  **rack-level** seek that reaches every device mid-run, and the two keys.
- **Reverse of any kind** does not exist.

**What would have to move.** For **back and forward**: a rack-level `seek` that fans out like `sync_transport`, plus the two
cells. The transport row is already five cells wide (RUN, STOP, TEMPO −, TEMPO +, VOLUME), which is the *master fader* entry's
problem arriving early. For **reverse per pattern**: a direction on the device's own clock — `last_position` and the sequencer's
wrap both assume one direction — and **a flag in memory, not a property of the pattern**, so it costs no `SAVE_VERSION` bump;
if the player wants reverse stored with a pattern instead, that is a version bump and lands with the save round. Slides and
accents read oddly backwards, so the rule for a slide into a step that has already passed is a design question, not a detail.
For **transpose**: riding the tuning path has a consequence the entry must face — `persist::Knobs.tuning` is a *saved* knob, so
"kept out of the pattern" is not the same as "not saved", and a transposed rehearsal would write itself into the song through
the knobs instead. Decide whether transpose is a knob (saved) or a performance value (not saved, a new field that is deliberately
not persisted). And note the face already has an octave `-`/`+` pair in the performance row, which transposes by **rewriting the
stored pattern** and marking the song dirty (`tests/perform_live.rs` pins that a transposed pattern is worth saving): those keys
either become this control or are retired, and that is a behaviour change to something a player can already press.

### Phase 3 — the drum machine

**What it would do.** A second device kind that plays drums rather than a bass line, in the same rack, on its own ports and
channel, with the same transport.

**Why not now.** See *A second device type* — and note the correction the code makes to the obvious reading: the `Engine` is not
owned by `device::Device` (which holds `kind`, `state`, `bank`, `inputs`, `number`, `name`, `scheme`), it lives in
`audio::renderer::AudioRenderer`, one per mixer slot. So the trait object belongs there, and the device kind stays a catalogue
entry.

**What would have to move.** The pattern type is the hard part. A drum machine's step is not `{ note, octave, velocity, accent,
slide }` — it is a set of voices hit at once — so either the pattern grows a shape both machines can hold, or a device carries
its own pattern type and the clipboard, the bank and the save file learn two. Deciding that before writing the machine is the
whole phase.

### Phase 4 — saving and loading banks, patterns and songs

**What it would do.** Write and read those three things as files, not just the session: a bank to a file and back into any
slot, a single pattern likewise, and a song as today *once* *Save the whole rack* lands — that round is this phase's song work,
since a song that loses every device past the first is not a song to export.

**Why not now.** Songs already save, load and export (`persist::save_song`, `import_song`, `export_song`); a bank and a pattern
have no document of their own, only a place inside one, and the save file's own gap is the entry above.

**What would have to move.** A bank file is a `SavedState`-shaped document with its own `SAVE_VERSION` and its own `sanitize`
pass. The app does not have one version number today — the clipboard has carried its own `VERSION` and magic line
(`clipboard.rs`) since it was written — but the *save* version is one number for every saved document, and a bank is the second
kind. Patterns are already text on the clipboard (`Ctrl+C`/`Ctrl+V`), so a pattern file is that format with a name on it; the
bank is the one that needs a decision about where a loaded bank lands (the current slot, the next free slot, or a slot chosen in
the dialog). The dialogs stay in `main`, the path functions in `persist`.

### Phase 5 — 1/32 and 1/8T patterns

**What it would do.** A pattern whose grid is not sixteenths, chosen per pattern: thirty-seconds, which are finer than a
sixteenth, or eighth-note triplets, which are twelve to the bar and *longer* than a sixteenth, so they sit off its grid rather
than inside it.

**Why not now.** Every pattern is sixteen steps and every clock agrees on the step index. *Pattern length per device* holds the
part of this that is about length; this entry is the part it leaves open, and the answer — "the bar" becomes a property of the
pattern that every consumer asks for — is the blocker it did not name. Concretely: `pattern_swap::swap_now` compares
`position + 1 >= STEPS_PER_PATTERN`, so a twelve-step bar would fire the pattern swap on every step. Correcting that one
function is the price of entry, and `STEPS_PER_PATTERN` is imported by everything that thinks it knows the bar.

**What would have to move.** The clock's step length becomes a function of the pattern's grid (`timing.rs` computes
`samples_per_step` from a fixed sixteenth today), and every consumer of "the bar" asks the pattern instead of knowing the
answer. The panel is sixteen keys with sixteen lamps and sixteen numbers: at twelve the row is short, at thirty-two it wraps or
shrinks — `ui/strip.rs` first, sequencer second. Note that this entry and *Pattern length per device* cannot both be designed
separately: they are the same change, and whichever gets a plan should absorb the other.

### Phase 6 — the sampler

**What it would do.** A third device kind that plays recorded sound: load a sample, pitch it from the keyboard, put it under
the same sequencer, transport and rack.

**Why not now.** It is the same trait object as Phase 3, built a second time, and it is the first thing in the project that has
to read a file the player chooses and hold it in memory while the audio thread reads it.

**What would have to move.** The audio callback allocates nothing and holds no locks (`tests/no_alloc.rs` is the gate), so a
sample cannot be loaded on the audio path: it arrives as an `Arc` swapped in through an `ArcSwap`, decoded off-thread, with a
rule for when a half-loaded sample is allowed to sound — not at all, or the previous one until the new one is whole. Memory is
the other half: a bank of samples is the first thing here that can exhaust a machine, so the count and the length need limits
the way devices do. Decoding is a new dependency in `acid-core`'s clean dependency graph, which is a decision about where the
sampler lives, not a detail of its implementation.
