# Plan 01 — the whole rack in the song

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of the
> branch at the pull request (`AGENTS.md`).

**Goal:** a song saves and loads **every** machine in the rack — kind, name, colour, ports, channel, role, patch, bank, kit,
swing and learned controllers — instead of the first device. `SAVE_VERSION` **12**.

**Spec:** [`2026-11-22-banks-racks-and-songs-design.md`](../../docs/superpowers/specs/2026-11-22-banks-racks-and-songs-design.md),
sections *The song document*, *Two seams the whole-rack round has to touch*, *5.1*. It also implements the wishlist's *Save the
whole rack* (`plans/future/wishlist.md`), unchanged from how that entry describes it.

## What is wrong today

- `persist::SavedState` (`SAVE_VERSION` 11) holds one bank and one patch under flat field names. `SAVE_VERSION` cannot say
  four devices exist.
- `main::save` and `main::session_for_disk` snapshot `rack.saved_device()` — the **first** device — and after a load
  `main::load_song_now` applies the file into `rack.device_mut()`, the **selected** one. So a save forgets devices 2–4 and a load
  writes over whichever device the player had selected. `persist::apply_saved` is a whole-session pour, not a device's.
- `Rack::from_saved` builds exactly one `Device`. `SavedState` carries no kind, so a drum session already comes back as a 303
  (`state.rs`: *"saves its first device whatever it is and loads it as a 303"*).
- `Device::name` and `Device::scheme` are memory-only, and so is `InputFilter`. Everything the strip shows about a machine past
  the first is lost on restart, and the `ADD DEVICE` picker says so in a footnote.
- `panel.songs.file` is not seeded on launch, so after a restart the first `SAVE` writes `Name (2)` beside the song it just
  opened: `save_target(None, typed)` is a new file by definition.

## The design

**The file.** `SavedState` keeps every field it has, in place, and appends `devices: Vec<SavedDevice>` last. The flat fields
become device 1's mirror: still written, still read by a v11 build, and kept identical to `devices[0]` by one function so the
two cannot disagree in a file this build writes.

```rust
pub struct SavedDevice {
    pub number: usize,
    pub name: String,
    pub kind: String,        // DeviceKind::name(): "ACID" | "DRUMS"
    pub scheme: String,      // Scheme::name(): "BLACK" | "VINYL NOIR" | ...
    pub inputs: SavedInputs, // Ports | Channel | Role, serde-derived in midi::route
    pub bank: PatternBank,
    pub knobs: Knobs,
    pub swing: f32,
    pub cc_map: Vec<u8>,
    pub kit: Option<String>,
}
```

**Catalogue entries by caption.** `DeviceKind::from_name` and `Scheme::from_name` scan their own `ALL`, so the words in the
file are the words on the face and there is no second list. `sanitize` repairs an unknown caption to the default (`ACID`,
`BLACK`) rather than refusing the file: a hand-edited caption must not cost the player their song.

**One mapping for the rack's bytes.** `persist::rack_to_saved(&Rack) -> SavedState` builds the whole document — `devices` from
every device, the mirror from `devices[0]` — and `main` stops snapshotting a device itself. `persist::saved_device_from(&Device)
-> SavedDevice` is the per-device half; `shared_to_saved(shared, bank)` stays as the one-device document the tests and
`new_song` already use.

**Loading replaces the rack.** `Rack::replace_from_saved(&SavedState)` rebuilds the device list on the rack's own tempo cell,
keeps the bus and the transport, selects slot 0, sets `next_number` above the highest number it read, and publishes once. The
panel's `pending_pattern`, `pattern_id` and an open device card reset against slot 0 in the same function's caller, because the
rack they pointed into is gone.

**Kits load per device, after the rack exists.** `main::kit_wanted` becomes a list of `(index, name)`, drained on the frame the
sample rate is known; a kit that will not decode is a notice and leaves `Init`, exactly as today.

**Ports are kept as written, and a missing one is said once.** After a load, `main` compares each device's `Ports::Named` with
`self.midi.enabled()` and leaves one notice naming the ports that are not open. Nothing is rewritten.

**A v11 file.** `devices` is empty, so `sanitize` builds one device from the mirror with **kind inferred**: `DRUMS` when `kit`
is `Some(_)` — only a machine with a kit ever writes a kit name — else `ACID`. A drum device that never left `Init` opens as an
acid; the file does not say, and nothing can guess.

**Ruling, and what it costs if wrong:** the mirror stays and `devices` is appended. If wrong, the file carries each machine twice
and `sanitize` has one rule to keep them in step; the alternative costs every existing song a migration and costs an older build
the ability to read a newer file at all.

## Tests that make it safe

In `rust/crates/acid-app/tests/persist.rs`, `tests/rack.rs` and `tests/songs_live.rs`:

- `the_save_version_is_pinned_at_twelve` — replaces `..._at_eleven`; the byte pin asserts `"version": 12`.
- `a_three_device_song_round_trips_every_field` — kind, name, scheme, ports, channel, role, patch, bank, swing, cc map and kit
  all come back; the flat mirror equals `devices[0]`.
- `a_v11_file_is_one_device_and_its_kit_names_the_kind` — a document with no `devices` and `kit: Some("808")` loads as one
  `DRUMS` device; the same document with `kit: None` loads as `ACID`.
- `sanitize_repairs_a_devices_list` — empty → one; five → four; duplicate numbers → renumbered; duplicate and blank names →
  distinct and defaulted; unknown `kind`/`scheme` captions → `ACID`/`BLACK`; `swing` clamped; a too-long `cc_map` → the factory
  pairing.
- `a_saved_rack_of_three_devices_opens_as_three` and `replacing_the_rack_publishes_in_order_and_selects_zero`
  (`tests/rack.rs`).
- `a_two_device_song_saved_through_the_card_comes_back_with_both` — through `draw_panel`, selecting the second device **before**
  the load: this is the bug the round fixes, so the test would fail on `main`'s old `device_mut()` path.
- `a_song_from_another_machine_loads_deaf_and_says_which_ports` — a device naming `No Such Port` loads, keeps the name, and
  leaves one notice.
- `a_restart_then_save_keeps_the_same_file` — seed `state.json` with `song: Some("Acid 1")`, open the panel, press `SAVE`,
  assert `songs/Acid 1.json` was rewritten and no `Acid 1 (2)` exists.

Mutations (a `SKIP` is never a pass): put back the first-device snapshot; remove the mirror rewrite; infer `ACID`
unconditionally; drop one field from `saved_device_from`; skip `publish` after a replace; drop the port notice.

## Tasks

### Task 1 — the device document

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`SavedDevice`, `SavedInputs`, `SAVE_VERSION`, `sanitize`, `upgraded`)
- Modify: `rust/crates/acid-app/src/device.rs` (`DeviceKind::from_name`, `Scheme::from_name`)
- Modify: `rust/crates/acid-app/src/midi/route.rs` (serde on `Ports`, `Channel`, `Role`)
- Test: `rust/crates/acid-app/tests/persist.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `persist::SavedDevice` and `persist::SavedInputs` as above; `DeviceKind::from_name(&str) -> Option<DeviceKind>`;
  `Scheme::from_name(&str) -> Option<Scheme>`; `persist::SAVE_VERSION == 12`.

1. Write `a_three_device_song_round_trips_every_field`, `a_v11_file_is_one_device_and_its_kit_names_the_kind` and
   `sanitize_repairs_a_devices_list`; run them — expected FAIL to compile (`SavedDevice` does not exist).
2. Add the two structs, the serde derives on the route types, the two `from_name` functions (`ALL.iter().find(|k| k.name() ==
   name)` — no second list), the field, and `SAVE_VERSION = 12`. `upgraded` gains its 12 arm: nothing to rewrite, the same
   comment the 9→10 bump carries.
3. Implement `sanitize`: rebuild from the mirror when empty (with the inferred kind), truncate to `MAX_DEVICES`, renumber
   duplicates above the highest, make names distinct via the rule `Rack::rename` already uses, default a blank name, repair the
   captions, clamp `swing`, repair each `cc_map`. Rewrite the mirror from `devices[0]` last, so one rule owns the equality.
4. Rename the version pin to `the_save_version_is_pinned_at_twelve`, update its byte assertion, and run the whole file — expected
   PASS.
5. Gates and commit.

### Task 2 — one builder for the rack's bytes

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`saved_device_from`, `saved_device_to_shared`, `rack_to_saved`, `sanitize`'s
  mirror)
- Test: `rust/crates/acid-app/tests/persist.rs`

**Interfaces:**
- Consumes: Task 1's types and `from_name`.
- Produces: `pub fn saved_device_from(device: &Device) -> SavedDevice`;
  `pub fn saved_device_to_shared(saved: &SavedDevice) -> Arc<SharedState>`;
  `pub fn rack_to_saved(rack: &Rack) -> SavedState`; `apply_saved(&SavedState, &SharedState, &mut PatternBank)` keeps its
  signature for one-device callers.

1. Write `a_rack_of_two_devices_snapshots_both_and_mirrors_the_first` — build a `Rack`, add a `DRUMS` device, name and recolour
   both, `rack_to_saved`, assert `devices.len() == 2`, the second's kind is `DRUMS`, and the flat `bank`/`knobs` equal
   `devices[0]`'s. Run — expected FAIL to compile.
2. Implement the three functions. `saved_device_to_shared` is `apply_saved`'s body moved to one device (knobs through `Knob::ALL`,
   `volume`, the cc map with the factory fill, `swing`, the pattern); `apply_saved` keeps the tempo so `new_song` and the demo
   path still pour a whole document.
3. Run the tests — expected PASS. Re-run `tests/persist.rs` and `tests/songs_live.rs` unchanged: the old one-device tests must
   still pass, which is what "the mirror is device 1" means in practice.
4. Gates and commit.

### Task 3 — a rack builds from a saved session, and is replaced by one

**Files:**
- Modify: `rust/crates/acid-app/src/device.rs` (`Rack::from_saved`, new `Rack::replace_from_saved`)
- Test: `rust/crates/acid-app/tests/rack.rs`

**Interfaces:**
- Consumes: `persist::saved_device_to_shared`, `persist::SavedState`.
- Produces: `Rack::from_saved(&SavedState) -> Rack` (N devices);
  `pub fn replace_from_saved(&mut self, saved: &SavedState)`; `Rack::selected()` is `0` after a replace.

1. Write `a_saved_rack_of_three_devices_opens_as_three` and
   `replacing_the_rack_publishes_in_order_selects_zero_and_keeps_the_tempo_cell`; run — expected FAIL.
2. Implement: `from_saved` maps every `SavedDevice` through `Device::adopted` (state from `saved_device_to_shared`, bank cloned,
   name, scheme, inputs), then `with_device` the first and push the rest; `replace_from_saved` rebuilds on `self.tempo_bpm`'s
   cell (`SharedState::with_tempo_for`), sets `next_number`, `set_tempo_bpm(saved.bpm)`, `select(0)`, `sync_transport()`,
   `publish()`.
3. Run the tests — expected PASS, and `tests/rack.rs`'s existing `a_rack_from_a_saved_session_is_the_session_it_was` must still
   pass with one device.
4. Gates and commit.

### Task 4 — `main` saves a rack and loads one

**Files:**
- Modify: `rust/crates/acid-app/src/main.rs` (`session_for_disk`, `save`, `write_song`, `load_song_now`, `kit_wanted`, the panel
  construction)
- Modify: `rust/crates/acid-app/src/ui/device_card.rs` (the footnote)
- Modify: `docs/guide/src/19-a-devices-card.md`
- Test: `rust/crates/acid-app/tests/songs_live.rs`

**Interfaces:**
- Consumes: `persist::rack_to_saved`, `Rack::replace_from_saved`.
- Produces: `fn session_for_disk(&self) -> SavedState` (no name argument) — the one builder `save`, `write_song` and the `EXPORT`
  arm call; `kit_wanted: Vec<(usize, String)>`.

1. Write the three live tests (`a_two_device_song_...`, `a_song_from_another_machine_...`, `a_restart_then_save_...`); run —
   expected FAIL.
2. Collapse `session_for_disk` and `save` onto `persist::rack_to_saved`, with `midi_input`, `song`, `tooltips`, `autosave`,
   `density` and `scale` filled from `main`/the panel as they are today; the `EXPORT` arm overrides `saved.song` with its
   suggested name.
3. `load_song_now`: `self.save()` first (unchanged rule), then `self.rack.replace_from_saved(&saved)`; reset
   `pending_pattern`, the open device card and `pattern_swap::adopt`; fill `kit_wanted` per device with `kind.has_kit()`; collect
   the missing ports into one notice; keep every panel field the function already restores.
4. Seed `panel.songs.file` from `saved.song` in the panel construction.
5. Delete `NOT_SAVED` from `ui/device_card.rs` and the sentence it feeds, and page 19's paragraph with it.
6. Run the tests — expected PASS. Gates and commit.

### Task 5 — the 808 default is stated

**Files:**
- Modify: `rust/crates/acid-app/src/kits.rs` (`DEFAULT_FACTORY_KIT`, `default_name`)
- Test: `rust/crates/acid-app/tests/kits.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `pub const DEFAULT_FACTORY_KIT: &str = "808"`; `default_name() -> &'static str` unchanged in signature.

1. Write `the_default_kit_is_the_808_by_name` — `default_name() == "808"`, the name is in `FACTORY`, and
   `factory_kit(default_name(), 44_100.0)` is `Some`. Run — expected PASS on today's table order (it is true by accident),
   which is exactly why the assertion is worth having.
2. Add the constant and return it. Run — expected PASS.
3. Mutation: change the constant to `"909"` and confirm the test fails; record it.
4. Gates and commit.

### Task 6 — the book

**Files:**
- Modify: `docs/guide/src/18-a-rack-of-devices.md`, `19-a-devices-card.md` (if Task 4's sentence is not the only one),
  `20-reference.md`
- Modify: `docs/guide/check.sh`
- Modify: `plans/phase 5/00-phase-5.md` (State)

1. Page 18: the restart paragraph says every device is saved, with its name, colour, ports and kit.
2. Page 20: the files table's `state.json` row loses "the first device only"; add the pin
   `grep -qF 'every machine in the rack'` (or the sentence the page actually uses — copy it, do not paraphrase).
3. Page 19: the footnote's absence, so the picker no longer claims the loss.
4. `./docs/guide/check.sh` — expected ok. Gates and commit.

## Non-goals

An arrangement; a rack document that loads; a bank file (round 02); refusing a file from a newer build; moving `MUTE`, `SOLO`, a
master fader or stems into the document; per-device audio outs; restoring the *selected* device across a load or a restart;
rewriting a v11 song on read (a load does not save; only a save writes the new shape).

## Review focus

Inputs the spec implies and no task's tests otherwise cover, most likely first:

- A **hand-edited** song with four devices sharing a name or a number: the strip must show four distinct captions and the rack
  must not index past its end.
- A device whose `kind` caption this build does not know: the song must open (as an acid) or refuse cleanly — never panic, never
  silently drop the other devices.
- A song loaded **while the device card is open** on device 3 of a one-device rack: the card must close or clamp.
- A device naming a port that is not open, on a session with no MIDI at all (`midi.enabled()` empty): the notice must say which
  port, and must not name every device's ports as missing when it simply has no ports.
- A v12 `state.json` opened by a v11 build: device 1 keeps its bank and patch, and the extra devices are dropped on that build's
  next save — the risk the spec records and this round does not fix.

## Progress

- [x] **Task 1 — the device document.** `SavedDevice`, `SavedInputs`, serde on `Ports` / `Channel` / `Role` (`midi::route`),
      `DeviceKind::from_name` and `Scheme::from_name` (each scanning its own `ALL`), `SAVE_VERSION` **12**, `devices` appended
      after `kit`, and `sanitize` split into `sanitize_knobs`, `sanitize_map` and `sanitize_devices` so the flat mirror and the
      list are repaired by the same code. The duplicate-name rule moved out of `Rack::rename` into `persist::distinct_name`,
      which `free_name` and `sanitize` now share: this task was about to write the third copy of it.
      **Tests:** `the_save_version_is_pinned_at_twelve` (renamed; its byte pin now proves `devices` follows `kit`),
      `a_three_device_song_round_trips_every_field` (the mirror is deliberately **stale**, so every rewrite line is
      load-bearing), `a_version_eleven_file_is_one_device_and_its_kit_names_the_kind`, `sanitize_repairs_a_devices_list`.
      **Mutations: 4 of 4 caught, 0 skipped** (`.superpowers/mutate_phase5_task1.py`) — the mirror not rewritten, the kind
      always inferred as an acid, no truncation, no distinct names. The first run **missed** the mirror mutation because that
      test's mirror happened to agree with device 1 already; the fixture was made stale on purpose and the mutation then bit.
      Recorded because a mutation that passes is a test that was not asking.
      Gates: `cargo test --workspace` **914**, clippy `-D warnings`, `cargo fmt --check`. No book change in this task: nothing
      a player sees moved, and the picker's footnote goes in Task 4 with the load path that makes it true.
- [x] **Task 2 — one builder for the rack's bytes.** `saved_device_from`, `saved_device_to_shared` and `rack_to_saved` join
      `shared_to_saved`, which stays as the one-device document; `apply_saved` splits into the rack half (tempo, bank) and
      `apply_device` (patch, shuffle, rig), with `publish_pattern` and `saved_fields` as the two rules both halves share. A
      document with no list is poured from its mirror, so every existing caller is unchanged.
      **Tests:** `a_rack_of_two_devices_snapshots_both_and_mirrors_the_first` — two devices, a shuffle and a learned
      controller set on device 1 so a dropped field is caught rather than defaulted.
      **Mutations: 6 of 6 caught, 0 skipped** (`.superpowers/mutate_phase5_task2.py`) — the list never written, the outgoing
      document not repaired, the kind always an acid, the shuffle dropped, the rig dropped, the colour dropped.
      **One harness fact worth keeping:** two lines with identical text (`}` alone) share one hashline anchor, so a range ending
      on a bare brace can resolve to the wrong one — it happened here and the block was repaired by hand. Target ranges by a
      distinctive line, not by `}`.
      Gates: `cargo test --workspace` **915**, clippy `-D warnings`, `cargo fmt --check`.
- [x] **Task 3 — a rack builds from a saved session, and is replaced by one.** `Rack::from_saved` builds every device the
      document carries; `Rack::replace_from_saved` rebuilds the list on **this** rack's tempo cell, transport and bus, selects
      slot 0 and republishes. `with_device` is gone; `devices_for` is the one place a device is built from a `SavedDevice`
      (with `SavedDevice::from_mirror` for a document `sanitize` has not seen, which is what `SavedState::default()` is), and
      `persist::saved_device_to_shared` now takes the tempo cell so N engines share one clock.
      **Tests:** `a_saved_rack_of_three_devices_opens_as_three` (names, kinds, colours, numbers, one tempo cell by `Arc::ptr_eq`,
      the bus published for three, the next number above the file's highest) and
      `replacing_the_rack_publishes_in_order_selects_zero_and_keeps_the_tempo_cell`.
      **Mutations: 6 of 6 caught, 0 skipped** (`.superpowers/mutate_phase5_task3.py`) — a clock per machine, the next number
      not taken from the file (twice: a load and a replace), the old selection kept, no publish, a replace on a new cell.
      **The first run missed one**, and it was the script's fault, not the test's: the counter mutation was paired with the
      *load* test while the line it changed lives in *replace*. The gap it exposed was real — the replace test never asked what
      number the next device gets — so an assertion was added and the mutation split in two. Recorded for the same reason as
      Task 1's miss.
      Gates: `cargo test --workspace` **917**, clippy `-D warnings`, `cargo fmt --check`.
- [ ] **Task 4 — `main` saves a rack and loads one.**
- [ ] **Task 5 — the 808 default is stated.**
- [ ] **Task 6 — the book.**
