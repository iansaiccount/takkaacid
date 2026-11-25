# Plan 03 — duplicate a device

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** a `DUPLICATE` key on the device card makes a second machine of the same kind carrying everything the file would keep of
the first — bank, patch, kit, colour, routes, shuffle, learned controllers — under a name made distinct and a number never
reused. At four machines it refuses with the sentence the picker already says.

**Spec:** [`2026-11-25-rack-documents-design.md`](../../docs/superpowers/specs/2026-11-25-rack-documents-design.md), *Duplicate a
device*. This plan takes the two things that spec left to the builder: where the key sits, and the kit a copy arrives on.

## What is wrong today

- **A machine cannot be copied.** `Rack::add` builds a fresh one from `SharedState::for_kind` — the starter phrase, the launch
  patch, the default kit. To get a second of a patch you have built, the only route is to build it again: rename, retype the
  knob positions, copy the bank pattern by pattern.
- **The mapping already exists, in two halves.** `persist::saved_device_from(&Device)` writes what a device *is* for the file
  (`number`, `name`, `kind`, `scheme`, `inputs`, `bank`, `knobs`, `kit`, `swing`, `cc_map`) and `persist::saved_device_to_shared`
  reads it back, so a copy needs no third description of a machine.
- **The kit's samples are not in that mapping.** `apply_device` pours knobs, volume, shuffle and the learned rig; it does not
  decode a kit, which is `main`'s because it needs the running sample rate. A device built from a document arrives on the
  default kit and `main` decodes the named one after — which is what `wanted_kits` does for a load, and what a copy has to do too.
- **The card's key row is full.** `width` at the card's floor is 360 − 2·12 − 78 = **258 px**, and `MUTE` (66) + `REMOVE` (84) +
  `CLOSE` (80) plus two gaps costs **246** of it. A fourth key cannot join that row without shrinking a word the player reads.
- **`AddError` has one variant** (`AtCapacity`) and `RemoveError` has two (`LastDevice`, `NoSuchDevice`-equivalent wording), so a
  refusal with two reasons has a house shape to follow.

## The design

### The rack

```rust
// acid-app/src/device.rs
pub enum AddError {
    /// [`MAX_DEVICES`] is already holding devices.
    AtCapacity,
    /// There is no device at that index.
    NoSuchDevice,
}

impl Rack {
    /// Another machine of the source's kind, carrying everything the file would keep of it, under a name made distinct by the
    /// rule every named thing in this app uses. Numbered from the high-water mark, placed last, published; it does **not**
    /// take the selection — the panel asks, as it asks after `add`.
    pub fn duplicate(&mut self, index: usize) -> Result<usize, AddError>;
}
```

The copy is `saved_device_from(&source)` with `number` from the high-water mark and `name` through `persist::distinct_name`
against the rack's names, then the state from `saved_device_to_shared` on the **rack's** tempo cell, then `Device::adopted` with
the source's `name`, `scheme` and `inputs` — the four things `devices_for` also sets by hand. Names the rule gives: duplicating
`ACID 1` twice gives `ACID 1 (2)` then `ACID 1 (3)`; duplicating `ACID 1 (2)` gives `ACID 1 (2) (2)`. That is `distinct_name`,
and it is the rule on purpose.

`main` learns the kit to decode from the source's saved shape before it asks for the copy
(`persist::saved_device_from(&rack.devices()[index]).kit`) and decodes it after, through the existing
`load_kit_now(index, name, rate)`.

### The card

The device card grows **one key row**. `card_rect`'s height gains `ROW + GAP` for every card, `Rows` gains `duplicate`, and the
keys read:

```
[ MUTE OFF ]  [ REMOVE ]
[ DUPLICATE ]                                    [ CLOSE ]
```

`DUPLICATE` sits at the left of the second row, over `MUTE`, and `CLOSE` keeps the right end of the card — so the destructive key
and the key that ends the card are never neighbours. The new key asks `Request::Duplicate(index)` through
`device_request`, like every other key on the card; any click that asks something else disarms `REMOVE`, which the card's one
`if let Some(request) = asked` block already does for every variant it does not name.

`main`'s arm: `Ok(index)` → select it, `pattern_swap::follow`, open the card on the copy with its landed name, decode its kit,
and say `duplicated <name>`; `Err(AtCapacity)` → the picker's own sentence, `four devices is the limit`; `Err(NoSuchDevice)` →
nothing, because the panel was a frame behind a removal and the next frame redraws without it.

## Tests that make it safe

- **`tests/rack.rs`** — `duplicate` copies the bank, the patch, the kit's name, the colour, the shuffle, the learned rig and the
  routes, gives a fresh number above the high-water mark and a distinct name, and leaves the selection alone; the copy is the
  source `device for device` apart from number and name; duplicating a copy is `ACID 1 (2) (2)`; at four devices it is
  `AtCapacity` and the rack is unchanged; an index that is not there is `NoSuchDevice` and changes nothing.
- **`tests/duplicate_live.rs`** (new) — `DUPLICATE` on an open device card asks `Request::Duplicate(0)`; the key is drawn on the
  card at the design size and at the smallest window (`layout::scale` at its floor, the card at 360 px) with every key inside the
  card; the card's rows still sit inside the card after the extra row.
- **The book**: `DUPLICATE` is on the card's list of keys, and `check.sh` pins the word.

## Tasks

### Task 1 — the rack can copy a machine

**Files:** modify `rust/crates/acid-app/src/device.rs` (`AddError`, `Rack::duplicate`); test `rust/crates/acid-app/tests/rack.rs`.

**Interfaces:** produces `AddError::NoSuchDevice` and `Rack::duplicate(index) -> Result<usize, AddError>`; consumes
`persist::saved_device_from`, `persist::saved_device_to_shared`, `persist::distinct_name`, `Device::adopted`, `publish`.

- [ ] **Step 1** Write the failing tests above, driven from a rack built by `three_devices()` with its names, patch, kit,
      colour and routes changed on one machine.
- [ ] **Step 2** Run `cargo test -p acid-app --test rack` and watch them fail (`no method duplicate`).
- [ ] **Step 3** Implement `duplicate` and the new error variant; keep `add` untouched.
- [ ] **Step 4** Run them, then the workspace — `add`'s callers now have a second variant to answer, so `main`'s `Add` arm
      needs its `NoSuchDevice` case (it cannot be reached from `add`, and the match is what says so).
- [ ] **Step 5** Gates and commit — `-m "feat(rack): a machine can be copied"`.

### Task 2 — the key, its row, and `main`'s arm

**Files:** modify `rust/crates/acid-app/src/ui/device_card.rs`, `rust/crates/acid-app/src/device.rs` (`Request`),
`rust/crates/acid-app/src/main.rs`; test `rust/crates/acid-app/tests/duplicate_live.rs` (new).

**Interfaces:** produces `Rows::duplicate`, `Request::Duplicate(usize)`, the id `device-card-duplicate`; consumes Task 1's
`duplicate`, `load_kit_now`, `pattern_swap::follow`.

- [ ] **Step 1** Write the failing live tests: the card's second key row asks `Request::Duplicate(index)` for the device it was
      drawn for; every key is inside the card at the design size and at `scale` 0.65.
- [ ] **Step 2** Run them and watch them fail (no `duplicate` rect; the card is a row short).
- [ ] **Step 3** Implement: `Rows::duplicate`, the extra row in `card_rect`, the key and its tip, `Request::Duplicate`, and
      `main`'s arm as the design says.
- [ ] **Step 4** Run the test file, `tests/device_card_live.rs` and `tests/rack.rs`, then the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(panel): DUPLICATE makes a second of this machine"`.

### Task 3 — the book, the pin and the picture

**Files:** modify `docs/guide/src/19-a-devices-card.md`, `docs/guide/check.sh`,
`docs/guide/src/img/README.md`; retake `docs/guide/src/img/device-card.png`.

- [ ] **Step 1** Page 19's list of the card's keys gains `DUPLICATE`: what it copies, that the copy lands last with a `(2)` name,
      and that at four machines it refuses. The card's picture shows the new row.
- [ ] **Step 2** Pin `DUPLICATE` in `check.sh`'s fact list.
- [ ] **Step 3** Take `device-card.png` again: the display at least `2000 1400` before launching, the window `1960x1340`, a fresh
      `TAKKA_ACID_DATA_DIR`, the pointer parked away, and the card's two key rows visible. Record the row in `img/README.md`.
- [ ] **Step 4** `./docs/guide/check.sh` → `guide: ok`; gates; commit — `-m "docs: DUPLICATE, and what it copies"`.

## Non-goals

- A duplicate of a device **in another rack**, or of a rack: there is one rack.
- A shortcut key, a context menu, or a duplicate on the strip.
- Choosing what *not* to copy (the routes, the learned rig): the copy is the file's own description of a machine, whole.
- Sharing a bank between two devices: a copy owns its own, which is what the file already does.
- Carrying mute, `REVERSE` or transpose into the copy: performance, and a copy arrives as a launch builds one.

## Review focus

1. **One copy of "what a device is".** The copy goes through `saved_device_from` / `saved_device_to_shared`; a fourth hand-written
   field list is the bug this project has paid for twice.
2. **The kit's samples.** A copied drum machine must sound like its source, not like the default kit — Task 2's arm decodes the
   source's kit name, and a test that copies a `909` machine and asserts the copy's kit name is the cheapest proof.
3. **Numbers and names.** The copy's number is the high-water mark's, never a reused one; its name is `distinct_name`'s answer and
   the card shows that answer, not what was typed anywhere.
4. **The card's geometry at 360 px.** A fourth key on the existing row would overlap `CLOSE`; the second row is why it does not,
   and the live test asserts every key is inside the card at both sizes.
5. **A stale index moves nothing.** `NoSuchDevice` has no notice and no state change.

## Progress

*(One line per task, written as it is done: what landed, the test that proved it, and any ruling with what it costs if
wrong. The branch review's findings and what was done about each go here too, before the pull request.)*
