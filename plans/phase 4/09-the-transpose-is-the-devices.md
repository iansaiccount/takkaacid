# Plan 09 — the transpose is the device's, not the rack's

**Goal:** `TRANS -` / `TRANS +` move the machine on the panel by a semitone and leave every other device where it was. The
readout between the keys shows that device's number, the row's keys light from it, and the other machines keep playing at their
own transposes.

**Spec:** none. The player's words: *"the transpose should apply to the current device - not the whole rack."*

## What is wrong today

- **The number is the rack's, and the device's cell is only a mirror.** `device::Rack` carries `transpose: AtomicI32` with
  `Rack::transpose()` / `Rack::set_transpose()`, and `sync_transport` fans that number into every device's own atomic — the one
  the engine reads — on every transport change and every join. A second device therefore hears the first device's transpose,
  and a device added while the rack is up a semitone arrives transposed.
- **The panel already reads the device.** `row_transport` takes its readout value and its `TRANS -` / `TRANS +` litness from
  `shared.transpose()`, which is the *displayed* device's cell. So the row claims to show "the transpose" while the write behind
  it moves everything: one key, two meanings, and the meaning the row prints is not the one it does.
- **The words say rack.** `ui::tips`' two transpose tips and their doc comments say "the whole rack", and pages 4 and 18 of the
  book say it twice more ("the transpose pair moves the whole rack", "what the whole rack forgets together — … the rack's
  transpose").

## The design

- **The number lives on the device.** `Rack` loses its `transpose` field, `Rack::transpose()`, `Rack::set_transpose()` and the
  fan-out in `sync_transport`; `DeviceState`'s own `transpose` cell — already the thing the engine reads and the row prints —
  becomes the only one. `set_transpose` keeps its octave clamp. A device that joins starts at 0, which is the machine being a
  machine rather than the rack's mistake.
- **One writer, in the library.** `main`'s `Request::Transpose` arm moves into `transport::apply_transpose(rack, semitones)`, the
  way `apply_nudge` owns the tempo: the request is answered in one place, the clamp lives there, and the rule can be tested
  without a display. It writes the rack's **selected** device (`Rack::state()`, the same cell `draw_panel` is handed), so the
  row, the readout and the write cannot disagree about which machine is meant.
- **Nothing else moves.** The engine already reads each device's own cell (`renderer`'s params), `row_transport`'s readout and
  litness already read the displayed device, and the transpose is performance: no `SavedState` field, no `SAVE_VERSION` bump
  (still 11), and a session restart still forgets it — each device's own, now, rather than one for the rack.
- **The words say device.** The two transpose tips and their docs read "this device"; page 4 says the pair moves the device on
  the panel, and page 18's "what the rack forgets together" becomes two per-device things (`REVERSE` and the transpose, each
  the device's own). A `check.sh` pin keeps the new claim said.

## Tests that make it safe

- `rack.rs::a_device_carries_its_own_transpose` — replacing
  `the_transpose_clamps_to_an_octave_and_reaches_every_device`: a device's cell takes a value, clamps at ±12, and a second
  device's cell is untouched. Written against the devices, because there is no rack-wide number left to test.
- `transport::tests::the_transpose_moves_the_device_on_the_panel_and_no_other` — two devices, select the second,
  `apply_transpose(&rack, 3)`: the second is at 3 and the first at 0; then select the first and transpose to -20: it clamps to
  -12 and the second keeps its 3. The pure rule, no display.
- `transport_live::the_readout_shows_the_device_on_the_panel` — the paste-played row prints `+3` for a device whose cell says 3
  and `+0` after selecting the other one, so the row that shows a number shows the number its keys move.
- `mixer.rs` needs no change: it already feeds each machine its own device's `transpose()`.

Mutations: `apply_transpose` writing `rack.devices()` by index 0 instead of the selection; the clamp dropped; and the readout
reading something other than the displayed device — each must fail a named test.

## Tasks

### Task 1 — the cell and the writer

**Files:** `rust/crates/acid-app/src/device.rs` (the field, the two rack methods, `sync_transport`, the `DeviceState` docs),
`src/ui/transport.rs` (`apply_transpose`, `Request::Transpose`'s doc, a unit test), `src/main.rs` (the arm), `src/ui/row_transport.rs`
(the module doc and the pair's comment), `src/ui/tips.rs` (the two `Control` docs and their `TIPS` copy), `src/audio/renderer.rs`
(the two doc lines that call it the rack's), `tests/rack.rs`, `tests/transport_live.rs`.

**Interfaces:** produces `pub fn apply_transpose(rack: &crate::device::Rack, semitones: i32)`; removes
`Rack::transpose` / `Rack::set_transpose`; `DeviceState::transpose` / `set_transpose` unchanged in shape.

1. Write the failing tests: `rack.rs::a_device_carries_its_own_transpose`,
   `transport::tests::the_transpose_moves_the_device_on_the_panel_and_no_other`, and
   `transport_live::the_readout_shows_the_device_on_the_panel`. Run — expected FAIL (the first two do not compile against the
   new API; the live test fails on the second device's readout).
2. Remove the rack's field and methods, add `apply_transpose`, point `main` at it, and rewrite the docs and the two tips'
   words. Run — expected PASS.
3. Mutation check, recorded in Progress.
4. Commit.

### Task 2 — the book

**Files:** `docs/guide/src/04-the-face.md`, `18-a-rack-of-devices.md`, `docs/guide/check.sh`, `plans/phase 4/00-phase-4.md`.

1. Page 4: the transpose pair moves **the device on the panel**. Page 18: `REVERSE` and the transpose are each the device's
   own, so "what the whole rack forgets together" becomes what each device forgets for itself.
2. A `check.sh` pin for the device claim; keep the existing `TRANS -` pin. List the round in `00-phase-4.md` and run
   `./docs/guide/check.sh`.
3. Commit.

## Non-goals

A transpose in the save file (it is performance, like `REVERSE`); a MIDI CC for it; a per-device transpose in the strip or the
device card; changing the octave keys or the pattern-level octave transpose (`OCT -` / `OCT +`, which rewrites the notes and is
saved); the phase 2 and 3 plans that describe building the transpose rack-wide — they are the record of what those rounds did,
and the book is what must not lie.

## Rulings, and what they cost if wrong

| Ruling | Cost if wrong |
|---|---|
| The transpose is the selected device's, and a new device starts at 0 | The rack loses a single performance control a player might have wanted for a whole-set lift. If that is wanted back, it is a second request (`TransposeRack`) and a key or a modifier, not this round. |
| The write lives in `transport::apply_transpose`, not in `main` | One more function in the transport module for a two-line rule — and the rule is testable, which the arm in `main` was not. |
| `sync_transport` fans out the transport only | A device that joins mid-set no longer inherits a transpose; that is the point. If a late joiner should match the rack's *transport*, it still does. |
| The phase 2/3 plans keep their rack-wide prose | Those rounds did build it that way, and a plan is a record of decisions, not a description of today. The book, the tips and the code are what must not disagree with the app. |

## Progress

- [x] **Task 1 — the cell and the writer.** The transpose lives on the device now: `device::Rack` lost its `transpose` field,
      `Rack::transpose()` / `Rack::set_transpose()` and the fan-out in `sync_transport` (which carries the transport and nothing
      else), and `state::SharedState`'s own cell — the one the engine reads and the row prints — is the only one, still clamped
      to an octave. `transport::apply_transpose(rack, semitones)` is the one place a `Request::Transpose` writes, beside
      `apply_nudge`: it moves the rack's selected device, and `main`'s arm is one call to it. The row's readout and both
      keys' litness already read that cell, so the panel needed no change. Words follow: the `Request` doc, the row's module doc
      and the transpose pair's comment, the two `Control` docs and their tips ("Moves this device down a semitone … the other
      devices stay put …"), and the renderer's two doc lines that called it the rack's. The engine was already per device, so no
      audio change; the transpose is still in no `SavedState` field and `SAVE_VERSION` stays **11**.
      Tests: `rack::a_device_carries_its_own_transpose` (replacing `the_transpose_clamps_to_an_octave_and_reaches_every_device` —
      one device's cell takes a value, clamps, and a second's is untouched), `rack::a_transport_change_carries_no_transpose` (a
      play and a bare `sync_transport` leave both devices' transposes where they were), `transport::tests::the_transpose_moves_the_device_on_the_panel_and_no_other`
      (select the second, `apply_transpose(3)`: it is at 3 and the first at 0; select the first, `-20`: it clamps to -12 and the
      second keeps its 3) and `transport_live::the_readout_shows_the_device_on_the_panel` (the same panel over a device at +3 and
      one at +0 prints both numbers). `tests/mixer.rs` wrote the old `solo.set_transpose(5)`, so it now sets the device's own
      cell — the renderer's block and the machine's compare exactly as before. **Mutations: 4 of 4 caught**
      (`.superpowers/mutate_transpose.py`): the write landing on the first device, the cell's clamp dropped, the readout printing
      a constant, and the old fan-out put back. The last one **survived the first run** — nothing changed a device's transpose
      and then fanned out — so `a_transport_change_carries_no_transpose` was written to catch it.
      The branch review at the pull request has not happened yet (AGENTS.md: it is the branch, not the commit).
      Gates: `cargo test --workspace` **905**, clippy `-D warnings`, `fmt --check`.
- [x] **Task 2 — the book.** Page 4: `REVERSE` is this device's direction and the transpose pair moves **the device on the
      panel**, with the readout between the keys said to be that device's and a second machine said to keep its own. Page 18's
      «what the whole rack forgets together — each device's `REVERSE` direction and the rack's transpose» is now «two things each
      device forgets for itself — its `REVERSE` direction and its own transpose», with the consequence spelled out (one machine a
      semitone up never drags the others). `check.sh` pins `moves **the device on the panel**` and guards both retired sentences
      («moves the whole rack», «the whole rack forgets together»); `00-phase-4.md` lists 08 and 09 and its State line is brought
      up to **905** tests and eight rounds. **Mutations: 3 of 3 caught** (`.superpowers/mutate_transpose_book.py`): the rack-wide
      claim put back, the device claim dropped, and page 18's per-device sentence reverted. The third **survived the first
      run** — page 18's claim had no guard of its own — so `check.sh` gained the second negative grep, which is what caught it.
      No review was dispatched: the branch is reviewed at the pull request.
      Gates: `./docs/guide/check.sh` ok; the code gates are Task 1's and unchanged by this commit.


