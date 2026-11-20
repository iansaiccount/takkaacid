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
`{ name, kind, inputs, bank, knobs }`; the flat fields (`bank`, `knobs`, `bpm`, `swing`, `midi_input`, `cc_map`, `kit`) stay as
device 1's mirror so a file written by a build before this round still loads as a one-device song and a file written by this
build still opens in an older build with device 1 intact. **`SAVE_VERSION` goes to 12** — the drum machine's kit took 11, so
the rack round is 12 — and `tests/persist.rs` gains the new pin next to the old
one, and the picker card's footnote goes away. `sanitize` learns to repair a `devices` list the way it repairs a bank:
one device minimum, at most `MAX_DEVICES`, names made distinct.

## The save file

### A file from a newer build is refused, not relabelled

**What it would do.** `persist::open`, `load_song` and `import_song` compare a document's `version` with `SAVE_VERSION` and
refuse one that is newer, naming the version it found — instead of today's behaviour, where `upgraded` stamps its own version
over whatever it read and the next save quietly rewrites a newer document as an older one. Phase 4's spec names this exact
loss as a risk: "a version-9 build reading a version-10 file loses the drum steps silently, which `SAVE_VERSION` is there to
prevent." The number does not prevent it today; nothing reads it before `upgraded` overwrites them.

**Why not now.** Phase 4's task 2 bumps the version to 10 and pins it in `tests/persist.rs`, which is what a bump has to do.
Teaching the load path to refuse is a behaviour change to the one path every file goes through, and it needs a decision the
bump does not: what the app opens instead of the file, and what it says about it.

**What would have to move.** `persist::load` already returns a `Result`, so the refusal is a new error variant naming the
version and the file; the decision is in `open`, whose rule is "a wrong save file means start again, so the app always opens".
A newer file needs the second face of that rule — a first run, and a `PanelState::notice` on the status line saying the file was
newer than this build. `import_song` and `load_song` follow it. The comparison belongs in `persist`; the notice and its live test
belong with the panel.

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

### A third device type

`device::DeviceKind` is a two-entry catalogue (`ACID`, `DRUMS`) with an exhaustive `match`, on purpose: adding a variant
fails to compile wherever a decision is owed. The drum machine was the first second type, and it landed as
`audio::renderer::Machine` — an enum inside `AudioRenderer`, one arm per kind — rather than as the `Box<dyn Voice>` trait
object this entry used to imagine: two kinds of machine are an enum with every arm written out, and a trait object earns its
place at the third, when the arms stop fitting on a screen. When that third kind arrives, the trait is the shape to reach
for, and the enum stays as the front door that says which one. Note where it lives: not on `device::Device` (`state`,
`bank`, `inputs`, `number`, `name`, `scheme`) but in `audio::renderer::AudioRenderer`, one per mixer slot, built from the
`kind` the device's `SharedState` carries.

### A rate change must not undo the player's mutes

Re-decoding every drum device's kit at a new sample rate (`main`'s rate-change arm) goes through `load_kit_now`, which is a
**load** and calls `SharedState::adopt_kit` — so the kit's own `mute` flags become the mask again, and the player's
shift-clicked mutes come back to life when they switch audio device. That call is right for a kit the player picked and
wrong for a kit being re-decoded; phase 4's task 8 made it audible, because before that the mask reached no machine.

What would have to move: `load_kit_now` grows the one thing it is missing — whether this is a load or a re-decode — or the
rate-change arm publishes with `set_kit` (the edit path, which keeps the mask) and leaves the first load on `adopt_kit`. It
is `main`'s, which has no test, so paying it means the rate-change arm leaving `main` for a function a test can call.

### A synthesized 808 kick, rather than a recording of one

**What it would do.** Give the `808` kit a kick built the way the machine built it: a pitch-swept sine (about 120 Hz down to
50 Hz in 40 ms) with a short click on top, peak-normalised and faded like every other file. It keeps an 808's pitch *and* its
attack, which none of the kicks auditioned from Sonic Pi's set has in both at once (VCSL's bass drums were not auditioned).

**Why not now.** The `808`'s kick was a 40 Hz sine nobody could hear (plan 03, `plans/phase 4/03-the-kick-and-the-voice-keys.md`);
`bd_fat` fixed the report with a recording, which is the smaller change and keeps `CREDITS.md`'s claim true — every file in
`kits/` is a recording from Sonic Pi's set or VCSL, and the page says there are two collections. A synthesized file is a third
provenance kind.

**What would have to move.** `refresh.py` grows one branch that writes a voice from code instead of fetching it, and the credits
generator learns to print "synthesized by this script" for that row with the sweep's numbers beside it.
`every_factory_kick_is_audible_above_the_sub_bass` in `tests/kits.rs` already holds whatever lands.

### The drum machine's gain, and the snap it cannot take

`Engine::set_volume` jumps its smoother when nothing is playing and the drum machine's gain always ramps
(`audio::renderer::apply_volume`), so a VOLUME moved on a stopped drum device fades the next `HIT` in over its 10 ms where a
303 would start at level.

What would have to move: `Drums` needs to answer the same question the engine answers — `Engine::is_playing_something()` is only
half of it, and the condition `set_volume` actually snaps on is `!playing && !voice.is_active()`, so the machine needs both a
"the transport is not running" and a "no voice is sounding" answer over its sixteen voices and its sequencer — and then
`apply_volume` takes them and snaps, exactly as `Engine::set_volume` does. It is deferred because it is one more predicate with
no consumer until something else needs it, and because the fade it costs is 10 ms of a hit that was just pressed. The test that
would see it does not exist yet: it needs the transport **stopped** (which is when the snap fires, and which
`the_volume_knob_sets_a_drum_devices_level` is not — its rack is playing), a `HIT` requested in the block the knob moves, and the
hit's opening level compared against its settled one.

### A drum voice takes no controller, so `LEARN` has half a face on one

`LEARN` is on every face, `VOLUME` answers it on both, and a drum machine's seven dials, filter key and `HIT` do not: there is
no `midi::cc::CcTarget` for a drum voice this phase (the spec's non-goal), so pressing `LEARN` on a drum face and clicking a
dial picks nothing. The dials now stand still while it is choosing — a drag used to turn the voice and, on a factory kit, copy
the whole kit under a free name — but the key still reads as if the face could be taught.

**What would have to move.** Either a `CcTarget` per voice (a filled extension of `midi::cc`, a mapping from `Drum` to a
controller number, and the renderer reading sixteen more CCs — the round that also gives the machine MIDI pads), or hiding
`LEARN` on a drum face the way the 303's rows are hidden, which is a face change and therefore the player's call. Until then
the page says which of the two it is (page 22: "`LEARN` has nothing to teach here").

### The retired `Arc<Kit>` is freed on the audio thread

`audio::renderer`'s module doc names it: when a kit is swapped, the mixer's copy of the old `Arc<Kit>` drops its last
reference during the render call, and `tests/no_alloc.rs` counts allocations, so it cannot see a **free**. It happens once per
published kit, which a dial drag makes once per value — a small, bounded cost on the audio thread that is deliberate and
written down rather than hidden.

What would have to move: publish on pointer release rather than per frame (a `response.drag_stopped()` arm in
`ui::kits::edit`), or hand the retired `Arc` back to the UI thread through a slot the callback only ever pushes to and never
frees. Either is a change to `ui::kits::edit`, which is the one seam a kit edit goes through.


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

Where they will live: **phase 3 opens `plans/phase 3/`** with a `00-` overview and its first plan, and each of 4 to 7 does the
same when it earns one. Naming a phase here is not opening it.

### Phase 3 — the tape transport, and the row it frees

**What it would do.** A conventional tape transport for the whole instrument — play, pause, stop, back, forward, record — with
the tempo's `+` and `−` beside it, and the song's load and save icons where the session name is. The transport leaves the synth
altogether: the row under the panel loses `RUN`, `STOP`, `TEMPO −`, `TEMPO +` and keeps `VOLUME`, and the four cells it frees
become `REVERSE`, `TBC`, `TRANSPOSE −`, `TRANSPOSE +` (one semitone at a time). Reverse per pattern, so one device can run its
line backwards while the others run forwards; transpose the device on the panel, as plan 09 later settled it. `BACK`, `FORWARD`
and `RECORD`
are drawn and reachable by keyboard now, and recede as not-yet-wired; `LOAD` and `SAVE` wait for the song document, because the
room they would reserve is the status line's (`plans/phase 3/01-the-tape-transport.md`).

**What is asked for and not designed yet.** Whether the tape sits in the top band (where the tempo readout is) or in the row it
empties; what a key whose feature does not exist does when clicked (a notice, or nothing at all); which keys `BACK`, `FORWARD`
and `RECORD` take, given that the arrows already walk the step selection and space already toggles the transport; whether
`REVERSE` and `TRANSPOSE` are per device (they are, today) with a rack-wide transpose as well. Screenshots showing the transport
row change with the row.

**A rename this phase carries.** The panel's `SONGS` browser lists *pattern banks* — the demos and the player's own — not songs.
It becomes `BANKS`; a song stays the saved session, and the day it holds every machine it will be the document the load and save
icons read. The save-bank work belongs to Phase 5 (renamed from 4), which is where bank files land.

**What already exists.** More of this is built than it looks, which is why the entry says where each piece would ride rather
than proposing new seams:

- **Play and stop** are the transport already, and already rack-wide: `Rack::playing` and `Rack::tempo_bpm` (`state.rs`), fanned
  out by `Device::sync_transport`. `RUN`/`STOP` are two of the five cells in the bottom row (`transport::TRANSPORT_CELLS`), and
  `space` toggles them.
- **Transpose** is the surprise: `param::tuning_semitones` is exactly ±12 semitones (`param.rs`), applied at the voice by
  `Voice::set_tuning_semitones` in `apply_pitch`, under everything `base_hz` came from — so it transposes sequencer notes and
  MIDI alike. The tuning path is the transpose; what is missing is a *quantised, rack-wide* control on it rather than a knob per
  device.
- **Seek** exists per device: `AudioRenderer::seek` sets a `pending_seek`, which is how a device that joins a running rack lands
  on the bar everybody else is on (`Engine::start_at`, `Sequencer::start_at`). What is missing is a **rack-level** seek that
  reaches every device mid-run.
- **Reverse of any kind** does not exist.
- **Two transpose controls already collide.** The performance row's octave `-`/`+` transpose by *rewriting the stored pattern*
  and mark the song dirty (`tests/perform_live.rs` pins that). A semitone transpose that is a performance value must either
  retire those keys or say which of the two owns "transpose".

**What it costs, per piece.** For **back and forward**: a rack-level `seek` that fans out like `sync_transport`, and the keys —
the arrows already walk the step selection, so these need their own. For **reverse per pattern**: a direction on the device's
own clock (`last_position` and the sequencer's wrap both assume one direction), and **a flag in memory, not a property of the
pattern**, so it costs no `SAVE_VERSION` bump; if the player wants reverse stored with a pattern, that is a version bump and
lands with the save round. Slides and accents read oddly backwards, so a slide into a step that has already passed is a design
question, not a detail. For **transpose**: riding the tuning path has a consequence the entry must face —
`persist::Knobs.tuning` is a *saved* knob, so "kept out of the pattern" is not "not saved", and a rehearsal would write itself
into the song through the knobs instead. For **pause**: play and stop have no third state today (`Rack::playing` is a bool), so
pause is a new state and every reader of `is_playing` has to decide what it means. For **record**: nothing exists to record into;
the key is drawn and refuses until the song document does.

### Phase 4 — the drum machine

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

### Phase 5 — saving and loading banks, patterns and songs

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

### Phase 6 — 1/32 and 1/8T patterns

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

### Phase 7 — the sampler

**What it would do.** A third device kind that plays recorded sound: load a sample, pitch it from the keyboard, put it under
the same sequencer, transport and rack.

**Why not now.** It is the same trait object as Phase 4, built a second time, and it is the first thing in the project that has
to read a file the player chooses and hold it in memory while the audio thread reads it.

**What would have to move.** The audio callback allocates nothing and holds no locks (`tests/no_alloc.rs` is the gate), so a
sample cannot be loaded on the audio path: it arrives as an `Arc` swapped in through an `ArcSwap`, decoded off-thread, with a
rule for when a half-loaded sample is allowed to sound — not at all, or the previous one until the new one is whole. Memory is
the other half: a bank of samples is the first thing here that can exhaust a machine, so the count and the length need limits
the way devices do. Decoding is a new dependency in `acid-core`'s clean dependency graph, which is a decision about where the
sampler lives, not a detail of its implementation.
